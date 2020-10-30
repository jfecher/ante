use crate::nameresolution::NameResolver;
use crate::types::{ TypeVariableId, TypeInfoId, TypeInfo, Type, TypeInfoBody };
use crate::types::{ TypeBinding, LetBindingLevel, Kind };
use crate::types::traits::{ RequiredImpl, RequiredTrait };
use crate::error::location::{ Location, Locatable };
use crate::parser::ast::{ Ast, Definition, TraitDefinition, TraitImpl, TypeAnnotation };
use crate::cache::unsafecache::UnsafeCache;

use std::path::{ Path, PathBuf };
use std::collections::HashMap;

mod unsafecache;

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

    /// Maps DefinitionInfoId -> DefinitionInfo
    /// Filled out during name resolution.
    pub definition_infos: Vec<DefinitionInfo<'a>>,

    /// Maps VariableInfoId -> VariableInfo
    /// Each ast::Variable node stores the required impls for use while
    /// codegening the variable's definition. These impls are filled out
    /// during type inference (see typechecker::find_impl). Unlike
    /// DefinitionInfos, VariableInfos are per usage of the variable.
    pub trait_bindings: Vec<TraitBinding>,

    /// Maps TypeVariableId -> Type
    /// Unique TypeVariableIds are generated during name
    /// resolution and are unified during type inference
    pub type_bindings: Vec<TypeBinding>,

    /// Maps TypeInfoId -> TypeInfo
    /// Filled out during name resolution
    pub type_infos: Vec<TypeInfo<'a>>,

    /// Maps TraitInfoId -> TraitInfo
    /// Filled out during name resolution
    pub trait_infos: Vec<TraitInfo<'a>>,

    /// Maps ImplInfoId -> ImplInfo
    /// Filled out during name resolution, though
    /// definitions within impls aren't publically exposed.
    pub impl_infos: Vec<ImplInfo<'a>>,

    /// Maps ImplScopeId -> Vec<ImplInfo>
    /// Name resolution needs to store the impls visible to
    /// each variable so when any UnknownTraitImpls are resolved
    /// during type inference the inferencer can quickly get the
    /// impls that should be in scope and select an instance.
    pub impl_scopes: Vec<Vec<ImplInfoId>>,

    /// Ante represents each member access (foo.bar) as a trait (.foo)
    /// that is generated for each new field name used globally.
    pub member_access_traits: HashMap<String, TraitInfoId>,

    /// Used to give a unique ID to each node so they can later be
    /// used during static trait dispatch.
    pub variable_nodes: Vec</* name: */String>,

    pub prelude_path: PathBuf,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ModuleId(pub usize);

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct DefinitionInfoId(pub usize);

impl std::fmt::Debug for DefinitionInfoId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "${}", self.0)
    }
}

#[derive(Debug)]
pub enum DefinitionKind<'a> {
    Definition(&'a mut Definition<'a>),
    TraitDefinition(&'a mut TraitDefinition<'a>),
    Extern(&'a mut TypeAnnotation<'a>),

    /// A TypeConstructor function to construct a type.
    /// If the constructed type is a tagged union, tag will
    /// be Some, otherwise if it is a struct, tag is None.
    TypeConstructor { name: String, tag: Option<u8> },

    Parameter,

    /// Any variable declared in a match pattern. E.g. 'a' in
    /// match None with
    /// | a -> ()
    MatchPattern,
}

#[derive(Debug)]
pub struct DefinitionInfo<'a> {
    pub name: String,
    pub location: Location<'a>,

    /// Where this name was defined. It is expected that type checking
    /// this Definition kind should result in self.typ being filled out.
    pub definition: Option<DefinitionKind<'a>>,

    /// True if this definition can be reassigned to.
    pub mutable: bool,

    /// Some(trait_id) if this is a definition from a trait. Note that
    /// this is still None for definitions from trait impls.
    pub trait_info: Option<TraitInfoId>,

    /// For a given definition like:
    /// foo (a: a) -> a
    ///   given Add a, Print a = ...
    /// required_traits is the "given ..." part of the signature
    pub required_traits: Vec<RequiredTrait>,

    pub typ: Option<Type>,
    pub uses: u32,
}

impl<'a> Locatable<'a> for DefinitionInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TraitBindingId(pub usize);

