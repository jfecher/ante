#![macro_use]
use crate::parser::ast;
use crate::parser::ast::Ast;
use crate::types::{ Type, TypeInfo, TypeVariableId, TypeInfoId };
use crate::error::location::Locatable;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DefinitionInfoId(usize);
pub struct DefinitionInfo<'a> {
    pub definition: &'a ast::Definition<'a>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TraitInfoId(usize);
pub struct TraitInfo<'a> {
    pub definition: &'a ast::TraitDefinition<'a>,
    pub typeargs: Vec<TypeVariableId>,
    pub fundeps: Vec<TypeVariableId>,
}

#[derive(Debug)]
pub struct Scope {
    pub definitions: HashMap<String, DefinitionInfoId>,
    pub types: HashMap<String, TypeInfoId>,
    pub traits: HashMap<String, TraitInfoId>,
}

impl Scope {
    fn new() -> Scope {
        Scope {
            definitions: HashMap::new(),
            types: HashMap::new(),
            traits: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct NameResolver {
    filepath: PathBuf,
    imports: Vec<NameResolver>,
    scopes: Vec<Scope>,
    exports: Scope,
}

impl PartialEq for NameResolver {
    fn eq(&self, other: &NameResolver) -> bool {
        self.filepath == other.filepath
    }
}

#[derive(PartialEq)]
enum NameResolutionState {
    NotStarted,
    InProgress,
    Done(NameResolver),
}

impl Default for NameResolutionState {
    fn default() -> NameResolutionState {
        NameResolutionState::NotStarted
    }
}

#[derive(Default)]
pub struct ModuleCache<'a> {
    modules: HashMap<PathBuf, NameResolutionState>,
    type_bindings: HashMap<TypeVariableId, Type>,
    type_info: Vec<TypeInfo<'a>>,
    definition_infos: Vec<DefinitionInfo<'a>>,
}

impl<'a> NameResolver {
    pub fn resolve(ast: &Ast<'a>, cache: &'a mut ModuleCache<'a>) -> &'a NameResolver {
        let location = ast.locate();
        let filepath = location.filename.to_owned();

        assert!(cache.modules[&filepath] == NameResolutionState::NotStarted);

        let mut resolver = NameResolver {
            filepath: filepath.clone(),
            imports: vec![],
            scopes: vec![Scope::new()],
            exports: Scope::new(),
        };

        cache.modules.insert(filepath.clone(), NameResolutionState::InProgress);
        ast.resolve(&mut resolver, cache);
        cache.modules.insert(filepath.clone(), NameResolutionState::Done(resolver));

        match &cache.modules[&filepath] {
            NameResolutionState::Done(resolver) => &resolver,
            _ => unreachable!(),
        }
    }

    fn new_scope(&mut self) {
        self.scopes.push(Scope {
            definitions: HashMap::new(),
            types: HashMap::new(),
            traits: HashMap::new(),
        });
    }
}

pub trait Resolvable {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache);
}

impl<'a> Resolvable for Ast<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
        dispatch_on_expr!(self, Resolvable::resolve, resolver, cache);
    }
}

impl<'a> Resolvable for ast::Literal<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::Variable<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::Lambda<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::FunctionCall<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::Definition<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::If<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::Match<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::TypeDefinition<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::TypeAnnotation<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::Import<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::TraitDefinition<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::TraitImpl<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}

impl<'a> Resolvable for ast::Return<'a> {
    fn resolve(&self, resolver: &mut NameResolver, cache: &mut ModuleCache) {
    }
}
