use crate::parser::ast;
use crate::parser::ast::{ Ast, Variable::* };
use crate::types::TypeInfoId;
use crate::error::location::{ Location, Locatable };
use crate::nameresolution::modulecache::{ ModuleCache, DefinitionInfoId, TraitInfoId, ModuleId };
use crate::nameresolution::scope::{ Scope, FunctionScope };
use crate::lexer::{ Lexer, token::Token };
use crate::parser;

use std::fs::File;
use std::io::{ BufReader, Read };
use std::path::{ Path, PathBuf };

mod scope;
mod unsafecache;
pub mod modulecache;

/// There are four states for a module undergoing name resolution:
/// NotStarted, Declared, and Defined. If a module is Declared it has
/// finished scanning its top-level exports for use by other modules.
/// If it is Defined it is completely Done and can move on to type inference.
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
    imports: Vec<NameResolver>,
    callstack: Vec<scope::FunctionScope>,
    exports: scope::Scope,
    state: NameResolutionState,
    module_id: ModuleId,
    auto_declare: bool,
}

impl PartialEq for NameResolver {
    fn eq(&self, other: &NameResolver) -> bool {
        self.filepath == other.filepath
    }
}

macro_rules! lookup_fn {
    ( $name:ident , $stack_field:ident , $return_type:ty ) => {
        fn $name<'a>(&self, name: &'a str) -> Option<$return_type> {
            let top = self.callstack.len() - 1;
            for stack in self.callstack[top].iter() {
                if let Some(id) = stack.$stack_field.get(name) {
                    return Some(*id);
                }
            }

            // Check globals/imports in global scope
            if let Some(id) = self.global_scope().$stack_field.get(name) {
                return Some(*id);
            }

            None
        }
    };
}

impl NameResolver {
    lookup_fn!(lookup_definition, definitions, DefinitionInfoId);
    lookup_fn!(lookup_type, types, TypeInfoId);
    lookup_fn!(lookup_trait, traits, TraitInfoId);

    pub fn push_scope(&mut self) {
        let top = self.callstack.len() - 1;
        self.callstack[top].push();
    }

    pub fn pop_scope(&mut self) {
        let top = self.callstack.len() - 1;
        self.callstack[top].pop();
    }

    pub fn push_function(&mut self) {
        self.callstack.push(FunctionScope::new());
    }

    pub fn pop_function(&mut self) {
        self.callstack.pop();
    }

    pub fn current_scope(&mut self) -> &mut Scope {
        let top = self.callstack.len() - 1;
        self.callstack[top].top()
    }

    pub fn global_scope(&self) -> &Scope {
        self.callstack[0].bottom()
    }

    pub fn push_definition<'a, 'b>(&'a mut self, name: String, cache: &'a mut ModuleCache<'b>, location: Location<'b>) -> DefinitionInfoId {
        if let Some(existing_definition) = self.lookup_definition(&name) {
            println!("{}", error!(location, "{} is already in scope", name));
            let previous_location = cache.definition_infos[existing_definition.0].location;
            println!("{}", note!(previous_location, "{} previously defined here", name));
        }

        let id = cache.push_definition(location);
        if self.callstack.len() == 1 {
            self.exports.definitions.insert(name.clone(), id);
        }
        self.current_scope().definitions.insert(name, id);
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
            imports: vec![],
            callstack: vec![FunctionScope::new()],
            exports: Scope::default(),
            state: NameResolutionState::DeclareInProgress,
            auto_declare: false,
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
        self.callstack.pop();
    }
}