/// These are stored on ast::Variables and detail any
/// required_impls needed to compile the definitions
/// of these variables.
pub struct TraitBinding {
    pub required_impls: Vec<RequiredImpl>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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

impl<'a> TraitInfo<'a> {
    pub fn is_member_access(&self) -> bool {
        self.name.starts_with(".")
    }
}

impl<'a> Locatable<'a> for TraitInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImplInfoId(pub usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImplScopeId(pub usize);

#[derive(Debug)]
pub struct ImplInfo<'a> {
    pub trait_id: TraitInfoId,
    pub typeargs: Vec<Type>,
    pub location: Location<'a>,
    pub definitions: Vec<DefinitionInfoId>,
    pub trait_impl: &'a mut TraitImpl<'a>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct VariableId(pub usize);

impl<'a> ModuleCache<'a> {
    pub fn new(project_directory: &'a Path) -> ModuleCache<'a> {
        ModuleCache {
            relative_roots: vec![project_directory.to_owned(), dirs::config_dir().unwrap().join("stdlib")],
            // Really wish you could do ..Default::default() for each field
            modules: HashMap::default(),
            parse_trees: UnsafeCache::default(),
            name_resolvers: UnsafeCache::default(),
            filepaths: Vec::default(),
            definition_infos: Vec::default(),
            trait_bindings: Vec::default(),
            type_bindings: Vec::default(),
            type_infos: Vec::default(),
            trait_infos: Vec::default(),
            impl_infos: Vec::default(),
            impl_scopes: Vec::default(),
            member_access_traits: HashMap::default(),
            variable_nodes: vec![],
            prelude_path: dirs::config_dir().unwrap().join("stdlib/prelude"),
        }
    }

    pub fn push_filepath(&mut self, path: PathBuf) -> &'a Path {
        let index = self.filepaths.len();
        self.filepaths.push(path);
        let path: &Path = &self.filepaths[index];
        unsafe { std::mem::transmute(path) }
    }

    pub fn push_definition(&mut self, name: &str, mutable: bool, location: Location<'a>) -> DefinitionInfoId {
        let id = self.definition_infos.len();
        self.definition_infos.push(DefinitionInfo {
            name: name.to_string(),
            definition: None,
            trait_info: None,
            required_traits: vec![],
            mutable,
            location,
            typ: None,
            uses: 0,
        });
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

    pub fn next_type_variable_id(&mut self, level: LetBindingLevel) -> TypeVariableId {
        let id = self.type_bindings.len();
        self.type_bindings.push(TypeBinding::Unbound(level, Kind::Normal(0)));
        TypeVariableId(id)
    }

    pub fn next_type_variable(&mut self, level: LetBindingLevel) -> Type {
        let id = self.next_type_variable_id(level);
        Type::TypeVariable(id)
    }

    pub fn push_trait_definition(&mut self, name: String, typeargs: Vec<TypeVariableId>,
                fundeps: Vec<TypeVariableId>,  location: Location<'a>) -> TraitInfoId {

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

    pub fn push_trait_impl(&mut self, trait_id: TraitInfoId, typeargs: Vec<Type>,
            definitions: Vec<DefinitionInfoId>, trait_impl: &'a mut TraitImpl<'a>, location: Location<'a>) -> ImplInfoId {

        let id = self.impl_infos.len();
        self.impl_infos.push(ImplInfo {
            trait_id,
            typeargs,
            definitions,
            location,
            trait_impl,
        });
        ImplInfoId(id)
    }

    pub fn push_impl_scope(&mut self) -> ImplScopeId {
        let id = self.impl_scopes.len();
        self.impl_scopes.push(vec![]);
        ImplScopeId(id)
    }

    pub fn push_trait_binding(&mut self) -> TraitBindingId {
        let id = self.trait_bindings.len();
        self.trait_bindings.push(TraitBinding {
            required_impls: vec![]
        });
        TraitBindingId(id)
    }

    /// Get or create an instance of the '.' trait family for the given field name
    pub fn get_member_access_trait(&mut self, field_name: &str, level: LetBindingLevel) -> TraitInfoId {
        match self.member_access_traits.get(field_name) {
            Some(id) => *id,
            None => {
                let trait_name = ".".to_string() + field_name;
                let collection_type = self.next_type_variable_id(level);
                let field_type = self.next_type_variable_id(level);
                let id = self.push_trait_definition(trait_name, vec![collection_type], vec![field_type], Location::builtin());
                self.member_access_traits.insert(field_name.to_string(), id);
                id
            },
        }
    }

    pub fn next_variable_node(&mut self, name: &str) -> VariableId {
        let id = VariableId(self.variable_nodes.len());
        self.variable_nodes.push(name.to_string());
        id
    }
}
