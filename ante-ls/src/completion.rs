use std::collections::{BTreeSet, HashSet};

use ante::incremental::{Db, GetItemRaw, Parse, Resolve, TypeCheck, VisibleDefinitions, VisibleTypes};
use ante::name_resolution::namespace::SourceFileId;
use ante::name_resolution::Origin;
use ante::parser::context::TopLevelContext;
use ante::parser::ids::{NameId, TopLevelId, TopLevelName};
use ante::type_inference::dependency_graph::TypeCheckResult;
use ropey::Rope;

use tower_lsp::lsp_types::{CompletionItem, CompletionItemLabelDetails};

use crate::auto_import::{build_import_edit, for_each_export, for_each_other_module, Candidate, ItemKind};
use crate::util::{format_doc_comments, is_internal_only_type, SpanSearcher};

/// Minimum number of characters the user must have typed before we start
/// suggesting out-of-scope items for auto-import candidates. A value of 0/1 would mean
/// we'd be flooding the completion list with hundreds of stdlib names.
const MIN_AUTO_IMPORT_PREFIX: usize = 3;

/// Collect completion candidates for `byte_offset` in `file_id`.
///
/// Returns the full candidate set: in-scope top-level definitions, types,
/// imported modules, and locals from the enclosing top-level item.
/// Closely matching items not in scope are also suggested.
pub fn completions_at(
    compiler: &Db, file_id: SourceFileId, byte_offset: usize, rope: &Rope, prefix: &str,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let visible = VisibleDefinitions(file_id).get(compiler);
    for (name, top_level_name) in visible.definitions.iter() {
        items.push(enriched(compiler, name.as_str(), ItemKind::Function, top_level_name));
    }
    for name in visible.imported_modules.keys() {
        items.push(simple(name.as_str(), ItemKind::Module));
    }

    let types = VisibleTypes(file_id).get(compiler);
    for (name, (top_level_name, _kind)) in types.iter() {
        items.push(enriched(compiler, name.as_str(), ItemKind::Type, top_level_name));
    }

    push_local_names(compiler, file_id, byte_offset, &mut items);
    push_out_of_scope(compiler, file_id, rope, prefix, &mut items);

    items
}

fn simple(label: &str, kind: ItemKind) -> CompletionItem {
    CompletionItem { label: label.to_string(), kind: Some(kind.lsp_kind()), ..Default::default() }
}

// Adds an item's type & docs
fn enriched(compiler: &Db, label: &str, kind: ItemKind, top_level_name: &TopLevelName) -> CompletionItem {
    let detail = type_detail(compiler, label, top_level_name.top_level_item, top_level_name.local_name_id);
    let documentation = doc_comments_for(compiler, top_level_name.top_level_item);

    CompletionItem {
        label: label.to_string(),
        kind: Some(kind.lsp_kind()),
        detail,
        documentation,
        ..Default::default()
    }
}

fn type_detail(compiler: &Db, label: &str, item_id: TopLevelId, name_id: NameId) -> Option<String> {
    let tc = TypeCheck(item_id).get(compiler);
    format_type_detail(&tc, compiler, label, name_id)
}

fn format_type_detail(tc: &TypeCheckResult, compiler: &Db, label: &str, name_id: NameId) -> Option<String> {
    let typ = tc.result.maps.name_types.get(&name_id)?.follow(&tc.bindings);
    if is_internal_only_type(typ) {
        return None;
    }
    let type_str = typ.to_string(&tc.bindings, &tc.result.context, compiler);
    Some(format!("{label} : {type_str}"))
}

fn doc_comments_for(compiler: &Db, item_id: TopLevelId) -> Option<tower_lsp::lsp_types::Documentation> {
    let (item, _ctx) = GetItemRaw(item_id).get(compiler);
    format_doc_comments(&item.comments)
}

/// Locals are scoped per top-level item: find which item the cursor is in,
/// then enumerate every NameId whose Origin is Local. Origin::Local(decl_id)
/// values point at the declaration site, so deduping on that gives one entry
/// per binding rather than one per use site.
///
/// Lexical scope inside the item is not respected: a let-binding from one
/// arm of an if/match will appear from anywhere in the enclosing item.
fn push_local_names(compiler: &Db, file_id: SourceFileId, byte_offset: usize, items: &mut Vec<CompletionItem>) {
    let parse = Parse(file_id).get(compiler);

    // TopLevelContext.location is a placeholder; derive the item's bounding
    // span from the union of its name / path / pattern locations and pick the
    // tightest containing item.
    let mut searcher = SpanSearcher::new(byte_offset);
    let mut best: Option<TopLevelId> = None;
    for item in &parse.cst.top_level_items {
        let Some(ctx) = parse.top_level_data.get(&item.id) else { continue };
        let Some((start, end)) = item_bounding_span(ctx) else { continue };
        if searcher.try_offer(start, end) {
            best = Some(item.id);
        }
    }
    let Some(item_id) = best else { return };
    let Some(ctx) = parse.top_level_data.get(&item_id) else { return };
    let resolve = Resolve(item_id).get(compiler);
    let tc = TypeCheck(item_id).get(compiler);

    let mut seen = BTreeSet::new();
    for origin in resolve.name_origins.values() {
        if let Origin::Local(decl_id) = *origin {
            if seen.insert(decl_id) {
                if let Some(name) = ctx.names.get(decl_id) {
                    let label = name.as_str();
                    let detail = format_type_detail(&tc, compiler, label, decl_id);
                    items.push(CompletionItem {
                        label: label.to_string(),
                        kind: Some(ItemKind::Function.lsp_kind()),
                        detail,
                        ..Default::default()
                    });
                }
            }
        }
    }
}

