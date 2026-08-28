use ante::diagnostics::{Location as AnteLocation, Position, Span};
use ante::incremental::{Db, GetItem, Parse};
use ante::name_resolution::{namespace::SourceFileId, resolve_path_qualifier, Qualifier};
use ante::parser::ids::{IdStore, PathId, TopLevelId};

use crate::rename::{self, RenameError, RenameTarget};
use crate::util::SpanSearcher;

/// Find the definition location of the symbol under `byte_offset`.
///
/// Returns `None` for builtins, unresolved names, or when no symbol covers the
/// given byte offset.
pub fn definition_at(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Option<AnteLocation> {
    match rename::symbol_at(compiler, file_id, byte_offset) {
        Ok(symbol) => rename::declaration_location(compiler, &symbol.target),
        // symbol_at only offers final path components, the cursor may be on a module or type qualifier
        Err(RenameError::NoSymbol) => qualifier_definition_at(compiler, file_id, byte_offset),
        Err(_) => None,
    }
}

fn qualifier_definition_at(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Option<AnteLocation> {
    qualifier_definition_inner(compiler, file_id, byte_offset)
        .or_else(|| (byte_offset > 0).then(|| qualifier_definition_inner(compiler, file_id, byte_offset - 1)).flatten())
}

fn qualifier_definition_inner(compiler: &Db, file_id: SourceFileId, byte_offset: usize) -> Option<AnteLocation> {
    let parse = Parse(file_id).get(compiler);
    let mut searcher = SpanSearcher::new(byte_offset);
    let mut best: Option<(TopLevelId, PathId, usize)> = None;

    for item in &parse.cst.top_level_items {
        let (_, ctx) = GetItem(item.id).get(compiler);
        for (path_id, _) in ctx.path_locations() {
            let components = &ctx.get_path(path_id).components;
            for (index, (_, loc)) in components.iter().enumerate().take(components.len() - 1) {
                if searcher.try_offer(loc.span.start.byte_index, loc.span.end.byte_index) {
                    best = Some((item.id, path_id, index));
                }
            }
        }
    }

    let (item_id, path_id, index) = best?;
    let (_, ctx) = GetItem(item_id).get(compiler);
    let components = &ctx.get_path(path_id).components;

    match resolve_path_qualifier(compiler, file_id, components, index)? {
        Qualifier::Type(name) => rename::declaration_location(compiler, &RenameTarget::top_level(name)),
        Qualifier::Module(module) => {
            let start = Position::start();
            Some(Span { start, end: start }.in_file(module))
        },
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

    /// A type qualifier in an explicit method path resolves to the type's definition.
    #[test]
    fn definition_of_type_qualifier() {
        let source = "type Box = val: I32\n\nBox.get (b: Box) = b.val\n\nmain () =\n    b = Box 3\n    Box.get b\n";
        let (db, file_id) = db_with_source(source, "def_type_qualifier.an");
        let use_offset = source.rfind("Box.get").unwrap();
        let loc = definition_at(&db, file_id, use_offset).expect("type qualifier should resolve");
        assert_eq!(loc.span.start.byte_index, source.find("Box").unwrap(), "must resolve to the type declaration");
    }

    /// Goto-definition on an export entry: the qualifier jumps to the type, the
    /// entry name to the method's declaration.
    #[test]
    fn definition_of_export_entry_parts() {
        let source = "export Box, Box.get\n\ntype Box = val: I32\n\nBox.get (b: Box) = b.val\n\nmain () = ()\n";
        let (db, file_id) = db_with_source(source, "def_export_entry.an");
        let entry_offset = source.find("Box.get").unwrap();

        let loc = definition_at(&db, file_id, entry_offset).expect("export qualifier should resolve");
        assert_eq!(loc.span.start.byte_index, source.find("type Box").unwrap() + 5, "must jump to the type");

        let loc = definition_at(&db, file_id, entry_offset + 4).expect("export entry name should resolve");
        assert_eq!(loc.span.start.byte_index, source.find("Box.get (").unwrap() + 4, "must jump to the method");
    }

    /// The one-byte end bias also applies to goto-definition.
    #[test]
    fn definition_with_caret_at_identifier_end() {
        let source = "f x =\n    x\n";
        let (db, file_id) = db_with_source(source, "def_caret_end.an");
        let after_use = source.rfind('x').unwrap() + 1;
        let loc = definition_at(&db, file_id, after_use).expect("caret just past the identifier should resolve");
        assert_eq!(loc.span.start.byte_index, source.find('x').unwrap());
    }

    /// A module qualifier resolves to the start of the module's file, and a crate
    /// component to the crate's root file.
    #[test]
    fn definition_of_module_and_crate_qualifiers() {
        let source = "main () =\n    v = Std.Vec.of [1]\n    ()\n";
        let (db, file_id) = db_with_source(source, "def_module_qualifier.an");

        let vec_offset = source.find("Vec").unwrap();
        let loc = definition_at(&db, file_id, vec_offset).expect("module qualifier should resolve");
        assert_ne!(loc.file_id, file_id, "must resolve into the stdlib Vec module");
        assert_eq!(loc.span.start.byte_index, 0);

        let std_offset = source.find("Std").unwrap();
        let loc = definition_at(&db, file_id, std_offset).expect("crate component should resolve");
        assert_ne!(loc.file_id, file_id, "must resolve into the stdlib crate root");
        assert_eq!(loc.span.start.byte_index, 0);
    }
}
