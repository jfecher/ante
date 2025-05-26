//! nameresolution/mod.rs - Defines the name resolution `declare` and
//! `define` passes via the Resolvable trait. Name resolution follows
//! parsing and is followed by type inference in the compiler pipeline.
//!
//! The goal of the name resolution passes are to handle Scopes + imports and link
//! each variable to its definition (via setting its DefinitionInfoId field)
//! so that no subsequent pass needs to care about scoping.
//!
//! Name resolution is split up into two passes:
//! 1. `declare` collects the publically exported symbols of every module
//!    to enable mutually recursive functions that may be defined in separate
//!    modules. Since this pass only needs to collect these symbols, it skips
//!    over most Ast node types, looking for only top-level Definition nodes.
//! 2. `define` does the bulk of name resolution, creating DefinitionInfos for
//!    each variable definition, linking each variable use to the corresponding
//!    DefinitionInfo it was defined in, creating a TypeInfo for each type
//!    definition, and a TraitInfo for each trait definition. This will also
//!    issue unused variable warnings at the end of a scope for any unused symbols.
//!
//! Both of these passes walk the Ast in a flat manor compared to subsequent
//! passes like codegen which uses the results of name resolution to walk the Ast
//! and follow definition links to compile definitions lazily as they're used.
//!
//! The recommended start point when reading this file to understand how name
//! resolution works is the `NameResolver::start` function. Which when called
//! will resolve an entire program.
//!
//! The name resolution passes fill out the following fields in the Ast:
//!   - For `ast::Variable`s:
//!       `definition: Option<DefinitionInfoId>`,
//!       `impl_scope: Option<ImplScopeId>,
//!       `id: Option<VariableId>`,
//!   - `level: Option<LetBindingLevel>` for
//!       `ast::Definition`s, `ast::TraitDefinition`s, and `ast::Extern`s,
//!   - `info: Option<DefinitionInfoId>` for `ast::Definition`s,
//!   - `type_info: Option<TypeInfoId>` for `ast::TypeDefinition`s,
//!   - `trait_info: Option<TraitInfoId>` for `ast::TraitDefinition`s and `ast::TraitImpl`s
//!   - `impl_id: Option<ImplInfoId>` for `ast::TraitImpl`s
//!   - `module_id: Option<ModuleId>` for `ast::Import`s,
use crate::cache::{DefinitionInfoId, EffectInfoId, ModuleCache, ModuleId};
use crate::cache::{DefinitionKind, ImplInfoId, TraitInfoId};
use crate::error::{
    location::{Locatable, Location},
    DiagnosticKind as D,
};
use crate::lexer::{token::Token, Lexer};
use crate::nameresolution::scope::{FunctionScopes, Scope};
use crate::parser::ast::{EffectAst, EffectName};
use crate::parser::{self, ast, ast::Ast};
use crate::types::effects::EffectSet;
use crate::types::traits::ConstraintSignature;
use crate::types::typed::Typed;
use crate::types::{
    Field, FunctionType, GeneralizedType, LetBindingLevel, PrimitiveType, Type, TypeConstructor, TypeInfoBody,
    TypeInfoId, TypeTag, TypeVariableId, INITIAL_LEVEL, STRING_TYPE,
};
use crate::util::{fmap, timing, trustme};

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub mod builtin;
pub mod free_variables;
mod scope;
pub mod visitor;

/// Specifies how far a particular module is in name resolution.
/// Keeping this properly up to date for each module is the
/// key for preventing infinite recursion when declaring recursive imports.
///
/// For example, if we're in the middle of defining a module, and we
/// try to import another file that has DefineInProgress, we know not
/// to recurse into that module. In this case, since the module has already
/// finished the declare phase, we can still import any needed public symbols
/// and continue resolving the current module. The other module will finish
/// being defined sometime after the current one is since we detected a cycle.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NameResolutionState {
    NotStarted,
    DeclareInProgress,
    Declared,
    DefineInProgress,
    Defined,
}

/// The NameResolver struct contains the context needed for resolving
/// a single file of the program. There is 1 NameResolver per file, and
/// different files may be in different `NameResolutionState`s.
#[derive(Debug)]
pub struct NameResolver {
    filepath: PathBuf,

    /// The stack of functions scopes we are currently compiling.
    /// Since we do not follow function calls, we are only inside multiple
    /// functions when their definitions are nested, e.g. in:
    ///
    /// foo () =
    ///     bar () = 3
    ///     bar () + 2
    ///
    /// Our callstack would consist of [main/global scope, foo, bar]
    scopes: Vec<FunctionScopes>,

    /// Contains all the publically exported symbols of the current module.
    /// The purpose of the 'declare' pass is to fill this field out for
    /// all modules used in the program. The exported symbols need not
    /// be defined until the 'define' pass later however.
    pub exports: Scope,

    /// module scopes to look up definitions and types from
    pub module_scopes: HashMap<ModuleId, Scope>,

    /// Type variable scopes are separate from other scopes since in general
    /// type variables do not follow normal scoping rules. For example, in the trait:
    ///
    /// trait Foo a
    ///     foo a -> a
    ///
    /// A new type variable scope should be started before the trait and should
    /// be popped after the trait is defined. However, the declarations inside
    /// the trait should still be in the global scope, yet these declarations
    /// should be able to access the type variables when other global declarations
    /// should not. There is a similar problem for type definitions.
    type_variable_scopes: Vec<scope::TypeVariableScope>,

    state: NameResolutionState,

    module_id: ModuleId,

    let_binding_level: LetBindingLevel,

    // Implementation detail fields:
    /// When this is true encountered symbols will be declared instead
    /// of looked up in the symbol table.
    auto_declare: bool,

    /// The trait we're currently declaring. While this is Some(id) all
    /// declarations will be declared as part of the trait.
    current_trait: Option<TraitInfoId>,

    // The name and ID of the function we're currently compiling, if any
    current_function: Option<(String, DefinitionInfoId)>,

    /// When compiling a trait impl, these are the definitions we're requiring
    /// be implemented. A definition is removed from this list once defined and
    /// after compiling the impl this list must be empty.  Otherwise, the user
    /// did not implement all the definitions of a trait.
    required_definitions: Option<Vec<DefinitionInfoId>>,

    /// Keeps track of all the definitions collected within a pattern so they
    /// can all be tagged with the expression they were defined as later
    definitions_collected: Vec<DefinitionInfoId>,
}

impl PartialEq for NameResolver {
    fn eq(&self, other: &NameResolver) -> bool {
        self.filepath == other.filepath
    }
}

macro_rules! lookup_fn {
    ( $name:ident , $stack_field:ident , $cache_field:ident, $return_type:ty ) => {
        fn $name(&self, name: &str, cache: &mut ModuleCache<'_>) -> Option<$return_type> {
            let function_scope = self.scopes.last().unwrap();
            for stack in function_scope.iter().rev() {
                if let Some(id) = stack.$stack_field.get(name) {
                    cache.$cache_field[id.0].uses += 1;
                    return Some(*id);
                }
            }

            // Check globals/imports in global scope
            if let Some(id) = self.global_scope().$stack_field.get(name) {
                cache.$cache_field[id.0].uses += 1;
                return Some(*id);
            }

            None
        }
    };
}

