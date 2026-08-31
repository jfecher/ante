use std::borrow::Cow;
use std::collections::HashMap;

use ante::{
    incremental::{Db, Parse},
    name_resolution::namespace::SourceFileId,
    parser::context::TopLevelContext,
    parser::cst::{Cst, ExportEntry, Pattern, TopLevelItemKind},
    parser::ids::{IdStore, NameStore, PatternId, TopLevelId},
};
use tower_lsp::lsp_types::{CodeAction, CodeActionKind, TextEdit, Url, WorkspaceEdit};

use crate::util::{byte_range_to_lsp_range, SpanSearcher};

/// If the cursor sits on the declaration for a top-level name, offer a code
/// action that adds that name to the file's export list.
pub fn add_to_exports_action(
    compiler: &Db, file_id: SourceFileId, start_byte: usize, _end_byte: usize, uri: &Url, rope: &ropey::Rope,
) -> Option<CodeAction> {
    let parse = Parse(file_id).get(compiler);
    let name = find_top_level_name_at(&parse.cst, &parse.top_level_data, start_byte)?;
    let edit = build_export_edit(&name, &parse.cst, rope)?;

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    Some(CodeAction {
        title: format!("Add `{name}` to exports"),
        kind: Some(CodeActionKind::REFACTOR),
        edit: Some(WorkspaceEdit { changes: Some(changes), ..Default::default() }),
        is_preferred: Some(true),
        ..Default::default()
    })
}

/// Find the name of the top-level item whose own declaration contains `byte_offset`.
fn find_top_level_name_at(
    cst: &Cst, top_level_data: &std::collections::BTreeMap<TopLevelId, std::sync::Arc<TopLevelContext>>,
    byte_offset: usize,
) -> Option<String> {
    let mut searcher = SpanSearcher::new(byte_offset);
    let mut hit = None;

    for item in &cst.top_level_items {
        let Some(ctx) = top_level_data.get(&item.id) else { continue };

        let found = match &item.kind {
            TopLevelItemKind::TypeDefinition(def) => {
                let loc = &ctx.name_locations[def.name];
                searcher
                    .try_offer(loc.span.start.byte_index, loc.span.end.byte_index)
                    .then(|| ctx.get_name(def.name).to_string())
            },
            TopLevelItemKind::Definition(def) => {
                let loc = &ctx.pattern_locations[def.pattern];
                if searcher.try_offer(loc.span.start.byte_index, loc.span.end.byte_index) {
                    exportable_pattern_name(def.pattern, ctx)
                } else {
                    None
                }
            },
            TopLevelItemKind::TraitDefinition(def) | TopLevelItemKind::EffectDefinition(def) => {
                let loc = &ctx.name_locations[def.name];
                searcher
                    .try_offer(loc.span.start.byte_index, loc.span.end.byte_index)
                    .then(|| ctx.get_name(def.name).to_string())
            },
            TopLevelItemKind::TraitImpl(_) | TopLevelItemKind::Comptime(_) => None,
        };

        if let Some(name) = found {
            hit = Some(name);
        }
    }

    hit
}

/// The name a pattern's export entry needs, in export-list syntax.
/// Methods are exported qualified with their type, e.g. `Vec.push`.
fn exportable_pattern_name(pattern_id: PatternId, ctx: &TopLevelContext) -> Option<String> {
    match ctx.get_pattern(pattern_id) {
        Pattern::Variable(name) => Some(to_export_syntax(ctx.get_name(*name)).into_owned()),
        Pattern::TypeAnnotation(inner, _) => exportable_pattern_name(*inner, ctx),
        Pattern::MethodName { type_name, item_name } => {
            Some(format!("{}.{}", ctx.get_name(*type_name), to_export_syntax(ctx.get_name(*item_name))))
        },
        Pattern::Alias(name, _) | Pattern::ConstructorRest(_, name) => {
            Some(to_export_syntax(ctx.get_name(*name)).into_owned())
        },
        Pattern::Or(_) | Pattern::Literal(_) | Pattern::Constructor(..) | Pattern::Error => None,
    }
}

/// Names that aren't a plain identifier/type-name (e.g. operators like `,`)
/// must be parenthesized to round-trip through the parser as valid export syntax.
fn to_export_syntax(name: &str) -> Cow<'_, str> {
    let is_plain = name.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_alphanumeric() || c == '_');
    if is_plain {
        name.into()
    } else {
        format!("({name})").into()
    }
}

