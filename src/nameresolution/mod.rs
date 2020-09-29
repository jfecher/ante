use crate::parser::{ self, ast, ast::Ast };
use crate::types::{ TypeInfoId, TypeVariableId, Type, PrimitiveType, TypeInfoBody };
use crate::types::{ TypeConstructor, Field, LetBindingLevel, STRING_TYPE };
use crate::types::traits::Impl;
use crate::error::{ self, location::{ Location, Locatable } };
use crate::cache::{ ModuleCache, DefinitionInfoId, ModuleId };
use crate::cache::{ TraitInfoId, ImplInfoId, DefinitionNode, ImplScopeId };
use crate::nameresolution::scope::Scope;
use crate::lexer::Lexer;
use crate::util::{ fmap, trustme };

use colored::Colorize;

use std::fs::File;
use std::io::{ BufReader, Read };
use std::path::{ Path, PathBuf };

mod scope;
pub mod builtin;

/// Specifies how far a particular module is in name resolution.
/// Keeping this properly up to date for each module is the
/// key for preventing infinite recursion when declaring recursive imports.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum NameResolutionState {
    NotStarted,
    DeclareInProgress,
    Declared,
    DefineInProgress,
    Defined,
}

#[derive(Debug)]
pub struct NameResolver {
    filepath: PathBuf,

    /// The stack of functions we are currently compiling.
    /// Since we do not follow function calls, we are only inside multiple
    /// functions when their definitions are nested, e.g. in:
    ///
    /// foo () =
    ///     bar () = 3
    ///     bar () + 2
    ///
    /// Our callstack would consist of [main/global scope, foo, bar]
    scopes: Vec<Scope>,