impl<'c> NameResolver {
    // lookup_fn!(lookup_definition, definitions, definition_infos, DefinitionInfoId);
    lookup_fn!(lookup_type, types, type_infos, TypeInfoId);
    lookup_fn!(lookup_trait, traits, trait_infos, TraitInfoId);
    lookup_fn!(lookup_effect, effects, effect_infos, EffectInfoId);

    /// Similar to the lookup functions above, but will also lookup variables that are
    /// defined in a parent function to keep track of which variables closures
    /// will need in their environment.
    fn reference_definition(
        &mut self, name: &str, location: Location<'c>, cache: &mut ModuleCache<'c>,
    ) -> Option<DefinitionInfoId> {
        let current_function_scope = self.scopes.last().unwrap();

        for stack in current_function_scope.iter().rev() {
            if let Some(&id) = stack.definitions.get(name) {
                cache.definition_infos[id.0].uses += 1;

                if self.in_global_scope() && matches!(self.state, NameResolutionState::DefineInProgress) {
                    cache.global_dependency_graph.add_edge(id);
                }

                return Some(id);
            }
        }

        // If name wasn't found yet, try any parent function scopes.
        // If we find it here, also mark the current lambda as a closure.
        let range = 1..std::cmp::max(1, self.scopes.len() - 1);

        for function_scope_index in range.rev() {
            let function_scope = &self.scopes[function_scope_index];

            for stack in function_scope.iter().rev() {
                if let Some(&from) = stack.definitions.get(name) {
                    return Some(self.create_closure(from, name, function_scope_index, location, cache));
                }
            }
        }

        // Otherwise, check globals/imports.
        // We must be careful here to separate items within this final FunctionScope:
        // - True globals are at the first scope and shouldn't create closures
        // - Anything after is local to a block, e.g. in a top-level if-then.
        //   These items should create a closure if referenced.
        let global_index = 0;
        let function_scope = &self.scopes[global_index];

        for (i, stack) in function_scope.iter().enumerate().rev() {
            if i == 0 {
                // Definition is globally visible, no need to create a closure
                if let Some(id) = self.global_scope().definitions.get(name).copied() {
                    cache.definition_infos[id.0].uses += 1;

                    if matches!(self.state, NameResolutionState::DefineInProgress) {
                        cache.global_dependency_graph.add_edge(id);
                    }
                    return Some(id);
                }
            } else if let Some(&from) = stack.definitions.get(name) {
                return Some(self.create_closure(from, name, global_index, location, cache));
            }
        }

        None
    }

    /// Adds a given environment variable (along with its name and the self.scopes index of the function it
    /// was found in) to a function, thus marking that function as being a closure. This works by
    /// creating a new parameter in the current function and creating a mapping between the
    /// environment variable and that parameter slot so that codegen will know to automatically
    /// pass in the required environment variable(s).
    ///
    /// This also handles the case of transitive closures. When we add an environment variable to
    /// a closure, we may also need to create more closures along the way to be able to thread
    /// through our environment variables to reach any closures within other closures.
    fn create_closure(
        &mut self, mut environment: DefinitionInfoId, environment_name: &str, environment_function_index: usize,
        location: Location<'c>, cache: &mut ModuleCache<'c>,
    ) -> DefinitionInfoId {
        let mut ret = None;
        cache.definition_infos[environment.0].uses += 1;

        // Traverse through each function from where the environment variable is defined
        // to the closure that uses it, and add the environment variable to each closure.
        // Usually, this is only one function but in cases like
        //
        // x = 2
        // fn _ -> fn _ -> x
        //
        // we have to traverse multiple functions, marking them all as closures along
        // the way while adding `x` as a parameter to each.
        for origin_fn in environment_function_index..self.scopes.len() - 1 {
            let next_fn = origin_fn + 1;

            let to = self.add_closure_parameter_definition(environment_name, next_fn, location, cache);
            self.scopes[next_fn].add_closure_environment_variable_mapping(environment, to, location, cache);
            environment = to;
            ret = Some(to);
        }

        ret.unwrap()
    }

    fn add_closure_parameter_definition(
        &mut self, parameter: &str, function_scope_index: usize, location: Location<'c>, cache: &mut ModuleCache<'c>,
    ) -> DefinitionInfoId {
        let function_scope = &mut self.scopes[function_scope_index];
        let scope = function_scope.first_mut();

        let id = cache.push_definition(parameter, false, location);
        cache.definition_infos[id.0].definition = Some(DefinitionKind::Parameter);
        cache.definition_infos[id.0].uses = 1;

        let existing = scope.definitions.insert(parameter.to_string(), id);
        assert!(existing.is_none());

        id
    }

    fn lookup_type_variable(&self, name: &str) -> Option<(TypeVariableId, Rc<String>)> {
        for scope in self.type_variable_scopes.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry.clone());
            }
        }

        None
    }

    fn push_scope(&mut self, cache: &mut ModuleCache) {
        self.function_scopes().push_new_scope(cache);
        let impl_scope = self.current_scope().impl_scope;

        // TODO optimization: this really shouldn't be necessary to copy all the
        // trait impl ids for each scope just so Variables can store their scope
        // for the type checker to do trait resolution.
        for scope in self.scopes[0].iter().rev() {
            for (_, impls) in scope.impls.iter() {
                cache.impl_scopes[impl_scope.0].append(&mut impls.clone());
            }
        }
    }

    fn push_lambda(&mut self, lambda: &mut ast::Lambda<'c>, cache: &mut ModuleCache<'c>) {
        let function_id = self.current_function.as_ref().map(|(_, id)| *id);
        self.scopes.push(FunctionScopes::from_lambda(lambda, function_id));
        self.push_type_variable_scope();
        self.push_scope(cache);
    }

    fn push_type_variable_scope(&mut self) {
        self.type_variable_scopes.push(scope::TypeVariableScope::default());
    }

    fn push_existing_type_variable(
        &mut self, key: &str, id: TypeVariableId, location: Location<'c>, cache: &mut ModuleCache<'c>,
    ) -> (TypeVariableId, Rc<String>) {
        let top = self.type_variable_scopes.len() - 1;

        if self.type_variable_scopes[top].push_existing_type_variable(key.to_owned(), id).is_none() {
            cache.push_diagnostic(location, D::TypeVariableAlreadyInScope(key.to_owned()));
        }
        (id, Rc::new(key.to_owned()))
    }

    fn push_new_type_variable(
        &mut self, key: &str, location: Location<'c>, cache: &mut ModuleCache<'c>,
    ) -> (TypeVariableId, Rc<String>) {
        let id = cache.next_type_variable_id(self.let_binding_level);
        self.push_existing_type_variable(key, id, location, cache)
    }

    fn pop_scope(&mut self, cache: &mut ModuleCache<'_>, warn_unused: bool, id_to_ignore: Option<DefinitionInfoId>) {
        if warn_unused {
            self.current_scope().check_for_unused_definitions(cache, id_to_ignore);
        }
        self.function_scopes().pop();
    }

    fn pop_lambda(&mut self, cache: &mut ModuleCache<'_>) {
        let function = self.function_scopes();
        let function_id = function.function_id;
        assert_eq!(function.scopes.len(), 1);
        self.pop_type_variable_scope();
        self.pop_scope(cache, true, function_id);
        self.scopes.pop();
    }

    fn pop_type_variable_scope(&mut self) {
        self.type_variable_scopes.pop();
    }

    fn function_scopes(&mut self) -> &mut FunctionScopes {
        self.scopes.last_mut().unwrap()
    }

    fn current_scope(&mut self) -> &mut Scope {
        let function_scopes = self.function_scopes();
        function_scopes.last_mut()
    }

    fn global_scope(&self) -> &Scope {
        self.scopes[0].first()
    }

    fn in_global_scope(&self) -> bool {
        self.scopes.len() == 1
    }

    fn push_let_binding_level(&mut self) {
        self.let_binding_level = LetBindingLevel(self.let_binding_level.0 + 1);
    }

    fn pop_let_binding_level(&mut self) {
        self.let_binding_level = LetBindingLevel(self.let_binding_level.0 - 1);
    }

    /// Checks that the given variable name is required by the trait for the impl we're currently resolving.
    /// If it is not, an error is issued that the impl does not need to implement this function.
    /// Otherwise, the name is removed from the list of required_definitions for the impl.
    fn check_required_definitions(&mut self, name: &str, cache: &mut ModuleCache<'c>, location: Location<'c>) {
        let required_definitions = self.required_definitions.as_mut().unwrap();
        if let Some(index) = required_definitions.iter().position(|id| cache.definition_infos[id.0].name == name) {
            required_definitions.swap_remove(index);
        } else {
            let trait_info = &cache.trait_infos[self.current_trait.unwrap().0];
            cache.push_diagnostic(location, D::ItemNotRequiredByTrait(name.to_string(), trait_info.name.clone()))
        }
    }

    /// Add a DefinitionInfoId to a trait's list of required definitions and add
    /// the trait to the DefinitionInfo's list of required traits.
    fn attach_to_trait(&mut self, id: DefinitionInfoId, trait_id: TraitInfoId, cache: &mut ModuleCache<'_>) {
        let trait_info = &mut cache.trait_infos[trait_id.0];
        trait_info.definitions.push(id);

        // TODO: Is this still necessary? Can we remove the args field of trait_info below?
        let args =
            trait_info.typeargs.iter().chain(trait_info.fundeps.iter()).map(|id| Type::TypeVariable(*id)).collect();

        let info = &mut cache.definition_infos[id.0];
        info.trait_info = Some((trait_id, args));
    }

    /// Push a new Definition onto the current scope.
    fn push_definition(&mut self, name: &str, cache: &mut ModuleCache<'c>, location: Location<'c>) -> DefinitionInfoId {
        let in_global_scope = self.in_global_scope();
        let id = cache.push_definition(name, in_global_scope, location);

        // if shadows
        if let Some(existing_definition) = self.current_scope().definitions.get(name) {
            // disallow shadowing in global scopes
            if in_global_scope {
                cache.push_diagnostic(location, D::AlreadyInScope(name.to_owned()));
                let previous_location = cache.definition_infos[existing_definition.0].location;
                cache.push_diagnostic(previous_location, D::PreviouslyDefinedHere(name.to_owned()));
            } else {
                // allow shadowing in local scopes
                self.current_scope().check_for_unused_definitions(cache, None);
            }
        }

        if self.required_definitions.is_some() {
            // We're inside a trait impl right now, any definitions shouldn't be put in scope else
            // they could collide with the definitions from other impls of the same trait. Additionally, we
            // must ensure the definition is one of the ones required by the trait in required_definitions.
            self.check_required_definitions(name, cache, location);
        } else {
            // Prevent _ from being referenced and allow it to be redefined as needed.
            // This can be removed if ante ever allows shadowing by default.
            if name != "_" {
                if self.in_global_scope() {
                    self.exports.definitions.insert(name.to_owned(), id);
                }
                self.current_scope().definitions.insert(name.to_owned(), id);
            }

            // If we're currently in a trait, add this definition to the trait's list of definitions
            if let Some(trait_id) = self.current_trait {
                self.attach_to_trait(id, trait_id, cache);
            }
        }
        id
    }

    pub fn push_type_info(
        &mut self, name: String, args: Vec<TypeVariableId>, cache: &mut ModuleCache<'c>, location: Location<'c>,
    ) -> TypeInfoId {
        if let Some(existing_definition) = self.current_scope().types.get(&name) {
            cache.push_diagnostic(location, D::AlreadyInScope(name.clone()));
            let previous_location = cache.type_infos[existing_definition.0].locate();
            cache.push_diagnostic(previous_location, D::PreviouslyDefinedHere(name.clone()));
        }

        let id = cache.push_type_info(name.clone(), args, location);
        if self.in_global_scope() {
            self.exports.types.insert(name.clone(), id);
        }
        self.current_scope().types.insert(name, id);
        id
    }

    fn push_trait(
        &mut self, name: String, args: Vec<TypeVariableId>, fundeps: Vec<TypeVariableId>,
        node: &'c mut ast::TraitDefinition<'c>, cache: &mut ModuleCache<'c>, location: Location<'c>,
    ) -> TraitInfoId {
        if let Some(existing_definition) = self.current_scope().traits.get(&name) {
            cache.push_diagnostic(location, D::AlreadyInScope(name.clone()));
            let previous_location = cache.trait_infos[existing_definition.0].locate();
            cache.push_diagnostic(previous_location, D::PreviouslyDefinedHere(name.clone()));
        }

        let id = cache.push_trait_definition(name.clone(), args, fundeps, Some(node), location);
        if self.in_global_scope() {
            self.exports.traits.insert(name.clone(), id);
        }
        self.current_scope().traits.insert(name, id);
        id
    }

    fn push_effect(
        &mut self, name: String, args: Vec<TypeVariableId>, cache: &mut ModuleCache<'c>, location: Location<'c>,
    ) -> EffectInfoId {
        if let Some(existing_definition) = self.current_scope().effects.get(&name) {
            cache.push_diagnostic(location, D::AlreadyInScope(name.clone()));
            let previous_location = cache.effect_infos[existing_definition.0].locate();
            cache.push_diagnostic(previous_location, D::PreviouslyDefinedHere(name.clone()));
        }

        let id = cache.push_effect_definition(name.clone(), args, location);
        if self.in_global_scope() {
            self.exports.effects.insert(name.clone(), id);
        }
        self.current_scope().effects.insert(name, id);
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn push_trait_impl(
        &mut self, trait_id: TraitInfoId, args: Vec<Type>, definitions: Vec<DefinitionInfoId>,
        trait_impl: &'c mut ast::TraitImpl<'c>, given: Vec<ConstraintSignature>, cache: &mut ModuleCache<'c>,
        location: Location<'c>,
    ) -> ImplInfoId {
        // Any overlapping impls are only reported when they're used during typechecking
        let id = cache.push_trait_impl(trait_id, args, definitions, trait_impl, given, location);
        if self.in_global_scope() {
            self.exports.impls.entry(trait_id).or_default().push(id);
            cache.impl_scopes[self.exports.impl_scope.0].push(id);
        }

        self.current_scope().impls.entry(trait_id).or_default().push(id);
        cache.impl_scopes[self.current_scope().impl_scope.0].push(id);
        id
    }

    fn add_module_scope_and_import_impls(
        &mut self, relative_path: &str, location: Location<'c>, cache: &mut ModuleCache<'c>,
    ) -> Option<ModuleId> {
        if let Some(module_id) = declare_module(Path::new(&relative_path), cache, location) {
            self.current_scope().modules.insert(relative_path.to_owned(), module_id);
            if let Some(exports) = define_module(module_id, cache, location) {
                self.current_scope().import_impls(exports, cache);
                self.module_scopes.insert(module_id, exports.to_owned());
                return Some(module_id);
            }
        }

        None
    }

    fn validate_type_application(
        &self, constructor: &Type, args: &[Type], location: Location<'c>, cache: &mut ModuleCache<'c>,
    ) {
        let expected = self.get_expected_type_argument_count(constructor, cache);
        if args.len() != expected && !matches!(constructor, Type::TypeVariable(_) | Type::NamedGeneric(..)) {
            let typename = constructor.display(cache).to_string();
            cache.push_diagnostic(location, D::IncorrectConstructorArgCount(typename, args.len(), expected));
        }

        // Check argument is an integer/float type (issue #146)
        if let Some(first_arg) = args.get(0) {
            match constructor {
                Type::Primitive(PrimitiveType::IntegerType) => {
                    if !matches!(first_arg, Type::Primitive(PrimitiveType::IntegerTag(_)) | Type::TypeVariable(_) | Type::NamedGeneric(..)) {
                        let typename = first_arg.display(cache).to_string();
                        cache.push_diagnostic(location, D::NonIntegerType(typename));
                    }
                },
                Type::Primitive(PrimitiveType::FloatType) => {
                    if !matches!(first_arg, Type::Primitive(PrimitiveType::FloatTag(_)) | Type::TypeVariable(_) | Type::NamedGeneric(..)) {
                        let typename = first_arg.display(cache).to_string();
                        cache.push_diagnostic(location, D::NonFloatType(typename));
                    }
                },
                _ => (),
            }
        }
    }

    fn get_expected_type_argument_count(&self, constructor: &Type, cache: &ModuleCache) -> usize {
        match constructor {
            Type::Primitive(PrimitiveType::Ptr) => 1,
            Type::Primitive(PrimitiveType::IntegerType) => 1,
            Type::Primitive(PrimitiveType::FloatType) => 1,
            Type::Primitive(_) => 0,
            Type::Function(_) => 0,
            // Type variables should be unbound before type checking
            Type::TypeVariable(_) => 0,
            Type::UserDefined(id) => cache[*id].args.len(),
            Type::TypeApplication(_, _) => 0,
            Type::Ref { .. } => 1,
            Type::Struct(_, _) => 0,
            Type::Effects(_) => 0,
            Type::Tag(_) => 0,
            Type::NamedGeneric(..) => 0,
        }
    }

    /// Re-insert the given type variables into the current scope.
    /// Currently used for remembering type variables from type and trait definitions that
    /// were created in the declare pass and need to be used later in the define pass.
    fn add_existing_type_variables_to_scope(
        &mut self, existing_typevars: &[String], ids: &[TypeVariableId], location: Location<'c>,
        cache: &mut ModuleCache<'c>,
    ) {
        // re-insert the typevars into scope.
        // These names are guarenteed to not collide since we just pushed a new scope.
        assert_eq!(existing_typevars.len(), ids.len());
        for (key, id) in existing_typevars.iter().zip(ids) {
            self.push_existing_type_variable(key, *id, location, cache);
        }
    }

    /// Performs name resolution on an entire program, starting from the
    /// given Ast and all imports reachable from it.
    pub fn start(ast: Ast<'c>, cache: &mut ModuleCache<'c>) {
        timing::start_time("Name Resolution (Declare)");

        builtin::define_builtins(cache);
        let resolver = NameResolver::declare(ast, cache);

        timing::start_time("Name Resolution (Define)");
        resolver.define(cache);
    }

    /// Creates a NameResolver and performs the declare pass on
    /// the given ast, collecting all of its publically exported symbols
    /// into the `exports` field.
    pub fn declare(ast: Ast<'c>, cache: &mut ModuleCache<'c>) -> &'c mut NameResolver {
        let filepath = ast.locate().filename;

        let existing = cache.get_name_resolver_by_path(filepath);
        assert!(existing.is_none());

        let module_id = cache.push_ast(ast);
        cache.modules.insert(filepath.to_owned(), module_id);

        let mut resolver = NameResolver {
            module_scopes: HashMap::new(),
            filepath: filepath.to_owned(),
            scopes: vec![FunctionScopes::new()],
            exports: Scope::new(cache),
            type_variable_scopes: vec![scope::TypeVariableScope::default()],
            state: NameResolutionState::DeclareInProgress,
            auto_declare: false,
            current_trait: None,
            required_definitions: None,
            current_function: None,
            definitions_collected: vec![],
            let_binding_level: LetBindingLevel(INITIAL_LEVEL),
            module_id,
        };

        resolver.push_scope(cache);

        let existing = cache.get_name_resolver_by_path(filepath);
        let existing_state = existing.map_or(NameResolutionState::NotStarted, |x| x.state);
        assert!(existing_state == NameResolutionState::NotStarted);

        cache.name_resolvers.push(resolver);
        let resolver = cache.name_resolvers.get_mut(module_id.0).unwrap();
        builtin::import_prelude(resolver, cache);

        let ast = cache.parse_trees.get_mut(module_id.0).unwrap();
        ast.declare(resolver, cache);
        resolver.state = NameResolutionState::Declared;

        resolver
    }

    /// Performs the define pass on the current NameResolver, linking all
    /// variables to their definition, filling in each XXXInfoId field, etc.
    /// See the module-level comment for more details on the define pass.
    pub fn define(&mut self, cache: &mut ModuleCache<'c>) {
        let ast = cache.parse_trees.get_mut(self.module_id.0).unwrap();

        assert!(self.state == NameResolutionState::Declared);

        self.state = NameResolutionState::DefineInProgress;
        ast.define(self, cache);
        self.state = NameResolutionState::Defined;
    }

    pub fn convert_type(&mut self, cache: &mut ModuleCache<'c>, ast_type: &ast::Type<'c>) -> Type {
        match ast_type {
            ast::Type::Integer(Some(kind), _) => Type::int(*kind),
            ast::Type::Integer(None, _) => Type::Primitive(PrimitiveType::IntegerType),
            ast::Type::Float(Some(kind), _) => Type::float(*kind),
            ast::Type::Float(None, _) => Type::Primitive(PrimitiveType::FloatType),
            ast::Type::Char(_) => Type::Primitive(PrimitiveType::CharType),
            ast::Type::String(_) => Type::UserDefined(STRING_TYPE),
            ast::Type::Pointer(_) => Type::Primitive(PrimitiveType::Ptr),
            ast::Type::Boolean(_) => Type::Primitive(PrimitiveType::BooleanType),
            ast::Type::Unit(_) => Type::UNIT,
            ast::Type::Function(function) => {
                let parameters = fmap(&function.parameters, |arg| self.convert_type(cache, arg));
                let return_type = Box::new(self.convert_type(cache, &function.return_type));

                let environment = Box::new(if function.is_closure {
                    cache.next_type_variable(self.let_binding_level)
                } else {
                    Type::UNIT
                });

                let effects = if let Some(effects) = &function.effects {
                    Box::new(self.convert_effects(effects, cache))
                } else {
                    cache.push_diagnostic(function.location, D::FunctionEffectsNotSpecified);
                    Box::new(Type::Effects(EffectSet::pure()))
                };

                let has_varargs = function.has_varargs;
                Type::Function(FunctionType { parameters, return_type, environment, has_varargs, effects })
            },
            ast::Type::TypeVariable(name, location) => match self.lookup_type_variable(name) {
                Some((id, name)) => Type::NamedGeneric(id, name),
                None => {
                    if self.auto_declare {
                        let (id, name) = self.push_new_type_variable(name, *location, cache);
                        Type::NamedGeneric(id, name)
                    } else {
                        cache.push_diagnostic(*location, D::NotInScope("Type variable", name.clone()));
                        Type::UNIT
                    }
                },
            },
            ast::Type::UserDefined(name, location) => match self.lookup_type(name, cache) {
                Some(id) => Type::UserDefined(id),
                None => {
                    cache.push_diagnostic(*location, D::NotInScope("Type", name.clone()));
                    Type::UNIT
                },
            },
            ast::Type::TypeApplication(constructor, args, _) => {
                let constructor = Box::new(self.convert_type(cache, constructor));
                let args = fmap(args, |arg| self.convert_type(cache, arg));
                self.validate_type_application(&constructor, &args, ast_type.locate(), cache);
                Type::TypeApplication(constructor, args)
            },
            ast::Type::Pair(first, rest, location) => {
                let args = vec![self.convert_type(cache, first), self.convert_type(cache, rest)];

                let pair = match self.lookup_type(&Token::Comma.to_string(), cache) {
                    Some(id) => Type::UserDefined(id),
                    None => {
                        cache.push_diagnostic(*location, D::NotInScope("The pair type", "(,)".into()));
                        Type::UNIT
                    },
                };

                Type::TypeApplication(Box::new(pair), args)
            },
            ast::Type::Reference(sharedness, mutability, _) => {
                // When translating ref types, all have a hidden lifetime variable that is unified
                // under the hood by the compiler to determine the reference's stack lifetime.
                // This is never able to be manually specified by the programmer, so we use
                // next_type_variable_id on the cache rather than the NameResolver's version which
                // would add a name into scope.
                let lifetime = Box::new(cache.next_type_variable(self.let_binding_level));

                let sharedness = Box::new(match sharedness {
                    ast::Sharedness::Polymorphic => cache.next_type_variable(self.let_binding_level),
                    ast::Sharedness::Shared => Type::Tag(TypeTag::Shared),
                    ast::Sharedness::Owned => Type::Tag(TypeTag::Owned),
                });

                let mutability = Box::new(match mutability {
                    ast::Mutability::Polymorphic => cache.next_type_variable(self.let_binding_level),
                    ast::Mutability::Immutable => Type::Tag(TypeTag::Immutable),
                    ast::Mutability::Mutable => Type::Tag(TypeTag::Mutable),
                });

                Type::Ref { sharedness, mutability, lifetime }
            },
        }
    }

    fn convert_effects(&mut self, effects: &[EffectAst<'c>], cache: &mut ModuleCache<'c>) -> Type {
        let mut new_effects = Vec::new();
        let mut extension_var: Option<(EffectName, Location<'c>, TypeVariableId)> = None;

        let starts_with_uppercase = |name: &str| name.chars().next().map_or(false, |c| c.is_uppercase());

        for (effect_name, name_location, effect_args) in effects {
            match effect_name {
                EffectName::Name(effect_name) if starts_with_uppercase(effect_name) => {
                    let id = self.lookup_effect(effect_name, cache);
                    let args = fmap(effect_args, |arg| self.convert_type(cache, arg));

                    if let Some(id) = id {
                        new_effects.push((id, args));
                    } else {
                        cache.push_diagnostic(*name_location, D::NotInScope("Effect", effect_name.clone()));
                    }
                },
                name_or_id @ EffectName::Name(effect_name) => {
                    if let Some((previous_name, previous_location, _)) = &extension_var {
                        match previous_name {
                            EffectName::Name(previous_name) => {
                                cache.push_diagnostic(
                                    *name_location,
                                    D::EffectVariableAlreadyUsed {
                                        unnecessary_var_name: effect_name.clone(),
                                        old_name: previous_name.clone(),
                                    },
                                );
                                cache.push_diagnostic(
                                    *previous_location,
                                    D::EffectVariableAlreadyUsedNote { old_name: previous_name.clone() },
                                );
                            },
                            EffectName::ImplicitEffect(_) => {
                                cache.push_diagnostic(
                                    *previous_location,
                                    D::ImplicitEffectVariableMustBeExplicit { explicit_arg_name: effect_name.clone() },
                                );
                                cache.push_diagnostic(
                                    *name_location,
                                    D::ImplicitEffectVariableMustBeExplicitNote {
                                        explicit_arg_name: effect_name.clone(),
                                    },
                                );
                            },
                        }
                    } else {
                        let var = cache.next_type_variable_id(self.let_binding_level);
                        extension_var = Some((name_or_id.clone(), *name_location, var));
                    }
                },
                // This is an implicitly inserted type variable from desugar_function_effect_variables
                EffectName::ImplicitEffect(id) => {
                    if let Some((previous_name, previous_location, _)) = &extension_var {
                        let explicit_arg_name = match previous_name {
                            EffectName::Name(name) => name.clone(),
                            EffectName::ImplicitEffect(_) => unreachable!("An implicit effect should never be added by the compiler to the same function type twice"),
                        };

                        cache.push_diagnostic(
                            *name_location,
                            D::ImplicitEffectVariableMustBeExplicit { explicit_arg_name: explicit_arg_name.clone() },
                        );
                        cache.push_diagnostic(
                            *previous_location,
                            D::ImplicitEffectVariableMustBeExplicitNote { explicit_arg_name },
                        );
                    } else {
                        extension_var = Some((effect_name.clone(), *name_location, *id));
                    }
                },
            }
        }

        let extension = extension_var.map(|(_, _, type_variable)| type_variable);
        Type::Effects(EffectSet { effects: new_effects, extension })
    }

    /// The collect* family of functions recurs over an irrefutable pattern, either declaring or
    /// defining each node and tagging the declaration with the given DefinitionNode.
    fn resolve_declarations<F>(&mut self, ast: &mut Ast<'c>, cache: &mut ModuleCache<'c>, mut definition: F)
    where
        F: FnMut() -> DefinitionKind<'c>,
    {
        self.definitions_collected.clear();
        self.auto_declare = true;
        ast.declare(self, cache);
        self.auto_declare = false;
        for id in self.definitions_collected.iter() {
            cache.definition_infos[id.0].definition = Some(definition());
        }
    }

    fn resolve_definitions<T, F>(&mut self, ast: &mut T, cache: &mut ModuleCache<'c>, definition: F)
    where
        T: Resolvable<'c>,
        T: std::fmt::Display,
        F: FnMut() -> DefinitionKind<'c>,
    {
        self.resolve_all_definitions(vec![ast].into_iter(), cache, definition);
    }

    fn resolve_extern_definitions(&mut self, extern_: &mut ast::Extern<'c>, cache: &mut ModuleCache<'c>) {
        self.definitions_collected.clear();
        self.auto_declare = true;

        for declaration in &mut extern_.declarations {
            self.push_type_variable_scope();
            declaration.define(self, cache);
            self.pop_type_variable_scope();
        }

        self.auto_declare = false;

        for id in self.definitions_collected.iter() {
            let extern_ = trustme::extend_lifetime(extern_);
            cache.definition_infos[id.0].definition = Some(DefinitionKind::Extern(extern_));
        }
    }

    fn resolve_trait_impl_declarations(
        &mut self, definitions: &mut [ast::Definition<'c>], cache: &mut ModuleCache<'c>,
    ) -> Vec<DefinitionInfoId> {
        self.definitions_collected.clear();
        self.auto_declare = true;

        let mut all_definitions = vec![];

        for definition in definitions {
            // This chunk is largely taken from Definition::declare, but altered
            // slightly so that we can collect all self.definitions_collected instead
            // of clearing them after each Definition
            self.push_let_binding_level();
            self.push_type_variable_scope();
            definition.pattern.declare(self, cache);
            self.pop_let_binding_level();
            self.pop_type_variable_scope();

            for id in std::mem::take(&mut self.definitions_collected) {
                let definition = definition as *const ast::Definition;
                let definition = || DefinitionKind::Definition(trustme::make_mut(definition));
                cache.definition_infos[id.0].definition = Some(definition());
                all_definitions.push(id);
            }
        }

        self.definitions_collected.clear();
        self.auto_declare = false;
        all_definitions
    }

    fn resolve_all_definitions<'a, T, It, F>(&mut self, patterns: It, cache: &mut ModuleCache<'c>, mut definition: F)
    where
        It: Iterator<Item = &'a mut T>,
        T: Resolvable<'c> + 'a,
        F: FnMut() -> DefinitionKind<'c>,
    {
        self.definitions_collected.clear();
        self.auto_declare = true;
        for pattern in patterns {
            pattern.define(self, cache);
        }
        self.auto_declare = false;
        for id in self.definitions_collected.iter() {
            cache.definition_infos[id.0].definition = Some(definition());
        }
    }

    fn resolve_required_traits(
        &mut self, given: &[ast::Trait<'c>], cache: &mut ModuleCache<'c>,
    ) -> Vec<ConstraintSignature> {
        let mut required_traits = Vec::with_capacity(given.len());
        for trait_ in given {
            if let Some(trait_id) = self.lookup_trait(&trait_.name, cache) {
                required_traits.push(ConstraintSignature {
                    trait_id,
                    args: fmap(&trait_.args, |arg| self.convert_type(cache, arg)),
                    id: cache.next_trait_constraint_id(),
                });
            } else {
                cache.push_diagnostic(trait_.location, D::NotInScope("Trait", trait_.name.clone()));
            }
        }
        required_traits
    }

    fn try_set_current_function(&mut self, definition: &ast::Definition<'c>) {
        if let (Ast::Variable(variable), Ast::Lambda(_)) = (definition.pattern.as_ref(), definition.expr.as_ref()) {
            let function = (variable.to_string(), variable.definition.unwrap());
            self.current_function = Some(function);
        }
    }

    fn try_add_current_function_to_scope(&mut self) {
        if let Some((name, id)) = self.current_function.take() {
            let existing = self.current_scope().definitions.insert(name, id);
            assert!(existing.is_none());
        }
    }

    /// If any parameters have a function type (not contained within another type) without an explicit
    /// `can` clause, this adds an implicit `can e` to both that parameter and to this outer lambda.
    fn desugar_function_effect_variables<'local, 'a: 'local, T>(
        &self, args: T, effects: &mut Option<Vec<EffectAst<'a>>>, cache: &mut ModuleCache,
    ) where
        T: IntoIterator<Item = &'local mut ast::Type<'a>>,
    {
        let mut new_effects = Vec::new();

        for arg in args {
            if let ast::Type::Function(function) = arg {
                if function.effects.is_none() {
                    let location = function.location;

                    let effect = cache.next_type_variable_id(self.let_binding_level);
                    let new_effect = (EffectName::ImplicitEffect(effect), location, Vec::new());

                    function.effects = Some(vec![new_effect.clone()]);
                    new_effects.push(new_effect);
                }
            }
        }

        if !new_effects.is_empty() {
            if let Some(effects) = effects {
                effects.extend(new_effects);
            } else {
                *effects = Some(new_effects);
            }
        } else if effects.is_none() {
            *effects = Some(Vec::new());
        }
    }

    fn desugar_function_effect_variables_in_ast<'a>(
        &self, args: &mut [ast::Ast<'a>], effects: &mut Option<Vec<EffectAst<'a>>>, cache: &mut ModuleCache,
    ) {
        let arg_types = args.iter_mut().filter_map(|arg| match arg {
            ast::Ast::TypeAnnotation(annotation) => Some(&mut annotation.rhs),
            _ => None,
        });

        self.desugar_function_effect_variables(arg_types, effects, cache);
    }

    fn desugar_function_effect_variables_in_type<'a>(&self, typ: &mut ast::Type<'a>, cache: &mut ModuleCache) {
        if let ast::Type::Function(function) = typ {
            self.desugar_function_effect_variables(&mut function.parameters, &mut function.effects, cache);
        }
    }
}

pub trait Resolvable<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>);
    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>);
}

