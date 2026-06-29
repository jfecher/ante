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
            let (_, ctx) = GetItem(item_id).get(compiler);
            let location = ctx.name_locations().find(|(id, _)| *id == name_id).map(|(_, loc)| loc.clone());
            location
        },
        Origin::TypeResolution | Origin::Builtin(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante::incremental::Db;
    use ropey::Rope;
    use std::path::PathBuf;

    fn db_with_source(source: &str, file_name: &str) -> (Db, SourceFileId) {
        let ante_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("ante-ls must live inside the ante workspace")
            .to_path_buf();
        let file = ante_root.join(file_name);
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &ante_root);
        let roots = crate::diagnostics::CrateRoots::new(&db, ante_root);
        crate::diagnostics::set_file_content(&mut db, &roots, &file, &Rope::from_str(source));
        let file_id = crate::diagnostics::file_id_for_path(&roots, &file);
        (db, file_id)
    }

    /// goto-definition on a plain parameter use resolves to the parameter's binding.
    #[test]
    fn definition_of_a_parameter() {
        let source = "f x =\n    x\n";
        let (db, file_id) = db_with_source(source, "def_param.an");
        let use_offset = source.rfind('x').unwrap();
        let binding_offset = source.find('x').unwrap();
        let loc = definition_at(&db, file_id, use_offset).expect("parameter use should resolve");
        assert_eq!(loc.span.start.byte_index, binding_offset);
    }

    /// Regression test for `recur` inside of a `loop`. The old code used `name_locations`,
    /// instead of the extended context, and goto-definition did nothing.
    #[test]
    fn definition_of_synthetic_recur_local() {
        let source = "main () =\n    loop (i = 0) ->\n        recur i\n";
        let (db, file_id) = db_with_source(source, "def_recur.an");
        let recur_offset = source.find("recur").unwrap();
        assert!(
            definition_at(&db, file_id, recur_offset).is_some(),
            "goto-definition on a synthetic `recur` local must resolve, not return None"
        );
    }
}
