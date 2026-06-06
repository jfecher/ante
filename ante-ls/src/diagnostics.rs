use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use ante::{
    diagnostics::{Diagnostic as AnteDiagnostic, DiagnosticKind},
    find_files::{self, CrateGraph},
    incremental::{CheckAll, Db, GetCrateGraph, SourceFile, TargetPointerSize},
    name_resolution::namespace::{CrateId, SourceFileId},
};

use dashmap::DashMap;
use futures::future::join_all;
use ropey::Rope;
use tokio::sync::{Notify, RwLock};
use tower_lsp::{lsp_types::*, Client};

use crate::util::byte_range_to_lsp_range;

/// One-time setup: pointer size + full crate graph scan (local files + stdlib).
/// `local_crate_root` is the workspace directory whose `src/` subtree holds the local crate
pub fn init_db(db: &mut Db, local_crate_root: &Path) {
    TargetPointerSize.set(db, 8);
    find_files::populate_crates_and_files(db, local_crate_root, &[]);
}

/// Incrementally update a single file's content. The file is matched against the
/// crate graph so that e.g. editing a stdlib file updates the stdlib crate's existing
/// `SourceFileId` in place - registering it under the local crate instead would leave
/// the real module frozen at its on-disk contents and compile the file twice. If this
/// is the first time we've seen this path, also register it with the owning crate's
/// `source_files` so `CheckAll` will visit it so that diagnostics work for loose files
/// opened outside the workspace's `src/` tree.
pub fn set_file_content(db: &mut Db, roots: &CrateRoots, path: &Path, rope: &Rope) {
    let (crate_id, relative_path, canonical_path) = roots.find_owning_crate(path);
    let file_id = SourceFileId::new(crate_id, &relative_path);
    // Store the canonical path, matching the initial scan in `find_files`, so the same
    // file always maps to a single URI regardless of how the editor spelled the path.
    file_id.set(db, Arc::new(SourceFile::new(Arc::new(canonical_path), rope.to_string())));

    let key = Arc::new(relative_path);
    if GetCrateGraph.get(db).get(&crate_id).is_some_and(|c| c.source_files.contains_key(&key)) {
        return;
    }

    // Mutate the graph without deep-cloning it: swap the Db's graph out for an empty
    // placeholder, leaving our local Arc as the sole strong reference. `make_mut` then
    // hands us `&mut CrateGraph` in place (no clone), and we put the modified Arc back.
    let mut graph_arc = GetCrateGraph.get(db);
    GetCrateGraph.set(db, Arc::new(CrateGraph::new()));
    let graph = Arc::make_mut(&mut graph_arc);
    if let Some(crate_) = graph.get_mut(&crate_id) {
        crate_.source_files.insert(key, file_id);
    }
    GetCrateGraph.set(db, graph_arc);
}

/// The [SourceFileId] for the given file path, matched against the crate graph the
/// same way [set_file_content] stores content, so lookups and updates agree on ids.
pub fn file_id_for_path(roots: &CrateRoots, path: &Path) -> SourceFileId {
    let (crate_id, relative_path, _) = roots.find_owning_crate(path);
    SourceFileId::new(crate_id, &relative_path)
}

/// The canonicalized `src/` root of every crate in the graph, computed once after
/// [init_db] (the graph never gains crates afterwards) so that matching a file path
/// to its owning crate doesn't have to re-canonicalize on every edit and request.
pub struct CrateRoots {
    /// The workspace directory whose `src/` subtree holds the local crate.
    pub local_crate_root: PathBuf,
    /// Canonical `<crate>/src` directory of each crate, in crate-id order
    /// (stdlib first, then the local crate, then dependencies).
    src_roots: Vec<(CrateId, PathBuf)>,
}

impl CrateRoots {
    pub fn new(db: &Db, local_crate_root: PathBuf) -> CrateRoots {
        let src_roots = GetCrateGraph
            .get(db)
            .iter()
            .map(|(crate_id, crate_)| {
                let src_root = crate_.path.join("src");
                let src_root = src_root.canonicalize().unwrap_or(src_root);
                (*crate_id, src_root)
            })
            .collect();
        CrateRoots { local_crate_root, src_roots }
    }

    /// Find the crate whose `src/` tree contains `path`, returning its id, the path
    /// relative to that `src/` directory (the form `SourceFileId`s are keyed by), and
    /// the canonical full path (the form `SourceFile`s store). Falls back to the local
    /// crate for loose files outside every crate's `src/` tree.
    fn find_owning_crate(&self, path: &Path) -> (CrateId, PathBuf, PathBuf) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        for (crate_id, src_root) in &self.src_roots {
            if let Ok(relative) = canonical.strip_prefix(src_root) {
                return (*crate_id, relative.to_path_buf(), canonical);
            }
        }

