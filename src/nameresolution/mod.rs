use crate::parser::ast;
use crate::parser::ast::Ast;
use crate::types::TypeInfoId;
use crate::error::location::{ Location, Locatable };
use crate::nameresolution::modulecache::{ ModuleCache, NameResolutionState, DefinitionInfoId, TraitInfoId };
use crate::nameresolution::scope::{ Scope, FunctionScope };

use std::path::PathBuf;

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
        let top = self.callstack.len() - 1;
        self.callstack[top].pop();
    }

    pub fn current_scope(&mut self) -> &mut Scope {
        let top = self.callstack.len() - 1;
        self.callstack[top].top()
    }

    pub fn push_definition<'a>(&mut self, name: &str, cache: &mut ModuleCache<'a>, location: Location<'a>) -> DefinitionInfoId {
        if let Some(existing_definition) = self.lookup_definition(name) {
            println!("{}", error!(location, "{} is already in scope", name));
            let previous_location = cache.definition_infos[existing_definition.0].location;
            println!("{}", note!(previous_location, "{} previously defined here", name));
        }

        let id = cache.push_definition(location);
        self.current_scope().definitions.insert(name.to_string(), id);
        id
    }
}

impl<'a, 'b> NameResolver {
    pub fn resolve(ast: &'a mut Ast<'a>, cache: &'b mut ModuleCache<'a>) -> &'b NameResolver {
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
        cache.modules.insert(filepath.clone(), NameResolutionState::Done(resolver));

        match &cache.modules[&filepath] {
            NameResolutionState::Done(resolver) => &resolver,
            _ => unreachable!(),
        }
    }
}

pub trait Resolvable<'ast, 'cache> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>);
}

impl<'a, 'b> Resolvable<'a, 'b> for Ast<'a> {
    fn resolve(&'a mut self, resolver: &'b mut NameResolver, cache: &'b mut ModuleCache<'a>) {
        dispatch_on_expr!(self, Resolvable::resolve, resolver, cache);
    }
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Literal<'a> {
    fn resolve(&mut self, _: &mut NameResolver, _: &mut ModuleCache) {}
}

impl<'a, 'b> Resolvable<'a, 'b> for ast::Variable<'a> {
    fn resolve(&'a mut self, resolver: &mut NameResolver, cache: &mut ModuleCache<'a>) {
        use ast::Variable::*;
        match self {
            Operator(token, location, definition, _) => {
                let name = token.to_string();
                if resolver.auto_declare {
                    *definition = Some(resolver.push_definition(&name, cache, *location));
                } else {
                    *definition = resolver.lookup_definition(&token.to_string());
                }
                if !definition.is_some() {
                    println!("{}", error!(*location, "Operator {} was not found in scope", name));
                }
            },
            Identifier(name, location, definition, _) => {
                if resolver.auto_declare {
                    *definition = Some(resolver.push_definition(name, cache, *location));
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

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::Lambda<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        resolver.push_function();
        resolver.auto_declare = true;
        self.args.iter_mut().for_each(|arg| arg.resolve(resolver, cache));
        resolver.auto_declare = false;
        resolver.pop_function();
    }
}

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::FunctionCall<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        self.function.resolve(resolver, cache);
        self.args.iter_mut().for_each(|arg| arg.resolve(resolver, cache));
    }
}

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::Definition<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        use ast::{Ast::Variable, Variable::Identifier};
        let name = match *self.pattern {
            Variable(Identifier(name, _, _, _)) => name,
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

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::If<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
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

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::Match<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
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

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::TypeDefinition<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        unimplemented!();
    }
}

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::TypeAnnotation<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        self.lhs.resolve(resolver, cache);
    }
}

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::Import<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        unimplemented!();
    }
}

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::TraitDefinition<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        unimplemented!();
    }
}

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::TraitImpl<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        unimplemented!();
    }
}

impl<'ast, 'cache> Resolvable<'ast, 'cache> for ast::Return<'ast> {
    fn resolve(&'ast mut self, resolver: &'cache mut NameResolver, cache: &'cache mut ModuleCache<'ast>) {
        self.expression.resolve(resolver, cache);
    }
}
