use super::NameResolver;
use std::path::{ Path, PathBuf };
use std::collections::HashMap;
use crate::types::{ TypeVariableId, TypeInfoId, TypeInfo, Type, TypeInfoBody };
use crate::error::location::{ Location, Locatable };
use crate::parser::ast::Ast;
use crate::nameresolution::unsafecache::UnsafeCache;

#[derive(Debug)]
pub struct ModuleCache<'a> {
    /// All the 'root' directories for imports. In practice this will contain
    /// the directory of the driver module as well as all directories containing
    /// any libraries used by the program, including the standard library.
    pub relative_roots: Vec<PathBuf>,

    /// Maps ModuleId -> Ast
    /// Contains all the parse trees parsed by the program.
    pub parse_trees: UnsafeCache<'a, Ast<'a>>,

    /// Used to map paths to parse trees or name resolvers
    pub modules: HashMap<PathBuf, ModuleId>,

    /// Maps ModuleId -> CompilationState
    pub name_resolvers: UnsafeCache<'a, NameResolver>,

    /// Holds all the previously seen filenames referenced by Locations
    /// Used to lengthen the lifetime of Locations and the parse tree past
    /// the lifetime of the file that was read from.
    pub filepaths: Vec<PathBuf>,

    /// Maps TypeVariableId -> Type
    /// Unique TypeVariableIds are generated during name
    /// resolution and are unified during type inference
    pub type_bindings: Vec<Option<Type>>,

    /// Maps TypeInfoId -> TypeInfo
    /// Filled out during name resolution
    pub type_infos: Vec<TypeInfo<'a>>,

    /// Maps TraitInfoId -> TraitInfo
    /// Filled out during name resolution
    pub trait_infos: Vec<TraitInfo<'a>>,

    /// Maps DefinitionInfoId -> DefinitionInfo
    /// Filled out during name resolution
    pub definition_infos: Vec<DefinitionInfo<'a>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ModuleId(pub usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DefinitionInfoId(pub usize);

#[derive(Debug)]
pub struct DefinitionInfo<'a> {
    pub name: String,
    pub location: Location<'a>,
    pub trait_id: Option<TraitInfoId>,
    pub uses: u32,
}

impl<'a> Locatable<'a> for DefinitionInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TraitInfoId(pub usize);

#[derive(Debug)]
pub struct TraitInfo<'a> {
    pub name: String,
    pub typeargs: Vec<TypeVariableId>,
    pub fundeps: Vec<TypeVariableId>,
    pub location: Location<'a>,
    pub definitions: Vec<DefinitionInfoId>,
    pub uses: u32,
}


impl<'a> ModuleCache<'a> {
    pub fn new(project_directory: &'a Path) -> ModuleCache<'a> {
        ModuleCache {
            relative_roots: vec![project_directory.to_owned()],
            modules: HashMap::default(),
            parse_trees: UnsafeCache::default(),
            name_resolvers: UnsafeCache::default(),
            filepaths: Vec::default(),
            type_bindings: Vec::default(),
            type_infos: Vec::default(),
            trait_infos: Vec::default(),
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

    pub fn push_definition(&mut self, name: String, trait_id: Option<TraitInfoId>, location: Location<'a>) -> DefinitionInfoId {
        let id = self.definition_infos.len();
        self.definition_infos.push(DefinitionInfo { name, location, trait_id, uses: 0 });
        DefinitionInfoId(id)
    }

    pub fn push_ast(&mut self, ast: Ast<'a>) -> ModuleId {
        ModuleId(self.parse_trees.push(ast))
    }

    pub fn push_type_info(&mut self, name: String, args: Vec<TypeVariableId>, location: Location<'a>) -> TypeInfoId {
        let id = self.type_infos.len();
        let type_info = TypeInfo { name, args, location, uses: 0, body: TypeInfoBody::Unknown };
        self.type_infos.push(type_info);
        TypeInfoId(id)
    }

    pub fn get_name_resolver_by_path(&self, path: &Path) -> Option<&mut NameResolver> {
        let id = self.modules.get(path)?;
        self.name_resolvers.get_mut(id.0)
    }

    pub fn next_type_variable(&mut self) -> TypeVariableId {
        let id = self.type_bindings.len();
        self.type_bindings.push(None);
        TypeVariableId(id)
    }

    pub fn push_trait_definition(&mut self, name: String, typeargs: Vec<TypeVariableId>, fundeps: Vec<TypeVariableId>,  location: Location<'a>) ->TraitInfoId {
        let id = self.trait_infos.len();
        self.trait_infos.push(TraitInfo {
            name,
            typeargs,
            fundeps,
            definitions: vec![],
            location,
            uses: 0,
        });
        TraitInfoId(id)
    }
}
