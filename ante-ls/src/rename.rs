//! Symbol rename and find-references.
//!
//! The compiler has no reverse reference index, so references are found by scanning
//! every top-level item's origin maps for the target. The type checker's extended
//! context is used since it also covers dot-syntax method calls and other origins
//! resolved during type inference.
//!
//! Known limitations:
//! - Type qualifiers inside paths (e.g. `Vec` in `Vec.push v x`) are not rewritten
//!   when renaming the type. They are rewritten inside export statements however.
//! - Renaming modules is not implemented.
//! - Mentions in comments or strings are not rewritten.

use std::collections::{BTreeMap, HashMap};

use ante::diagnostics::Location as AnteLocation;
use ante::incremental::{
    AllDefinitions, AllTypes, Db, ExportedDefinitions, ExportedTypes, GetCrateGraph, GetItem, Parse, Resolve,
    TypeCheck, VisibleDefinitionsResult,
};
use ante::name_resolution::{
    namespace::{CrateId, SourceFileId},
    Origin, ResolutionResult,
};
use ante::parser::ids::{IdStore, NameId, NameStore, PathId, TopLevelId, TopLevelName};

use dashmap::DashMap;
use ropey::Rope;
use tower_lsp::lsp_types::{Location as LspLocation, TextEdit, Url, WorkspaceEdit};

use crate::diagnostics::rope_for_file;
use crate::util::{byte_range_to_lsp_range, SpanSearcher};

/// The definition the symbol under the cursor refers to
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct RenameTarget {
    item: TopLevelId,
    name_id: NameId,
    is_top_level: bool,
}

impl RenameTarget {
    pub fn local(item: TopLevelId, name_id: NameId) -> RenameTarget {
        RenameTarget { item, name_id, is_top_level: false }
    }

    pub fn top_level(name: TopLevelName) -> RenameTarget {
        RenameTarget { item: name.top_level_item, name_id: name.local_name_id, is_top_level: true }
    }

    fn top_level_name(self) -> TopLevelName {
        TopLevelName { top_level_item: self.item, local_name_id: self.name_id }
    }
}

#[derive(Debug)]
pub enum RenameError {
    NoSymbol,
    ExternalCrate,
    InvalidName(String),
}

pub struct SymbolAtCursor {
    pub target: RenameTarget,
    pub cursor_span: (usize, usize),
    pub old_name: String,
}

impl SymbolAtCursor {
    /// Only symbols defined in the local crate may be renamed
    pub fn is_renameable(&self) -> bool {
        self.target.item.source_file.crate_id == CrateId::LOCAL
    }
}

