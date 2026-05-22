use std::sync::Arc;

use ante::incremental::{Db, GetItem, Parse, TypeCheck};
use ante::name_resolution::namespace::SourceFileId;
use ante::parser::desugar_context::DesugarContext;
use ante::parser::ids::{IdStore, NameStore};

use crate::util::{is_internal_only_type, SpanSearcher};

/// Find the innermost node (path, name, or pattern) at `byte_offset` in
/// `file_id` and return a hover string of the form `name : Type`.
///
/// Position lookups are done against the **desugared** context from `GetItem`
/// rather than the raw parse result, because type-checking runs on the
/// desugared form and the node IDs must match.
pub fn hover_at(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Option<String> {
    use ante::parser::cst::Pattern;
    use ante::parser::ids::{NameId, PathId, PatternId, TopLevelId};

    enum Hit {
        Path(PathId),
        Name(NameId),
        Pattern(PatternId),
    }

    let parse = Parse(file_id).get(compiler);

    let mut searcher = SpanSearcher::new(byte_offset);
    let mut best: Option<(Hit, TopLevelId, Arc<DesugarContext>)> = None;

    for item in &parse.cst.top_level_items {
        // Use the desugared context so node IDs match what TypeCheck stored.
        let (_, ctx) = GetItem(item.id).get(compiler);

        for (name_id, loc) in ctx.name_locations() {
            if searcher.try_offer(loc.span.start.byte_index, loc.span.end.byte_index) {
                best = Some((Hit::Name(name_id), item.id, ctx.clone()));
                // No smaller unit than a name so we can break early
                break;
            }
        }
        for (path_id, loc) in ctx.path_locations() {
            if searcher.try_offer(loc.span.start.byte_index, loc.span.end.byte_index) {
                best = Some((Hit::Path(path_id), item.id, ctx.clone()));
                // The only unit smaller than a path is a name so we can also break early here
                break;
            }
        }
        for (pattern_id, loc) in ctx.pattern_locations() {
            if searcher.try_offer(loc.span.start.byte_index, loc.span.end.byte_index) {
                best = Some((Hit::Pattern(pattern_id), item.id, ctx.clone()));
                // We cannot break early, there may be other patterns in a nested span.
            }
        }
    }

    let (hit, item_id, ctx) = best?;
    let tc = TypeCheck(item_id).get(compiler);

    let (name, typ) = match hit {
        Hit::Path(path_id) => {
            let typ = tc.result.maps.path_types.get(&path_id)?.follow(&tc.bindings);
            (ctx.get_path(path_id).last_ident().to_owned(), typ)
        },
        Hit::Name(name_id) => {
            let typ = tc.result.maps.name_types.get(&name_id)?.follow(&tc.bindings);
            (ctx.get_name(name_id).as_str().to_owned(), typ)
        },
        Hit::Pattern(pattern_id) => {
            let typ = tc.result.maps.pattern_types.get(&pattern_id)?.follow(&tc.bindings);
            // Only Pattern::Variable carries a hoverable name; other kinds are skipped.
            let Pattern::Variable(name_id) = ctx.get_pattern(pattern_id) else { return None };
            (ctx.get_name(*name_id).as_str().to_owned(), typ)
        },
    };

    if is_internal_only_type(typ) {
        return None;
    }
    let type_str = typ.to_string(&tc.bindings, &tc.result.context, compiler);
    Some(format!("{name} : {type_str}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante::incremental::Db;
    use ropey::Rope;
    use std::path::PathBuf;

    /// Regression for a hover failure where `try` in ante's parser kept around
    /// Paths which were not chosen for the parse, which overlapped with the actual path of `iota`
    /// in the test below. This broke ante-ls's iteration over all paths/names/exprs/patterns.
    #[test]
    fn hover_on_imported_iota() {
        let source = "import Std.Stream.iota\n\nmain () =\n    iota 3\n    ()\n";

        let ante_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("ante-ls must live inside the ante workspace")
            .to_path_buf();

        let foo = ante_root.join("foo.an");
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &ante_root);
        crate::diagnostics::set_file_content(&mut db, &ante_root, &foo, &Rope::from_str(source));

        let file_id = ante::name_resolution::namespace::SourceFileId::for_local_path(&ante_root, &foo);
        // Byte 38 is the 'i' in `iota` on line 4.
        let result = hover_at(&db, file_id, 38);
        assert_eq!(result.as_deref(), Some("iota : fn Usz -> fn (Emit Usz) [Usz] -> Unit"));
    }
}