    /// Contains all the publically exported symbols of the current module.
    /// The purpose of the 'declare' pass is to fill this field out for
    /// all modules used in the program. The exported symbols need not
    /// be defined until the 'define' pass later however.
    pub exports: Scope,

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
        fn $name<'b>(&self, name: &str, cache: &mut ModuleCache<'b>) -> Option<$return_type> {
            for stack in self.scopes.iter().rev() {
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

impl NameResolver {
    lookup_fn!(lookup_definition, definitions, definition_infos, DefinitionInfoId);
    lookup_fn!(lookup_type, types, type_infos, TypeInfoId);
    lookup_fn!(lookup_trait, traits, trait_infos, TraitInfoId);

    fn lookup_type_variable(&self, name: &str) -> Option<TypeVariableId> {
        for scope in self.type_variable_scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }

        None
    }

    pub fn push_scope(&mut self, cache: &mut ModuleCache) {
        self.scopes.push(Scope::new(cache));
        let impl_scope = self.current_scope().impl_scope;

        // TODO optimization: this really shouldn't be necessary to copy all the
        // trait impl ids for each scope just so Variables can store their scope
        // for the type checker to do trait resolution.
        for scope in self.scopes.iter().rev() {
            for (_, impls) in scope.impls.iter() {
                cache.impl_scopes[impl_scope.0].append(&mut impls.clone());
            }
        }
    }

    pub fn push_type_variable_scope(&mut self) {
        self.type_variable_scopes.push(scope::TypeVariableScope::default());
    }

    pub fn push_existing_type_variable(&mut self, key: String, id: TypeVariableId) -> TypeVariableId {
        let top = self.type_variable_scopes.len() - 1;
        self.type_variable_scopes[top].push_existing_type_variable(key, id)
    }

    pub fn push_new_type_variable<'b>(&mut self, key: String, cache: &mut ModuleCache<'b>) -> TypeVariableId {
        let id = cache.next_type_variable_id(self.let_binding_level);
        self.push_existing_type_variable(key, id)
    }

    pub fn pop_scope<'b>(&mut self, cache: &mut ModuleCache<'b>, warn_unused: bool) {
        if warn_unused {
            self.current_scope().check_for_unused_definitions(cache);
        }
        self.scopes.pop();
    }

    pub fn pop_type_variable_scope(&mut self) {
        self.type_variable_scopes.pop();
    }

    pub fn current_scope(&mut self) -> &mut Scope {
        let top = self.scopes.len() - 1;
        &mut self.scopes[top]
    }

    pub fn global_scope(&self) -> &Scope {
        &self.scopes[0]
    }

    fn in_global_scope(&self) -> bool {
        self.scopes.len() == 1
    }

    pub fn attach_to_trait<'a>(definition_id: DefinitionInfoId, trait_id: TraitInfoId, impl_scope_id: ImplScopeId, cache: &mut ModuleCache<'a>) {
        let trait_info = &mut cache.trait_infos[trait_id.0];
        trait_info.definitions.push(definition_id);

        // Tag the function with Trait a b c as the required impl using the original
        // type arguments from the trait declaration.
        let args = trait_info.typeargs.iter().chain(trait_info.fundeps.iter())
            .copied().map(Type::TypeVariable).collect();

        let binding_id = cache.push_impl_binding();
        let def = &mut cache.definition_infos[definition_id.0];
        def.required_impls.push(Impl::new(trait_id, impl_scope_id, binding_id, args));
    }

    fn check_required_definitions<'b>(&mut self, name: &str, id: DefinitionInfoId, cache: &mut ModuleCache<'b>, location: Location<'b>) {
        if let Some(existing_id) = self.current_scope().definitions.get(name) {
            let existing_definition = &cache.definition_infos[existing_id.0];

            // required_impls is only ever pushed to for trait functions during name resolution
            // so it can be used here to check if we're re-using a name that is not from the
            // trait we're currently implementing.
            if existing_definition.required_impls.is_empty() {
                error!(location, "{} is already in scope", name);
                note!(existing_definition.location, "{} previously defined here", name);
            }
        }

        let required_definitions = self.required_definitions.as_mut().unwrap();
        if let Some(index) = required_definitions.iter().position(|id| &cache.definition_infos[id.0].name == name) {
            let definition = required_definitions[index];
            cache.definition_infos[id.0].trait_definition = Some(definition);
            required_definitions.swap_remove(index);
        } else {
            let trait_info = &cache.trait_infos[self.current_trait.unwrap().0];
            error!(location, "{} is not required by {}", name, trait_info.name);
        }
    }

    pub fn push_definition<'b>(&mut self, name: &str, cache: &mut ModuleCache<'b>, location: Location<'b>) -> DefinitionInfoId {
        let id = cache.push_definition(name.to_owned(), location);
        if self.required_definitions.is_some() {
            // We're inside a trait impl right now, any definitions shouldn't be put in scope
            // else they'd collide with the declaration from the trait. Additionally, we must
            // ensure the definition is one of the ones required by the trait in required_definitions.
            self.check_required_definitions(name, id, cache, location);
        } else {
            if let Some(existing_definition) = self.current_scope().definitions.get(name) {
                error!(location, "{} is already in scope", name);
                let previous_location = cache.definition_infos[existing_definition.0].location;
                note!(previous_location, "{} previously defined here", name);
            }

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
                NameResolver::attach_to_trait(id, trait_id, self.current_scope().impl_scope, cache);
            }
        }
        id
    }

    pub fn push_type_info<'b>(&mut self, name: String, args: Vec<TypeVariableId>, cache: &mut ModuleCache<'b>, location: Location<'b>) ->  TypeInfoId {
        if let Some(existing_definition) = self.current_scope().types.get(&name) {
            error!(location, "{} is already in scope", name);
            let previous_location = cache.type_infos[existing_definition.0].locate();
            note!(previous_location, "{} previously defined here", name);
        }

        let id = cache.push_type_info(name.clone(), args, location);
        if self.in_global_scope() {
            self.exports.types.insert(name.clone(), id);
        }
        self.current_scope().types.insert(name, id);
        id
    }

    pub fn push_trait<'b>(&mut self, name: String, args: Vec<TypeVariableId>,
                fundeps: Vec<TypeVariableId>, cache: &mut ModuleCache<'b>, location: Location<'b>) -> TraitInfoId {

        if let Some(existing_definition) = self.current_scope().traits.get(&name) {
            error!(location, "{} is already in scope", name);
            let previous_location = cache.type_infos[existing_definition.0].locate();
            note!(previous_location, "{} previously defined here", name);
        }

        let id = cache.push_trait_definition(name.clone(), args, fundeps, location);
        if self.in_global_scope() {
            self.exports.traits.insert(name.clone(), id);
        }
        self.current_scope().traits.insert(name, id);
        id
    }

    pub fn push_trait_impl<'b>(&mut self, trait_id: TraitInfoId, args: Vec<Type>,
                definitions: Vec<DefinitionInfoId>, trait_impl: &'b mut ast::TraitImpl<'b>,
                cache: &mut ModuleCache<'b>, location: Location<'b>) -> ImplInfoId {

        // Any overlapping impls are only reported when they're used during typechecking
        let id = cache.push_trait_impl(trait_id, args, definitions, trait_impl, location);
        if self.in_global_scope() {
            self.exports.impls.entry(trait_id).or_default().push(id);
            cache.impl_scopes[self.exports.impl_scope.0].push(id);
        }

        self.current_scope().impls.entry(trait_id).or_default().push(id);
        cache.impl_scopes[self.current_scope().impl_scope.0].push(id);
        id
    }
}