        let relative = SourceFileId::normalize_path(&self.local_crate_root, path).to_path_buf();
        (CrateId::LOCAL, relative, canonical)
    }
}

/// How long the diagnostics worker waits for the edit stream to go quiet before
/// compiling, so that bursts of keystrokes coalesce into a single `CheckAll`.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Upper bound on how long continuous typing can postpone compilation: diagnostics
/// still refresh at least this often while edits keep arriving.
const MAX_WAIT: Duration = Duration::from_secs(1);

/// The set of documents edited since the last compile, plus a wakeup signal for
/// the [DiagnosticsWorker]. Notification handlers call [DirtyDocs::mark] synchronously.
#[derive(Default)]
pub struct DirtyDocs {
    docs: Mutex<HashSet<Url>>,
    notify: Notify,
}

impl DirtyDocs {
    pub fn mark(&self, uri: Url) {
        self.docs.lock().unwrap().insert(uri);
        self.notify.notify_one();
    }

    fn drain(&self) -> Vec<Url> {
        self.docs.lock().unwrap().drain().collect()
    }
}

/// Single-flight diagnostics worker. All compiler content updates and `CheckAll` runs
/// funnel through this one task, so each compile sees the freshest rope of every dirty
/// document and diagnostics are always published in compile order - notification
/// handlers themselves only mark documents dirty and return. Spawned in `initialize`.
pub struct DiagnosticsWorker {
    pub client: Client,
    pub document_map: Arc<DashMap<Url, Rope>>,
    pub compiler: Arc<RwLock<Db>>,
    pub dirty: Arc<DirtyDocs>,
    pub crate_roots: Arc<CrateRoots>,
}

impl DiagnosticsWorker {
    pub async fn run(self) {
        // URIs whose last publish was non-empty, so clears are only sent where needed.
        let mut published = HashSet::new();
        loop {
            self.dirty.notify.notified().await;

            // Trailing-edge debounce: re-arm on every new edit until the stream has been
            // quiet for DEBOUNCE, but never postpone the compile past MAX_WAIT. Awaiting
            // `notified()` here also consumes permits stored by edits during the wait, so
            // they don't wake the next iteration spuriously.
            let deadline = tokio::time::Instant::now() + MAX_WAIT;
            loop {
                let rearm = tokio::time::timeout(DEBOUNCE, self.dirty.notify.notified());
                match tokio::time::timeout_at(deadline, rearm).await {
                    Ok(Ok(())) => continue, // another edit arrived; restart the quiet timer
                    _ => break,             // quiet for DEBOUNCE, or MAX_WAIT exceeded
                }
            }

            self.update_diagnostics(&mut published).await;
        }
    }

    /// Update the compiler database with the latest in-memory content of every dirty
    /// document, then collect and publish diagnostics for the whole crate graph.
    async fn update_diagnostics(&self, published: &mut HashSet<Url>) {
        let mut changed = Vec::new();
        for uri in self.dirty.drain() {
            let Ok(path) = uri.to_file_path() else {
                self.client.log_message(MessageType::ERROR, format!("Failed to convert URI to path: {uri}")).await;
                continue;
            };
            // Snapshot the rope only now, after the debounce, so the freshest content wins.
            let rope = match self.document_map.get(&uri).map(|rope| rope.clone()) {
                Some(rope) => rope,
                // The document was closed: its truth reverts to the on-disk contents.
                None => match tokio::fs::read_to_string(&path).await {
                    Ok(contents) => Rope::from_str(&contents),
                    Err(_) => continue, // e.g. an unsaved buffer that never existed on disk
                },
            };
            changed.push((path, rope));
        }

        if changed.is_empty() {
            return;
        }

        {
            let mut compiler = self.compiler.write().await;
            for (path, rope) in &changed {
                set_file_content(&mut compiler, &self.crate_roots, path, rope);
            }
        }

        // Read phase: collect diagnostics without blocking writers unnecessarily.
        let mut lsp_diagnostics = {
            let compiler = self.compiler.read().await;
            collect_lsp_diagnostics(&compiler, &self.document_map)
        };

        // Publish empty lists for files whose diagnostics were all fixed so their stale
        // squiggles are cleared, and remember who has diagnostics now for next time.
        let now_published: HashSet<Url> = lsp_diagnostics.keys().cloned().collect();
        for uri in published.iter() {
            if !lsp_diagnostics.contains_key(uri) {
                lsp_diagnostics.insert(uri.clone(), Vec::new());
            }
        }
        *published = now_published;

        join_all(lsp_diagnostics.into_iter().map(|(u, d)| self.client.publish_diagnostics(u, d, None))).await;
    }
}