impl<'c> Resolvable<'c> for Ast<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        dispatch_on_expr!(self, Resolvable::declare, resolver, cache);
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        dispatch_on_expr!(self, Resolvable::define, resolver, cache);
    }
}

impl<'c> Resolvable<'c> for ast::Literal<'c> {
    /// Purpose of the declare pass is to collect all the names of publicly exported symbols
    /// so the define pass can work in the presense of mutually recursive modules.
    fn declare(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}

    /// Go through a module and annotate each variable with its declaration.
    /// Display any errors for variables without declarations.
    fn define(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}
}

impl<'c> Resolvable<'c> for ast::Variable<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        if resolver.auto_declare {
            use ast::VariableKind::*;
            let (name, should_declare) = match &self.kind {
                Operator(token) => {
                    // TODO: Disabling should_declare only for `,` is a hack to make tuple
                    // patterns work without rebinding the `,` symbol.
                    (Cow::Owned(token.to_string()), *token != Token::Comma)
                },
                Identifier(name) => (Cow::Borrowed(name), true),
                TypeConstructor(name) => (Cow::Borrowed(name), false),
            };

            if should_declare {
                let id = resolver.push_definition(&name, cache, self.location);
                resolver.definitions_collected.push(id);
                self.definition = Some(id);
            } else {
                self.definition = resolver.reference_definition(&name, self.location, cache);
            }