pub trait Resolvable<'a, 'b: 'a> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>);
    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>);
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for Ast<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        dispatch_on_expr!(self, Resolvable::declare, resolver, cache);
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        dispatch_on_expr!(self, Resolvable::define, resolver, cache);
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Literal<'b> {
    /// Purpose of the declare pass is to collect all the names of publically exported symbols
    /// so the define pass can work in the presense of mutually recursive modules.
    fn declare(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}

    /// Go through a module and annotate each variable with its declaration.
    /// Display any errors for variables without declarations.
    fn define(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Variable<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        match self {
            Operator(Token::Semicolon, _, _, _) => {
                // Ignore definition for the sequencing operator, its not a "true"
                // operator since it cannot be redefined
            },
            Operator(token, location, definition, _) => {
                let name = token.to_string();
                if resolver.auto_declare {
                    *definition = Some(resolver.push_definition(name.clone(), cache, *location));
                }
            },
            Identifier(name, location, definition, _) => {
                if resolver.auto_declare {
                    *definition = Some(resolver.push_definition(name.to_string(), cache, *location));
                }
            },
        }
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        match self {
            Operator(Token::Semicolon, _, _, _) => {
                // Ignore definition for the sequencing operator, its not a "true"
                // operator since it cannot be redefined
            },
            Operator(token, location, definition, _) => {
                let name = token.to_string();
                if definition.is_none() {
                    if resolver.auto_declare {
                        *definition = Some(resolver.push_definition(name.clone(), cache, *location));
                    } else {
                        *definition = resolver.lookup_definition(&token.to_string());
                    }
                }
                if !definition.is_some() {
                    println!("{}", error!(*location, "Operator {} was not found in scope", name));
                }
            },
            Identifier(name, location, definition, _) => {
                if definition.is_none() {
                    if resolver.auto_declare {
                        *definition = Some(resolver.push_definition(name.to_string(), cache, *location));
                    } else {
                        *definition = resolver.lookup_definition(name);
                    }
                }
                if !definition.is_some() {
                    println!("{}", error!(*location, "{} was not found in scope", name));
                }
            },
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Lambda<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        resolver.push_function();
        resolver.auto_declare = true;
        for arg in self.args.iter_mut() {
            arg.define(resolver, cache);
        }
        resolver.auto_declare = false;
        self.body.define(resolver, cache);
        resolver.pop_function();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::FunctionCall<'b> {
    fn declare(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        // We only need to go through sequenced ; expressions to find all the top-level declarations.
        match self.function.as_ref() {
            Ast::Variable(Operator(Token::Semicolon, _, _, _)) => {
                for arg in self.args.iter_mut() {
                    arg.declare(resolver, cache)
                }
            },
            _ => (),
        }
    }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.function.define(resolver, cache);
        for arg in self.args.iter_mut() {
            arg.define(resolver, cache)
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Definition<'b> {
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

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::If<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.condition.define(resolver, cache);
        
        resolver.push_scope();
        self.then.define(resolver, cache);
        resolver.pop_scope();

        if let Some(otherwise) = &mut self.otherwise {
            resolver.push_scope();
            otherwise.define(resolver, cache);
            resolver.pop_scope();
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Match<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.expression.define(resolver, cache);

        for (pattern, rhs) in self.branches.iter_mut() {
            resolver.push_scope();
            resolver.auto_declare = true;
            pattern.define(resolver, cache);
            resolver.auto_declare = false;

            rhs.define(resolver, cache);
            resolver.pop_scope();
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TypeDefinition<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }

    fn define(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TypeAnnotation<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.lhs.define(resolver, cache);
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

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Import<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        let relative_path = self.path.clone().join("/");
        let (file, path) = match find_file(&relative_path, cache) {
            Some((f, p)) => (f, p),
            _ => {
                println!("{}", error!(self.location, "Couldn't open file for import: {}.an", relative_path));
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
                println!("{}", error!(self.location, "Internal compiler error: imported module has been defined but not declared"))
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

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TraitDefinition<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }

    fn define(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TraitImpl<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }

    fn define(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Return<'b> {
    fn declare(&'a mut self, _resolver: &'a mut NameResolver, _cache: &'a mut ModuleCache<'b>) { }

    fn define(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.expression.define(resolver, cache);
    }
}
