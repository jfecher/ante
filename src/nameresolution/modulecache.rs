use super::NameResolver;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::types::{ TypeVariableId, TypeInfo, Type };
use crate::error::location::Location;

/// There are three states for a module undergoing name resolution:
/// NotStarted, InProgress, and Done. If a module is Done it can be
/// retrieved from the ModuleCache with name information. If it is
/// InProgress it is an error to import the module since the module
/// graph must be acyclic.
#[derive(Debug, PartialEq)]
pub enum NameResolutionState {
    NotStarted,
    InProgress,
    Done(NameResolver),
}

impl Default for NameResolutionState {
    fn default() -> NameResolutionState {
        NameResolutionState::NotStarted
    }
}

#[derive(Debug, Default)]
pub struct ModuleCache<'a> {
    /// The cache for each module that has undergone name resolution, used
    /// to prevent cyclic module graphs and ensure the same module is not checked twice.
    pub modules: HashMap<PathBuf, NameResolutionState>,

    /// Maps TypeVariableId -> Type
    /// Filled out during type inference
    pub type_bindings: Vec<Type>,

    /// Maps TypeInfoId -> TypeInfo
    /// Filled out during name resolution
    pub type_info: Vec<TypeInfo<'a>>,

    /// Maps DefinitionInfoId -> DefinitionInfo
    /// Filled out during name resolution
    pub definition_infos: Vec<DefinitionInfo<'a>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DefinitionInfoId(pub usize);

#[derive(Debug)]
pub struct DefinitionInfo<'a> {
    pub location: Location<'a>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TraitInfoId(usize);

#[derive(Debug)]
pub struct TraitInfo<'a> {
    pub typeargs: Vec<TypeVariableId>,
    pub fundeps: Vec<TypeVariableId>,
    pub location: Location<'a>,
}


impl<'a> ModuleCache<'a> {
    pub fn push_definition(&mut self, location: Location<'a>) -> DefinitionInfoId {
        let id = DefinitionInfoId(self.definition_infos.len());
        self.definition_infos.push(DefinitionInfo { location });
        id
    }
}