            self.id = Some(cache.push_variable(name.into_owned(), self.location));
            self.impl_scope = Some(resolver.current_scope().impl_scope);
        }
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        if self.definition.is_none() {
            if resolver.auto_declare {
                self.declare(resolver, cache);
            } else {
                use ast::VariableKind::*;
                let name = match &self.kind {
                    Operator(token) => Cow::Owned(token.to_string()),
                    Identifier(name) => Cow::Borrowed(name),
                    TypeConstructor(name) => Cow::Borrowed(name),
                };

                if self.module_prefix.is_empty() {
                    self.impl_scope = Some(resolver.current_scope().impl_scope);
                    self.definition = resolver.reference_definition(&name, self.location, cache);
                    self.id = Some(cache.push_variable(name.into_owned(), self.location));
                } else {
                    // resolve module
                    let relative_path = self.module_prefix.join("/");

                    let mut module_id = resolver.current_scope().modules.get(&relative_path).copied();
                    if module_id.is_none() {
                        module_id = resolver.add_module_scope_and_import_impls(&relative_path, self.location, cache);
                    }

                    if let Some(module_id) = module_id {
                        self.definition = resolver.module_scopes[&module_id].definitions.get(name.as_ref()).copied();
                        self.impl_scope = Some(resolver.current_scope().impl_scope);
                        self.id = Some(cache.push_variable(name.into_owned(), self.location));
                    } else {
                        cache.push_diagnostic(self.location, D::CouldNotFindModule(relative_path));
                    }
                }
            }

            // If it is still not declared, print an error
            if self.definition.is_none() {
                cache.push_diagnostic(self.location, D::NoDeclarationFoundInScope(self.to_string()));
            }
        } else if resolver.in_global_scope() {
            let id = self.definition.unwrap();
            cache.global_dependency_graph.set_definition(id);
        }
    }
}