impl<'b> NameResolver {
    pub fn start(ast: Ast<'b>, cache: &mut ModuleCache<'b>) -> Result<(), ()> {
        builtin::define_builtins(cache);
        let resolver = NameResolver::declare(ast, cache);
        resolver.define(cache);
        if error::get_error_count() != 0 {
            Err(())
        } else {
            Ok(())
        }
    }

    pub fn declare(ast: Ast<'b>, cache: &mut ModuleCache<'b>) -> &'b mut NameResolver {
        let filepath = ast.locate().filename;

        let existing = cache.get_name_resolver_by_path(&filepath);
        assert!(existing.is_none());

        let module_id = cache.push_ast(ast);
        cache.modules.insert(filepath.to_owned(), module_id);

        let mut resolver = NameResolver {
            filepath: filepath.to_owned(),
            scopes: vec![],
            exports: Scope::new(cache),
            type_variable_scopes: vec![scope::TypeVariableScope::default()],
            state: NameResolutionState::DeclareInProgress,
            auto_declare: false,
            current_trait: None,
            required_definitions: None,
            definitions_collected: vec![],
            let_binding_level: LetBindingLevel(1),
            module_id,
        };

        resolver.push_scope(cache);

        let existing = cache.get_name_resolver_by_path(&filepath);
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

    pub fn define(&mut self, cache: &mut ModuleCache<'b>) {
        let ast = cache.parse_trees.get_mut(self.module_id.0).unwrap();

        assert!(self.state == NameResolutionState::Declared);

        self.state = NameResolutionState::DefineInProgress;
        ast.define(self, cache);
        self.state = NameResolutionState::Defined;
    }

    /// Converts an ast::Type to a types::Type, expects all typevars to be in scope
    pub fn convert_type(&mut self, cache: &mut ModuleCache<'b>, ast_type: &ast::Type<'b>) -> Type {
        match ast_type {
            ast::Type::IntegerType(_) => Type::Primitive(PrimitiveType::IntegerType),
            ast::Type::FloatType(_) => Type::Primitive(PrimitiveType::FloatType),
            ast::Type::CharType(_) => Type::Primitive(PrimitiveType::CharType),
            ast::Type::StringType(_) => Type::UserDefinedType(STRING_TYPE),
            ast::Type::BooleanType(_) => Type::Primitive(PrimitiveType::BooleanType),
            ast::Type::UnitType(_) => Type::Primitive(PrimitiveType::UnitType),
            ast::Type::ReferenceType(_) => Type::Primitive(PrimitiveType::ReferenceType),
            ast::Type::FunctionType(args, ret, _) => {
                let args = fmap(args, |arg| self.convert_type(cache, arg));
                let ret = self.convert_type(cache, ret);
                Type::Function(args, Box::new(ret))
            },
            ast::Type::TypeVariable(name, location) => {
                match self.lookup_type_variable(name) {
                    Some(id) => Type::TypeVariable(id),
                    None => {
                        if self.auto_declare {
                            // TODO: This usage of MAX_BINDING_LEVEL is definitely unsound
                            let id = self.push_new_type_variable(name.clone(), cache);
                            Type::TypeVariable(id)
                        } else {
                            error!(*location, "Type variable {} was not found in scope", name);
                            Type::Primitive(PrimitiveType::IntegerType)
                        }
                    },
                }
            },
            ast::Type::UserDefinedType(name, location) => {
                match self.lookup_type(name, cache) {
                    Some(id) => Type::UserDefinedType(id),
                    None => {
                        error!(*location, "Type {} was not found in scope", name);
                        Type::Primitive(PrimitiveType::IntegerType)
                    },
                }
            },
            ast::Type::TypeApplication(constructor, args, _) => {
                let constructor = Box::new(self.convert_type(cache, constructor));
                let args = fmap(args, |arg| self.convert_type(cache, arg));
                Type::TypeApplication(constructor, args)
            },
        }
    }

    /// The collect* family of functions recurses over an irrefutable pattern, either declaring or
    /// defining each node and tagging the declaration with the given DefinitionNode.
    fn resolve_declarations<F>(&mut self, ast: &mut Ast<'b>, cache: &mut ModuleCache<'b>, mut definition: F)
        where F: FnMut() -> DefinitionNode<'b>
    {
        self.definitions_collected.clear();
        self.auto_declare = true;
        ast.declare(self, cache);
        self.auto_declare = false;
        for id in self.definitions_collected.iter() {
            cache.definition_infos[id.0].definition = Some(definition());
        }
    }

    fn resolve_definitions<T, F>(&mut self, ast: &mut T, cache: &mut ModuleCache<'b>, definition: F)
        where T: Resolvable<'b>,
              F: FnMut() -> DefinitionNode<'b>
    {
        self.resolve_all_definitions(vec![ast].into_iter(), cache, definition);
    }

    fn resolve_extern_definitions(&mut self, declaration: &mut ast::TypeAnnotation<'b>, cache: &mut ModuleCache<'b>) {
        self.definitions_collected.clear();
        self.auto_declare = true;
        declaration.define(self, cache);
        self.auto_declare = false;
        for id in self.definitions_collected.iter() {
            let declaration = trustme::extend_lifetime(declaration);
            cache.definition_infos[id.0].definition = Some(DefinitionNode::Extern(declaration));
        }
    }

    fn resolve_trait_impl_declarations<'a, T: 'a, It>(&mut self, patterns: It, cache: &mut ModuleCache<'b>) -> Vec<DefinitionInfoId>
        where It: Iterator<Item = &'a mut T>,
              T: Resolvable<'b>,
    {
        self.definitions_collected.clear();
        self.auto_declare = true;
        for pattern in patterns {
            pattern.declare(self, cache);
        }
        self.auto_declare = false;
        for id in self.definitions_collected.iter() {
            cache.definition_infos[id.0].definition = Some(DefinitionNode::Impl);
        }
        self.definitions_collected.clone()
    }

    fn resolve_all_definitions<'a, T: 'a, It, F>(&mut self, patterns: It, cache: &mut ModuleCache<'b>, mut definition: F)
        where It: Iterator<Item = &'a mut T>,
              T: Resolvable<'b>,
              F: FnMut() -> DefinitionNode<'b>
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
}

pub trait Resolvable<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>);
    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>);
}

