use std::{collections::HashMap, path::Path, sync::Arc};

use ante::{
    diagnostics::{Diagnostic as AnteDiagnostic, DiagnosticKind},
    find_files::{self, CrateGraph},
    incremental::{CheckAll, Db, GetCrateGraph, SourceFile, TargetPointerSize},
    name_resolution::namespace::{CrateId, SourceFileId},
};

use dashmap::DashMap;
use futures::future::join_all;
use ropey::Rope;
use tower_lsp::lsp_types::*;

use crate::{util::byte_range_to_lsp_range, Backend};

/// One-time setup: pointer size + full crate graph scan (local files + stdlib).
/// `local_crate_root` is the workspace directory whose `src/` subtree holds the local crate
pub fn init_db(db: &mut Db, local_crate_root: &Path) {
    TargetPointerSize.set(db, 8);
    find_files::populate_crates_and_files(db, local_crate_root, &[]);
}

/// Incrementally update a single file's content. If this is the first time we've
/// seen this path, also register it with the local crate's `source_files` so
/// `CheckAll` will visit it so that diagnostics work for loose files
/// opened outside the workspace's `src/` tree.
pub fn set_file_content(db: &mut Db, local_crate_root: &Path, path: &Path, rope: &Rope) {
    let relative_path = SourceFileId::normalize_path(local_crate_root, path).to_path_buf();
    let file_id = SourceFileId::new(CrateId::LOCAL, &relative_path);
    file_id.set(db, Arc::new(SourceFile::new(Arc::new(path.to_path_buf()), rope.to_string())));

    let key = Arc::new(relative_path);
    if GetCrateGraph.get(db).get(&CrateId::LOCAL).is_some_and(|c| c.source_files.contains_key(&key)) {
        return;
    }

    // Mutate the graph without deep-cloning it: swap the Db's graph out for an empty
    // placeholder, leaving our local Arc as the sole strong reference. `make_mut` then
    // hands us `&mut CrateGraph` in place (no clone), and we put the modified Arc back.
    let mut graph_arc = GetCrateGraph.get(db);
    GetCrateGraph.set(db, Arc::new(CrateGraph::new()));
    let graph = Arc::make_mut(&mut graph_arc);
    if let Some(local_crate) = graph.get_mut(&CrateId::LOCAL) {
        local_crate.source_files.insert(key, file_id);
    }
    GetCrateGraph.set(db, graph_arc);
}

impl Backend {
    /// Update the compiler database with the latest in-memory file content, then
    /// collect and publish diagnostics for the local crate.
    pub(super) async fn update_diagnostics(&self, uri: Url, rope: &Rope) {
        let Ok(path) = uri.to_file_path() else {
            self.client.log_message(MessageType::ERROR, format!("Failed to convert URI to path: {uri}")).await;
            return;
        };

        let Some(root) = self.root() else {
            self.client.log_message(MessageType::ERROR, "update_diagnostics called before initialize()").await;
            return;
        };

        {
            let mut compiler = self.compiler.write().await;
            set_file_content(&mut compiler, root, &path, rope);
        }

        // Read phase: collect diagnostics without blocking writers unnecessarily.
        let lsp_diagnostics = {
            let compiler = self.compiler.read().await;
            collect_lsp_diagnostics(&compiler, &uri, rope, &self.document_map)
        };

        join_all(lsp_diagnostics.into_iter().map(|(u, d)| self.client.publish_diagnostics(u, d, None))).await;
    }
}

/// Pre-populate every local-crate file with an empty diagnostic list (so any file whose
/// errors were all fixed receives a publishDiagnostics call that clears stale squiggles),
/// then attach each accumulated compiler diagnostic to its source URI.
pub fn collect_lsp_diagnostics(
    compiler: &Db, current_uri: &Url, current_rope: &Rope, document_map: &DashMap<Url, Rope>,
) -> HashMap<Url, Vec<Diagnostic>> {
    let diagnostics = compiler.get_accumulated(CheckAll);
    let mut url_to_diagnostics = HashMap::<_, Vec<_>>::default();

    // Pre-populate with empty lists so that files whose errors were all fixed
    // get a publishDiagnostics call that clears the stale squiggles.
    let crates = GetCrateGraph.get(compiler);
    if let Some(local_crate) = crates.get(&CrateId::LOCAL) {
        for file_id in local_crate.source_files.values() {
            let source_file = file_id.get(compiler);
            if let Ok(uri) = Url::from_file_path(source_file.path.as_ref()) {
                url_to_diagnostics.entry(uri).or_default();
            }
        }
    }

    for diagnostic in diagnostics.iter() {
        if let Some((uri, lsp_diag)) = to_lsp_diagnostic(diagnostic, compiler, current_uri, current_rope, document_map)
        {
            url_to_diagnostics.entry(uri).or_default().push(lsp_diag);
        }
    }

    url_to_diagnostics
}

/// Convert a single compiler `Diagnostic` to an LSP `Diagnostic`, returning the
/// file URI it belongs to alongside it. Returns `None` if the location cannot be
/// mapped (e.g. the file path cannot be expressed as a URI).
fn to_lsp_diagnostic(
    diag: &AnteDiagnostic, compiler: &Db, current_uri: &Url, current_rope: &Rope, document_map: &DashMap<Url, Rope>,
) -> Option<(Url, Diagnostic)> {
    let loc = diag.location();
    let source_file = loc.file_id.get(compiler);

    let uri = Url::from_file_path(source_file.path.as_ref()).ok()?;

    let rope = rope_for_file(&uri, &source_file.contents, current_uri, current_rope, document_map);

    let range = byte_range_to_lsp_range(loc.span.start.byte_index, loc.span.end.byte_index, &rope).ok()?;

    let lsp_diag = Diagnostic {
        range,
        severity: Some(to_severity(diag.kind())),
        message: diag.message(),
        source: Some("ante-ls".to_string()),
        ..Default::default()
    };

    Some((uri, lsp_diag))
}

/// Return the in-memory rope for a file: the live rope for the file currently
/// being edited, a cached rope for other open files, or a rope built from the
/// on-disk content stored in the compiler database as a last resort.
pub fn rope_for_file(
    uri: &Url, disk_contents: &str, current_uri: &Url, current_rope: &Rope, document_map: &DashMap<Url, Rope>,
) -> Rope {
    if uri == current_uri {
        current_rope.clone()
    } else {
        document_map.get(uri).map(|r| r.clone()).unwrap_or_else(|| Rope::from_str(disk_contents))
    }
}

fn to_severity(kind: DiagnosticKind) -> DiagnosticSeverity {
    match kind {
        DiagnosticKind::Note => DiagnosticSeverity::HINT,
        DiagnosticKind::Warning => DiagnosticSeverity::WARNING,
        DiagnosticKind::Error => DiagnosticSeverity::ERROR,
    }
}
