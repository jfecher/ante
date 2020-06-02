use crate::parser::ast;
use crate::parser::ast::Ast;
use crate::types::TypeInfoId;
use crate::error::location::{ Location, Locatable };
use crate::nameresolution::modulecache::{ ModuleCache, NameResolutionState, DefinitionInfoId, TraitInfoId };
use crate::nameresolution::scope::{ Scope, FunctionScope };
use crate::lexer::Lexer;
use crate::parser;

use std::fs::File;
use std::io::{ BufReader, Read };
use std::path::{ Path, PathBuf };

mod scope;
pub mod modulecache;

#[derive(Debug)]
pub struct NameResolver {
    filepath: PathBuf,
    imports: Vec<NameResolver>,
    callstack: Vec<scope::FunctionScope>,
    exports: scope::Scope,
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

            for import in self.imports.iter() {
                if let Some(id) = import.$name(name) {
                    return Some(id);
                }
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
    pub fn resolve(ast: &'a mut Ast<'b>, cache: &'a mut ModuleCache<'b>) {
        let location = ast.locate();
        let filepath = location.filename.to_owned();

        assert!(cache.modules.entry(filepath.clone()).or_default() == &NameResolutionState::NotStarted);

        let mut resolver = NameResolver {
            filepath: filepath.clone(),
            imports: vec![],
            callstack: vec![FunctionScope::new()],
            exports: Scope::default(),
            auto_declare: false,
        };

        cache.modules.insert(filepath.clone(), NameResolutionState::InProgress);
        ast.resolve(&mut resolver, cache);
        resolver.callstack.pop();
        cache.modules.insert(filepath.clone(), NameResolutionState::Done(resolver));
    }
}

pub trait Resolvable<'a, 'b: 'a> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>);
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for Ast<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        dispatch_on_expr!(self, Resolvable::resolve, resolver, cache);
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Literal<'b> {
    fn resolve(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Variable<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        use ast::Variable::*;
        use crate::lexer::token::Token;
        match self {
            Operator(Token::Semicolon, _, _, _) => {
                // Ignore definition for the sequencing operator, its not a "true"
                // operator since it cannot be redefined
            },
            Operator(token, location, definition, _) => {
                let name = token.to_string();
                if resolver.auto_declare {
                    *definition = Some(resolver.push_definition(name.clone(), cache, *location));
                } else {
                    *definition = resolver.lookup_definition(&token.to_string());
                }
                if !definition.is_some() {
                    println!("{}", error!(*location, "Operator {} was not found in scope", name));
                }
            },
            Identifier(name, location, definition, _) => {
                if resolver.auto_declare {
                    *definition = Some(resolver.push_definition(name.to_string(), cache, *location));
                } else {
                    *definition = resolver.lookup_definition(name);
                }
                if !definition.is_some() {
                    println!("{}", error!(*location, "{} was not found in scope", name));
                }
            },
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Lambda<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        resolver.push_function();
        resolver.auto_declare = true;
        for arg in self.args.iter_mut() {
            arg.resolve(resolver, cache);
        }
        resolver.auto_declare = false;
        self.body.resolve(resolver, cache);
        resolver.pop_function();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::FunctionCall<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.function.resolve(resolver, cache);
        for arg in self.args.iter_mut() {
            arg.resolve(resolver, cache)
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Definition<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        use ast::{Ast::Variable, Variable::*};
        let name = match self.pattern.as_ref() {
            Variable(Identifier(name, _, _, _)) => name.to_string(),
            Variable(Operator(token, _, _, _)) => token.to_string(),
            _ => unimplemented!(),
        };

        let is_function = match *self.expr { ast::Ast::Lambda(_) => true, _ => false };

        // If this is a function, define the name first
        if is_function {
            self.info = Some(resolver.push_definition(name, cache, self.location));
            self.expr.resolve(resolver, cache);
        }else {
            self.expr.resolve(resolver, cache);
            self.info = Some(resolver.push_definition(name, cache, self.location));
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::If<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.condition.resolve(resolver, cache);
        
        resolver.push_scope();
        self.then.resolve(resolver, cache);
        resolver.pop_scope();

        if let Some(otherwise) = &mut self.otherwise {
            resolver.push_scope();
            otherwise.resolve(resolver, cache);
            resolver.pop_scope();
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Match<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.expression.resolve(resolver, cache);

        for (pattern, rhs) in self.branches.iter_mut() {
            resolver.push_scope();
            resolver.auto_declare = true;
            pattern.resolve(resolver, cache);
            resolver.auto_declare = false;

            rhs.resolve(resolver, cache);
            resolver.pop_scope();
        }
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TypeDefinition<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TypeAnnotation<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.lhs.resolve(resolver, cache);
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
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        let relative_path = self.path.clone().join("/");
        let (file, path) = match find_file(&relative_path, cache) {
            Some((f, p)) => (f, p),
            _ => {
                println!("{}", error!(self.location, "Couldn't open file for import: {}", relative_path));
                return;
            },
        };

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

        let mut ast = result.unwrap();
        NameResolver::resolve(&mut ast, cache);
        resolver.current_scope().import(path, cache, self.location);
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TraitDefinition<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::TraitImpl<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        unimplemented!();
    }
}

impl<'a, 'b: 'a> Resolvable<'a, 'b> for ast::Return<'b> {
    fn resolve(&'a mut self, resolver: &'a mut NameResolver, cache: &'a mut ModuleCache<'b>) {
        self.expression.resolve(resolver, cache);
    }
}