fn item_bounding_span(ctx: &TopLevelContext) -> Option<(usize, usize)> {
    ctx.name_locations
        .values()
        .chain(ctx.path_locations.values())
        .chain(ctx.pattern_locations.values())
        .map(|loc| (loc.span.start.byte_index, loc.span.end.byte_index))
        .reduce(|(amin, amax), (bmin, bmax)| (amin.min(bmin), amax.max(bmax)))
}

/// Walk every module in the crate graph and surface names that start with
/// `prefix` and aren't already in scope. Names that aren't in scope have
/// `additional_text_edits` that, when accepted, inserts the right `import`
/// line alongside the name.
fn push_out_of_scope(compiler: &Db, file_id: SourceFileId, rope: &Rope, prefix: &str, items: &mut Vec<CompletionItem>) {
    if prefix.len() < MIN_AUTO_IMPORT_PREFIX {
        return;
    }

    let visible = VisibleDefinitions(file_id).get(compiler);
    let types = VisibleTypes(file_id).get(compiler);
    let visible = visible.definitions.keys().chain(types.keys()).chain(visible.imported_modules.keys());
    let in_scope: HashSet<&str> = visible.map(|name| name.as_str()).collect();

    let parse = Parse(file_id).get(compiler);
    let existing_imports = &parse.cst.imports;

    for_each_other_module(compiler, file_id, |crate_name, module_path, source_file_id| {
        // Defer Candidate allocation until we know we'll emit at least one
        // suggestion from this module; for most modules the prefix doesn't
        // match anything so the candidate is never built.
        let mut candidate: Option<Candidate> = None;
        let mut add_completion = |name: &str, top_level_name: Option<&TopLevelName>, kind| {
            let cand_ref = candidate.get_or_insert_with(|| Candidate::new(crate_name, module_path));
            if let Some(item) =
                build_item_completion(compiler, rope, cand_ref, existing_imports, name, top_level_name, kind)
            {
                items.push(item);
            }
        };

        for_each_export(compiler, source_file_id, |name, top_level_name, kind| {
            if !name.starts_with(prefix) || in_scope.contains(name) {
                return;
            }
            add_completion(name, Some(top_level_name), kind);
        });

        // The module itself, if its bare name matches the prefix. The `imported_modules` check
        // in `in_scope` already excludes modules the user has imported by namespace, but a user can
        // also have `import Std.Vec.push` which doesn't bring `Vec` as a namespace a namespace into
        // scope. `build_import_edit` handles that case by short-circuiting when an exact module
        // import already exists.
        let bare = module_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if !bare.is_empty() && bare.starts_with(prefix) && !in_scope.contains(bare) {
            add_completion(bare, None, ItemKind::Module);
        }
    });
}

fn build_item_completion(
    compiler: &Db, rope: &Rope, cand: &Candidate, existing_imports: &[ante::parser::cst::Import], name: &str,
    top_level_name: Option<&TopLevelName>, kind: ItemKind,
) -> Option<CompletionItem> {
    // This is the name after the `Module.` prefix
    let edit_name = match kind {
        ItemKind::Module => None,
        ItemKind::Function | ItemKind::Type => Some(name),
    };
    let edit = build_import_edit(edit_name, cand, existing_imports, rope)?;
    let description = format!("{} {}", kind.tag(), cand.dotted_display);
    let type_line = top_level_name.and_then(|tln| type_detail(compiler, name, tln.top_level_item, tln.local_name_id));
    let detail = match type_line {
        Some(line) => Some(format!("{description}\n{line}")),
        None => Some(description.clone()),
    };
    let documentation = top_level_name.and_then(|tln| doc_comments_for(compiler, tln.top_level_item));
    Some(CompletionItem {
        label: name.to_string(),
        kind: Some(kind.lsp_kind()),
        label_details: Some(CompletionItemLabelDetails { description: Some(description), detail: None }),
        detail,
        documentation,
        // `~` pushes items that need to be imported to the bottom of the completions list
        sort_text: Some(format!("~{name}")),
        additional_text_edits: Some(vec![edit]),
        ..Default::default()
    })
}