impl<'c> Resolvable<'c> for ast::Lambda<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        resolver.push_lambda(self, cache);
        resolver.try_add_current_function_to_scope();

        resolver.desugar_function_effect_variables_in_ast(&mut self.args, &mut self.effects, cache);
        resolver.resolve_all_definitions(self.args.iter_mut(), cache, || DefinitionKind::Parameter);

        if let Some(typ) = &self.return_type {
            // Auto-declare any new type variables within the return type
            let prev_auto_declare = resolver.auto_declare;
            resolver.auto_declare = true;
            self.body.set_type(resolver.convert_type(cache, typ));
            resolver.auto_declare = prev_auto_declare;
        }

        self.body.define(resolver, cache);
        resolver.pop_lambda(cache);
    }
}

impl<'c> Resolvable<'c> for ast::FunctionCall<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        // Don't auto-declare functions in patterns, only arguments
        let prev_auto_declare = resolver.auto_declare;
        resolver.auto_declare = false;
        self.function.define(resolver, cache);
        resolver.auto_declare = prev_auto_declare;

        for arg in self.args.iter_mut() {
            arg.define(resolver, cache)
        }
    }
}

impl<'c> Resolvable<'c> for ast::Definition<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        let definition = self as *const Self;
        let definition = || DefinitionKind::Definition(trustme::make_mut(definition));

        resolver.push_let_binding_level();
        resolver.push_type_variable_scope();

        resolver.resolve_declarations(self.pattern.as_mut(), cache, definition);
        for id in resolver.definitions_collected.iter() {
            cache[*id].mutable = self.mutable;
        }

        self.level = Some(resolver.let_binding_level);
        resolver.pop_type_variable_scope();
        resolver.pop_let_binding_level();
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        // Tag the symbol with its definition so while type checking we can follow
        // the symbol to its definition if it is undefined.
        let definition = self as *const Self;
        let definition = || DefinitionKind::Definition(trustme::make_mut(definition));

        let old_graph_state = cache.global_dependency_graph.enter_definition();

        resolver.push_let_binding_level();
        resolver.push_type_variable_scope();

        resolver.resolve_definitions(self.pattern.as_mut(), cache, definition);
        for id in resolver.definitions_collected.iter() {
            cache[*id].mutable = self.mutable;
        }

        self.level = Some(resolver.let_binding_level);

        resolver.try_set_current_function(self);
        self.expr.define(resolver, cache);

        resolver.pop_type_variable_scope();
        resolver.pop_let_binding_level();

        cache.global_dependency_graph.exit_definition(old_graph_state);
    }
}