impl<'b> Resolvable<'b> for Ast<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        dispatch_on_expr!(self, Resolvable::declare, resolver, cache);
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        dispatch_on_expr!(self, Resolvable::define, resolver, cache);
    }
}

impl<'b> Resolvable<'b> for ast::Literal<'b> {
    /// Purpose of the declare pass is to collect all the names of publically exported symbols
    /// so the define pass can work in the presense of mutually recursive modules.
    fn declare(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}

    /// Go through a module and annotate each variable with its declaration.
    /// Display any errors for variables without declarations.
    fn define(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}
}

impl<'b> Resolvable<'b> for ast::Variable<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        use ast::VariableKind::*;
        match &mut self.kind {
            Operator(token) => {
                let name = token.to_string();
                if resolver.auto_declare {
                    let id = resolver.push_definition(&name, cache, self.location);
                    resolver.definitions_collected.push(id);
                    self.definition = Some(id);
                }
            },
            Identifier(name) => {
                if resolver.auto_declare {
                    let id = resolver.push_definition(&name, cache, self.location);
                    resolver.definitions_collected.push(id);
                    self.definition = Some(id);
                }
            },
            TypeConstructor(name) => self.definition = resolver.lookup_definition(name, cache),
        }

        self.impl_scope = Some(resolver.current_scope().impl_scope);
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        if self.definition.is_none() {
            if resolver.auto_declare {
                self.declare(resolver, cache);
            } else {
                use ast::VariableKind::*;
                match &mut self.kind {
                    Operator(token) => {
                        self.definition = resolver.lookup_definition(&token.to_string(), cache);
                    },
                    Identifier(name) => {
                        self.definition = resolver.lookup_definition(name, cache);
                    },
                    TypeConstructor(name) => {
                        self.definition = resolver.lookup_definition(name, cache);
                    },
                }
                self.impl_scope = Some(resolver.current_scope().impl_scope);
            }

            // If it is still not declared, print an error
            if self.definition.is_none() {
                error!(self.locate(), "No declaration for {} was found in scope", self);
            }
        }
    }
}

