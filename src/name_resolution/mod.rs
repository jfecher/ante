use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use namespace::{CRATE_ROOT_MODULE, Namespace, SourceFileId};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

pub mod builtin;
pub mod namespace;

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::{
        self, DbHandle, ExportedTypes, GetCrateGraph, GetItem, Resolve, VisibleDefinitions, VisibleDefinitionsResult,
    },
    iterator_extensions::mapvec,
    name_resolution::{builtin::Builtin, namespace::CrateId},
    parser::{
        cst::{
            Comptime, Constructor, Definition, Expr, Generics, Handle, ItemName, Name, Path, Pattern, TopLevelItemKind,
            Type, TypeDefinition, TypeDefinitionBody, TypeKind,
        },
        desugar_context::DesugarContext,
        ids::{ExprId, NameId, PathId, PatternId, TopLevelId, TopLevelName},
    },
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

    /// Each name defined by this top-level item that may be visible externally. This includes
    /// names that are directly visible such as `a, b` in `a, b = 1, 2` but also names that are
    /// visible in any namespace exported by this item, such as `Foo` and `Bar` in
    /// `type Union = | Foo | Bar` which are normally accessed via `Union.Foo` and `Union.Bar`.
    pub top_level_names: Vec<NameId>,
}

struct Resolver<'local, 'inner> {
    item: TopLevelId,
    path_links: BTreeMap<PathId, Origin>,
    name_links: BTreeMap<NameId, Origin>,
    names_in_global_scope: Arc<VisibleDefinitionsResult>,
    names_in_local_scope: Vec<BTreeMap<Name, NameId>>,
    context: &'local DesugarContext,
    compiler: &'local DbHandle<'inner>,
    referenced_items: BTreeSet<TopLevelId>,
    top_level_names: Vec<NameId>,

    /// Nesting depth of enclosing `while`/`for` loops. Used to reject `break`/`continue` outside of loops.
    loop_depth: u32,
}

/// Where was this variable defined?
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum Origin {
    /// This name comes from another top level definition
    /// The `NameId` here is local to the given top-level definition, using it in another context
    /// is always a bug.
    TopLevelDefinition(TopLevelName),
    /// This name comes from a local binding (parameter, let-binding, match-binding, etc)
    Local(NameId),
    /// This name did not resolve, try to perform type based resolution on it during type inference
    TypeResolution,
    /// This name refers to a builtin item such as `Unit`, `Char`, etc.
    Builtin(Builtin),
}

impl Origin {
    /// True if this Origin _may_ be a type. This does not have the proper context to check whether
    /// any internal IDs actually refer to types.
    pub fn may_be_a_type(self) -> bool {
        match self {
            Origin::TopLevelDefinition(..) | Origin::Local(_) => true,
            Origin::TypeResolution => false,
            Origin::Builtin(builtin) => matches!(builtin, Builtin::Unit | Builtin::Char),
        }
    }