impl<'c> Resolvable<'c> for ast::If<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.condition.define(resolver, cache);

        resolver.push_scope(cache);
        self.then.define(resolver, cache);
        resolver.pop_scope(cache, true, None);

        resolver.push_scope(cache);
        self.otherwise.define(resolver, cache);
        resolver.pop_scope(cache, true, None);
    }
}

impl<'c> Resolvable<'c> for ast::Match<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.expression.define(resolver, cache);

        for (pattern, rhs) in self.branches.iter_mut() {
            resolver.push_scope(cache);

            resolver.resolve_definitions(pattern, cache, || DefinitionKind::MatchPattern);

            rhs.define(resolver, cache);
            resolver.pop_scope(cache, true, None);
        }
    }
}

/// Given "type T a b c = ..." return
/// forall a b c. args -> T a b c
fn create_variant_constructor_type(
    parent_type_id: TypeInfoId, args: Vec<Type>, cache: &mut ModuleCache,
) -> GeneralizedType {
    let info = &cache.type_infos[parent_type_id.0];
    let mut result = Type::UserDefined(parent_type_id);

    // Apply T to [a, b, c] if [a, b, c] is non-empty
    if !info.args.is_empty() {
        let type_variables = fmap(&info.args, |id| Type::TypeVariable(*id));
        result = Type::TypeApplication(Box::new(result), type_variables);
    }

    let type_args = info.args.clone();

    // Create the arguments to the function type if this type has arguments
    if !args.is_empty() {
        result = Type::Function(FunctionType {
            parameters: args,
            return_type: Box::new(result),
            environment: Box::new(Type::UNIT),
            effects: Box::new(Type::Effects(EffectSet::pure())),
            has_varargs: false,
        });
    }

    // finally, wrap the type in a forall if it has type variables
    if !type_args.is_empty() {
        GeneralizedType::PolyType(type_args, result)
    } else {
        GeneralizedType::MonoType(result)
    }
}

type Variants<'c> = Vec<(String, Vec<ast::Type<'c>>, Location<'c>)>;

/// Declare variants of a sum type given:
/// vec: A vector of each variant. Has a tuple of the variant's name arguments, and location for each.
/// parent_type_id: The TypeInfoId of the parent type.
fn create_variants<'c>(
    vec: &Variants<'c>, parent_type_id: TypeInfoId, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>,
) -> Vec<TypeConstructor<'c>> {
    let mut tag = 0;
    fmap(vec, |(name, types, location)| {
        let args = fmap(types, |t| resolver.convert_type(cache, t));

        let id = resolver.push_definition(name, cache, *location);
        let constructor_type = create_variant_constructor_type(parent_type_id, args.clone(), cache);

        cache.definition_infos[id.0].typ = Some(constructor_type);
        cache.definition_infos[id.0].definition =
            Some(DefinitionKind::TypeConstructor { name: name.clone(), tag: Some(tag) });

        tag += 1;
        TypeConstructor { name: name.clone(), args, id, location: *location }
    })
}

type Fields<'c> = Vec<(String, ast::Type<'c>, Location<'c>)>;

fn create_fields<'c>(vec: &Fields<'c>, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) -> Vec<Field<'c>> {
    fmap(vec, |(name, field_type, location)| {
        let field_type = resolver.convert_type(cache, field_type);

        Field { name: name.clone(), field_type, location: *location }
    })
}

impl<'c> Resolvable<'c> for ast::TypeDefinition<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        let args = fmap(&self.args, |_| cache.next_type_variable_id(resolver.let_binding_level));
        let id = resolver.push_type_info(self.name.clone(), args, cache, self.location);
        self.type_info = Some(id);
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        if self.type_info.is_none() {
            self.declare(resolver, cache);
        }

        resolver.push_type_variable_scope();
        let id = self.type_info.unwrap();

        // Re-add the typevariables we created in TypeDefinition::declare back into scope
        let existing_ids = cache.type_infos[id.0].args.clone();
        resolver.add_existing_type_variables_to_scope(&self.args, &existing_ids, self.location, cache);

        let type_id = self.type_info.unwrap();
        match &self.definition {
            ast::TypeDefinitionBody::Union(vec) => {
                let variants = create_variants(vec, type_id, resolver, cache);
                let type_info = &mut cache.type_infos[type_id.0];
                type_info.body = TypeInfoBody::Union(variants);
            },
            ast::TypeDefinitionBody::Struct(vec) => {
                let fields = create_fields(vec, resolver, cache);
                let field_types = fmap(&fields, |field| field.field_type.clone());

                let type_info = &mut cache.type_infos[type_id.0];
                type_info.body = TypeInfoBody::Struct(fields);

                // Create the constructor for this type.
                // This is done inside create_variants for tagged union types
                let id = resolver.push_definition(&self.name, cache, self.location);
                let constructor_type = create_variant_constructor_type(type_id, field_types, cache);

                cache.definition_infos[id.0].typ = Some(constructor_type);
                cache.definition_infos[id.0].definition =
                    Some(DefinitionKind::TypeConstructor { name: self.name.clone(), tag: None });
            },
            ast::TypeDefinitionBody::Alias(typ) => {
                let typ = resolver.convert_type(cache, typ);
                let type_info = &mut cache.type_infos[self.type_info.unwrap().0];
                type_info.body = TypeInfoBody::Alias(typ);
            },
        }

        resolver.pop_type_variable_scope();
    }
}

