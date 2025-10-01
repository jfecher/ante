use std::{path::PathBuf, sync::Arc};

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::{
        self, DbHandle, Definitions, ExportedDefinitions, ExportedTypes, GetCrateGraph, GetImports, Methods, Parse,
        VisibleDefinitions, VisibleDefinitionsResult, VisibleTypes,
    },
    name_resolution::namespace::SourceFileId,
    parser::{
        cst::{Import, ItemName, Literal, Pattern, TopLevelItemKind, TypeDefinitionBody},
        ids::{NameId, PatternId, TopLevelId},
        TopLevelContext,
    },
};

/// Collect all definitions which should be visible to expressions within this file.
/// This includes all top-level definitions within this file, as well as any imported ones.
pub fn visible_definitions_impl(context: &VisibleDefinitions, db: &DbHandle) -> Arc<VisibleDefinitionsResult> {
    incremental::enter_query();
    incremental::println(format!("Collecting visible definitions in {:?}", context.0));

    let mut visible = ExportedDefinitions(context.0).get(db).as_ref().clone();

    // This should always be cached. Ignoring errors here since they should already be
    // included in ExportedDefinitions' errors
    let ast = Parse(context.0).get(db);

    for import in &ast.cst.imports {
        // Ignore errors from imported files. We want to only collect errors
        // from this file. Otherwise we'll duplicate errors.
        // TODO: Still issue an error if the file name is not found
        let Some(import_file_id) = get_file_id(&import.crate_name, &import.module_path, db) else {
            push_no_such_file_error(import, db);
            continue;
        };
        let exported = ExportedDefinitions(import_file_id).get(db);

        for (exported_name, exported_id) in &exported.definitions {
            if let Some(existing) = visible.definitions.get(exported_name) {
                // This reports the location the item was defined in, not the location it was imported at.
                // I could improve this but instead I'll leave it as an exercise for the reader!
                let first_location = existing.location(db);
                let second_location = import.location.clone();
                let name = exported_name.clone();
                db.accumulate(Diagnostic::ImportedNameAlreadyInScope { name, first_location, second_location });
            } else {
                visible.definitions.insert(exported_name.clone(), *exported_id);
            }
        }
    }

    incremental::exit_query();
    Arc::new(visible)
}

fn push_no_such_file_error(import: &Import, db: &DbHandle) {
    let location = import.location.clone();
    let module_name = import.module_path.clone();
    let crate_name = import.crate_name.clone();
    db.accumulate(Diagnostic::UnknownImportFile { crate_name, module_name, location })
}

fn get_file_id(target_crate_name: &String, module_path: &PathBuf, db: &DbHandle) -> Option<SourceFileId> {
    let crates = GetCrateGraph.get(db);

    for (_, crate_) in crates.iter() {
        if crate_.name == *target_crate_name {
            return crate_.source_files.get(module_path).copied();
        }
    }

    None
}

pub fn visible_types_impl(context: &VisibleTypes, db: &DbHandle) -> Definitions {
    incremental::enter_query();
    incremental::println(format!("Collecting visible types in {:?}", context.0));

    let mut definitions = ExportedTypes(context.0).get(db);

    // This should always be cached. Ignoring errors here since they should already be
    // included in ExportedTypes' errors
    let ast = Parse(context.0).get(db);

    for import in &ast.cst.imports {
        // Ignore errors from imported files. We want to only collect errors
        // from this file. Otherwise we'll duplicate errors.
        let Some(import_file_id) = get_file_id(&import.crate_name, &import.module_path, db) else {
            continue;
        };
        let exports = ExportedTypes(import_file_id).get(db);

        for (exported_name, exported_id) in exports {
            if let Some(existing) = definitions.get(&exported_name) {
                // This reports the location the item was defined in, not the location it was imported at.
                // I could improve this but instead I'll leave it as an exercise for the reader!
                let first_location = existing.location(db);
                let second_location = import.location.clone();
                let name = exported_name;
                db.accumulate(Diagnostic::ImportedNameAlreadyInScope { name, first_location, second_location });
            } else {
                definitions.insert(exported_name, exported_id);
            }
        }
    }

    incremental::exit_query();
    definitions
}

/// Collect only the exported types within a file.
pub fn exported_types_impl(context: &ExportedTypes, db: &DbHandle) -> Definitions {
    incremental::enter_query();
    incremental::println(format!("Collecting exported definitions in {:?}", context.0));

    let result = Parse(context.0).get(db);
    let mut definitions = Definitions::default();

    // Collect each definition, issuing an error if there is a duplicate name (imports are not counted)
    for item in result.cst.top_level_items.iter() {
        if let TopLevelItemKind::TypeDefinition(definition) = &item.kind {
            let name = &result.top_level_data[&item.id].names[definition.name];

            if let Some(existing) = definitions.get(name) {
                let first_location = existing.location(db);
                let second_location = item.id.location(db);
                let name = name.clone();
                db.accumulate(Diagnostic::NameAlreadyInScope { name, first_location, second_location });
            } else {
                definitions.insert(name.clone(), item.id);
            }
        }
    }

    incremental::exit_query();
    definitions
}