/// An existing entry rendered in the same syntax `exportable_pattern_name` produces.
fn render_entry(entry: &ExportEntry) -> String {
    match &entry.qualifier {
        Some((qualifier, _)) => format!("{qualifier}.{}", to_export_syntax(&entry.name)),
        None => to_export_syntax(&entry.name).into_owned(),
    }
}

/// `name` must already be in export-list syntax.
fn build_export_edit(name: &str, cst: &Cst, rope: &ropey::Rope) -> Option<TextEdit> {
    if let Some(exports) = &cst.exports {
        if exports.iter().any(|entry| render_entry(entry) == name) {
            return None;
        }
        let insert_byte = exports.last()?.location.span.end.byte_index;
        let range = byte_range_to_lsp_range(insert_byte, insert_byte, rope).ok()?;
        return Some(TextEdit { range, new_text: format!(", {name}") });
    }

    if let Some(last_import) = cst.imports.last() {
        let insert_byte = last_import.location.span.end.byte_index;
        let range = byte_range_to_lsp_range(insert_byte, insert_byte, rope).ok()?;
        Some(TextEdit { range, new_text: format!("\n\nexport {name}") })
    } else {
        let range = byte_range_to_lsp_range(0, 0, rope).ok()?;
        Some(TextEdit { range, new_text: format!("export {name}\n\n") })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ante::incremental::Db;
    use ropey::Rope;
    use std::path::PathBuf;

    fn setup(source: &str) -> (Db, SourceFileId) {
        let ante_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("ante-ls must live inside the ante workspace")
            .to_path_buf();

        let foo = ante_root.join("foo.an");
        let mut db = Db::default();
        crate::diagnostics::init_db(&mut db, &ante_root);
        let roots = crate::diagnostics::CrateRoots::new(&db, ante_root);
        crate::diagnostics::set_file_content(&mut db, &roots, &foo, &Rope::from_str(source));
        let file_id = crate::diagnostics::file_id_for_path(&roots, &foo);
        (db, file_id)
    }

    fn edit_at(source: &str, byte_offset: usize) -> Option<TextEdit> {
        let (db, file_id) = setup(source);
        let parse = Parse(file_id).get(&db);
        let name = find_top_level_name_at(&parse.cst, &parse.top_level_data, byte_offset)?;
        let rope = Rope::from_str(source);
        build_export_edit(&name, &parse.cst, &rope)
    }

    #[test]
    fn extends_existing_export_list() {
        let source = "export Foo\n\nbar () = ()\n";
        // Byte offset of `bar`.
        let byte_offset = source.find("bar").unwrap();
        let edit = edit_at(source, byte_offset).expect("expected an edit");
        assert_eq!(edit.new_text, ", bar");
    }

    #[test]
    fn already_exported_yields_no_action() {
        let source = "export Foo\n\nFoo = 1\n";
        let byte_offset = source.rfind("Foo").unwrap();
        assert!(edit_at(source, byte_offset).is_none());
    }

    #[test]
    fn creates_export_statement_with_no_imports() {
        let source = "foo () = ()\n";
        let byte_offset = source.find("foo").unwrap();
        let edit = edit_at(source, byte_offset).expect("expected an edit");
        assert_eq!(edit.new_text, "export foo\n\n");
        assert_eq!(edit.range.start.line, 0);
        assert_eq!(edit.range.start.character, 0);
    }

    #[test]
    fn creates_export_statement_after_imports() {
        let source = "import Std.Io.println\n\nfoo () = ()\n";
        let byte_offset = source.find("foo").unwrap();
        let edit = edit_at(source, byte_offset).expect("expected an edit");
        assert_eq!(edit.new_text, "\n\nexport foo");
    }

    #[test]
    fn non_top_level_name_yields_no_action() {
        let source = "foo x = x\n";
        let byte_offset = source.rfind('x').unwrap();
        assert!(edit_at(source, byte_offset).is_none());
    }

    #[test]
    fn method_export_action_uses_qualified_name() {
        let source = "export Box\n\ntype Box = val: I32\n\nBox.get (b: Box) = b.val\n";
        let byte_offset = source.find("Box.get").unwrap();
        let edit = edit_at(source, byte_offset).expect("expected an edit");
        assert_eq!(edit.new_text, ", Box.get");
    }

    #[test]
    fn already_exported_qualified_method_yields_no_action() {
        let source = "export Box, Box.get\n\ntype Box = val: I32\n\nBox.get (b: Box) = b.val\n";
        let byte_offset = source.find("Box.get (").unwrap();
        assert!(edit_at(source, byte_offset).is_none());
    }
}
