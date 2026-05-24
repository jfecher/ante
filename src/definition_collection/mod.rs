use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use inc_complete::DbGet;

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::{
        self, AllDefinitions, AllTypes, DbHandle, Definitions, ExportedDefinitions, ExportedTypes, GetCrateGraph,
        GetImports, Methods, Parse, TypeDefinitions, ValidateExports, VisibleDefinitions, VisibleDefinitionsResult,
        VisibleTypes,
    },
    name_resolution::namespace::SourceFileId,
    parser::{
        ParseResult,
        context::TopLevelContext,
        cst::{Import, ItemName, Literal, Name, Pattern, TopLevelItemKind, TypeDefinition, TypeDefinitionBody},
        ids::{NameId, PatternId, TopLevelId, TopLevelName},
    },
    type_inference::kinds::Kind,
};

pub mod visible_implicits;

/// Collect all definitions which should be visible to expressions within this file.
/// This includes all top-level definitions within this file, as well as any imported ones.
pub fn visible_definitions_impl(context: &VisibleDefinitions, db: &DbHandle) -> Arc<VisibleDefinitionsResult> {
    incremental::enter_query();
    incremental::println(format!("Collecting visible definitions in {:?}", context.0));

    let mut visible = AllDefinitions(context.0).get(db).as_ref().clone();

    // This should always be cached. Ignoring errors here since they should already be
    // included in ExportedDefinitions' errors
    let ast = Parse(context.0).get(db);

    for import in &ast.cst.imports {
        // Ignore errors from imported files. We want to only collect errors
        // from this file. Otherwise we'll duplicate errors.
        let Some(import_file_id) = get_file_id(&import.crate_name, &import.module_path, db) else {
            // When module_path is empty, items may refer to submodules (e.g. `import Std.Vec`)
            if import.module_path.as_os_str().is_empty() {
                resolve_submodule_imports(import, &mut visible, db);
            } else {
                push_no_such_file_error(import, db);
            }
            continue;
        };
        let exported = ExportedDefinitions(import_file_id).get(db);

        for (exported_name, exported_id) in &exported.definitions {
            // Check if this matches the name of any imported item
            if !import.items.iter().any(|(item, _)| item == exported_name) {
                continue;
            }
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

        // Import methods as if they were defined in their parent module as free functions
        for methods in exported.methods.values() {
            for (exported_name, exported_id) in methods {
                if !import.items.iter().any(|(item, _)| item == exported_name) {
                    continue;
                }
                if !visible.definitions.contains_key(exported_name) {
                    visible.definitions.insert(exported_name.clone(), *exported_id);
                }
            }
        }

        // Report errors for any explicitly requested items not found in the module
        let exported_types = ExportedTypes(import_file_id).get(db);
        for (name, item_location) in &import.items {
            let in_definitions = exported.definitions.contains_key(name);
            let in_types = exported_types.contains_key(name);
            let in_methods = exported.methods.values().any(|m| m.contains_key(name));

            if !in_definitions && !in_types && !in_methods {
                // Check if the name exists but isn't exported
                let all_defs = AllDefinitions(import_file_id).get(db);
                let exists_in_all = all_defs.definitions.contains_key(name)
                    || AllTypes(import_file_id).get(db).contains_key(name)
                    || all_defs.methods.values().any(|m| m.contains_key(name));

                let name = name.clone();
                let module = import.module_path.clone();
                let location = item_location.clone();

                if exists_in_all {
                    db.accumulate(Diagnostic::ItemNotExported { name, module, location });
                } else {
                    db.accumulate(Diagnostic::UnknownImportItem { name, module, location });
                }
            }
        }
    }

    // If this file is not the Prelude, implicitly import the Prelude.
    // Skip any names that are imported elsewhere instead of erroring.
    if context.0 != SourceFileId::prelude() {
        let prelude = ExportedDefinitions(SourceFileId::prelude()).get(db);
        for (exported_name, exported_id) in &prelude.definitions {
            visible.definitions.entry(exported_name.clone()).or_insert(*exported_id);
        }
        for (id, methods) in &prelude.methods {
            visible.methods.entry(*id).or_default().extend(methods.iter().map(|(k, v)| (k.clone(), *v)));
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
    let module_file = module_path.with_extension("an");

    for (_, crate_) in crates.iter() {
        if crate_.name == *target_crate_name {
            return crate_.source_files.get(&module_file).copied();
        }
    }

    None
}

/// When module_path is empty (crate root), try resolving each imported item as a submodule.
/// E.g. `import Std.Vec` resolves "Vec" as the Vec.an submodule in the Std crate.
fn resolve_submodule_imports(import: &Import, visible: &mut VisibleDefinitionsResult, db: &DbHandle) {
    let crates = GetCrateGraph.get(db);
    let Some(crate_) = crates.values().find(|c| c.name == import.crate_name) else {
        push_no_such_file_error(import, db);
        return;
    };

    for (item_name, item_location) in &import.items {
        let module_file = PathBuf::from(item_name.as_str()).with_extension("an");
        if let Some(&file_id) = crate_.source_files.get(&module_file) {
            visible.imported_modules.insert(item_name.clone(), file_id);
        } else {
            let location = item_location.clone();
            let module_name = import.module_path.clone();
            let crate_name = import.crate_name.clone();
            db.accumulate(Diagnostic::UnknownImportFile { crate_name, module_name, location });
        }
    }
}

pub fn visible_types_impl(context: &VisibleTypes, db: &DbHandle) -> Arc<TypeDefinitions> {
    incremental::enter_query();
    incremental::println(format!("Collecting visible types in {:?}", context.0));

    let all = AllTypes(context.0).get(db);

    // This should always be cached. Ignoring errors here since they should already be
    // included in ExportedTypes' errors
    let ast = Parse(context.0).get(db);

    if ast.cst.imports.is_empty() {
        incremental::exit_query();
        return all;
    }

    let mut definitions = (*all).clone();
    for import in &ast.cst.imports {
        // Ignore errors from imported files. We want to only collect errors
        // from this file. Otherwise we'll duplicate errors.
        let Some(import_file_id) = get_file_id(&import.crate_name, &import.module_path, db) else {
            continue;
        };
        let exports = ExportedTypes(import_file_id).get(db);

        for (exported_name, exported_id) in exports.iter() {
            if !import.items.iter().any(|(item, _)| item == exported_name) {
                continue;
            }
            if let Some(existing) = definitions.get(exported_name) {
                // This reports the location the item was defined in, not the location it was imported at.
                // I could improve this but instead I'll leave it as an exercise for the reader!
                let first_location = existing.location(db);
                let second_location = import.location.clone();
                let name = exported_name.clone();
                db.accumulate(Diagnostic::ImportedNameAlreadyInScope { name, first_location, second_location });
            } else {
                definitions.insert(exported_name.clone(), *exported_id);
            }
        }
    }

    incremental::exit_query();
    Arc::new(definitions)
}

pub(crate) fn kind_of_type_definition(definition: &TypeDefinition) -> Kind {
    use std::num::NonZeroUsize;
    let n = definition.generics.len();
    if n == 0 {
        Kind::Type
    } else if definition.generics.iter().all(|p| p.kind.is_none()) {
        // Common case: all generics default to Kind::Type, no allocation needed.
        Kind::TypeConstructorSimple(NonZeroUsize::new(n).unwrap())
    } else {
        let kinds = definition.generics.iter().map(kind_of_generic_param).collect();
        Kind::TypeConstructorComplex(kinds)
    }
}

fn kind_of_generic_param(param: &crate::parser::cst::GenericParam) -> Kind {
    use crate::parser::cst::KindAnnotation;
    match param.kind {
        None | Some(KindAnnotation::Type) => Kind::Type,
        Some(KindAnnotation::U32) => Kind::U32,
    }
}

/// Insert `(name, TopLevelName)` into `map`, accumulating a `NameAlreadyInScope`
/// diagnostic if the name was already present.
fn insert_unique_name(
    db: &DbHandle, map: &mut Definitions, item_id: TopLevelId, name_id: NameId, context: &TopLevelContext,
) {
    let name = &context.names[name_id];
    if let Some(existing) = map.get(name) {
        let first_location = existing.location(db);
        let second_location = context.name_locations[name_id].clone();
        let name = name.clone();
        db.accumulate(Diagnostic::NameAlreadyInScope { name, first_location, second_location });
    } else {
        map.insert(name.clone(), TopLevelName::new(item_id, name_id));
    }
}

/// Collect all type definitions within a file (unfiltered by export list).
pub fn all_types_impl(context: &AllTypes, db: &DbHandle) -> Arc<TypeDefinitions> {
    incremental::enter_query();
    incremental::println(format!("Collecting all types in {:?}", context.0));

    let result = Parse(context.0).get(db);
    let mut definitions = TypeDefinitions::default();

    // Collect each definition, issuing an error if there is a duplicate name (imports are not counted)
    for item in result.cst.top_level_items.iter() {
        let item_context = &result.top_level_data[&item.id];

        // AbilityDefinitions are desugared into TypeDefinitions by `GetItem`, so treat them
        // as types here without needing the desugar step.
        let type_name = match &item.kind {
            TopLevelItemKind::TypeDefinition(definition) => definition.name,
            TopLevelItemKind::AbilityDefinition(ability) => ability.name,
            _ => continue,
        };

        insert_unique_name(db, &mut definitions, item.id, type_name, item_context);
    }

    incremental::exit_query();
    Arc::new(definitions)
}

/// Build the set of names listed in this file's `export` clause, or `None` if there's no
/// clause (in which case the file exports everything).
fn build_export_set(parse: &ParseResult) -> Option<HashSet<&Name>> {
    parse.cst.exports.as_ref().map(|exports| exports.iter().map(|(n, _)| n).collect())
}

/// Collect exported type definitions, filtered by the file's export list.
pub fn exported_types_impl(context: &ExportedTypes, db: &DbHandle) -> Arc<TypeDefinitions> {
    incremental::enter_query();
    let types = AllTypes(context.0).get(db);
    let parse = Parse(context.0).get(db);

    // If we knew on the export itself whether each item was a type or not we could skip
    // the AllTypes query and only require the export itself. Union variants & type constructors
    // make this impossible with Ante' current syntax however.
    let result = match build_export_set(&parse) {
        None => types,
        Some(export_set) => {
            let mut filtered = (*types).clone();
            filtered.retain(|name, _| export_set.contains(name));
            Arc::new(filtered)
        },
    };

    incremental::exit_query();
    result
}

/// Collect all definitions within a file (unfiltered by export list).
pub fn all_definitions_impl(context: &AllDefinitions, db: &DbHandle) -> Arc<VisibleDefinitionsResult> {
    incremental::enter_query();
    incremental::println(format!("Collecting all definitions in {:?}", context.0));

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
            TopLevelItemKind::AbilityDefinition(ability) => {
                for declaration in &ability.body {
                    declare_method(ability.name, declaration.name);
                }
            },
            _ => (),
        }

        // Ability methods are callable as free identifiers without qualifying by the ability name,
        // so also expose them in the regular definitions namespace.
        if let TopLevelItemKind::AbilityDefinition(ability) = &item.kind {
            let ctx = &result.top_level_data[&item.id];
            for declaration in &ability.body {
                declarer.declare_single(declaration.name, item.id, ctx);
            }
        }
    }

    incremental::exit_query();
    Arc::new(VisibleDefinitionsResult {
        definitions: declarer.definitions,
        methods: declarer.methods,
        imported_modules: BTreeMap::new(),
    })
}

/// Collect exported definitions, filtered by the file's export list.
/// If the file has no `export` statement, all definitions are exported.
pub fn exported_definitions_impl(context: &ExportedDefinitions, db: &DbHandle) -> Arc<VisibleDefinitionsResult> {
    incremental::enter_query();
    let all = AllDefinitions(context.0).get(db);
    let parse = Parse(context.0).get(db);

    let Some(export_set) = build_export_set(&parse) else {
        incremental::exit_query();
        return all;
    };

    let in_exports = |(name, _): &(&Name, &TopLevelName)| export_set.contains(name);
    let collect_functions = |items: &BTreeMap<Name, TopLevelName>| {
        items.iter().filter(in_exports).map(|(k, v)| (k.clone(), *v)).collect::<BTreeMap<_, _>>()
    };

    let definitions = collect_functions(&all.definitions);

    let methods = all.methods.iter().map(|(type_id, methods)| (*type_id, collect_functions(methods)));
    let methods = methods.filter(|(_, methods)| !methods.is_empty()).collect();

    incremental::exit_query();
    Arc::new(VisibleDefinitionsResult { definitions, methods, imported_modules: BTreeMap::new() })
}

/// Verify every name listed in this file's `export` statement is actually defined or imported
/// here, issuing `ExportedItemNotFound` diagnostics if not.
pub fn validate_exports_impl(context: &ValidateExports, db: &DbHandle) {
    incremental::enter_query();
    let parse = Parse(context.0).get(db);

    if let Some(exports) = &parse.cst.exports {
        let defs = AllDefinitions(context.0).get(db);
        let types = AllTypes(context.0).get(db);
        let import_names: HashSet<&Name> =
            parse.cst.imports.iter().flat_map(|i| i.items.iter().map(|(n, _)| n)).collect();

        for (name, location) in exports {
            let exists = defs.definitions.contains_key(name)
                || types.contains_key(name)
                || defs.methods.values().any(|m| m.contains_key(name))
                || import_names.contains(name);

            if !exists {
                let name = name.clone();
                let location = location.clone();
                db.accumulate(Diagnostic::ExportedItemNotFound { name, location });
            }
        }
    }

    incremental::exit_query();
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
            // `declare_names_in_pattern` is only for top-level patterns, which must be irrefutable
            Pattern::Or(_) => {
                let location = context.pattern_locations[pattern].clone();
                self.db.accumulate(Diagnostic::InvalidPattern { location });
            },
        }
    }

    fn declare_single(&mut self, name_id: NameId, id: TopLevelId, context: &TopLevelContext) {
        self.declare_single_helper(name_id, id, context, |this| &mut this.definitions);
    }

    fn declare_single_helper(
        &mut self, name_id: NameId, id: TopLevelId, context: &TopLevelContext,
        definitions: impl Fn(&mut Self) -> &mut Definitions,
    ) {
        insert_unique_name(self.db, definitions(self), id, name_id, context);
    }

    fn declare_method(
        &mut self, type_name_id: NameId, item_name_id: NameId, id: TopLevelId, context: &TopLevelContext,
    ) {
        let type_name = &context.names[type_name_id];

        // Methods can only be declared on a type declared in the same file, so look in the same file for the type.
        if let Some(object_type) = self.definitions.get(type_name) {
            let object_type = object_type.top_level_item;
            self.declare_single_helper(item_name_id, id, context, |this| this.methods.entry(object_type).or_default());
        } else if id.source_file == SourceFileId::prelude() {
            // Let the prelude define methods on builtin types
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

/// Helper function to collect all items in the program.
/// This function is discouraged since it limits parallelism but required for certain passes like
/// monomorphization which need access to the entire program.
///
/// TODO: Test performance
pub fn collect_all_items<Db>(compiler: &Db) -> Vec<TopLevelId>
where
    Db: DbGet<GetCrateGraph> + DbGet<Parse>,
{
    let mut items = Vec::new();

    for crate_ in GetCrateGraph.get(compiler).values() {
        for file in crate_.source_files.values() {
            let parse = Parse(*file).get(compiler);
            for item in parse.cst.top_level_items.iter() {
                items.push(item.id);
            }
        }
    }
    items
}
