use std::collections::BTreeSet;

use ante::incremental::{Db, GetItemRaw, Parse, Resolve, TypeCheck, VisibleDefinitions, VisibleTypes};
use ante::name_resolution::namespace::SourceFileId;
use ante::name_resolution::Origin;
use ante::parser::context::TopLevelContext;
use ante::parser::ids::{NameId, TopLevelId, TopLevelName};

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

use crate::util::{format_doc_comments, is_internal_only_type, SpanSearcher};

/// Collect completion candidates for `byte_offset` in `file_id`.
///
/// Returns the full candidate set (top-level definitions, types, imported
/// modules, keywords, and locals from the enclosing top-level item). Prefix
/// filtering is done by the LSP client.
pub fn completions_at(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let visible = VisibleDefinitions(file_id).get(compiler);
    for (name, top_level_name) in visible.definitions.iter() {
        items.push(enriched(compiler, name.as_str(), CompletionItemKind::FUNCTION, top_level_name));
    }
    for name in visible.imported_modules.keys() {
        items.push(simple(name.as_str(), CompletionItemKind::MODULE));
    }

    let types = VisibleTypes(file_id).get(compiler);
    for (name, (top_level_name, _kind)) in types.iter() {
        items.push(enriched(compiler, name.as_str(), CompletionItemKind::CLASS, top_level_name));
    }

    push_local_names(compiler, file_id, byte_offset, &mut items);
    items
}

fn simple(label: &str, kind: CompletionItemKind) -> CompletionItem {
    CompletionItem { label: label.to_string(), kind: Some(kind), ..Default::default() }
}

// Adds an item's type & docs
fn enriched(compiler: &Db, label: &str, kind: CompletionItemKind, top_level_name: &TopLevelName) -> CompletionItem {
    let detail = type_detail(compiler, label, top_level_name.top_level_item, top_level_name.local_name_id);
    let documentation = doc_comments_for(compiler, top_level_name.top_level_item);

    CompletionItem { label: label.to_string(), kind: Some(kind), detail, documentation, ..Default::default() }
}

fn type_detail(compiler: &Db, label: &str, item_id: TopLevelId, name_id: NameId) -> Option<String> {
    let tc = TypeCheck(item_id).get(compiler);
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
                    let typ = tc.result.maps.name_types.get(&decl_id);
                    // Filter out erroring types or hidden types like NoClosureEnv
                    let detail = typ.map(|typ| typ.follow(&tc.bindings)).filter(|typ| !is_internal_only_type(typ)).map(|typ| {
                        let type_str = typ.to_string(&tc.bindings, &tc.result.context, compiler);
                        format!("{label} : {type_str}")
                    });
                    items.push(CompletionItem {
                        label: label.to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail,
                        ..Default::default()
                    });
                }
            }
        }
    }
}

fn item_bounding_span(ctx: &TopLevelContext) -> Option<(usize, usize)> {
    let mut min = usize::MAX;
    let mut max = 0usize;
    let mut any = false;
    let mut consume = |loc: &ante::diagnostics::Location| {
        any = true;
        min = min.min(loc.span.start.byte_index);
        max = max.max(loc.span.end.byte_index);
    };
    for (_, loc) in ctx.name_locations.iter() {
        consume(loc);
    }
    for (_, loc) in ctx.path_locations.iter() {
        consume(loc);
    }
    for (_, loc) in ctx.pattern_locations.iter() {
        consume(loc);
    }
    if any {
        Some((min, max))
    } else {
        None
    }
}
