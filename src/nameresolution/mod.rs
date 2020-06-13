use crate::parser::ast;
use crate::parser::ast::Ast;
use crate::types::{ TypeInfoId, TypeVariableId, Type, PrimitiveType, TypeInfoBody };
use crate::types::{ TypeConstructor, Field };
use crate::error::location::{ Location, Locatable };
use crate::nameresolution::modulecache::{ ModuleCache, DefinitionInfoId, ModuleId, TraitInfoId };
use crate::nameresolution::scope::Scope;
use crate::lexer::Lexer;
use crate::parser;
use crate::util::fmap;

use std::fs::File;
use std::io::{ BufReader, Read };
use std::path::{ Path, PathBuf };

mod scope;
mod unsafecache;
pub mod modulecache;

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
    exports: Scope,

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
    required_definitions: Option<Vec<String>>,
}

impl PartialEq for NameResolver {
    fn eq(&self, other: &NameResolver) -> bool {
        self.filepath == other.filepath
    }
}

macro_rules! lookup_fn {
    ( $name:ident , $stack_field:ident , $cache_field:ident, $return_type:ty ) => {
        fn $name<'a, 'b>(&'a self, name: &'a str, cache: &'a mut ModuleCache<'b>) -> Option<$return_type> {
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

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub fn push_type_variable_scope(&mut self) {
        self.type_variable_scopes.push(scope::TypeVariableScope::default());
    }

    pub fn push_existing_type_variable(&mut self, key: String, id: TypeVariableId) -> TypeVariableId {
        let top = self.type_variable_scopes.len() - 1;
        self.type_variable_scopes[top].push_existing_type_variable(key, id)
    }

    pub fn push_new_type_variable<'a, 'b>(&'a mut self, key: String, cache: &'a mut ModuleCache<'b>) -> TypeVariableId {
        let id = cache.next_type_variable_id();
        self.push_existing_type_variable(key, id)
    }

    pub fn pop_scope<'a, 'b>(&'a mut self, cache: &'a mut ModuleCache<'b>, warn_unused: bool) {
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

    fn check_required_definitions<'a, 'b>(&'a mut self, name: String, cache: &'a mut ModuleCache<'b>, location: Location<'b>) {
        if let Some(existing_id) = self.lookup_definition(&name, cache) {
            let existing_definition = &cache.definition_infos[existing_id.0];

            // Ignore the repeated definition if it is from the trait we're implementing
            if existing_definition.trait_id.is_none() {
                error!(location, "{} is already in scope", name);
                note!(existing_definition.location, "{} previously defined here", name);
            }
        }

        let required_definitions = self.required_definitions.as_mut().unwrap();
        if let Some(index) = required_definitions.iter().position(|x| x == &name) {
            required_definitions.swap_remove(index);
        } else {
            let trait_info = &cache.trait_infos[self.current_trait.unwrap().0];
            error!(location, "{} is not required by {}", name, trait_info.name);
        }
    }

    pub fn push_definition<'a, 'b>(&'a mut self, name: String, cache: &'a mut ModuleCache<'b>, location: Location<'b>) -> DefinitionInfoId {
        let id = cache.push_definition(name.clone(), self.current_trait, location);
        if self.required_definitions.is_some() {
            // We're inside a trait impl right now, any definitions shouldn't be put in scope
            // else they'd collide with the declaration from the trait. Additionally, we must
            // ensure the definition is one of the ones required by the trait in required_definitions.
            self.check_required_definitions(name, cache, location);
        } else {
            if let Some(existing_definition) = self.lookup_definition(&name, cache) {
                error!(location, "{} is already in scope", name);
                let previous_location = cache.definition_infos[existing_definition.0].location;
                note!(previous_location, "{} previously defined here", name);
            }

            if self.in_global_scope() {
                self.exports.definitions.insert(name.clone(), id);
            }
            self.current_scope().definitions.insert(name, id);

            // If we're currently in a trait, add this definition to the trait's list of definitions
            if let Some(trait_id) = self.current_trait {
                let trait_info = &mut cache.trait_infos[trait_id.0];
                trait_info.definitions.push(id);
                let def = &mut cache.definition_infos[id.0];
                def.trait_id = Some(trait_id);
            }
        }
        id
    }

    pub fn push_type_info<'a, 'b>(&'a mut self, name: String, args: Vec<TypeVariableId>, cache: &'a mut ModuleCache<'b>, location: Location<'b>) ->  TypeInfoId {
        if let Some(existing_definition) = self.lookup_type(&name, cache) {
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

    pub fn push_trait<'a, 'b>(&'a mut self, name: String, args: Vec<TypeVariableId>, fundeps: Vec<TypeVariableId>, cache: &'a mut ModuleCache<'b>, location: Location<'b>) -> TraitInfoId {
        if let Some(existing_definition) = self.lookup_trait(&name, cache) {
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
}

impl<'a, 'b> NameResolver {
    pub fn start(ast: Ast<'b>, cache: &'a mut ModuleCache<'b>) {
        let resolver = NameResolver::declare(ast, cache);
        resolver.define(cache);
    }

    pub fn declare(ast: Ast<'b>, cache: &'a mut ModuleCache<'b>) -> &'b mut NameResolver {
        let filepath = ast.locate().filename;

        let existing = cache.get_name_resolver_by_path(&filepath);
        assert!(existing.is_none());

        let module_id = cache.push_ast(ast);
        cache.modules.insert(filepath.to_owned(), module_id);

        let resolver = NameResolver {
            filepath: filepath.to_owned(),
            scopes: vec![Scope::default()],
            exports: Scope::default(),
            type_variable_scopes: vec![scope::TypeVariableScope::default()],
            state: NameResolutionState::DeclareInProgress,
            auto_declare: false,
            current_trait: None,
            required_definitions: None,
            module_id,
        };

        let existing = cache.get_name_resolver_by_path(&filepath);
        let existing_state = existing.map_or(NameResolutionState::NotStarted, |x| x.state);
        assert!(existing_state == NameResolutionState::NotStarted);

        cache.name_resolvers.push(resolver);
        let resolver = cache.name_resolvers.get_mut(module_id.0).unwrap();

        let ast = cache.parse_trees.get_mut(module_id.0).unwrap();
        ast.declare(resolver, cache);
        resolver.state = NameResolutionState::Declared;

        resolver
    }

    pub fn define(&mut self, cache: &'a mut ModuleCache<'b>) {
        let ast = cache.parse_trees.get_mut(self.module_id.0).unwrap();

        assert!(self.state == NameResolutionState::Declared);

        self.state = NameResolutionState::DefineInProgress;
        ast.define(self, cache);
        self.state = NameResolutionState::Defined;
    }

    /// Converts an ast::Type to a types::Type, expects all typevars to be in scope
    pub fn convert_type(&'a mut self, cache: &'a mut ModuleCache<'b>, ast_type: &'a ast::Type<'b>) -> Type {
        match ast_type {
            ast::Type::IntegerType(_) => Type::Primitive(PrimitiveType::IntegerType),
            ast::Type::FloatType(_) => Type::Primitive(PrimitiveType::FloatType),
            ast::Type::CharType(_) => Type::Primitive(PrimitiveType::CharType),
            ast::Type::StringType(_) => Type::Primitive(PrimitiveType::StringType),
            ast::Type::BooleanType(_) => Type::Primitive(PrimitiveType::BooleanType),
            ast::Type::UnitType(_) => Type::Primitive(PrimitiveType::UnitType),
            ast::Type::ReferenceType(_) => Type::Primitive(PrimitiveType::ReferenceType),
            ast::Type::FunctionType(args, ret, _) => {
                let args = fmap(args, |arg| self.convert_type(cache, arg));
                let ret = self.convert_type(cache, ret);
                Type::Function(args, Box::new(ret))
            },
            ast::Type::TypeVariable(name, location) => {
                assert!(!self.auto_declare);
                match self.lookup_type_variable(name) {
                    Some(id) => Type::TypeVariable(id),
                    None => {
                        error!(*location, "Type variable {} was not found in scope", name);
                        Type::Primitive(PrimitiveType::IntegerType)
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
}

pub trait Resolvable<'a, 'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>);
    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>);
}

impl<'a, 'b> Resolvable<'a, 'b> for Ast<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        dispatch_on_expr!(self, Resolvable::declare, resolver, cache);
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        dispatch_on_expr!(self, Resolvable::define, resolver, cache);
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Literal<'b> {
    /// Purpose of the declare pass is to collect all the names of publically exported symbols
    /// so the define pass can work in the presense of mutually recursive modules.
    fn declare(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}

    /// Go through a module and annotate each variable with its declaration.
    /// Display any errors for variables without declarations.
    fn define(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Variable<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        use ast::VariableKind::*;
        match &mut self.kind {
            Operator(token) => {
                let name = token.to_string();
                if resolver.auto_declare {
                    self.definition = Some(resolver.push_definition(name, cache, self.location));
                }
            },
            Identifier(name) => {
                if resolver.auto_declare {
                    self.definition = Some(resolver.push_definition(name.clone(), cache, self.location));
                }
            },
            TypeConstructor(..) => unreachable!(), // Never need to auto-declare type constructors
        }
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
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
            }

            // If it is still not declared, print an error
            if self.definition.is_none() {
                error!(self.locate(), "No declaration for {} was found in scope", self);
            }
        }
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Lambda<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        resolver.push_scope();
        resolver.auto_declare = true;
        for arg in self.args.iter_mut() {
            arg.define(resolver, cache);
        }
        resolver.auto_declare = false;
        self.body.define(resolver, cache);
        resolver.pop_scope(cache, true);
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::FunctionCall<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.function.define(resolver, cache);
        for arg in self.args.iter_mut() {
            arg.define(resolver, cache)
        }
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Definition<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        resolver.auto_declare = true;
        self.pattern.declare(resolver, cache);
        resolver.auto_declare = false;
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        resolver.auto_declare = true;
        self.pattern.define(resolver, cache);
        resolver.auto_declare = false;
        self.expr.define(resolver, cache);
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::If<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.condition.define(resolver, cache);
        
        resolver.push_scope();
        self.then.define(resolver, cache);
        resolver.pop_scope(cache, true);

        if let Some(otherwise) = &mut self.otherwise {
            resolver.push_scope();
            otherwise.define(resolver, cache);
            resolver.pop_scope(cache, true);
        }
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Match<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.expression.define(resolver, cache);

        for (pattern, rhs) in self.branches.iter_mut() {
            resolver.push_scope();
            resolver.auto_declare = true;
            pattern.define(resolver, cache);
            resolver.auto_declare = false;

            rhs.define(resolver, cache);
            resolver.pop_scope(cache, true);
        }
    }
}

fn create_variants<'a, 'b>(vec: &'a Vec<(String, Vec<ast::Type<'b>>, Location<'b>)>, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) -> Vec<TypeConstructor<'b>> {
    vec.iter().map(|(name, types, location)| {
        let args = types.iter().map(|t|
            resolver.convert_type(cache, t)
        ).collect();

        resolver.push_definition(name.clone(), cache, *location);
        TypeConstructor { name: name.clone(), args, location: *location }
    }).collect()
}

fn create_fields<'a, 'b>(vec: &'a Vec<(String, ast::Type<'b>, Location<'b>)>, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) -> Vec<Field<'b>> {
    vec.iter().map(|(name, field_type, location)| {
        let field_type = resolver.convert_type(cache, field_type);

        Field { name: name.clone(), field_type, location: *location }
    }).collect()
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::TypeDefinition<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        let args = self.args.iter().map(|_| cache.next_type_variable_id()).collect();
        let id = resolver.push_type_info(self.name.clone(), args, cache, self.location);
        self.type_info = Some(id);
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
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

        match &self.definition {
            ast::TypeDefinitionBody::UnionOf(vec) => {
                let variants = create_variants(vec, resolver, cache);
                let type_info = &mut cache.type_infos[self.type_info.unwrap().0];
                type_info.body = TypeInfoBody::Union(variants);
            },
            ast::TypeDefinitionBody::StructOf(vec) => {
                let fields = create_fields(vec, resolver, cache);
                let type_info = &mut cache.type_infos[self.type_info.unwrap().0];
                type_info.body = TypeInfoBody::Struct(fields);
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

impl<'a, 'b> Resolvable<'a, 'b> for ast::TypeAnnotation<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.lhs.define(resolver, cache);
        let rhs = resolver.convert_type(cache, &self.rhs);
        self.typ = Some(rhs);
    }
}

fn find_file<'a>(relative_import_path: &str, cache: &mut ModuleCache) -> Option<(File, PathBuf)> {
    let relative_path = Path::new(relative_import_path);
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

impl<'a, 'b> Resolvable<'a, 'b> for ast::Import<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        let relative_path = self.path.clone().join("/");
        let (file, path) = match find_file(&relative_path, cache) {
            Some((f, p)) => (f, p),
            _ => {
                error!(self.location, "Couldn't open file for import: {}.an", relative_path);
                return;
            },
        };

        if let Some(module_id) = cache.modules.get(&path) {
            let existing_resolver = cache.name_resolvers.get_mut(module_id.0).unwrap();
            match existing_resolver.state {
                NameResolutionState::NotStarted => (),
                _ => {
                    self.module_id = Some(existing_resolver.module_id);
                    return; // already declared
                },
            }
        }

        let path = cache.push_filepath(PathBuf::from(&path));

        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        reader.read_to_string(&mut contents).unwrap();

        let tokens = Lexer::new(&path, &contents).collect::<Vec<_>>();
        let result = parser::parse(&tokens);

        if let Err(err) = result {
            println!("{}", err);
            return;
        }

        let ast = result.unwrap();
        let import_resolver = NameResolver::declare(ast, cache);
        self.module_id = Some(import_resolver.module_id);
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        // TODO: this will fail for non-top-level imports
        let module_id = self.module_id.unwrap().0;
        let import = cache.name_resolvers.get_mut(module_id).unwrap();
        match import.state {
            NameResolutionState::NotStarted
            | NameResolutionState::DeclareInProgress => {
                error!(self.location, "Internal compiler error: imported module has been defined but not declared")
            },
            | NameResolutionState::Declared => {
                import.define(cache);
            },
            // Any module that is at least declared should already have its public exports available
            | NameResolutionState::DefineInProgress
            | NameResolutionState::Defined => (),
        }

        resolver.current_scope().import(&import.exports, cache, self.location);
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::TraitDefinition<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        resolver.push_type_variable_scope();

        let args = self.args.iter().map(|arg| 
            resolver.push_new_type_variable(arg.clone(), cache)
        ).collect();

        let fundeps = self.fundeps.iter().map(|arg|
            resolver.push_new_type_variable(arg.clone(), cache)
        ).collect();

        assert!(resolver.current_trait.is_none());
        let trait_id = resolver.push_trait(self.name.clone(), args, fundeps, cache, self.location);
        resolver.current_trait = Some(trait_id);

        resolver.auto_declare = true;
        for declaration in self.declarations.iter_mut() {
            declaration.lhs.declare(resolver, cache);
        }
        resolver.auto_declare = false;

        resolver.current_trait = None;
        self.trait_info = Some(trait_id);
        resolver.pop_type_variable_scope();
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        if self.trait_info.is_none() {
            self.declare(resolver, cache);
        }
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::TraitImpl<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
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

        let trait_info = &cache.trait_infos[trait_id.0];
        resolver.required_definitions = Some(trait_info.definitions.iter().map(|id| cache.definition_infos[id.0].name.clone()).collect());

        resolver.auto_declare = true;
        resolver.push_scope();
        // Declare the names first so we can check them all against the required_definitions
        for definition in self.definitions.iter_mut() {
            definition.pattern.define(resolver, cache);
        }
        resolver.auto_declare = false;

        for required_definition in resolver.required_definitions.as_mut().unwrap() {
            error!(self.location, "impl is missing a definition for {}", required_definition);
        }

        resolver.required_definitions = None;
        resolver.current_trait = None;

        // All the names are present, now define them.
        for definition in self.definitions.iter_mut() {
            definition.expr.define(resolver, cache);
        }
        resolver.pop_scope(cache, false);
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Return<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.expression.define(resolver, cache);
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Sequence<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        for statement in self.statements.iter_mut() {
            statement.declare(resolver, cache)
        }
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        for statement in self.statements.iter_mut() {
            statement.define(resolver, cache)
        }
    }
}