impl<'b> Resolvable<'b> for ast::Lambda<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        resolver.push_scope(cache);
        resolver.resolve_all_definitions(self.args.iter_mut(), cache, || DefinitionNode::Parameter);
        self.body.define(resolver, cache);
        resolver.pop_scope(cache, true);
    }
}

impl<'b> Resolvable<'b> for ast::FunctionCall<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        self.function.define(resolver, cache);
        for arg in self.args.iter_mut() {
            arg.define(resolver, cache)
        }
    }
}

impl<'b> Resolvable<'b> for ast::Definition<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        let definition = self as *const Self;
        let definition = || DefinitionNode::Definition(trustme::make_mut(definition));

        resolver.let_binding_level = LetBindingLevel(resolver.let_binding_level.0 + 1);
        resolver.resolve_declarations(self.pattern.as_mut(), cache, definition);
        self.level = Some(resolver.let_binding_level);
        resolver.let_binding_level = LetBindingLevel(resolver.let_binding_level.0 - 1);
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        // Tag the symbol with its definition so while type checking we can follow
        // the symbol to its definition if it is undefined.
        let definition = self as *const Self;
        let definition = || DefinitionNode::Definition(trustme::make_mut(definition));

        resolver.let_binding_level = LetBindingLevel(resolver.let_binding_level.0 + 1);
        resolver.resolve_definitions(self.pattern.as_mut(), cache, definition);
        self.level = Some(resolver.let_binding_level);

        self.expr.define(resolver, cache);
        resolver.let_binding_level = LetBindingLevel(resolver.let_binding_level.0 - 1);
    }
}

impl<'b> Resolvable<'b> for ast::If<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        self.condition.define(resolver, cache);

        resolver.push_scope(cache);
        self.then.define(resolver, cache);
        resolver.pop_scope(cache, true);

        if let Some(otherwise) = &mut self.otherwise {
            resolver.push_scope(cache);
            otherwise.define(resolver, cache);
            resolver.pop_scope(cache, true);
        }
    }
}

