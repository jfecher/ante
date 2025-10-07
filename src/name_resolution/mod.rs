use std::{collections::{BTreeMap, BTreeSet}, sync::Arc};

use namespace::{Namespace, SourceFileId, LOCAL_CRATE};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

pub mod builtin;
pub mod namespace;

use crate::{
    diagnostics::{Diagnostic, Location}, incremental::{
        self, DbHandle, ExportedTypes, GetCrateGraph, GetItem, Resolve, VisibleDefinitions, VisibleDefinitionsResult
    }, name_resolution::builtin::Builtin, parser::{
        context::TopLevelContext, cst::{
            Comptime, Constructor, Declaration, Definition, EffectDefinition, EffectType, Expr, Extern, Generics, ItemName, Path, Pattern, TopLevelItemKind, TraitDefinition, TraitImpl, Type, TypeDefinition, TypeDefinitionBody
        }, ids::{ExprId, NameId, PathId, PatternId, TopLevelId}
    }
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionResult {
    /// This resolution is for a single top level id so all expressions within are in the
    /// context of that id.
    pub path_origins: BTreeMap<PathId, Origin>,
    pub name_origins: BTreeMap<NameId, Origin>,

    /// Each other top-level item this item referenced. Used to build a dependency graph for type
    /// inference.
    pub referenced_items: BTreeSet<TopLevelId>,
}

struct Resolver<'local, 'inner> {
    item: TopLevelId,
    path_links: BTreeMap<PathId, Origin>,
    name_links: BTreeMap<NameId, Origin>,
    names_in_global_scope: Arc<VisibleDefinitionsResult>,
    names_in_local_scope: Vec<BTreeMap<Arc<String>, NameId>>,
    context: &'local TopLevelContext,
    compiler: &'local DbHandle<'inner>,
    referenced_items: BTreeSet<TopLevelId>,
}

/// Where was this variable defined?
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum Origin {
    /// This name comes from this top level definition
    TopLevelDefinition(TopLevelId),
    /// This name comes from a local binding (parameter, let-binding, match-binding, etc)
    Local(NameId),
    /// This name did not resolve, try to perform type based resolution on it during type inference
    TypeResolution,
    /// This name refers to a builtin item such as `String`, `Int`, `Unit`, `,` etc.
    Builtin(Builtin),
}

impl Origin {
    /// True if this Origin _may_ be a type. This does not have the proper context to check whether
    /// any internal IDs actually refer to types.
    pub fn is_type(self) -> bool {
        match self {
            Origin::TopLevelDefinition(_) | Origin::Local(_) => true,
            Origin::TypeResolution => false,
            Origin::Builtin(builtin) => matches!(
                builtin,
                Builtin::Unit | Builtin::Int | Builtin::Char | Builtin::Float | Builtin::String | Builtin::PairType
            ),
        }
    }
}

pub fn resolve_impl(context: &Resolve, compiler: &DbHandle) -> ResolutionResult {
    incremental::enter_query();
    let (statement, statement_ctx) = GetItem(context.0).get(compiler);
    incremental::println(format!("Resolving {:?}", statement.kind.name()));

    // Note that we discord errors here because they're errors for the entire file and we are
    // resolving just one statement in it. This does mean that `CompileFile` will later need to
    // manually query `VisibleDefinition` to pick these errors back up.
    let visible = VisibleDefinitions(context.0.source_file).get(compiler);
    let mut resolver = Resolver::new(compiler, context, visible, &statement_ctx);

    match statement.kind.name() {
        ItemName::Single(name_id) => resolver.link_existing_global(name_id),
        ItemName::Pattern(pattern) => resolver.link_existing_pattern(pattern),
        ItemName::None => (),
    }

    match &statement.kind {
        TopLevelItemKind::Definition(definition) => {
            resolver.link_existing_pattern(definition.pattern);
            resolver.resolve_expr(definition.rhs);
        },
        TopLevelItemKind::TypeDefinition(type_definition) => resolver.resolve_type_definition(type_definition),
        TopLevelItemKind::TraitDefinition(trait_definition) => resolver.resolve_trait_definition(trait_definition),
        TopLevelItemKind::TraitImpl(trait_impl) => resolver.resolve_trait_impl(trait_impl),
        TopLevelItemKind::EffectDefinition(effect_definition) => resolver.resolve_effect_definition(effect_definition),
        TopLevelItemKind::Extern(extern_) => resolver.resolve_extern(extern_),
        TopLevelItemKind::Comptime(comptime_) => resolver.resolve_comptime(comptime_),
    }

    incremental::exit_query();
    resolver.result()
}

impl<'local, 'inner> Resolver<'local, 'inner> {
    fn new(
        compiler: &'local DbHandle<'inner>, resolve: &Resolve, visible_definitions: Arc<VisibleDefinitionsResult>,
        context: &'local TopLevelContext,
    ) -> Self {
        Self {
            compiler,
            item: resolve.0,
            names_in_global_scope: visible_definitions,
            path_links: Default::default(),
            name_links: Default::default(),
            names_in_local_scope: vec![Default::default()],
            referenced_items: Default::default(),
            context,
        }
    }

    fn result(self) -> ResolutionResult {
        ResolutionResult { path_origins: self.path_links, name_origins: self.name_links, referenced_items: self.referenced_items }
    }

    #[allow(unused)]
    fn namespace(&self) -> Namespace {
        Namespace::Module(self.item.source_file)
    }

    fn push_local_scope(&mut self) {
        self.names_in_local_scope.push(Default::default());
    }

    /// TODO: Check for unused names
    fn pop_local_scope(&mut self) {
        self.names_in_local_scope.pop();
    }

    /// Declares a name in local scope.
    fn declare_name(&mut self, id: NameId) {
        let scope = self.names_in_local_scope.last_mut().unwrap();
        let name = self.context.names[id].clone();
        scope.insert(name, id);
        self.name_links.insert(id, Origin::Local(id));
    }

    /// Retrieve each visible namespace in the given namespace, restricting the namespace
    /// to only items visible from `self.namespace()`
    fn get_child_namespace(&self, name: &String, namespace: Namespace) -> Option<Namespace> {
        match namespace {
            Namespace::Local => {
                if let Some(submodule) = self.get_item_in_submodule(self.item.source_file, name) {
                    return Some(submodule);
                }

                let type_id = self.names_in_global_scope.definitions.get(name)?;
                let (item, _) = GetItem(*type_id).get(self.compiler);
                if matches!(&item.kind, TopLevelItemKind::TypeDefinition(_)) {
                    Some(Namespace::Type(*type_id))
                } else {
                    None
                }
            },
            Namespace::Type(_) => None,
            Namespace::Module(id) => {
                if let Some(submodule) = self.get_item_in_submodule(id, name) {
                    return Some(submodule);
                }

                let exported = ExportedTypes(id).get(self.compiler);
                exported.get(name).copied().map(Namespace::Type)
            },
        }
    }

    fn get_item_in_submodule(&self, parent_module: SourceFileId, name: &str) -> Option<Namespace> {
        parent_module.get(self.compiler).submodules.get(name).copied().map(Namespace::Module)
    }

    /// Retrieve each visible item in the given namespace, restricting the namespace
    /// to only items visible from `self.namespace()`
    fn get_item_in_namespace(&mut self, name: &String, namespace: Namespace) -> Option<Origin> {
        match namespace {
            this if this == self.namespace() => self.lookup_local_name(name),
            Namespace::Local => self.lookup_local_name(name),
            Namespace::Module(file_id) => {
                let visible = &VisibleDefinitions(file_id).get(self.compiler);
                let id = *visible.definitions.get(name)?;
                self.referenced_items.insert(id);
                Some(Origin::TopLevelDefinition(id))
            },
            Namespace::Type(top_level_id) => {
                let visible = &VisibleDefinitions(top_level_id.source_file).get(self.compiler);
                let methods = visible.methods.get(&top_level_id)?;
                let id = *methods.get(name)?;
                self.referenced_items.insert(id);
                Some(Origin::TopLevelDefinition(id))
            },
        }
    }

    /// Lookup the given path in the given namespace
    fn lookup_in<'a, Iter>(
        &mut self, mut path: Iter, mut namespace: Namespace, allow_type_based_resolution: bool,
    ) -> Result<Origin, Diagnostic>
    where
        Iter: ExactSizeIterator<Item = &'a (String, Location)>,
    {
        while path.len() > 1 {
            let (item_name, item_location) = path.next().unwrap();

            if let Some(next_namespace) = self.get_child_namespace(item_name, namespace) {
                namespace = next_namespace;
            } else {
                let name = item_name.clone();
                let location = item_location.clone();
                return Err(Diagnostic::NamespaceNotFound { name, location });
            }
        }

        let (name, location) = path.next().unwrap();
        assert_eq!(path.len(), 0);

        if matches!(namespace, Namespace::Local) {
            if let Some(origin) = self.lookup_local_name(name) {
                return Ok(origin);
            }
        }

        if let Some(origin) = self.get_item_in_namespace(name, namespace) {
            return Ok(origin);
        }

        // No known origin.
        // If the name is capitalized we delay until type inference to auto-import variants
        let first_char = name.chars().next().unwrap();
        if allow_type_based_resolution && first_char.is_ascii_uppercase() && namespace == Namespace::Local {
            Ok(Origin::TypeResolution)
        } else if let Some(origin) = self.lookup_builtin_name(name, !allow_type_based_resolution) {
            Ok(origin)
        } else {
            let location = location.clone();
            let name = Arc::new(name.clone());
            Err(Diagnostic::NameNotInScope { name, location })
        }
    }

    fn lookup_builtin_name(&self, name: &str, is_type: bool) -> Option<Origin> {
        Builtin::from_name(name, is_type).map(Origin::Builtin)
    }

    /// Lookup a single name (not a full path) in local scope
    fn lookup_local_name(&mut self, name: &String) -> Option<Origin> {
        for scope in self.names_in_local_scope.iter().rev() {
            if let Some(expr) = scope.get(name) {
                return Some(Origin::Local(*expr));
            }
        }

        if let Some(item) = self.names_in_global_scope.definitions.get(name) {
            self.referenced_items.insert(*item);
            return Some(Origin::TopLevelDefinition(*item));
        }
        None
    }

    fn lookup(&mut self, path: &Path, allow_type_based_resolution: bool) -> Result<Origin, Diagnostic> {
        let mut components = path.components.iter().peekable();

        if components.len() > 1 {
            let (first, _) = components.peek().unwrap();

            // Check if it is an absolute path
            let crates = GetCrateGraph.get(self.compiler);
            let local_crate = &crates[&LOCAL_CRATE];

            for dependency_id in &local_crate.dependencies {
                let dependency = &crates[dependency_id];

                if **first == dependency.name {
                    // Discard the crate name
                    components.next();
                    return self.lookup_in(components, Namespace::crate_(*dependency_id), allow_type_based_resolution);
                }
            }
        }

        // Not an absolute path
        self.lookup_in(components, Namespace::Local, allow_type_based_resolution)
    }

    /// Links a path to its definition or errors if it does not exist
    fn link(&mut self, path: PathId, allow_type_based_resolution: bool) {
        match self.lookup(&self.context.paths[path], allow_type_based_resolution) {
            Ok(origin) => {
                self.path_links.insert(path, origin);
            },
            Err(diagnostic) => self.emit_diagnostic(diagnostic),
        }
    }

    /// Link a global whose name is expected to be in `self.names_in_global_scope`
    fn link_existing_global(&mut self, name_id: NameId) {
        let name = &self.context.names[name_id];
        // panic safety: `name` should already be declared in global scope
        let id = self.names_in_global_scope.definitions[name];
        let origin = Origin::TopLevelDefinition(id);
        self.name_links.insert(name_id, origin);
    }

    /// Link a method whose name is expected to be in `self.names_in_global_scope`
    fn link_existing_method(&mut self, type_name: NameId, item_name: NameId) {
        let item_name_string = &self.context.names[item_name];
        let type_name_string = &self.context.names[type_name];

        // panic safety: `type_name` should already be declared in global scope
        let type_id = self.names_in_global_scope.definitions[type_name_string];

        let methods = &self.names_in_global_scope.methods[&type_id];
        let method = methods[item_name_string];
        self.name_links.insert(type_name, Origin::TopLevelDefinition(type_id));
        self.name_links.insert(item_name, Origin::TopLevelDefinition(method));
    }

    fn link_existing_pattern(&mut self, pattern: PatternId) {
        match &self.context.patterns[pattern] {
            Pattern::Error => (),
            // The only literal pattern allowed in a global's name is `()` which has nothing to link
            Pattern::Literal(_) => (),
            Pattern::Variable(name_id) => self.link_existing_global(*name_id),
            Pattern::Constructor(constructor, args) => {
                self.link(*constructor, false);
                for arg in args {
                    self.link_existing_pattern(*arg);
                }
            },
            Pattern::TypeAnnotation(pattern, typ) => {
                self.link_existing_pattern(*pattern);
                self.resolve_type(typ, false);
            },
            Pattern::MethodName { type_name, item_name } => self.link_existing_method(*type_name, *item_name),
        }
    }

    fn emit_diagnostic(&self, diagnostic: Diagnostic) {
        self.compiler.accumulate(diagnostic);
    }

    fn resolve_expr(&mut self, expr: ExprId) {
        match &self.context.exprs[expr] {
            Expr::Literal(_literal) => (),
            Expr::Variable(path) => self.link(*path, true),
            Expr::Call(call) => {
                self.resolve_expr(call.function);
                for arg in &call.arguments {
                    self.resolve_expr(*arg);
                }
            },
            Expr::Lambda(lambda) => {
                // Resolve body with the parameter name in scope
                self.push_local_scope();
                for parameter in &lambda.parameters {
                    self.declare_names_in_pattern(parameter.pattern, true, false);
                }
                if let Some(return_type) = &lambda.return_type {
                    self.resolve_type(return_type, true);
                }
                self.resolve_expr(lambda.body);
                self.pop_local_scope();
            },
            Expr::Sequence(sequence) => {
                self.push_local_scope();
                for item in sequence {
                    self.resolve_expr(item.expr);
                }
                self.pop_local_scope();
            },
            Expr::Definition(definition) => self.resolve_definition(definition),
            Expr::MemberAccess(access) => {
                self.resolve_expr(access.object);
            },
            Expr::Index(index) => {
                self.resolve_expr(index.object);
                self.resolve_expr(index.index);
            },
            Expr::If(if_) => {
                self.resolve_expr(if_.condition);

                self.push_local_scope();
                self.resolve_expr(if_.then);
                self.pop_local_scope();

                if let Some(else_) = if_.else_ {
                    self.push_local_scope();
                    self.resolve_expr(else_);
                    self.pop_local_scope();
                }
            },
            Expr::Match(match_) => {
                self.resolve_expr(match_.expression);
                for (pattern, branch) in &match_.cases {
                    self.push_local_scope();
                    self.declare_names_in_pattern(*pattern, false, true);
                    self.resolve_expr(*branch);
                    self.pop_local_scope();
                }
            },
            Expr::Handle(_handle) => (), //TODO
            Expr::Reference(reference) => {
                self.resolve_expr(reference.rhs);
            },
            Expr::TypeAnnotation(type_annotation) => {
                self.resolve_expr(type_annotation.lhs);
                self.resolve_type(&type_annotation.rhs, false);
            },
            Expr::Constructor(constructor) => self.resolve_constructor(constructor, expr),
            Expr::Quoted(_) => (),
            Expr::Error => (),
        }
    }

    fn resolve_definition(&mut self, definition: &Definition) {
        let is_let_rec = matches!(&self.context.exprs[definition.rhs], Expr::Lambda(_));
        if is_let_rec {
            self.declare_names_in_pattern(definition.pattern, true, false);
        }

        self.resolve_expr(definition.rhs);

        if !is_let_rec {
            self.declare_names_in_pattern(definition.pattern, false, false);
        }
    }

    fn resolve_constructor(&mut self, constructor: &Constructor, id: ExprId) {
        self.resolve_type(&constructor.typ, false);

        // Ensure all fields of the type are used exactly once
        match self.get_fields_of_type(&constructor.typ) {
            FieldsResult::Fields(names) => {
                let mut given_fields = BTreeSet::default();
                let mut already_defined = FxHashMap::default();

                for (pattern, _) in &constructor.fields {
                    pattern.for_each_variable(self.context, &mut |name_id| {
                        let name = self.context.names[name_id].clone();
                        let location = self.context.name_locations[name_id].clone();

                        if let Some(first_location) = already_defined.get(&name).cloned() {
                            let second_location = location;
                            self.emit_diagnostic(Diagnostic::ConstructorFieldDuplicate { name, first_location, second_location });
                            return;
                        }

                        already_defined.insert(name.clone(), location.clone());

                        if !names.contains(&name) {
                            let typ = constructor.typ.display(self.context).to_string();
                            self.emit_diagnostic(Diagnostic::ConstructorNoSuchField { name, typ, location });
                        } else {
                            given_fields.insert(name);
                        }
                    });
                }

                let missing_fields = names.difference(&given_fields).map(ToString::to_string).collect::<Vec<_>>();
                if !missing_fields.is_empty() {
                    let location = self.context.expr_locations[id].clone();
                    self.emit_diagnostic(Diagnostic::ConstructorMissingFields { missing_fields, location });
                }
            },
            // We already issued an error when failing to resolve the path
            // of this type, avoid issuing another.
            FieldsResult::PriorError => (),
            FieldsResult::NotAStruct => {
                let typ = constructor.typ.display(self.context).to_string();
                let location = self.context.expr_locations[id].clone();
                self.emit_diagnostic(Diagnostic::ConstructorNotAStruct { typ, location });
            },
        }

        for (_, expr) in &constructor.fields {
            self.resolve_expr(*expr);
        }
    }

    /// If the given type is a struct type, return its fields. Otherwise return None.
    fn get_fields_of_type(&self, typ: &Type) -> FieldsResult {
        match typ {
            Type::Named(path) => {
                match self.path_links.get(path) {
                    Some(Origin::TopLevelDefinition(typ)) => {
                        let (item, item_context) = GetItem(*typ).get(self.compiler);
                        match &item.kind {
                            TopLevelItemKind::TypeDefinition(type_definition) => {
                                match &type_definition.body {
                                    TypeDefinitionBody::Error => FieldsResult::PriorError,
                                    TypeDefinitionBody::Enum(_) => FieldsResult::NotAStruct,
                                    TypeDefinitionBody::Alias(_) => todo!("get_fields_of_type: handle type aliases"),
                                    TypeDefinitionBody::Struct(fields) => {
                                        let names = fields.iter().map(|(name, _)| item_context.names[*name].clone());
                                        FieldsResult::Fields(names.collect())
                                    },
                                }
                            },
                            _ => FieldsResult::NotAStruct,
                        }
                    },
                    Some(_) => FieldsResult::NotAStruct,
                    None => FieldsResult::PriorError,
                }
            },
            // NOTE: Once type aliases are added, the fields of an alias may depend
            // on its generic arguments
            Type::Application(typ, _) => self.get_fields_of_type(typ),
            _ => FieldsResult::NotAStruct,
        }
    }

    /// Declare each name in a pattern position in the given pattern, pushing the old names
    /// if any existed in the declared list.
    ///
    /// If `declare_type_vars` is true, any type variables used that are not in scope will
    /// automatically be declared. Otherwise an error will be issued.
    fn declare_names_in_pattern(
        &mut self, pattern: PatternId, declare_type_vars: bool, allow_type_based_resolution: bool,
    ) {
        match &self.context.patterns[pattern] {
            Pattern::Variable(name) => {
                self.declare_name(*name);
            },
            Pattern::Literal(_) => (),
            // In a constructor pattern such as `Struct foo bar baz` or `(a, b)` the arguments
            // should be declared but the function itself should never be.
            Pattern::Constructor(function, args) => {
                self.link(*function, allow_type_based_resolution);
                for arg in args {
                    self.declare_names_in_pattern(*arg, declare_type_vars, allow_type_based_resolution);
                }
            },
            Pattern::Error => (),
            Pattern::TypeAnnotation(pattern, typ) => {
                self.declare_names_in_pattern(*pattern, declare_type_vars, allow_type_based_resolution);
                self.resolve_type(typ, declare_type_vars);
            },
            Pattern::MethodName { type_name, item_name } => {
                self.resolve_variable(*type_name, false);
                self.declare_name(*item_name);
            },
        }
    }

    /// Resolves a type ensuring all names used are in scope and issuing errors
    /// for any that are not. If `declare_type_vars` is set then any type variables
    /// not already in scope will be declared in the current local scope. Otherwise,
    /// an error will be issued.
    fn resolve_type(&mut self, typ: &Type, declare_type_vars: bool) {
        match typ {
            Type::Error | Type::Unit | Type::Integer(_) | Type::Float(_) | Type::String | Type::Char | Type::Pair | Type::Reference(..) => (),
            Type::Named(path) => self.link(*path, false),
            Type::Variable(name) => self.resolve_variable(*name, declare_type_vars),
            Type::Function(function) => {
                for parameter in &function.parameters {
                    self.resolve_type(parameter, declare_type_vars);
                }
                self.resolve_type(&function.return_type, declare_type_vars);

                if let Some(effects) = function.effects.as_ref() {
                    for effect in effects {
                        self.resolve_effect_type(effect, declare_type_vars);
                    }
                }
            },
            Type::Application(f, args) => {
                self.resolve_type(f, declare_type_vars);
                for arg in args {
                    self.resolve_type(arg, declare_type_vars);
                }
            },
        }
    }

    /// Resolve an effect type, ensuring all names used are in scope
    fn resolve_effect_type(&mut self, effect: &EffectType, declare_type_vars: bool) {
        match effect {
            EffectType::Known(path, args) => {
                self.link(*path, false);

                for arg in args {
                    self.resolve_type(arg, declare_type_vars);
                }
            },
            EffectType::Variable(name_id) => self.resolve_variable(*name_id, declare_type_vars),
        }
    }

    /// If `auto_declare` is true, automatically declare the name if not found instead of issuing
    /// an error.
    fn resolve_variable(&mut self, name_id: NameId, auto_declare: bool) {
        let name = &self.context.names[name_id];

        if let Some(origin) = self.lookup_local_name(name) {
            self.name_links.insert(name_id, origin);
        } else if auto_declare {
            self.declare_name(name_id);
        } else {
            let location = self.context.name_locations[name_id].clone();
            let name = self.context.names[name_id].clone();
            self.emit_diagnostic(Diagnostic::NameNotFound { name, location });
        }
    }

    fn resolve_type_definition(&mut self, type_definition: &TypeDefinition) {
        self.declare_generics(&type_definition.generics);

        match &type_definition.body {
            TypeDefinitionBody::Error => (),
            TypeDefinitionBody::Struct(fields) => {
                for (_name, field_type) in fields {
                    self.resolve_type(field_type, false);
                }
            },
            TypeDefinitionBody::Enum(variants) => {
                for (name, variant_args) in variants {
                    self.link_existing_method(type_definition.name, *name);
                    for arg in variant_args {
                        self.resolve_type(arg, false);
                    }
                }
            },
            TypeDefinitionBody::Alias(typ) => {
                self.resolve_type(typ, false);
            },
        }
    }

    fn declare_generics(&mut self, generics: &Generics) {
        for generic in generics {
            self.declare_name(*generic);
        }
    }

    fn declare(&mut self, declaration: &Declaration) {
        self.declare_name(declaration.name);
        self.resolve_type(&declaration.typ, true);
    }

    fn resolve_trait_definition(&mut self, trait_definition: &TraitDefinition) {
        self.declare_generics(&trait_definition.generics);
        self.declare_generics(&trait_definition.functional_dependencies);
        for declaration in &trait_definition.body {
            self.declare(declaration);
        }
    }

    fn resolve_trait_impl(&mut self, trait_impl: &TraitImpl) {
        self.link(trait_impl.trait_path, false);

        for arg in &trait_impl.trait_arguments {
            self.resolve_type(arg, true);
        }

        for definition in &trait_impl.body {
            self.resolve_definition(definition);
        }
    }

    fn resolve_effect_definition(&mut self, effect_definition: &EffectDefinition) {
        self.declare_generics(&effect_definition.generics);
        for declaration in &effect_definition.body {
            self.declare(declaration);
        }
    }

    fn resolve_extern(&mut self, extern_: &Extern) {
        self.declare(&extern_.declaration);
    }

    /// Does this require special handling? This should be resolved before runtime
    /// definitions are resolved.
    fn resolve_comptime(&mut self, comptime: &Comptime) {
        match comptime {
            Comptime::Expr(expr_id) => self.resolve_expr(*expr_id),
            Comptime::Derive(paths) => {
                for path in paths {
                    self.link(*path, false);
                }
            },
            Comptime::Definition(definition) => self.resolve_definition(definition),
        }
    }
}

enum FieldsResult {
    Fields(BTreeSet<Arc<String>>),
    /// A prior error occurred, avoid issuing another
    PriorError,
    NotAStruct,
}