    /// Return the fields of this type
    fn get_fields_of_type(self, db: &DbHandle) -> FieldsResult {
        match self {
            Origin::TopLevelDefinition(id) => {
                let (item, item_context) = GetItem(id.top_level_item).get(db);
                match &item.kind {
                    TopLevelItemKind::TypeDefinition(type_definition) => match &type_definition.body {
                        TypeDefinitionBody::Error => FieldsResult::PriorError,
                        TypeDefinitionBody::Enum(_) => FieldsResult::NotAStruct,
                        TypeDefinitionBody::Alias(_) => todo!("get_fields_of_type: handle type aliases"),
                        TypeDefinitionBody::Struct(fields) => {
                            let names = fields.iter().map(|(name, _)| item_context[*name].clone());
                            FieldsResult::Fields(names.collect())
                        },
                    },
                    _ => FieldsResult::NotAStruct,
                }
            },
            _ => FieldsResult::NotAStruct,
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
    let mut resolver = Resolver::new(compiler, context, visible, statement_ctx.as_ref());

    match statement.kind.name() {
        ItemName::Single(name_id) => resolver.link_existing_global(name_id),
        ItemName::Pattern(pattern) => resolver.link_existing_pattern(pattern),
        ItemName::None => (),
    }

    match &statement.kind {
        TopLevelItemKind::Definition(definition) => resolver.resolve_expr(definition.rhs),
        TopLevelItemKind::TypeDefinition(type_definition) => resolver.resolve_type_definition(type_definition),
        TopLevelItemKind::Comptime(comptime_) => resolver.resolve_comptime(comptime_),
        TopLevelItemKind::TraitDefinition(_) => unreachable!("Desugared by GetItem"),
        TopLevelItemKind::EffectDefinition(_) => unreachable!("Desugared by GetItem"),
        TopLevelItemKind::TraitImpl(_) => unreachable!("Desugared by GetItem"),
    }

    incremental::exit_query();
    resolver.result()
}

impl<'local, 'inner> Resolver<'local, 'inner> {
    fn new(
        compiler: &'local DbHandle<'inner>, resolve: &Resolve, visible_definitions: Arc<VisibleDefinitionsResult>,
        context: &'local DesugarContext,
    ) -> Self {
        Self {
            compiler,
            item: resolve.0,
            names_in_global_scope: visible_definitions,
            path_links: Default::default(),
            name_links: Default::default(),
            names_in_local_scope: vec![Default::default()],
            referenced_items: Default::default(),
            top_level_names: Vec::new(),
            loop_depth: 0,
            context,
        }
    }

    fn result(self) -> ResolutionResult {
        ResolutionResult {
            path_origins: self.path_links,
            name_origins: self.name_links,
            referenced_items: self.referenced_items,
            top_level_names: self.top_level_names,
        }
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
        let name = self.context[id].clone();
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

                if let Some(&module_id) = self.names_in_global_scope.imported_modules.get(name) {
                    return Some(Namespace::Module(module_id));
                }

                let type_id = self.names_in_global_scope.definitions.get(name)?;
                let (item, _) = GetItem(type_id.top_level_item).get(self.compiler);
                if matches!(&item.kind, TopLevelItemKind::TypeDefinition(_)) {
                    Some(Namespace::Type(type_id.top_level_item))
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
                let (id, _kind) = exported.get(name)?;
                Some(Namespace::Type(id.top_level_item))
            },
        }
    }

    fn get_item_in_submodule(&self, parent_module: SourceFileId, name: &str) -> Option<Namespace> {
        if parent_module.local_module_id == CRATE_ROOT_MODULE {
            let crates = GetCrateGraph.get(self.compiler);
            let crate_ = crates.get(&parent_module.crate_id)?;
            let module_file = std::path::PathBuf::from(name).with_extension("an");

            // TODO: This should be a relative lookup, not an absolute one in the current crate
            // TODO: calling `parent_module.get()` can panic if the parent module is not a valid
            //       source file to begin with. We should ensure it is always valid.
            if let Some(id) = crate_.source_files.get(&module_file).copied() {
                return Some(Namespace::Module(id));
            }

            // Fall back to absolute path (crate_root/src/Vec.an)
            let absolute = crate_.path.join("src").join(&module_file);
            if let Some(id) = crate_.source_files.get(&absolute).copied() {
                return Some(Namespace::Module(id));
            }

            None
        } else {
            parent_module.get(self.compiler).submodules.get(name).copied().map(Namespace::Module)
        }
    }

    /// Retrieve each visible item in the given namespace, restricting the namespace
    /// to only items visible from `self.namespace()`
    fn get_item_in_namespace(&mut self, name: &String, namespace: Namespace) -> Option<Origin> {
        match namespace {
            this if this == self.namespace() => self.lookup_local_name(name),
            Namespace::Local => self.lookup_local_name(name),
            Namespace::Module(file_id) => {
                let visible = &VisibleDefinitions(file_id).get(self.compiler);
                if let Some(&id) = visible.definitions.get(name) {
                    self.referenced_items.insert(id.top_level_item);
                    return Some(Origin::TopLevelDefinition(id));
                }
                // Also check methods defined on types in this module.
                // This just removes the need to type `Std.Vec.Vec.push` over `Std.Vec.push`
                for methods in visible.methods.values() {
                    if let Some(&id) = methods.get(name) {
                        self.referenced_items.insert(id.top_level_item);
                        return Some(Origin::TopLevelDefinition(id));
                    }
                }
                None
            },
            Namespace::Type(top_level_id) => {
                let visible = &VisibleDefinitions(top_level_id.source_file).get(self.compiler);
                let methods = visible.methods.get(&top_level_id)?;
                let id = *methods.get(name)?;
                self.referenced_items.insert(id.top_level_item);
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
        } else if let Some(origin) = self.lookup_builtin_name(name) {
            Ok(origin)
        // Ad-hoc check to define `intrinsic` only within the stdlib for compiler intrinsics
        } else if namespace == Namespace::Local
            && self.item.source_file.crate_id == CrateId::STDLIB
            && name == "intrinsic"
        {
            Ok(Origin::Builtin(Builtin::Intrinsic))
        } else {
            let location = location.clone();
            let name = Arc::new(name.clone());
            Err(Diagnostic::NameNotInScope { name, location })
        }
    }

    fn lookup_builtin_name(&self, name: &str) -> Option<Origin> {
        Builtin::from_name(name).map(Origin::Builtin)
    }

    /// Lookup a single name (not a full path) in local scope
    fn lookup_local_name(&mut self, name: &String) -> Option<Origin> {
        for scope in self.names_in_local_scope.iter().rev() {
            if let Some(expr) = scope.get(name) {
                return Some(Origin::Local(*expr));
            }
        }

        if let Some(id) = self.names_in_global_scope.definitions.get(name) {
            self.referenced_items.insert(id.top_level_item);
            return Some(Origin::TopLevelDefinition(*id));
        }
        None
    }

    fn lookup(&mut self, path: &Path, allow_type_based_resolution: bool) -> Result<Origin, Diagnostic> {
        let mut components = path.components.iter().peekable();

        if components.len() > 1 {
            let (first, _) = components.peek().unwrap();

            // Check if it is an absolute path
            let crates = GetCrateGraph.get(self.compiler);
            let local_crate = &crates[&CrateId::LOCAL];

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
    fn link(&mut self, path: PathId, allow_type_based_resolution: bool, is_type: bool) {
        match self.lookup(&self.context[path], allow_type_based_resolution) {
            Ok(origin) => {
                if !self.is_valid_for_position(origin, is_type) {
                    let last = self.context[path].components.last().unwrap();
                    let location = self.context.path_location(path).clone();
                    if is_type {
                        let name = Arc::new(last.0.clone());
                        self.emit_diagnostic(Diagnostic::TypeExpected { name, location });
                    } else {
                        let typ = Arc::new(last.0.clone());
                        self.emit_diagnostic(Diagnostic::ValueExpected { location, typ });
                    }
                }
                self.path_links.insert(path, origin);
            },
            Err(diagnostic) => self.emit_diagnostic(diagnostic),
        }
    }

    /// Link a global whose name is expected to be in `self.names_in_global_scope`
    fn link_existing_global(&mut self, name_id: NameId) {
        let name = &self.context[name_id];
        // panic safety: `name` should already be declared in global scope
        let id = self.names_in_global_scope.definitions[name];
        let origin = Origin::TopLevelDefinition(id);
        self.top_level_names.push(name_id);
        self.name_links.insert(name_id, origin);
    }

    /// Link a method whose name is expected to be in `self.names_in_global_scope`
    fn link_existing_union_variant(&mut self, type_name: NameId, item_name: NameId) {
        let type_name_string = &self.context[type_name];
        let item_name_string = &self.context[item_name];

        let Some(&type_id) = self.names_in_global_scope.definitions.get(type_name_string) else {
            // Definition collection / parse error
            return;
        };

        if let Some(methods) = &self.names_in_global_scope.methods.get(&type_id.top_level_item) {
            if let Some(method) = methods.get(item_name_string) {
                self.top_level_names.push(item_name);
                self.name_links.insert(type_name, Origin::TopLevelDefinition(type_id));
                self.name_links.insert(item_name, Origin::TopLevelDefinition(*method));
            }
        } else {
            println!(
                "Warning: expected existing union variant {type_name_string}.{item_name_string} to be declared but it is not"
            );
        }
    }

    fn link_existing_pattern(&mut self, pattern: PatternId) {
        match &self.context[pattern] {
            Pattern::Error => (),
            // The only literal pattern allowed in a global's name is `()` which has nothing to link
            Pattern::Literal(_) => (),
            Pattern::Variable(name_id) => self.link_existing_global(*name_id),
            Pattern::Constructor(constructor, args) => {
                self.link(*constructor, false, false);
                for arg in args {
                    self.link_existing_pattern(*arg);
                }
            },
            Pattern::TypeAnnotation(pattern, typ) => {
                self.link_existing_pattern(*pattern);
                self.resolve_type(typ, true);
            },
            Pattern::MethodName { type_name, item_name } => self.link_existing_union_variant(*type_name, *item_name),
        }
    }

    fn emit_diagnostic(&self, diagnostic: Diagnostic) {
        self.compiler.accumulate(diagnostic);
    }

    fn is_valid_for_position(&self, origin: Origin, is_type: bool) -> bool {
        match origin {
            // TypeResolution is always a value (deferred enum/struct constructor), never a type
            Origin::TypeResolution => !is_type,
            // Local names (type vars or value bindings) are accepted in either position
            Origin::Local(_) => true,
            Origin::TopLevelDefinition(name) => {
                let (item, _) = GetItem(name.top_level_item).get(self.compiler);
                match &item.kind {
                    TopLevelItemKind::TypeDefinition(def) if name.local_name_id == def.name => {
                        match &def.body {
                            // Struct type names may be used as a type OR value constructor
                            TypeDefinitionBody::Struct(_) | TypeDefinitionBody::Error => true,
                            // Union (enum) type name is only valid as a type; its variants are handled below
                            TypeDefinitionBody::Enum(_) | TypeDefinitionBody::Alias(_) => is_type,
                        }
                    },
                    // Enum variants are value constructors but accepted in either position
                    // (kind/type checking validates appropriate use later)
                    TopLevelItemKind::TypeDefinition(_) => true,
                    // Effect type name: only valid in type position
                    TopLevelItemKind::EffectDefinition(def) if name.local_name_id == def.name => is_type,
                    // Effect operations/methods: only valid in value position
                    TopLevelItemKind::EffectDefinition(_) => !is_type,
                    // All other items (Definition, Extern, TraitImpl, Comptime) are values
                    _ => !is_type,
                }
            },
            Origin::Builtin(b) => is_type == b.as_type().is_some(),
        }
    }

    fn resolve_expr(&mut self, expr: ExprId) {
        match &self.context[expr] {
            Expr::Literal(_literal) => (),
            Expr::Variable(path) => self.link(*path, true, false),
            Expr::Call(call) => {
                self.resolve_expr(call.function);
                for arg in &call.arguments {
                    self.resolve_expr(arg.expr);
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
            Expr::Is(_) => unreachable!("Expr::Is should be desugared during GetItem"),
            Expr::Do(_) => unreachable!("Expr::Do should be desugared during GetItem"),
            Expr::Handle(handle) => {
                // Handle's expression & branches will be lambdas which will push their
                // own scopes & introduce their patterns themselves.
                self.push_local_scope();
                self.declare_name(handle.handler_name);
                self.resolve_expr(handle.expression);
                self.pop_local_scope();
                self.check_handler_methods(handle, expr);
            },
            Expr::Reference(reference) => {
                self.resolve_expr(reference.rhs);
            },
            Expr::TypeAnnotation(type_annotation) => {
                self.resolve_expr(type_annotation.lhs);
                self.resolve_type(&type_annotation.rhs, false);
            },
            Expr::Constructor(constructor) => self.resolve_constructor(constructor, expr),
            Expr::Loop(_) => unreachable!("Loops should be desugared before name resolution"),
            Expr::While(while_) => {
                self.loop_depth += 1;
                self.resolve_expr(while_.condition);
                self.push_local_scope();
                self.resolve_expr(while_.body);
                self.loop_depth -= 1;
                self.pop_local_scope();
            },
            Expr::For(for_) => {
                self.resolve_expr(for_.start);
                self.resolve_expr(for_.end);
                self.push_local_scope();
                self.declare_name(for_.variable);
                self.loop_depth += 1;
                self.resolve_expr(for_.body);
                self.loop_depth -= 1;
                self.pop_local_scope();
            },
            Expr::Break => self.check_break_or_continue(true, expr),
            Expr::Continue => self.check_break_or_continue(false, expr),
            Expr::Quoted(_) => (),
            Expr::Return(return_) => self.resolve_expr(return_.expression),
            Expr::Assignment(assignment) => {
                // Mutability of the lhs is left to the type checker to check
                self.resolve_expr(assignment.lhs);
                self.resolve_expr(assignment.rhs);
                if let Some((_, op_expr)) = assignment.op {
                    self.resolve_expr(op_expr);
                }
            },
            Expr::Error => (),
            Expr::Extern(_) => (),
            Expr::InterpolatedString(_) => {
                unreachable!("InterpolatedString should be desugared before name resolution")
            },
            Expr::ArrayLiteral(elements) => {
                let elements = elements.clone();
                for element in elements {
                    self.resolve_expr(element);
                }
            },
        }
    }

    fn check_break_or_continue(&mut self, is_break: bool, id: ExprId) {
        if self.loop_depth == 0 {
            let location = self.context.expr_location(id).clone();
            if is_break {
                self.compiler.accumulate(Diagnostic::BreakNotInLoop { location });
            } else {
                self.compiler.accumulate(Diagnostic::ContinueNotInLoop { location });
            }
        }
    }

    fn resolve_definition(&mut self, definition: &Definition) {
        let is_lambda = matches!(&self.context[definition.rhs], Expr::Lambda(_));
        if is_lambda {
            // Lambda definitions can call themselves recursively, so the name must be in scope
            // before resolving the body.
            self.declare_names_in_pattern(definition.pattern, true, false);
        }
        // TODO: Type variables declared in pattern type annotations should be in scope for the rhs,
        // but the value variable itself should not see itself in its own rhs.
        self.resolve_expr(definition.rhs);
        if !is_lambda {
            self.declare_names_in_pattern(definition.pattern, true, false);
        }
    }

    fn resolve_constructor(&mut self, constructor: &Constructor, id: ExprId) {
        self.resolve_type(&constructor.typ, false);

        // Ensure all fields of the type are used exactly once
        match self.get_fields_of_type(&constructor.typ) {
            FieldsResult::Fields(names) => {
                let mut given_fields = BTreeSet::default();
                let mut already_defined = FxHashMap::default();

                for (name_id, _) in &constructor.fields {
                    let name = self.context[*name_id].clone();
                    let location = self.context.name_location(*name_id).clone();

                    if let Some(first_location) = already_defined.get(&name).cloned() {
                        let second_location = location;
                        self.emit_diagnostic(Diagnostic::ConstructorFieldDuplicate {
                            name,
                            first_location,
                            second_location,
                        });
                        return;
                    }

                    already_defined.insert(name.clone(), location.clone());

                    if !names.contains(&name) {
                        let typ = constructor.typ.display(self.context.as_top_level_context()).to_string();
                        self.emit_diagnostic(Diagnostic::NoSuchFieldForType { name, typ, location });
                    } else {
                        given_fields.insert(name);
                    }
                }

                let missing_fields = names.difference(&given_fields).map(ToString::to_string).collect::<Vec<_>>();
                if !missing_fields.is_empty() {
                    let location = self.context.expr_location(id).clone();
                    self.emit_diagnostic(Diagnostic::ConstructorMissingFields { missing_fields, location });
                }
            },
            // We already issued an error when failing to resolve the path
            // of this type, avoid issuing another.
            FieldsResult::PriorError => (),
            FieldsResult::NotAStruct => {
                let typ = constructor.typ.display(self.context.as_top_level_context()).to_string();
                let location = self.context.expr_location(id).clone();
                self.emit_diagnostic(Diagnostic::ConstructorNotAStruct { typ, location });
            },
        }

        for (_, expr) in &constructor.fields {
            self.resolve_expr(*expr);
        }
    }

    /// If the given type is a struct type, return its fields. Otherwise return None.
    fn get_fields_of_type(&self, typ: &Type) -> FieldsResult {
        match &typ.kind {
            TypeKind::Named(path) => match self.path_links.get(path) {
                Some(origin) => origin.get_fields_of_type(self.compiler),
                None => FieldsResult::PriorError,
            },
            // NOTE: Once type aliases are added, the fields of an alias may depend
            // on its generic arguments
            TypeKind::Application(typ, _) => self.get_fields_of_type(typ),
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
        match &self.context[pattern] {
            Pattern::Variable(name) => {
                self.declare_name(*name);
            },
            Pattern::Literal(_) => (),
            // In a constructor pattern such as `Struct foo bar baz` or `(a, b)` the arguments
            // should be declared but the function itself should never be.
            Pattern::Constructor(function, args) => {
                self.link(*function, allow_type_based_resolution, false);
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
        match &typ.kind {
            TypeKind::Error
            | TypeKind::Unit
            | TypeKind::Integer(_)
            | TypeKind::Float(_)
            | TypeKind::Char
            | TypeKind::NoClosureEnv
            | TypeKind::Pointer
            | TypeKind::Hole
            | TypeKind::Reference(..)
            | TypeKind::IntegerConstant(_) => (),
            TypeKind::Named(path) => self.link(*path, false, true),
            TypeKind::Variable(name) => self.resolve_variable(*name, declare_type_vars),
            TypeKind::Function(function) => {
                for parameter in &function.parameters {
                    self.resolve_type(&parameter.typ, declare_type_vars);
                }
                if let Some(environment) = function.environment.as_ref() {
                    self.resolve_type(environment, declare_type_vars);
                }
                self.resolve_type(&function.return_type, declare_type_vars);
            },
            TypeKind::Application(f, args) => {
                self.resolve_type(f, declare_type_vars);
                for arg in args {
                    self.resolve_type(arg, declare_type_vars);
                }
            },
            TypeKind::Tuple(elements) => {
                for element in elements {
                    self.resolve_type(element, declare_type_vars);
                }
            },
        }
    }

    /// If `auto_declare` is true, automatically declare the name if not found instead of issuing
    /// an error.
    fn resolve_variable(&mut self, name_id: NameId, auto_declare: bool) {
        let name = &self.context[name_id];

        if let Some(origin) = self.lookup_local_name(name) {
            self.name_links.insert(name_id, origin);
        } else if auto_declare {
            self.declare_name(name_id);
        } else {
            let location = self.context.name_location(name_id).clone();
            let name = self.context[name_id].clone();
            //panic!("`{name}` is unresolved");
            self.emit_diagnostic(Diagnostic::NameNotInScope { name, location });
        }
    }

    fn resolve_type_definition(&mut self, type_definition: &TypeDefinition) {
        self.declare_generics(&type_definition.generics);

        match &type_definition.body {
            TypeDefinitionBody::Error => (),
            TypeDefinitionBody::Struct(fields) => {
                for (name, field_type) in fields {
                    if type_definition.is_trait || type_definition.is_effect {
                        self.link_existing_union_variant(type_definition.name, *name);
                    }
                    self.resolve_type(field_type, false);
                }
            },
            TypeDefinitionBody::Enum(variants) => {
                for (name, variant_args) in variants {
                    self.link_existing_union_variant(type_definition.name, *name);
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

    /// Does this require special handling? This should be resolved before runtime
    /// definitions are resolved.
    fn resolve_comptime(&mut self, comptime: &Comptime) {
        match comptime {
            Comptime::Expr(expr_id) => self.resolve_expr(*expr_id),
            Comptime::Derive(paths) => {
                for path in paths {
                    self.link(*path, false, true);
                }
            },
            Comptime::Definition(definition) => self.resolve_definition(definition),
        }
    }

    /// Resolve each effect case and report any duplicates, missing cases, or cases from other effects.
    fn check_handler_methods(&mut self, handle: &Handle, expr: ExprId) {
        // The BTreeSet here determines ordering in the 'missing methods' error so we can't use a FxHashSet
        let mut effect_info: Option<(TopLevelId, Name, BTreeSet<Name>)> = None;
        let mut seen_methods: BTreeMap<Name, Location> = BTreeMap::new();

        for (pattern, branch) in &handle.cases {
            let path = pattern.function;
            self.link(path, false, false);
            self.resolve_expr(*branch);

            let Some(Origin::TopLevelDefinition(name)) = self.path_links.get(&path).copied() else {
                continue;
            };
            let (item, item_context) = GetItem(name.top_level_item).get(self.compiler);
            let TopLevelItemKind::TypeDefinition(type_definition) = &item.kind else { continue };
            if !type_definition.is_effect {
                continue;
            }
            let TypeDefinitionBody::Struct(fields) = &type_definition.body else { continue };

            let method_name = item_context[name.local_name_id].clone();
            let method_location = self.context.path_location(path).clone();

            let mut is_same_effect = true;
            if let Some((effect_id, first_name, _)) = &effect_info {
                if *effect_id != name.top_level_item {
                    is_same_effect = false;

                    let first_effect = first_name.clone();
                    let second_effect = item_context[type_definition.name].clone();
                    let location = self.context.expr_location(expr).clone();
                    self.emit_diagnostic(Diagnostic::HandlerCrossEffect { first_effect, second_effect, location });
                }
            } else {
                let effect_name = item_context[type_definition.name].clone();
                let all_methods = fields.iter().map(|(n, _)| item_context[*n].clone()).collect();
                effect_info = Some((name.top_level_item, effect_name, all_methods));
            }

            if is_same_effect {
                if let Some(prev_location) = seen_methods.get(&method_name).cloned() {
                    self.emit_diagnostic(Diagnostic::HandlerDuplicateMethod {
                        name: method_name,
                        first_location: prev_location,
                        second_location: method_location,
                    });
                } else {
                    seen_methods.insert(method_name, method_location);
                }
            }
        }

        if let Some((_, effect_name, all_methods)) = effect_info {
            let missing = mapvec(all_methods.iter().filter(|m| !seen_methods.contains_key(*m)), |n| n.to_string());

            if !missing.is_empty() {
                let location = self.context.expr_location(expr).clone();
                self.emit_diagnostic(Diagnostic::HandlerMissingMethods {
                    effect_name,
                    missing_methods: missing,
                    location,
                });
            }
        }
    }
}

enum FieldsResult {
    Fields(BTreeSet<Name>),
    /// A prior error occurred, avoid issuing another
    PriorError,
    NotAStruct,
}