impl<'c> Resolvable<'c> for ast::TypeAnnotation<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.lhs.define(resolver, cache);

        if let ast::Type::Function(function) = &mut self.rhs {
            resolver.desugar_function_effect_variables(&mut function.parameters, &mut function.effects, cache);
        }

        let rhs = resolver.convert_type(cache, &self.rhs);
        self.typ = Some(rhs);
    }
}

fn absolute_path(relative_path: &Path, cache: &ModuleCache) -> Option<PathBuf> {
    let relative_path = PathBuf::from(relative_path);

    for root in cache.relative_roots.iter() {
        let path = root.join(&relative_path).with_extension("an");

        if !(cache.file_cache.contains_key(path.as_path()) || path.is_file()) {
            continue;
        };

        return Some(path);
    }
    None
}

pub fn declare_module<'a>(path: &Path, cache: &mut ModuleCache<'a>, error_location: Location<'a>) -> Option<ModuleId> {
    let path = match absolute_path(path, cache) {
        Some(p) => p,
        _ => {
            cache.push_diagnostic(error_location, D::CouldNotOpenFileForImport(path.to_owned()));
            return None;
        },
    };

    if let Some(module_id) = cache.modules.get(&path) {
        let existing_resolver = cache.name_resolvers.get_mut(module_id.0).unwrap();
        match existing_resolver.state {
            NameResolutionState::NotStarted => (),
            _ => {
                return Some(existing_resolver.module_id); // already declared
            },
        }
    }

    let path = cache.push_filepath(PathBuf::from(&path));

    let contents = cache.get_contents(path).unwrap();

    timing::start_time("Lexing");
    let tokens = Lexer::new(path, contents).collect::<Vec<_>>();

    timing::start_time("Parsing");
    let result = parser::parse(&tokens);

    timing::start_time("Name Resolution (Declare)");
    if result.is_err() {
        return None;
    }

    let ast = result.unwrap();
    let import_resolver = NameResolver::declare(ast, cache);
    Some(import_resolver.module_id)
}

pub fn define_module<'a>(
    module_id: ModuleId, cache: &mut ModuleCache<'a>, error_location: Location<'a>,
) -> Option<&'a Scope> {
    let import = cache.name_resolvers.get_mut(module_id.0).unwrap();
    match import.state {
        NameResolutionState::NotStarted | NameResolutionState::DeclareInProgress => {
            cache
                .push_diagnostic(error_location, D::InternalError("imported module has been defined but not declared"));
            return None;
        },
        NameResolutionState::Declared => {
            import.define(cache);
        },
        // Any module that is at least declared should already have its public exports available
        NameResolutionState::DefineInProgress | NameResolutionState::Defined => (),
    }

    Some(&import.exports)
}

impl<'c> Resolvable<'c> for ast::Import<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        let relative_path = self.path.clone().join("/");
        self.module_id = declare_module(Path::new(&relative_path), cache, self.location);
        if let Some(module_id) = self.module_id {
            resolver.current_scope().modules.insert(relative_path, module_id);
        }
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        if let Some(module_id) = self.module_id {
            if let Some(exports) = define_module(module_id, cache, self.location) {
                // import only the imported symbols
                resolver.current_scope().import(exports, cache, self.location, &self.symbols);
                // add the module scope itself
                resolver.module_scopes.insert(module_id, exports.to_owned());
            }
        }
    }
}

impl<'c> Resolvable<'c> for ast::TraitDefinition<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        // A trait definition's level is the outer level. The `let_binding_level + 1` is
        // only used while recurring _inside_ definitions, and trait definition's only
        // contain declarations which have no rhs to recur into. Changing this to
        // `let_binding_level + 1` will cause all trait functions to not be generalized.
        self.level = Some(resolver.let_binding_level);
        resolver.push_let_binding_level();

        let args = fmap(&self.args, |_| cache.next_type_variable_id(resolver.let_binding_level));
        let fundeps = fmap(&self.fundeps, |_| cache.next_type_variable_id(resolver.let_binding_level));

        assert!(resolver.current_trait.is_none());

        let trait_id =
            resolver.push_trait(self.name.clone(), args, fundeps, trustme::extend_lifetime(self), cache, self.location);

        resolver.current_trait = Some(trait_id);

        let self_pointer = self as *const _;
        for declaration in self.declarations.iter_mut() {
            let definition = || DefinitionKind::TraitDefinition(trustme::make_mut(self_pointer));

            resolver.desugar_function_effect_variables_in_type(&mut declaration.rhs, cache);
            resolver.resolve_declarations(declaration.lhs.as_mut(), cache, definition);
        }

        resolver.current_trait = None;
        self.trait_info = Some(trait_id);
        resolver.pop_let_binding_level();
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        if self.trait_info.is_none() {
            self.declare(resolver, cache);
        }

        if self.declarations.get(0).map_or(false, |decl| decl.typ.is_none()) {
            resolver.push_type_variable_scope();

            // Re-add the typevariables we created in TraitDefinition::declare back into scope
            let trait_info = &cache.trait_infos[self.trait_info.unwrap().0];
            let typeargs = trait_info.typeargs.clone();
            let fundeps = trait_info.fundeps.clone();

            resolver.add_existing_type_variables_to_scope(&self.args, &typeargs, self.location, cache);
            resolver.add_existing_type_variables_to_scope(&self.fundeps, &fundeps, self.location, cache);

            for declaration in self.declarations.iter_mut() {
                let prev_auto_declare = resolver.auto_declare;
                resolver.auto_declare = true;
                let rhs = resolver.convert_type(cache, &declaration.rhs);
                resolver.auto_declare = prev_auto_declare;
                declaration.typ = Some(rhs);
            }
            resolver.pop_type_variable_scope();
        }
    }
}

impl<'c> Resolvable<'c> for ast::TraitImpl<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        let id = resolver.lookup_trait(&self.trait_name, cache);
        self.trait_info = id;
        resolver.current_trait = id;

        let trait_id = match &self.trait_info {
            Some(id) => *id,
            None => {
                cache.push_diagnostic(self.location, D::NotInScope("Trait", self.trait_name.clone()));
                return;
            },
        };

        resolver.push_type_variable_scope();
        let prev_auto_declare = resolver.auto_declare;
        resolver.auto_declare = true;
        self.trait_arg_types = fmap(&self.trait_args, |arg| resolver.convert_type(cache, arg));
        resolver.auto_declare = prev_auto_declare;

        let trait_info = &cache.trait_infos[trait_id.0];
        resolver.required_definitions = Some(trait_info.definitions.clone());

        // The user is required to specify all of the trait's typeargs and functional dependencies.
        let required_arg_count = trait_info.typeargs.len() + trait_info.fundeps.len();
        if self.trait_args.len() != required_arg_count {
            let trait_name = self.trait_name.clone();
            let error = D::IncorrectImplTraitArgCount(trait_name, required_arg_count, self.trait_args.len());
            cache.push_diagnostic(self.location, error);
        }

        resolver.push_scope(cache);
        resolver.push_let_binding_level();

        // Declare the names first so we can check them all against the required_definitions
        let definitions = resolver.resolve_trait_impl_declarations(&mut self.definitions, cache);

        // TODO cleanup: is required_definitions still required since we can
        // resolve_all_definitions now? The checks in push_definition can probably
        // be moved here instead
        for required_definition in resolver.required_definitions.as_ref().unwrap() {
            let name = cache.definition_infos[required_definition.0].name.clone();
            cache.push_diagnostic(self.location, D::MissingImplDefinition(name));
        }

        resolver.required_definitions = None;
        resolver.current_trait = None;

        // All the names are present, now define them.
        for definition in self.definitions.iter_mut() {
            definition.expr.define(resolver, cache);
            definition.level = Some(resolver.let_binding_level);
        }

        let given = resolver.resolve_required_traits(&self.given, cache);

        resolver.pop_let_binding_level();
        resolver.pop_scope(cache, false, None);
        resolver.pop_type_variable_scope();

        let trait_impl = trustme::extend_lifetime(self);
        self.impl_id = Some(resolver.push_trait_impl(
            trait_id,
            self.trait_arg_types.clone(),
            definitions,
            trait_impl,
            given,
            cache,
            self.locate(),
        ));
    }
}

impl<'c> Resolvable<'c> for ast::Return<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.expression.define(resolver, cache);
    }
}

impl<'c> Resolvable<'c> for ast::Sequence<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        for statement in self.statements.iter_mut() {
            statement.declare(resolver, cache)
        }
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        for statement in self.statements.iter_mut() {
            statement.define(resolver, cache)
        }
    }
}