impl<'b> Resolvable<'b> for ast::Match<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        self.expression.define(resolver, cache);

        for (pattern, rhs) in self.branches.iter_mut() {
            resolver.push_scope(cache);
            resolver.auto_declare = true;
            pattern.define(resolver, cache);
            resolver.auto_declare = false;

            rhs.define(resolver, cache);
            resolver.pop_scope(cache, true);
        }
    }
}

/// Given "type T a b c = ..." return
/// forall a b c. args -> T a b c
fn create_variant_constructor_type<'b>(parent_type_id: TypeInfoId, args: Vec<Type>, cache: &ModuleCache<'b>) -> Type {
    let info = &cache.type_infos[parent_type_id.0];
    let mut result = Type::UserDefinedType(parent_type_id);

    // Apply T to [a, b, c] if [a, b, c] is non-empty
    if !info.args.is_empty() {
        let type_variables = fmap(&info.args, |id| Type::TypeVariable(*id));
        result = Type::TypeApplication(Box::new(result), type_variables);
    }

    // Create the arguments to the function type if this type has arguments
    if !args.is_empty() {
        result = Type::Function(args, Box::new(result));
    }

    // finally, wrap the type in a forall if it has type variables
    if !info.args.is_empty() {
        result = Type::ForAll(info.args.clone(), Box::new(result))
    }

    result
}

type Variants<'b> = Vec<(String, Vec<ast::Type<'b>>, Location<'b>)>;

/// Declare variants of a sum type given:
/// vec: A vector of each variant. Has a tuple of the variant's name arguments, and location for each.
/// parent_type_id: The TypeInfoId of the parent type.
fn create_variants<'b>(vec: &Variants<'b>, parent_type_id: TypeInfoId,
        resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) -> Vec<TypeConstructor<'b>> {

    let mut index = 0;
    fmap(&vec, |(name, types, location)| {
        let args = fmap(&types, |t| resolver.convert_type(cache, t));

        let id = resolver.push_definition(&name, cache, *location);
        cache.definition_infos[id.0].typ = Some(create_variant_constructor_type(parent_type_id, args.clone(), cache));
        cache.definition_infos[id.0].definition = Some(DefinitionNode::TypeConstructor { name: name.clone(), tag: Some(index) });
        index += 1;
        TypeConstructor { name: name.clone(), args, id, location: *location }
    })
}

type Fields<'b> = Vec<(String, ast::Type<'b>, Location<'b>)>;

fn create_fields<'b>(vec: &Fields<'b>, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) -> Vec<Field<'b>> {

    fmap(&vec, |(name, field_type, location)| {
        let field_type = resolver.convert_type(cache, field_type);

        Field { name: name.clone(), field_type, location: *location }
    })
}

impl<'b> Resolvable<'b> for ast::TypeDefinition<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        let args = fmap(&self.args, |_| cache.next_type_variable_id(resolver.let_binding_level));
        let id = resolver.push_type_info(self.name.clone(), args, cache, self.location);
        self.type_info = Some(id);
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        if self.type_info.is_none() {
            self.declare(resolver, cache);
        }

        resolver.push_type_variable_scope();
        let id = self.type_info.unwrap();

        {
            let keys = self.args.iter();
            let ids = &mut cache.type_infos[id.0].args.iter();
            // re-insert the typevars into scope.
            // These names are guarenteed to not collide since we just pushed a new scope.
            for (key, id) in keys.zip(ids) {
                resolver.push_existing_type_variable(key.clone(), *id);
            }
        }

        let type_id = self.type_info.unwrap();
        match &self.definition {
            ast::TypeDefinitionBody::UnionOf(vec) => {
                let variants = create_variants(vec, type_id, resolver, cache);
                let type_info = &mut cache.type_infos[type_id.0];
                type_info.body = TypeInfoBody::Union(variants);
            },
            ast::TypeDefinitionBody::StructOf(vec) => {
                let fields = create_fields(vec, resolver, cache);
                let field_types = fmap(&fields, |field| field.field_type.clone());

                let type_info = &mut cache.type_infos[type_id.0];
                type_info.body = TypeInfoBody::Struct(fields);

                // Create the constructor for this type.
                // This is done inside create_variants for tagged union types
                let id = resolver.push_definition(&self.name, cache, self.location);
                cache.definition_infos[id.0].typ = Some(create_variant_constructor_type(type_id, field_types, cache));
                cache.definition_infos[id.0].definition = Some(DefinitionNode::TypeConstructor { name: self.name.clone(), tag: None });
            },
            ast::TypeDefinitionBody::AliasOf(typ) => {
                let typ = resolver.convert_type(cache, typ);
                let type_info = &mut cache.type_infos[self.type_info.unwrap().0];
                type_info.body = TypeInfoBody::Alias(typ);
            },
        }

        resolver.pop_type_variable_scope();
    }
}