/// Collect only the exported definitions within a file.
pub fn exported_definitions_impl(context: &ExportedDefinitions, db: &DbHandle) -> Arc<VisibleDefinitionsResult> {
    incremental::enter_query();
    incremental::println(format!("Collecting exported definitions in {:?}", context.0));

    let result = Parse(context.0).get(db);
    let mut declarer = Declarer::new(db);

    // Collect each definition, issuing an error if there is a duplicate name (imports are not counted)
    for item in result.cst.top_level_items.iter() {
        let data = &result.top_level_data[&item.id];
        match item.kind.name() {
            ItemName::Single(name) => declarer.declare_single(name, item.id, data),
            ItemName::Pattern(pattern) => declarer.declare_names_in_pattern(pattern, item.id, data),
            ItemName::None => (),
        }

        let mut declare_method = |type_name, item_name| {
            let context = &result.top_level_data[&item.id];
            declarer.declare_method(type_name, item_name, item.id, context);
        };

        // Declare internal items
        // TODO: all internal items use the same TopLevelId from their parent TopLevelItemKind.
        // E.g. enum variant's use the type's TopLevelId. We'll need a separate id for each to
        // differentiate them.
        match &item.kind {
            TopLevelItemKind::TypeDefinition(type_definition) => {
                if let TypeDefinitionBody::Enum(variants) = &type_definition.body {
                    for (name, _) in variants {
                        declare_method(type_definition.name, *name);
                    }
                }
            },
            TopLevelItemKind::TraitDefinition(trait_) => {
                for declaration in &trait_.body {
                    declare_method(trait_.name, declaration.name);
                }
            },
            TopLevelItemKind::EffectDefinition(effect) => {
                for declaration in &effect.body {
                    declare_method(effect.name, declaration.name);
                }
            },
            _ => (),
        }
    }

    incremental::exit_query();
    Arc::new(VisibleDefinitionsResult { definitions: declarer.definitions, methods: declarer.methods })
}

struct Declarer<'local, 'db> {
    definitions: Definitions,
    methods: Methods,
    db: &'local DbHandle<'db>,
}

impl<'local, 'db> Declarer<'local, 'db> {
    fn new(db: &'local DbHandle<'db>) -> Self {
        Self { definitions: Default::default(), methods: Default::default(), db }
    }

    fn declare_names_in_pattern(&mut self, pattern: PatternId, id: TopLevelId, context: &TopLevelContext) {
        match &context.patterns[pattern] {
            Pattern::Error => (),
            // No variables in a unit literal to declare
            Pattern::Literal(Literal::Unit) => (),
            Pattern::Literal(_) => {
                let location = context.pattern_locations[pattern].clone();
                self.db.accumulate(Diagnostic::LiteralUsedAsName { location });
            },
            Pattern::Variable(name) => self.declare_single(*name, id, context),
            Pattern::Constructor(_, args) => {
                for arg in args {
                    self.declare_names_in_pattern(*arg, id, context);
                }
            },
            Pattern::TypeAnnotation(pattern, _) => {
                self.declare_names_in_pattern(*pattern, id, context);
            },
            Pattern::MethodName { type_name, item_name } => {
                self.declare_method(*type_name, *item_name, id, context);
            },
        }
    }

    fn declare_single(&mut self, name_id: NameId, id: TopLevelId, context: &TopLevelContext) {
        self.declare_single_helper(name_id, id, context, |this| &mut this.definitions);
    }

    fn declare_single_helper(
        &mut self, name_id: NameId, id: TopLevelId, context: &TopLevelContext,
        definitions: impl FnOnce(&mut Self) -> &mut Definitions,
    ) {
        let name = context.names[name_id].clone();

        if let Some(existing) = self.definitions.get(&name) {
            let first_location = existing.location(self.db);
            let second_location = context.name_locations[name_id].clone();
            self.db.accumulate(Diagnostic::NameAlreadyInScope { name, first_location, second_location });
        } else {
            definitions(self).insert(name, id);
        }
    }

    fn declare_method(
        &mut self, type_name_id: NameId, item_name_id: NameId, id: TopLevelId, context: &TopLevelContext,
    ) {
        let type_name = &context.names[type_name_id];

        // Methods can only be declared on a type declared in the same file, so look in the same file for the type.
        if let Some(object_type) = self.definitions.get(type_name) {
            let object_type = *object_type;
            self.declare_single_helper(item_name_id, id, context, |this| this.methods.entry(object_type).or_default());
        } else {
            let name = type_name.clone();
            let location = context.name_locations[type_name_id].clone();
            self.db.accumulate(Diagnostic::MethodDeclaredOnUnknownType { name, location });
        }
    }
}

/// Collects the file names of all imports within this file.
pub fn get_imports_impl(context: &GetImports, db: &DbHandle) -> Vec<(Arc<PathBuf>, Location)> {
    incremental::enter_query();
    incremental::println(format!("Collecting imports of {:?}", context.0));

    // Ignore parse errors for now, we can report them later
    let result = Parse(context.0).get(db);
    let mut imports = Vec::new();

    // Collect each definition, issuing an error if there is a duplicate name (imports are not counted)
    for import in result.cst.imports.iter() {
        // We don't care about duplicate imports.
        // This method is only used for finding input files and the top-level
        // will filter out any repeats.
        imports.push((import.module_path.clone(), import.location.clone()));
    }

    incremental::exit_query();
    imports
}
