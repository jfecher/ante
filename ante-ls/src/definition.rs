use ante::diagnostics::Location as AnteLocation;
use ante::incremental::{Db, GetItem, Parse, Resolve};
use ante::name_resolution::{namespace::SourceFileId, Origin};

use crate::util::SpanSearcher;

/// Find the definition location of the symbol (path) under `byte_offset`.
///
/// Paths (variable uses, function calls) are looked up via the `Resolve` query
/// which maps each `PathId` to its `Origin`. The definition location is then
/// read from the **raw** parse context (not desugared), because that is what
/// `TopLevelName::location` uses and where source positions are stored.
///
/// Returns `None` for builtins, unresolved names, or when no path covers the
/// given byte offset.
pub fn definition_at(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Option<AnteLocation> {
    use ante::parser::ids::{PathId, TopLevelId};

    let parse = Parse(file_id).get(compiler);

    let mut searcher = SpanSearcher::new(byte_offset);
    let mut best: Option<(PathId, TopLevelId)> = None;

    for item in &parse.cst.top_level_items {
        // Use desugared context: Resolve also operates on the desugared form,
        // so PathIds must come from the same source.
        let (_, ctx) = GetItem(item.id).get(compiler);
        for (path_id, loc) in ctx.path_locations() {
            if searcher.try_offer(loc.span.start.byte_index, loc.span.end.byte_index) {
                best = Some((path_id, item.id));
            }
        }
    }

    let (path_id, item_id) = best?;
    let resolve = Resolve(item_id).get(compiler);

    match *resolve.path_origins.get(&path_id)? {
        Origin::TopLevelDefinition(top_level_name) => {
            // Definition is in a (possibly different) top-level item's raw parse context.
            let def_parse = Parse(top_level_name.top_level_item.source_file).get(compiler);
            let def_ctx = def_parse.top_level_data.get(&top_level_name.top_level_item)?;
            def_ctx.name_locations.get(top_level_name.local_name_id).cloned()
        },
        Origin::Local(name_id) => {
            // Local binding (parameter, let-binding, etc.) in the same top-level item.
            let def_parse = Parse(item_id.source_file).get(compiler);
            let def_ctx = def_parse.top_level_data.get(&item_id)?;
            def_ctx.name_locations.get(name_id).cloned()
        },
        Origin::TypeResolution | Origin::Builtin(_) => None,
    }
}
