use std::collections::BTreeMap;
use std::path::Path;

use ante::{
    incremental::{Db, ExportedDefinitions, ExportedTypes, GetCrateGraph},
    name_resolution::namespace::SourceFileId,
    parser::cst::Import,
    parser::ids::TopLevelName,
};
use tower_lsp::lsp_types::{CompletionItemKind, TextEdit};

use crate::util::byte_range_to_lsp_range;

#[derive(Copy, Clone, Debug)]
pub enum ItemKind {
    Function,
    Type,
    Module,
}

impl ItemKind {
    pub fn lsp_kind(self) -> CompletionItemKind {
        match self {
            ItemKind::Function => CompletionItemKind::VARIABLE,
            ItemKind::Type => CompletionItemKind::CLASS,
            ItemKind::Module => CompletionItemKind::MODULE,
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            ItemKind::Function => "fn",
            ItemKind::Type => "type",
            ItemKind::Module => "module",
        }
    }
}

#[derive(Clone)]
pub struct Candidate {
    pub crate_name: String,
    /// Module path as it appears in `crate.source_files`, e.g. `Vec.an`, `Bar/Baz.an`.
    pub module_path_with_ext: std::path::PathBuf,
    /// User-facing dotted form like `Std.Vec` or `Std.Bar.Baz` for the action title.
    pub dotted_display: String,
}

impl Candidate {
    pub fn new(crate_name: &str, module_path_with_ext: &Path) -> Self {
        let dotted_display = dotted_module(crate_name, module_path_with_ext);
        Self {
            crate_name: crate_name.to_string(),
            module_path_with_ext: module_path_with_ext.to_path_buf(),
            dotted_display,
        }
    }
}

/// Invoke `f` for every (crate_name, module_path, source_file_id) tuple in the
/// crate graph, skipping `current_file_id`. The crate graph is fetched once.
pub fn for_each_other_module<F>(compiler: &Db, current_file_id: SourceFileId, mut f: F)
where
    F: FnMut(&str, &Path, SourceFileId),
{
    let crates = GetCrateGraph.get(compiler);
    for (_crate_id, crate_) in crates.iter() {
        let crate_name = crate_.name.as_str();
        for (module_path_with_ext, source_file_id) in crate_.source_files.iter() {
            if *source_file_id == current_file_id {
                continue;
            }
            f(crate_name, module_path_with_ext.as_path(), *source_file_id);
        }
    }
}

/// Reverse-lookup the crate and module path for a `SourceFileId`. Used to turn
/// a compiler-emitted `ImportSuggestion` (which only carries a `Location`) into
/// the `Candidate` shape that `build_import_edit` consumes. Returns `None` if
/// the file is not owned by any crate.
pub fn candidate_for_file(compiler: &Db, file_id: SourceFileId) -> Option<Candidate> {
    let crates = GetCrateGraph.get(compiler);
    let crate_ = crates.get(&file_id.crate_id)?;
    for (module_path_with_ext, source_file_id) in crate_.source_files.iter() {
        if *source_file_id == file_id {
            return Some(Candidate::new(&crate_.name, module_path_with_ext.as_path()));
        }
    }
    None
}

/// Iterate every (name, top-level-name, kind) exported by `source_file_id`.
/// Defs (functions) come first, then types.
pub fn for_each_export<F>(compiler: &Db, source_file_id: SourceFileId, mut f: F)
where
    F: FnMut(&str, &TopLevelName, ItemKind),
{
    let exported_defs = ExportedDefinitions(source_file_id).get(compiler);
    for (name, top_level_name) in exported_defs.definitions.iter() {
        f(name.as_str(), top_level_name, ItemKind::Function);
    }
    let exported_types = ExportedTypes(source_file_id).get(compiler);
    for (name, (top_level_name, _kind)) in exported_types.iter() {
        f(name.as_str(), top_level_name, ItemKind::Type);
    }
}

/// Build an index from exported name to the modules that export it, in one
/// pass over the crate graph. Each module appears at most once per name, even
/// if it exports both a function and a type with that name.
pub fn exports_index(compiler: &Db, current_file_id: SourceFileId) -> BTreeMap<String, Vec<Candidate>> {
    let mut index: BTreeMap<String, Vec<Candidate>> = BTreeMap::new();
    for_each_other_module(compiler, current_file_id, |crate_name, module_path, source_file_id| {
        let mut cand: Option<Candidate> = None;
        for_each_export(compiler, source_file_id, |name, _, _| {
            let entry = index.entry(name.to_string()).or_default();
            // Defs are visited before types for any one module, and the same
            // (crate, module) is processed contiguously, so the last entry
            // for this name is the most recent push. If it's us, we already
            // covered this (module, name) pair.
            let already = entry
                .last()
                .is_some_and(|c| c.crate_name == crate_name && c.module_path_with_ext.as_path() == module_path);
            if already {
                return;
            }
            let cand_ref = cand.get_or_insert_with(|| Candidate::new(crate_name, module_path));
            entry.push(cand_ref.clone());
        });
    });
    index
}

fn dotted_module(crate_name: &str, module_path_with_ext: &Path) -> String {
    let mut s = String::from(crate_name);
    let stripped = module_path_with_ext.with_extension("");
    for comp in stripped.components() {
        s.push('.');
        s.push_str(&comp.as_os_str().to_string_lossy());
    }
    s
}

/// Choose where and what to insert to bring `cand` into scope. With
/// `name = Some(n)`, this imports the item `n` from the module: if a matching
/// `import` line exists it's extended in place, otherwise a fresh line is
/// inserted. With `name = None`, the module itself is imported as a namespace,
/// short-circuiting if any line for the (crate, module) pair already exists.
pub fn build_import_edit(
    name: Option<&str>, cand: &Candidate, existing_imports: &[Import], rope: &ropey::Rope,
) -> Option<TextEdit> {
    let candidate_module_no_ext = cand.module_path_with_ext.with_extension("");

    for import in existing_imports {
        if import.crate_name != cand.crate_name {
            continue;
        }
        if *import.module_path != candidate_module_no_ext {
            continue;
        }
        // Module-only case: any matching import means the module is
        // already in scope.
        let name = name?;
        // A bare `import Std.Vec` parses with an empty module_path and the
        // module name living in `items`. Adding more items there would change
        // its meaning entirely (from importing the namespace to importing
        // individual names from a submodule named after the first item), so
        // we skip that case and let the new-line branch handle it instead.
        if import.module_path.as_os_str().is_empty() {
            continue;
        }
        if import.items.iter().any(|(item, _)| item == name) {
            return None;
        }
        let (insert_byte, insert_text) = if let Some((_, last_loc)) = import.items.last() {
            (last_loc.span.end.byte_index, format!(", {name}"))
        } else {
            (import.location.span.end.byte_index, format!(".{name}"))
        };
        let range = byte_range_to_lsp_range(insert_byte, insert_byte, rope).ok()?;
        return Some(TextEdit { range, new_text: insert_text });
    }

    let new_line = match name {
        Some(n) => format!("import {}.{n}", cand.dotted_display),
        None => format!("import {}", cand.dotted_display),
    };
    if let Some(last) = existing_imports.last() {
        let insert_byte = last.location.span.end.byte_index;
        let range = byte_range_to_lsp_range(insert_byte, insert_byte, rope).ok()?;
        Some(TextEdit { range, new_text: format!("\n{new_line}") })
    } else {
        let range = byte_range_to_lsp_range(0, 0, rope).ok()?;
        Some(TextEdit { range, new_text: format!("{new_line}\n") })
    }
}