impl<'b> Resolvable<'b> for ast::TypeAnnotation<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        self.lhs.define(resolver, cache);
        let rhs = resolver.convert_type(cache, &self.rhs);
        self.typ = Some(rhs);
    }
}

fn find_file<'a>(relative_path: &Path, cache: &mut ModuleCache) -> Option<(File, PathBuf)> {
    for root in cache.relative_roots.iter() {
        let path = root.join(relative_path).with_extension("an");

        let file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => continue,
        };

        return Some((file, path));
    }
    None
}

pub fn declare_module<'a>(path: &Path, cache: &mut ModuleCache<'a>, error_location: Location) -> Option<ModuleId> {
    let (file, path) = match find_file(path, cache) {
        Some((f, p)) => (f, p),
        _ => {
            error!(error_location, "Couldn't open file for import: {}.an", path.display());
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

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();

    let tokens = Lexer::new(&path, &contents).collect::<Vec<_>>();
    let result = parser::parse(&tokens);

    if result.is_err() {
        return None;
    }

    let ast = result.unwrap();
    let import_resolver = NameResolver::declare(ast, cache);
    Some(import_resolver.module_id)
}

pub fn define_module<'a>(module_id: ModuleId, cache: &mut ModuleCache<'a>, error_location: Location) -> Option<&'a Scope> {
    let import = cache.name_resolvers.get_mut(module_id.0).unwrap();
    match import.state {
        NameResolutionState::NotStarted
        | NameResolutionState::DeclareInProgress => {
            error!(error_location, "Internal compiler error: imported module has been defined but not declared");
            return None;
        },
        | NameResolutionState::Declared => {
            import.define(cache);
        },
        // Any module that is at least declared should already have its public exports available
        | NameResolutionState::DefineInProgress
        | NameResolutionState::Defined => (),
    }

    Some(&import.exports)
}

impl<'b> Resolvable<'b> for ast::Import<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        let relative_path = self.path.clone().join("/");
        self.module_id = declare_module(Path::new(&relative_path), cache, self.location);
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        let module_id = self.module_id.unwrap();
        define_module(module_id, cache, self.location).map(|exports| {
            resolver.current_scope().import(&exports, cache, self.location);
        });
    }
}

impl<'b> Resolvable<'b> for ast::TraitDefinition<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        resolver.push_type_variable_scope();

        let args = fmap(&self.args, |arg|
            resolver.push_new_type_variable(arg.clone(), cache));

        let fundeps = fmap(&self.fundeps, |arg|
            resolver.push_new_type_variable(arg.clone(), cache));

        assert!(resolver.current_trait.is_none());
        let trait_id = resolver.push_trait(self.name.clone(), args, fundeps, cache, self.location);
        resolver.current_trait = Some(trait_id);

        let self_pointer = self as *const _;
        for declaration in self.declarations.iter_mut() {
            let definition = || DefinitionNode::TraitDefinition(trustme::make_mut(self_pointer));
            resolver.resolve_declarations(declaration.lhs.as_mut(), cache, definition);

            resolver.auto_declare = true;
            let rhs = resolver.convert_type(cache, &declaration.rhs);
            resolver.auto_declare = false;
            declaration.typ = Some(rhs);
        }

        resolver.current_trait = None;
        self.trait_info = Some(trait_id);
        resolver.pop_type_variable_scope();
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        if self.trait_info.is_none() {
            self.declare(resolver, cache);
        }
    }
}