impl<'c> Resolvable<'c> for ast::Extern<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        // A trait definition's level is the outer level. The `let_binding_level + 1` is
        // only used while recurring _inside_ definitions, and trait definition's only
        // contain declarations which have no rhs to recur into. Changing this to
        // `let_binding_level + 1` will cause all trait functions to not be generalized.
        self.level = Some(resolver.let_binding_level);
        resolver.push_let_binding_level();
        resolver.resolve_extern_definitions(self, cache);
        resolver.pop_let_binding_level();
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        // Any extern in global scope should already be defined in the declaration pass
        if !resolver.in_global_scope() {
            self.declare(resolver, cache);
        }
    }
}

impl<'c> Resolvable<'c> for ast::MemberAccess<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.lhs.define(resolver, cache);
    }
}

impl<'c> Resolvable<'c> for ast::Assignment<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.lhs.define(resolver, cache);
        self.rhs.define(resolver, cache);
    }
}

impl<'c> Resolvable<'c> for ast::EffectDefinition<'c> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        resolver.push_type_variable_scope();

        // An effect definition's level is the outer level. The `let_binding_level + 1` is
        // only used while recurring _inside_ definitions, and effect definition's only
        // contain declarations which have no rhs to recur into. Changing this to
        // `let_binding_level + 1` will cause all effect functions to not be generalized.
        self.level = Some(resolver.let_binding_level);
        resolver.push_let_binding_level();

        let args = fmap(&self.args, |_| cache.next_type_variable_id(resolver.let_binding_level));

        let effect_id = resolver.push_effect(self.name.clone(), args, cache, self.location);

        let self_pointer = self as *const _;
        for declaration in self.declarations.iter_mut() {
            let definition = || DefinitionKind::EffectDefinition(trustme::make_mut(self_pointer));

            resolver.desugar_function_effect_variables_in_type(&mut declaration.rhs, cache);
            resolver.resolve_declarations(declaration.lhs.as_mut(), cache, definition);

            for definition in &resolver.definitions_collected {
                cache[effect_id].declarations.push(*definition);
            }

            if !matches!(&declaration.rhs, ast::Type::Function(..)) {
                cache.push_diagnostic(declaration.rhs.locate(), D::EffectsMustBeFunctions);
            }
        }

        self.effect_info = Some(effect_id);
        resolver.pop_type_variable_scope();
        resolver.pop_let_binding_level();
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        if self.effect_info.is_none() {
            self.declare(resolver, cache);
        }

        if self.declarations.get(0).map_or(false, |decl| decl.typ.is_none()) {
            resolver.push_type_variable_scope();

            // Re-add the typevariables we created in TraitDefinition::declare back into scope
            let typeargs = cache.effect_infos[self.effect_info.unwrap().0].typeargs.clone();
            resolver.add_existing_type_variables_to_scope(&self.args, &typeargs, self.location, cache);

            for declaration in self.declarations.iter_mut() {
                let prev_auto_declare = resolver.auto_declare;
                resolver.auto_declare = true;
                let rhs = resolver.convert_type(cache, &declaration.rhs);
                resolver.auto_declare = prev_auto_declare;
                declaration.typ = Some(rhs);
            }
            resolver.pop_type_variable_scope();
        }
    }
}

fn get_handled_effect_function<'a>(pattern: &ast::Ast<'a>, cache: &mut ModuleCache<'a>) -> Option<DefinitionInfoId> {
    let location = match pattern {
        Ast::FunctionCall(call) => match call.function.as_ref() {
            Ast::Variable(variable) => return variable.definition,
            _ => call.function.locate(),
        },
        Ast::Return(_) => return None,
        _ => pattern.locate(),
    };
    cache.push_diagnostic(location, D::InvalidHandlerPattern);
    None
}

impl<'c> Resolvable<'c> for ast::Handle<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.expression.define(resolver, cache);

        // A BTreeSet is used here over a HashSet to maintain a consistent
        // ordering for the error message issued at the end.
        let mut remaining_cases = BTreeSet::new();

        for (pattern, rhs) in self.branches.iter_mut() {
            resolver.push_scope(cache);
            resolver.resolve_definitions(pattern, cache, || DefinitionKind::MatchPattern);

            // Define an implicit 'resume' variable
            let resume = resolver.push_definition("resume", cache, pattern.locate());
            cache[resume].ignore_unused_warning = true;
            self.resumes.push(resume);

            rhs.define(resolver, cache);
            resolver.pop_scope(cache, true, None);

            if let Some(case) = get_handled_effect_function(pattern, cache) {
                // Remove the case from the remaining cases that we need to handle.
                // If it was not in the list then it is part of a new effect for
                // which we need to add the functions of to our remaining cases.
                if !remaining_cases.remove(&case) {
                    let info = &cache.definition_infos[case.0];

                    match &info.definition {
                        Some(DefinitionKind::EffectDefinition(effect)) => {
                            let id = effect.effect_info.unwrap();
                            remaining_cases.extend(cache[id].declarations.iter().copied());
                            remaining_cases.remove(&case);
                        },
                        _ => cache.push_diagnostic(pattern.locate(), D::NotAnEffect(info.name.clone())),
                    }
                }
            }
        }

        if !remaining_cases.is_empty() {
            let missing_cases = fmap(remaining_cases, |id| cache[id].name.clone());
            cache.push_diagnostic(self.location, D::HandlerMissingCases(missing_cases));
        }
    }
}

impl<'c> Resolvable<'c> for ast::NamedConstructor<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        let type_name = match self.constructor.as_ref() {
            Ast::Variable(ast::Variable { kind, .. }) => kind.name(),
            _ => {
                // This should never happen since constructor is parsed with the `variant` parser
                cache.push_diagnostic(
                    self.constructor.locate(),
                    D::InternalError("Expected constructor field to be a Variable"),
                );
                return;
            },
        };

        // This will increment the use count for that type.
        // It will result in it being one higher than it needs to,
        // as the define pass on the sequence will do it again,
        // since it ends with a FunctionCall. Is that a problem?
        let type_info = match resolver.lookup_type(type_name.as_ref(), cache) {
            Some(id) => &cache.type_infos[id.0],
            None => {
                cache.push_diagnostic(self.location, D::NotInScope("Type", type_name.into_owned()));
                return;
            },
        };

        // Field names in the order they appear in the type definition
        let struct_fields = match &type_info.body {
            TypeInfoBody::Struct(fields) => fields.iter().map(|field| &field.name),
            _ => {
                cache.push_diagnostic(self.constructor.locate(), D::NotAStruct(type_name.into_owned()));
                return;
            },
        };
        let statements = match self.sequence.as_mut() {
            Ast::Sequence(ast::Sequence { statements, .. }) => statements,
            _ => {
                // This should never happen again, but it's better to emit an error than panic
                cache.push_diagnostic(self.location, D::InternalError("Expected statements field to be a Variable"));
                return;
            },
        };

        // Fields referenced in the constructor
        let mut defined_fields = statements
            .iter()
            .map(|stmt| {
                let (variable, location) = match stmt {
                    Ast::Definition(ast::Definition { pattern, location, .. }) => (pattern.as_ref(), location),
                    Ast::Variable(v) => (stmt, &v.location),
                    _ => unreachable!(),
                };

                let name = match variable {
                    Ast::Variable(ast::Variable { kind: ast::VariableKind::Identifier(name), .. }) => name,
                    _ => unreachable!(),
                };

                (name, (variable, location))
            })
            .collect::<HashMap<_, _>>();

        let (missing_fields, args) =
            struct_fields.fold((Vec::new(), Vec::new()), |(mut missing_fields, mut args), field| {
                if let Some((variable, _)) = defined_fields.remove(field) {
                    args.push(variable.clone());
                } else {
                    missing_fields.push(field);
                }
                (missing_fields, args)
            });

        let has_missing_fields = !missing_fields.is_empty();
        let has_unknown_fields = !defined_fields.is_empty();

        if has_missing_fields {
            cache.push_diagnostic(
                self.constructor.locate(),
                D::MissingFields(missing_fields.into_iter().cloned().collect()),
            );
        }

        for (name, (_, location)) in defined_fields {
            cache.push_diagnostic(*location, D::NotAStructField(name.clone()));
        }

        if has_missing_fields || has_unknown_fields {
            return;
        }

        let call = ast::Ast::function_call(self.constructor.as_ref().clone(), args, self.location);

        // We only want to keep definitions in the sequence to keep the Hir simpler
        statements.retain(|stmt| matches!(stmt, Ast::Definition(_)));
        statements.push(call);

        resolver.push_scope(cache);
        self.sequence.define(resolver, cache);
        resolver.pop_scope(cache, false, None);
    }
}

impl<'c> Resolvable<'c> for ast::Reference<'c> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'c>) {}

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'c>) {
        self.expression.define(resolver, cache);
    }
}