/// Attach each accumulated compiler diagnostic to its source URI. Only files that
/// currently have diagnostics appear in the result; the [DiagnosticsWorker] remembers
/// which URIs it published previously and sends empty lists to clear files whose
/// diagnostics were all fixed.
pub fn collect_lsp_diagnostics(compiler: &Db, document_map: &DashMap<Url, Rope>) -> HashMap<Url, Vec<Diagnostic>> {
    let diagnostics = compiler.get_accumulated(CheckAll);
    let mut url_to_diagnostics = HashMap::<_, Vec<_>>::default();
    let mut rope_cache = HashMap::new();

    for diagnostic in diagnostics.iter() {
        if let Some((uri, lsp_diag)) = to_lsp_diagnostic(diagnostic, compiler, document_map, &mut rope_cache) {
            url_to_diagnostics.entry(uri).or_default().push(lsp_diag);
        }
    }

    url_to_diagnostics
}

/// Convert a single compiler `Diagnostic` to an LSP `Diagnostic`, returning the
/// file URI it belongs to alongside it. Returns `None` if the location cannot be
/// mapped (e.g. the file path cannot be expressed as a URI).
fn to_lsp_diagnostic(
    diag: &AnteDiagnostic, compiler: &Db, document_map: &DashMap<Url, Rope>, rope_cache: &mut HashMap<Url, Rope>,
) -> Option<(Url, Diagnostic)> {
    let loc = diag.location();
    let source_file = loc.file_id.get(compiler);

    let uri = Url::from_file_path(source_file.path.as_ref()).ok()?;

    // Building a rope for an unopened file is O(file size), so share it across the
    // (possibly many) diagnostics pointing into the same file.
    let rope =
        rope_cache.entry(uri.clone()).or_insert_with(|| rope_for_file(&uri, &source_file.contents, document_map));

    let range = byte_range_to_lsp_range(loc.span.start.byte_index, loc.span.end.byte_index, rope).ok()?;

    let lsp_diag = Diagnostic {
        range,
        severity: Some(to_severity(diag.kind())),
        message: diag.message(),
        source: Some("ante-ls".to_string()),
        ..Default::default()
    };

    Some((uri, lsp_diag))
}

/// Return the in-memory rope for a file: the live rope for any open file, or a rope
/// built from the on-disk content stored in the compiler database as a last resort.
pub fn rope_for_file(uri: &Url, disk_contents: &str, document_map: &DashMap<Url, Rope>) -> Rope {
    document_map.get(uri).map(|r| r.clone()).unwrap_or_else(|| Rope::from_str(disk_contents))
}

fn to_severity(kind: DiagnosticKind) -> DiagnosticSeverity {
    match kind {
        DiagnosticKind::Note => DiagnosticSeverity::HINT,
        DiagnosticKind::Warning => DiagnosticSeverity::WARNING,
        DiagnosticKind::Error => DiagnosticSeverity::ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante::paths::stdlib_path;

    /// Regression test: editing a stdlib file must update the stdlib crate's existing
    /// `SourceFileId` in place rather than registering a duplicate file under the local
    /// crate. The duplicate left the real stdlib module frozen at its on-disk contents,
    /// producing stale diagnostics that could only be cleared by restarting the server.
    #[test]
    fn editing_a_stdlib_file_updates_the_stdlib_source_file_in_place() {
        let ante_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("ante-ls must live inside the ante workspace")
            .to_path_buf();

        let mut db = Db::default();
        init_db(&mut db, &ante_root);
        let roots = CrateRoots::new(&db, ante_root);

        let seq_path = stdlib_path().join("src").join("Seq.an");
        let new_content = "// edited in memory\n";
        set_file_content(&mut db, &roots, &seq_path, &Rope::from_str(new_content));

        let stdlib_id = SourceFileId::new(CrateId::STDLIB, Path::new("Seq.an"));
        assert_eq!(stdlib_id.get(&db).contents, new_content);

        let graph = GetCrateGraph.get(&db);
        let local_has_clone = graph[&CrateId::LOCAL].source_files.keys().any(|key| key.ends_with("Seq.an"));
        assert!(!local_has_clone, "stdlib file must not be registered as a local-crate clone");
    }
}