impl<'b> Resolvable<'b> for ast::TraitImpl<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        let id = resolver.lookup_trait(&self.trait_name, cache);
        self.trait_info = id;
        resolver.current_trait = id;

        let trait_id = match &self.trait_info {
            Some(id) => *id,
            None => {
                error!(self.location, "Trait {} was not found in scope", self.trait_name);
                return;
            },
        };

        self.trait_arg_types = fmap(&self.trait_args, |arg| resolver.convert_type(cache, arg));

        let trait_info = &cache.trait_infos[trait_id.0];
        resolver.required_definitions = Some(trait_info.definitions.clone());

        // The user is required to specify all of the trait's typeargs and functional dependencies.
        let required_arg_count = trait_info.typeargs.len() + trait_info.fundeps.len();
        if self.trait_args.len() != required_arg_count {
            error!(self.location, "impl has {} type arguments but {} requires {}",
                   self.trait_args.len(), self.trait_name.blue(), required_arg_count);
        }

        resolver.push_scope(cache);

        // Declare the names first so we can check them all against the required_definitions
        resolver.let_binding_level = LetBindingLevel(resolver.let_binding_level.0 + 1);
        let definitions = resolver.resolve_trait_impl_declarations(self.definitions.iter_mut(), cache);

        // TODO cleanup: is required_definitions still required since we can
        // resolve_all_definitions now? The checks in push_definition can probably
        // be moved here instead
        for required_definition in resolver.required_definitions.as_ref().unwrap() {
            error!(self.location, "impl is missing a definition for {}", cache.definition_infos[required_definition.0].name);
        }

        resolver.required_definitions = None;
        resolver.current_trait = None;

        // All the names are present, now define them.
        for definition in self.definitions.iter_mut() {
            definition.expr.define(resolver, cache);
            definition.level = Some(resolver.let_binding_level);
        }
        resolver.let_binding_level = LetBindingLevel(resolver.let_binding_level.0 - 1);
        resolver.pop_scope(cache, false);

        let trait_impl = trustme::extend_lifetime(self);
        resolver.push_trait_impl(trait_id, self.trait_arg_types.clone(), definitions, trait_impl, cache, self.locate());
    }
}

impl<'b> Resolvable<'b> for ast::Return<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        self.expression.define(resolver, cache);
    }
}

impl<'b> Resolvable<'b> for ast::Sequence<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        for statement in self.statements.iter_mut() {
            statement.declare(resolver, cache)
        }
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        for statement in self.statements.iter_mut() {
            statement.define(resolver, cache)
        }
    }
}

impl<'b> Resolvable<'b> for ast::Extern<'b> {
    fn declare(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        for declaration in self.declarations.iter_mut() {
            resolver.resolve_extern_definitions(declaration, cache);
        }
    }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        // Any extern in global scope should already be defined in the declaration pass
        if !resolver.in_global_scope() {
            self.declare(resolver, cache);
        }
    }
}

impl<'b> Resolvable<'b> for ast::MemberAccess<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        self.lhs.define(resolver, cache);
    }
}

impl<'b> Resolvable<'b> for ast::Tuple<'b> {
    fn declare(&mut self, _resolver: &mut NameResolver, _cache: &mut ModuleCache<'b>) { }

    fn define(&mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'b>) {
        for element in self.elements.iter_mut() {
            element.define(resolver, cache);
        }
    }
}
