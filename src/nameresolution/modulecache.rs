use super::NameResolver;
use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use crate::types::{ TypeVariableId, TypeInfo, Type };
use crate::error::location::{ Location, Locatable };

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

#[derive(Debug)]
pub struct ModuleCache<'a> {
    /// All the 'root' directories for imports. In practice this will contain
    /// the directory of the driver module as well as all directories containing
    /// any libraries used by the program, including the standard library.
    pub relative_roots: Vec<PathBuf>,

    /// The cache for each module that has undergone name resolution, used
    /// to prevent cyclic module graphs and ensure the same module is not checked twice.
    pub modules: HashMap<PathBuf, NameResolutionState>,

    /// Holds all the previously seen filenames referenced by Locations
    /// Used to lengthen the lifetime of Locations and the parse tree past
    /// the lifetime of the file that was read from.
    pub filepaths: Vec<PathBuf>,

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

impl<'a> Locatable<'a> for DefinitionInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
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
    pub fn new(project_directory: &'a Path) -> ModuleCache<'a> {
        ModuleCache {
            relative_roots: vec![project_directory.to_owned()],
            modules: HashMap::default(),
            filepaths: Vec::default(),
            type_bindings: Vec::default(),
            type_info: Vec::default(),
            definition_infos: Vec::default(),
        }
    }

    pub fn push_filepath(&mut self, path: PathBuf) -> &'a Path {
        let index = self.filepaths.len();
        self.filepaths.push(path);
        let path: &Path = &self.filepaths[index];
        // TODO: Path should have 'a lifetime 
        unsafe { std::mem::transmute(path) }
    }

    pub fn push_definition(&mut self, location: Location<'a>) -> DefinitionInfoId {
        let id = DefinitionInfoId(self.definition_infos.len());
        self.definition_infos.push(DefinitionInfo { location });
        id
    }
}
