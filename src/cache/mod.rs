use crate::nameresolution::NameResolver;
use crate::types::{ TypeVariableId, TypeInfoId, TypeInfo, Type, TypeInfoBody };
use crate::types::{ TypeBinding, LetBindingLevel, traits::TraitList, Kind };
use crate::types::traits::{ Impl, ImplPrinter };
use crate::error::location::{ Location, Locatable };
use crate::parser::ast::{ Ast, Definition, TraitDefinition, TraitImpl, TypeAnnotation };
use crate::cache::unsafecache::UnsafeCache;

use std::path::{ Path, PathBuf };
use std::collections::HashMap;

mod unsafecache;

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

    /// Maps DefinitionInfoId -> DefinitionInfo
    /// Filled out during name resolution
    pub definition_infos: Vec<DefinitionInfo<'a>>,

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

    /// Maps ImplBindingId -> ImplInfo
    /// Each ast::Variable node stores one ImplBindingId for
    /// every trait that it must monomorphise during code generation.
    /// These bindings are mapped and filled out during trait
    /// resolution which occurs when type checking a ast::Definition node.
    pub impl_bindings: Vec<Option<ImplInfoId>>,

    /// Ante represents each member access (foo.bar) as a trait (.foo)
    /// that is generated for each new field name used globally.
    pub member_access_traits: HashMap<String, TraitInfoId>,

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

    /// Functions defined in impls are only accessible through
    /// usage of the trait they're implementing. They are tagged
    /// via this Impl tag.
    Impl,
}

#[derive(Debug)]
pub struct DefinitionInfo<'a> {
    pub name: String,
    pub location: Location<'a>,

    /// Where this name was defined. It is expected that type checking
    /// this Definition kind should result in self.typ being filled out.
    pub definition: Option<DefinitionKind<'a>>,

    /// If this definition is from a trait impl then this will contain the
    /// definition id from the trait's matching declaration. Used during
    /// codegen to help retrieve the compiled function without the impl information.
    pub trait_definition: Option<DefinitionInfoId>,

    pub required_impls: TraitList,

    pub typ: Option<Type>,
    pub uses: u32,
}

impl<'a> Locatable<'a> for DefinitionInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ImplInfoId(pub usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImplScopeId(pub usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImplBindingId(pub usize);

#[derive(Debug)]
pub struct ImplInfo<'a> {
    pub trait_id: TraitInfoId,
    pub typeargs: Vec<Type>,
    pub location: Location<'a>,
    pub definitions: Vec<DefinitionInfoId>,
    pub trait_impl: &'a mut TraitImpl<'a>,
}

impl<'b> ImplInfo<'b> {
    #[allow(dead_code)]
    pub fn display<'a>(&self, cache: &'a ModuleCache<'b>) -> ImplPrinter<'a, 'b> {
        Impl::new(self.trait_id, ImplScopeId(0), ImplBindingId(0), self.typeargs.clone()).display(cache)
    }
}

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
            type_bindings: Vec::default(),
            type_infos: Vec::default(),
            trait_infos: Vec::default(),
            impl_infos: Vec::default(),
            impl_scopes: Vec::default(),
            impl_bindings: Vec::default(),
            member_access_traits: HashMap::default(),
            prelude_path: dirs::config_dir().unwrap().join("stdlib/prelude"),
        }
    }

    pub fn push_filepath(&mut self, path: PathBuf) -> &'a Path {
        let index = self.filepaths.len();
        self.filepaths.push(path);
        let path: &Path = &self.filepaths[index];
        unsafe { std::mem::transmute(path) }
    }

    pub fn push_definition(&mut self, name: &str, location: Location<'a>) -> DefinitionInfoId {
        let id = self.definition_infos.len();
        self.definition_infos.push(DefinitionInfo {
            name: name.to_string(),
            definition: None,
            trait_definition: None,
            required_impls: vec![],
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

    pub fn push_impl_binding(&mut self) -> ImplBindingId {
        let id = self.impl_bindings.len();
        self.impl_bindings.push(None);
        ImplBindingId(id)
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
}