/// Resolve the symbol under `byte_offset`, keeping its `Origin` so callers can
/// find every other reference to it. Also handles import items and export list
/// entries, which are absent from the origin maps.
pub fn symbol_at(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Result<SymbolAtCursor, RenameError> {
    match symbol_at_inner(compiler, file_id, byte_offset) {
        // Editors commonly leave the caret just past the identifier
        Err(RenameError::NoSymbol) if byte_offset > 0 => symbol_at_inner(compiler, file_id, byte_offset - 1),
        other => other,
    }
}

fn symbol_at_inner(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Result<SymbolAtCursor, RenameError> {
    enum Hit {
        Name(NameId),
        Path(PathId),
    }

    let parse = Parse(file_id).get(compiler);

    let mut searcher = SpanSearcher::new(byte_offset);
    let mut best: Option<(TopLevelId, Hit)> = None;

    for item in &parse.cst.top_level_items {
        let (_, ctx) = GetItem(item.id).get(compiler);
        for (name_id, loc) in ctx.name_locations() {
            if searcher.try_offer(loc.span.start.byte_index, loc.span.end.byte_index) {
                best = Some((item.id, Hit::Name(name_id)));
            }
        }
        for (path_id, _) in ctx.path_locations() {
            let (_, last_loc) = ctx.get_path(path_id).components.last().unwrap();
            if searcher.try_offer(last_loc.span.start.byte_index, last_loc.span.end.byte_index) {
                best = Some((item.id, Hit::Path(path_id)));
            }
        }
    }

    let Some((item_id, hit)) = best else {
        return symbol_at_import_or_export(compiler, file_id, byte_offset);
    };
    let (_, ctx) = GetItem(item_id).get(compiler);
    let resolve = Resolve(item_id).get(compiler);

    let (origin, cursor_span, old_name) = match hit {
        Hit::Name(name_id) => {
            let origin = origin_with_type_fallback(resolve.name_origins.get(&name_id).copied(), || {
                TypeCheck(item_id).get(compiler).result.context.name_origin(name_id)
            });
            let loc = ctx.name_location(name_id);
            (origin, byte_span(loc), ctx.get_name(name_id).as_str().to_owned())
        },
        Hit::Path(path_id) => {
            let origin = origin_with_type_fallback(resolve.path_origins.get(&path_id).copied(), || {
                TypeCheck(item_id).get(compiler).result.context.path_origin(path_id)
            });
            let (name, loc) = ctx.get_path(path_id).components.last().unwrap();
            (origin, byte_span(loc), name.clone())
        },
    };

    let target = match origin {
        Some(Origin::Local(name_id)) => RenameTarget::local(item_id, name_id),
        Some(Origin::TopLevelDefinition(name)) => RenameTarget::top_level(name),
        Some(Origin::Builtin(_)) => return Err(RenameError::ExternalCrate),
        Some(Origin::TypeResolution) | None => return Err(RenameError::NoSymbol),
    };

    Ok(SymbolAtCursor { target, cursor_span, old_name })
}

/// Paths with a type-directed resolution get their final origin from the type checker
fn origin_with_type_fallback(resolved: Option<Origin>, fallback: impl FnOnce() -> Option<Origin>) -> Option<Origin> {
    match resolved {
        Some(Origin::TypeResolution) | None => fallback(),
        other => other,
    }
}

fn byte_span(loc: &AnteLocation) -> (usize, usize) {
    (loc.span.start.byte_index, loc.span.end.byte_index)
}

/// Import items and export list entries are matched by string in the compiler, so
/// resolve a cursor on one of them through the target file's exports instead.
/// TODO: Expose an id for these in the cst repr to remove the string match
fn symbol_at_import_or_export(
    compiler: &Db, file_id: SourceFileId, byte_offset: usize,
) -> Result<SymbolAtCursor, RenameError> {
    let parse = Parse(file_id).get(compiler);

    for import in &parse.cst.imports {
        for (name, loc) in &import.items {
            if !contains(loc, byte_offset) {
                continue;
            }
            let crates = GetCrateGraph.get(compiler);
            let target_file =
                import_target_file(&crates, &import.crate_name, &import.module_path).ok_or(RenameError::NoSymbol)?;
            let target = lookup_export(compiler, target_file, name).ok_or(RenameError::NoSymbol)?;
            return Ok(SymbolAtCursor {
                target: RenameTarget::top_level(target),
                cursor_span: byte_span(loc),
                old_name: name.as_str().to_owned(),
            });
        }
    }

    for entry in parse.cst.exports.iter().flatten() {
        if let Some((qualifier, qualifier_loc)) = &entry.qualifier {
            if contains(qualifier_loc, byte_offset) {
                let all = AllDefinitions(file_id).get(compiler);
                let target = all.definitions.get(qualifier).copied().ok_or(RenameError::NoSymbol)?;
                return Ok(SymbolAtCursor {
                    target: RenameTarget::top_level(target),
                    cursor_span: byte_span(qualifier_loc),
                    old_name: qualifier.as_str().to_owned(),
                });
            }
        }
        if contains(&entry.location, byte_offset) {
            let all = AllDefinitions(file_id).get(compiler);
            let target = resolve_export_entry(compiler, file_id, &all, entry).ok_or(RenameError::NoSymbol)?;
            return Ok(SymbolAtCursor {
                target: RenameTarget::top_level(target),
                cursor_span: byte_span(&entry.location),
                old_name: entry.name.as_str().to_owned(),
            });
        }
    }

    Err(RenameError::NoSymbol)
}

fn contains(loc: &AnteLocation, byte_offset: usize) -> bool {
    loc.span.start.byte_index <= byte_offset && byte_offset < loc.span.end.byte_index
}

fn import_target_file(
    crates: &ante::find_files::CrateGraph, crate_name: &str, module_path: &std::path::Path,
) -> Option<SourceFileId> {
    let module_file = module_path.with_extension("an");
    let (_, crate_) = crates.iter().find(|(_, crate_)| crate_.name == crate_name)?;
    crate_.source_files.get(&module_file).copied()
}

/// Every exported definition of `file_id` with the given name. Same-named methods
/// on different types are distinct exports, so a bare name can match several.
fn exported_targets_with_name(compiler: &Db, file_id: SourceFileId, name: &String) -> Vec<TopLevelName> {
    let exported = ExportedDefinitions(file_id).get(compiler);
    let mut targets = Vec::new();
    targets.extend(exported.definitions.get(name).copied());
    for methods in exported.methods.values() {
        targets.extend(methods.get(name).copied());
    }
    targets.extend(ExportedTypes(file_id).get(compiler).get(name).copied());

    // Exported types appear in both the definitions and types maps
    targets.sort_unstable();
    targets.dedup();
    targets
}

fn lookup_export(compiler: &Db, file_id: SourceFileId, name: &ante::parser::cst::Name) -> Option<TopLevelName> {
    exported_targets_with_name(compiler, file_id, name).into_iter().next()
}

/// Resolve an export entry to the definition it exports
fn resolve_export_entry(
    compiler: &Db, file_id: SourceFileId, all: &VisibleDefinitionsResult, entry: &ante::parser::cst::ExportEntry,
) -> Option<TopLevelName> {
    match &entry.qualifier {
        Some((qualifier, _)) => {
            let type_id = all.definitions.get(qualifier)?;
            all.methods.get(&type_id.top_level_item)?.get(&entry.name).copied()
        },
        None => all
            .definitions
            .get(&entry.name)
            .copied()
            .or_else(|| AllTypes(file_id).get(compiler).get(&entry.name).copied()),
    }
}

/// Every location that references `target`, declaration site included
pub fn collect_reference_locations(compiler: &Db, target: &RenameTarget) -> Vec<AnteLocation> {
    let mut locations = Vec::new();

    if !target.is_top_level {
        let resolve = Resolve(target.item).get(compiler);
        collect_in_item(compiler, target.item, &resolve, Origin::Local(target.name_id), &mut locations);
        return locations;
    }
    let name = target.top_level_name();

    // For a target in an external crate the scan below only covers local files,
    // which exclude the declaration so we push it manually here.
    if let Some(declaration) = declaration_location(compiler, target) {
        locations.push(declaration);
    }

    let target_origin = Origin::TopLevelDefinition(name);

    let is_method = AllDefinitions(name.top_level_item.source_file)
        .get(compiler)
        .methods
        .values()
        .any(|methods| methods.values().any(|method| *method == name));

    let crates = GetCrateGraph.get(compiler);
    let Some(local) = crates.get(&CrateId::LOCAL) else { return locations };
    for file_id in local.source_files.values() {
        let parse = Parse(*file_id).get(compiler);
        for item in &parse.cst.top_level_items {
            let resolve = Resolve(item.id).get(compiler);
            if !is_method {
                let references_target =
                    item.id == name.top_level_item || resolve.referenced_items.contains(&name.top_level_item);
                let has_type_resolution = resolve.path_origins.values().any(|o| *o == Origin::TypeResolution);
                if !references_target && !has_type_resolution {
                    continue;
                }
            }
            collect_in_item(compiler, item.id, &resolve, target_origin, &mut locations);
        }
    }

    locations
}

fn collect_in_item(
    compiler: &Db, item: TopLevelId, resolve: &ResolutionResult, target_origin: Origin,
    locations: &mut Vec<AnteLocation>,
) {
    let type_check = TypeCheck(item).get(compiler);
    let ctx = &type_check.result.context;

    let resolve_only_names =
        resolve.name_origins.iter().map(|(id, o)| (*id, *o)).filter(|(id, _)| ctx.name_origin(*id).is_none());
    for (name_id, origin) in ctx.name_origins().chain(resolve_only_names) {
        if origin == target_origin {
            locations.push(ctx.name_location(name_id));
        }
    }

    let resolve_only_paths =
        resolve.path_origins.iter().map(|(id, o)| (*id, *o)).filter(|(id, _)| ctx.path_origin(*id).is_none());
    for (path_id, origin) in ctx.path_origins().chain(resolve_only_paths) {
        if origin == target_origin {
            locations.push(ctx.get_path(path_id).components.last().unwrap().1.clone());
        }
    }
}

pub fn declaration_location(compiler: &Db, target: &RenameTarget) -> Option<AnteLocation> {
    let (_, ctx) = GetItem(target.item).get(compiler);
    Some(ctx.name_location(target.name_id).clone())
}

pub struct RenameLocations {
    pub replacements: Vec<AnteLocation>,

    /// Import items whose bare name also refers to other same-named exports.
    /// The old item must stay for these, so the new name is inserted after it
    /// (e.g. `push` becomes `push, append`) instead of replacing it.
    pub insert_after: Vec<AnteLocation>,
}

impl RenameLocations {
    /// Every referencing location
    pub fn into_references(self) -> Vec<AnteLocation> {
        let mut locations = self.replacements;
        locations.extend(self.insert_after);
        locations
    }
}

/// [collect_reference_locations] plus import items in every local file and the defining file's export list
pub fn collect_rename_locations(compiler: &Db, target: &RenameTarget, old_name: &str) -> RenameLocations {
    let mut locations =
        RenameLocations { replacements: collect_reference_locations(compiler, target), insert_after: Vec::new() };

    if !target.is_top_level {
        return locations;
    }
    let name = target.top_level_name();
    let defining_file = name.top_level_item.source_file;

    let def_parse = Parse(defining_file).get(compiler);
    if let Some(exports) = &def_parse.cst.exports {
        let all = AllDefinitions(defining_file).get(compiler);
        for entry in exports {
            // Renaming a type must also rewrite it where it qualifies a method export.
            if let Some((qualifier, qualifier_loc)) = &entry.qualifier {
                if qualifier.as_str() == old_name && all.definitions.get(qualifier) == Some(&name) {
                    locations.replacements.push(qualifier_loc.clone());
                }
            }
            if entry.name.as_str() == old_name
                && resolve_export_entry(compiler, defining_file, &all, entry) == Some(name)
            {
                locations.replacements.push(entry.location.clone());
            }
        }
    }

    // A bare import item can refer to several same-named exports at once, only
    // rewrite it when the rename target is the sole one.
    let exported_matches = exported_targets_with_name(compiler, defining_file, &old_name.to_string());
    let unique = exported_matches.len() == 1 && exported_matches[0] == name;
    let target_exported = exported_matches.contains(&name);

    let crates = GetCrateGraph.get(compiler);
    let Some(local) = crates.get(&CrateId::LOCAL) else { return locations };
    for file_id in local.source_files.values() {
        let parse = Parse(*file_id).get(compiler);
        for import in &parse.cst.imports {
            if import_target_file(&crates, &import.crate_name, &import.module_path) != Some(defining_file) {
                continue;
            }
            for (item_name, loc) in &import.items {
                if item_name.as_str() != old_name {
                    continue;
                }
                if unique {
                    locations.replacements.push(loc.clone());
                } else if target_exported {
                    locations.insert_after.push(loc.clone());
                }
            }
        }
    }

    locations
}

fn validate_new_name(old_name: &str, new_name: &str) -> Result<(), RenameError> {
    fn is_plain_identifier(name: &str) -> bool {
        name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }
    fn is_type_name(name: &str) -> bool {
        name.chars().next().is_some_and(char::is_uppercase)
    }

    if !is_plain_identifier(old_name) {
        return Err(RenameError::InvalidName(format!("Cannot rename `{old_name}`: not a plain identifier")));
    }
    if !is_plain_identifier(new_name) {
        return Err(RenameError::InvalidName(format!("`{new_name}` is not a valid identifier")));
    }
    if ante::lexer::token::lookup_keyword(new_name).is_some() {
        return Err(RenameError::InvalidName(format!("`{new_name}` is a keyword")));
    }
    // The lexer decides identifier vs type name by the case of the first character.
    if is_type_name(old_name) != is_type_name(new_name) {
        let expected = if is_type_name(old_name) { "capitalized" } else { "lowercase" };
        return Err(RenameError::InvalidName(format!("`{new_name}` must be {expected} to replace `{old_name}`")));
    }
    Ok(())
}

/// Validate the new name, then rewrite every reference, import item, and export
/// entry of `symbol` into a workspace edit.
pub fn rename_symbol(
    compiler: &Db, symbol: &SymbolAtCursor, new_name: &str, document_map: &DashMap<Url, Rope>,
) -> Result<Option<WorkspaceEdit>, RenameError> {
    validate_new_name(&symbol.old_name, new_name)?;
    let locations = collect_rename_locations(compiler, &symbol.target, &symbol.old_name);
    Ok(build_workspace_edit(compiler, locations, &symbol.old_name, new_name, document_map))
}

fn build_workspace_edit(
    compiler: &Db, locations: RenameLocations, old_name: &str, new_name: &str, document_map: &DashMap<Url, Rope>,
) -> Option<WorkspaceEdit> {
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for (uri, rope, spans) in group_by_file(compiler, locations.replacements, old_name, document_map) {
        let edits = spans
            .into_iter()
            .filter_map(|(start, end)| {
                let range = byte_range_to_lsp_range(start, end, &rope).ok()?;
                Some(TextEdit { range, new_text: new_name.to_string() })
            })
            .collect::<Vec<_>>();
        if !edits.is_empty() {
            changes.insert(uri, edits);
        }
    }

    // Shared import items keep the old name and gain the new one alongside it.
    for (uri, rope, spans) in group_by_file(compiler, locations.insert_after, old_name, document_map) {
        let edits = spans.into_iter().filter_map(|(_, end)| {
            let range = byte_range_to_lsp_range(end, end, &rope).ok()?;
            Some(TextEdit { range, new_text: format!(", {new_name}") })
        });
        changes.entry(uri).or_default().extend(edits);
    }

    (!changes.is_empty()).then(|| WorkspaceEdit { changes: Some(changes), ..Default::default() })
}

pub fn locations_to_lsp(
    compiler: &Db, locations: Vec<AnteLocation>, old_name: &str, document_map: &DashMap<Url, Rope>,
) -> Vec<LspLocation> {
    let mut result = Vec::new();
    for (uri, rope, spans) in group_by_file(compiler, locations, old_name, document_map) {
        for (start, end) in spans {
            if let Ok(range) = byte_range_to_lsp_range(start, end, &rope) {
                result.push(LspLocation { uri: uri.clone(), range });
            }
        }
    }
    result
}

/// Dedup locations and group their byte spans per file, keeping only spans whose
/// current text is `old_name`
fn group_by_file(
    compiler: &Db, locations: Vec<AnteLocation>, old_name: &str, document_map: &DashMap<Url, Rope>,
) -> Vec<(Url, Rope, Vec<(usize, usize)>)> {
    let mut by_file: BTreeMap<SourceFileId, Vec<(usize, usize)>> = BTreeMap::new();
    for location in locations {
        by_file.entry(location.file_id).or_default().push(byte_span(&location));
    }

    let mut result = Vec::new();
    for (file_id, mut spans) in by_file {
        spans.sort_unstable();
        spans.dedup();
        let source_file = file_id.get(compiler);
        let Ok(uri) = Url::from_file_path(source_file.path.as_ref()) else { continue };
        let rope = rope_for_file(&uri, &source_file.contents, document_map);
        spans.retain(|(start, end)| rope.get_byte_slice(*start..*end).is_some_and(|slice| slice == old_name));
        if !spans.is_empty() {
            result.push((uri, rope, spans));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante::incremental::Db;
    use std::path::PathBuf;

    fn ante_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("ante-ls must live inside the ante workspace")
            .to_path_buf()
    }

    /// Register each (file_name, source) as a loose file in the local crate.
    fn db_with_sources(sources: &[(&str, &str)]) -> (Db, Vec<SourceFileId>) {
        let root = ante_root();
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &root);
        let roots = crate::diagnostics::CrateRoots::new(&db, root.clone());
        let file_ids = sources
            .iter()
            .map(|(file_name, source)| {
                let path = root.join(file_name);
                crate::diagnostics::set_file_content(&mut db, &roots, &path, &Rope::from_str(source));
                crate::diagnostics::file_id_for_path(&roots, &path)
            })
            .collect();
        (db, file_ids)
    }

    fn renameable_symbol(db: &Db, file_id: SourceFileId, byte_offset: usize) -> SymbolAtCursor {
        let Ok(symbol) = symbol_at(db, file_id, byte_offset) else {
            panic!("expected a symbol at byte offset {byte_offset}");
        };
        assert!(symbol.is_renameable(), "symbol should be renameable");
        symbol
    }

    /// Rename the symbol at `byte_offset` and return the resulting edits per file.
    fn edits_at(db: &Db, file_id: SourceFileId, byte_offset: usize, new_name: &str) -> HashMap<Url, Vec<TextEdit>> {
        let symbol = renameable_symbol(db, file_id, byte_offset);
        let document_map = DashMap::new();
        let edit = rename_symbol(db, &symbol, new_name, &document_map)
            .expect("new name should be valid")
            .expect("expected a workspace edit");
        edit.changes.expect("expected changes")
    }

    fn total_edits(changes: &HashMap<Url, Vec<TextEdit>>) -> usize {
        changes.values().map(Vec::len).sum()
    }

    #[test]
    fn rename_local_parameter_renames_all_uses() {
        let source = "f x =\n    x + x\n";
        let (db, ids) = db_with_sources(&[("rename_param.an", source)]);
        let changes = edits_at(&db, ids[0], source.rfind('x').unwrap(), "y");
        assert_eq!(changes.len(), 1, "edits must be confined to one file");
        assert_eq!(total_edits(&changes), 3, "binding plus both uses");
        for edit in changes.values().flatten() {
            assert_eq!(edit.new_text, "y");
        }
    }

    #[test]
    fn rename_top_level_function_renames_declaration_and_uses() {
        let source = "foo () = ()\nmain () =\n    foo ()\n";
        let (db, ids) = db_with_sources(&[("rename_top_level.an", source)]);
        let changes = edits_at(&db, ids[0], source.rfind("foo").unwrap(), "bar");
        assert_eq!(changes.len(), 1);
        assert_eq!(total_edits(&changes), 2, "declaration and call site");
    }

    #[test]
    fn rename_stdlib_symbol_is_refused() {
        let source = "import Std.Stream.iota\n\nmain () =\n    iota 3\n    ()\n";
        let (db, ids) = db_with_sources(&[("rename_stdlib.an", source)]);

        let at_use = symbol_at(&db, ids[0], source.rfind("iota").unwrap()).expect("use site should resolve");
        assert!(!at_use.is_renameable(), "stdlib symbols must not be renameable");

        let at_import = symbol_at(&db, ids[0], source.find("iota").unwrap()).expect("import item should resolve");
        assert!(!at_import.is_renameable(), "stdlib symbols must not be renameable from the import either");
    }

    #[test]
    fn rename_includes_export_list_entry() {
        let source = "export foo\n\nfoo () = ()\n";
        let (db, ids) = db_with_sources(&[("rename_export.an", source)]);
        let changes = edits_at(&db, ids[0], source.rfind("foo").unwrap(), "bar");
        assert_eq!(total_edits(&changes), 2, "declaration and export entry");
    }

    #[test]
    fn rename_target_of_import_rewrites_import_item() {
        // The import line needs the local crate's name, so read it from the graph.
        let root = ante_root();
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &root);
        let crate_name = GetCrateGraph.get(&db)[&CrateId::LOCAL].name.clone();

        let def_source = "export foo\n\nfoo () = ()\n";
        let use_source = format!("import {crate_name}.RenameImportDef.foo\n\nmain () =\n    foo ()\n");

        let roots = crate::diagnostics::CrateRoots::new(&db, root.clone());
        let def_path = root.join("RenameImportDef.an");
        let use_path = root.join("RenameImportUse.an");
        crate::diagnostics::set_file_content(&mut db, &roots, &def_path, &Rope::from_str(def_source));
        crate::diagnostics::set_file_content(&mut db, &roots, &use_path, &Rope::from_str(&use_source));
        let use_file = crate::diagnostics::file_id_for_path(&roots, &use_path);

        let changes = edits_at(&db, use_file, use_source.rfind("foo").unwrap(), "bar");
        assert_eq!(changes.len(), 2, "both the defining and the importing file must be edited");
        assert_eq!(total_edits(&changes), 4, "declaration, export entry, import item, and call site");
    }

    #[test]
    fn rename_union_variant_via_type_resolution() {
        let source =
            "type Color =\n   | Red\n   | Green\n\ncheck (c: Color) =\n    match c\n    | Red -> 1\n    | Green -> 2\n";
        let (db, ids) = db_with_sources(&[("rename_variant.an", source)]);
        let changes = edits_at(&db, ids[0], source.rfind("Red").unwrap(), "Crimson");
        assert_eq!(total_edits(&changes), 2, "variant declaration and match pattern");
    }

    #[test]
    fn references_on_stdlib_symbol_include_declaration_and_local_uses() {
        let source = "import Std.Stream.iota\n\nmain () =\n    iota 3\n    ()\n";
        let (db, ids) = db_with_sources(&[("references_stdlib.an", source)]);

        let symbol = symbol_at(&db, ids[0], source.rfind("iota").unwrap()).expect("use site should resolve");
        assert!(!symbol.is_renameable());

        let locations = collect_rename_locations(&db, &symbol.target, &symbol.old_name).into_references();
        let document_map = DashMap::new();
        let references = locations_to_lsp(&db, locations, &symbol.old_name, &document_map);

        let files: std::collections::HashSet<_> = references.iter().map(|r| r.uri.clone()).collect();
        assert_eq!(files.len(), 2, "references must span the local file and the stdlib declaration");
        assert!(references.len() >= 3, "declaration, import item, and use, got {}", references.len());
    }

    #[test]
    fn rename_method_rewrites_dot_syntax_calls() {
        let source = "type Box = val: I32\n\nBox.get (b: Box) = b.val\n\nmain () =\n    b = Box 3\n    b.get ()\n";
        let (db, ids) = db_with_sources(&[("rename_dot_call.an", source)]);
        let changes = edits_at(&db, ids[0], source.find("Box.get").unwrap() + 4, "value");
        assert_eq!(total_edits(&changes), 2, "method declaration and dot-syntax call site");
    }

    #[test]
    fn references_include_dot_syntax_calls() {
        let source = "type Box = val: I32\n\nBox.get (b: Box) = b.val\n\nmain () =\n    b = Box 3\n    b.get ()\n";
        let (db, ids) = db_with_sources(&[("references_dot_call.an", source)]);

        let symbol = renameable_symbol(&db, ids[0], source.find("Box.get").unwrap() + 4);
        let locations = collect_rename_locations(&db, &symbol.target, &symbol.old_name).into_references();
        let document_map = DashMap::new();
        let references = locations_to_lsp(&db, locations, &symbol.old_name, &document_map);
        assert_eq!(references.len(), 2, "declaration and dot-syntax call site");
    }

    #[test]
    fn rename_match_bound_variable() {
        let source = "check x =\n    match x\n    | Some y -> y\n    | None -> 0\n";
        let (db, ids) = db_with_sources(&[("rename_match_local.an", source)]);
        let changes = edits_at(&db, ids[0], source.find("Some y").unwrap() + 5, "z");
        assert_eq!(total_edits(&changes), 2, "pattern binding and use");
    }

    #[test]
    fn symbol_found_with_caret_at_identifier_end() {
        let source = "foo () = ()\nmain () =\n    foo ()\n";
        let (db, ids) = db_with_sources(&[("rename_caret_end.an", source)]);
        let offset = source.rfind("foo").unwrap() + 3;
        let symbol = symbol_at(&db, ids[0], offset).expect("caret just past the identifier should still resolve");
        assert_eq!(symbol.old_name, "foo");
    }

    #[test]
    fn rename_method_rewrites_qualified_export_entry() {
        let source = "export Box, Box.get\n\ntype Box = val: I32\n\nBox.get (b: Box) = b.val\n\nmain () =\n    b = Box 3\n    b.get ()\n";
        let (db, ids) = db_with_sources(&[("rename_export_method.an", source)]);
        let changes = edits_at(&db, ids[0], source.find("Box.get (").unwrap() + 4, "value");
        assert_eq!(total_edits(&changes), 3, "declaration, dot-syntax call site, and export entry");
    }

    #[test]
    fn rename_same_named_method_leaves_the_other_types_method_alone() {
        let source = "export A, B, A.get, B.get\n\ntype A = x: I32\ntype B = y: I32\n\nA.get (a: A) = a.x\nB.get (b: B) = b.y\n\nmain () =\n    a = A 1\n    b = B 2\n    a.get () + b.get ()\n";
        let (db, ids) = db_with_sources(&[("rename_same_named.an", source)]);
        let changes = edits_at(&db, ids[0], source.find("A.get (a").unwrap() + 2, "fetch");
        assert_eq!(total_edits(&changes), 3, "A.get's declaration, call site, and export entry; B.get untouched");
    }

    #[test]
    fn rename_method_rewrites_cross_file_dot_call() {
        let root = ante_root();
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &root);
        let crate_name = GetCrateGraph.get(&db)[&CrateId::LOCAL].name.clone();

        let def_source = "export Box, Box.get\n\ntype Box = val: I32\n\nBox.get (b: Box) = b.val\n";
        let use_source = format!("import {crate_name}.CrossFileDef.Box\n\nmain () =\n    b = Box 3\n    b.get ()\n");

        let roots = crate::diagnostics::CrateRoots::new(&db, root.clone());
        let def_path = root.join("CrossFileDef.an");
        let use_path = root.join("CrossFileUse.an");
        crate::diagnostics::set_file_content(&mut db, &roots, &def_path, &Rope::from_str(def_source));
        crate::diagnostics::set_file_content(&mut db, &roots, &use_path, &Rope::from_str(&use_source));
        let def_file = crate::diagnostics::file_id_for_path(&roots, &def_path);

        // The dot call is invisible to the resolver's referenced_items, so this
        // exercises the method-target pruning bypass across files.
        let changes = edits_at(&db, def_file, def_source.find("Box.get (").unwrap() + 4, "value");
        assert_eq!(changes.len(), 2, "both the defining and the calling file must be edited");
        assert_eq!(total_edits(&changes), 3, "declaration, export entry, and cross-file dot call");
    }

    #[test]
    fn symbol_at_export_entry_resolves_method_and_qualifier() {
        let source = "export Box, Box.get\n\ntype Box = val: I32\n\nBox.get (b: Box) = b.val\n\nmain () = ()\n";
        let (db, ids) = db_with_sources(&[("export_entry_symbols.an", source)]);
        let entry_offset = source.find("Box.get").unwrap();

        let on_qualifier = renameable_symbol(&db, ids[0], entry_offset);
        assert_eq!(on_qualifier.old_name, "Box", "the entry qualifier must resolve to the type");

        let on_method = renameable_symbol(&db, ids[0], entry_offset + 4);
        assert_eq!(on_method.old_name, "get", "the entry name must resolve to the method");

        // Renaming from the export entry rewrites the same sites as from the declaration.
        let changes = edits_at(&db, ids[0], entry_offset + 4, "value");
        assert_eq!(total_edits(&changes), 2, "export entry and method declaration");
    }

    #[test]
    fn rename_shared_import_item_inserts_new_name_alongside() {
        let root = ante_root();
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &root);
        let crate_name = GetCrateGraph.get(&db)[&CrateId::LOCAL].name.clone();

        let def_source =
            "export A, B, A.get, B.get\n\ntype A = x: I32\ntype B = y: I32\n\nA.get (a: A) = a.x\nB.get (b: B) = b.y\n";
        let use_source =
            format!("import {crate_name}.RenameSharedDef.get, A\n\nmain () =\n    a = A 1\n    a.get ()\n");

        let roots = crate::diagnostics::CrateRoots::new(&db, root.clone());
        let def_path = root.join("RenameSharedDef.an");
        let use_path = root.join("RenameSharedUse.an");
        crate::diagnostics::set_file_content(&mut db, &roots, &def_path, &Rope::from_str(def_source));
        crate::diagnostics::set_file_content(&mut db, &roots, &use_path, &Rope::from_str(&use_source));
        let def_file = crate::diagnostics::file_id_for_path(&roots, &def_path);

        let changes = edits_at(&db, def_file, def_source.find("A.get (a").unwrap() + 2, "fetch");
        // The bare `import ...get` also names B.get, so it keeps `get` and gains `, fetch`.
        let insertions: Vec<_> = changes.values().flatten().filter(|edit| edit.new_text == ", fetch").collect();
        assert_eq!(insertions.len(), 1, "the shared import item must gain the new name alongside the old");
        assert_eq!(total_edits(&changes), 4, "declaration, export entry, dot call site, and import insertion");
    }

    #[test]
    fn rename_imported_type_rewrites_import_item() {
        let root = ante_root();
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &root);
        let crate_name = GetCrateGraph.get(&db)[&CrateId::LOCAL].name.clone();

        let def_source = "export Foo\n\ntype Foo = val: I32\n";
        let use_source = format!("import {crate_name}.RenameTypeDef.Foo\n\nmain () =\n    f = Foo 3\n    ()\n");

        let roots = crate::diagnostics::CrateRoots::new(&db, root.clone());
        let def_path = root.join("RenameTypeDef.an");
        let use_path = root.join("RenameTypeUse.an");
        crate::diagnostics::set_file_content(&mut db, &roots, &def_path, &Rope::from_str(def_source));
        crate::diagnostics::set_file_content(&mut db, &roots, &use_path, &Rope::from_str(&use_source));
        let def_file = crate::diagnostics::file_id_for_path(&roots, &def_path);

        let changes = edits_at(&db, def_file, def_source.find("type Foo").unwrap() + 5, "Bar");
        // An exported type is listed in both the definitions and types maps, the
        // import item must still count as unambiguous and be rewritten in place.
        for edit in changes.values().flatten() {
            assert_eq!(edit.new_text, "Bar", "the import item must be rewritten, not appended to");
        }
        assert_eq!(changes.len(), 2, "both files must be edited");
        assert_eq!(total_edits(&changes), 4, "type declaration, export entry, import item, and constructor use");
    }

    #[test]
    fn rename_type_rewrites_export_qualifier() {
        let source = "export Box, Box.get\n\ntype Box = val: I32\n\nBox.get (b: Box) = b.val\n\nmain () =\n    b = Box 3\n    b.get ()\n";
        let (db, ids) = db_with_sources(&[("rename_export_qualifier.an", source)]);
        let changes = edits_at(&db, ids[0], source.find("type Box").unwrap() + 5, "Crate2");
        let expected = [
            "type declaration",
            "method declaration qualifier",
            "annotation",
            "constructor",
            "bare export entry",
            "export qualifier",
        ];
        assert_eq!(total_edits(&changes), expected.len(), "expected edits: {expected:?}");
    }

    #[test]
    fn validate_new_name_rejects_bad_names() {
        assert!(validate_new_name("foo", "bar").is_ok());
        assert!(validate_new_name("Foo", "Bar").is_ok());
        assert!(validate_new_name("foo", "if").is_err(), "keywords are not identifiers");
        assert!(validate_new_name("foo", "1abc").is_err(), "identifiers cannot start with a digit");
        assert!(validate_new_name("foo", "Foo").is_err(), "case class must be preserved");
        assert!(validate_new_name("Foo", "foo").is_err(), "case class must be preserved");
        assert!(validate_new_name("foo", "").is_err());
        assert!(validate_new_name("+", "plus").is_err(), "operators cannot be renamed");
    }
}
