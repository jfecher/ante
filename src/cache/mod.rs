//! cache/mod.rs - Provides the ModuleCache struct which the ante compiler
//! pervasively uses to cache and store results from various compiler phases.
//! The most important things the cache stores are additional information about
//! the parse tree. For example, for each variable definition, there is a corresponding
//! `DefinitionInfo` struct and a `DefinitionInfoId` key that can be used on the
//! cache to access this struct. The `DefinitionInfo` struct stores additional information
//! about a definition like the `ast::Definition` node it was defined in, its name,
//! whether it is mutable, and how many times it is referenced in the program.
//! This XXXInfo and XXXInfoId pattern is also used for TraitDefinitions, TraitImpls, and Types.
//! See the corresponding structs further down in this file for more information.
//!
//! The ModuleCache itself is kept the entirely of compilation - its contents may be
//! used in any phase and thus nothing is freed until the program is fully linked.
//! Any pass-specific information that isn't needed for later phases shouldn't be
//! kept in the ModuleCache and instead should be in a special data structure for
//! the relevant phase. An example is the `llvm::Generator` in the llvm codegen phase.
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

/// The ModuleCache is for information needed until compilation is completely finished
/// (ie. not just for one phase). Accessing each `Vec` inside the `ModuleCache` is done
/// only through the XXXInfoId keys returned as a result of each of the `push_xxx` methods
/// on the `ModuleCache`. These keys are also often stored in the AST itself as a result
/// of various passes. For example, name resolution will fill in the `definition: Option<DefinitionInfoId>`
/// field of each `ast::Variable` node, which has the effect of pointing each variable to
/// where it was defined.
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

    /// Maps ModuleId -> NameResolver
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
    pub trait_bindings: Vec<TraitBinding<'a>>,

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

    /// The builtin `Int a` trait that arises when using polymorphic
    /// integer literals.
    pub int_trait: TraitInfoId,

    /// Used to give a unique ID to each node so they can later be
    /// used during static trait dispatch.
    pub variable_nodes: Vec</* name: */String>,

    /// The filepath to ante's stdlib/prelude.an file to be automatically
    /// included when defining a new ante module.
    pub prelude_path: PathBuf,
}

/// The key for accessing parse trees or `NameResolver`s
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
    /// A variable/function definition in the form `a = b`
    Definition(&'a mut Definition<'a>),

    /// A trait definition in the form `trait A a with ...`
    TraitDefinition(&'a mut TraitDefinition<'a>),

    /// An extern FFI definition with no body
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

/// Carries additional information about a variable's definition.
/// Note that two variables defined in the same pattern, e.g: `(a, b) = c`
/// will have their own unique `DefinitionInfo`s, but each DefinitionInfo
/// will refer to the same `ast::Definition` in its definition field.
///
/// The corresponding DefinitionInfoId is attatched to each
/// `ast::Variable` during name resolution.
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

    /// The type of this definition. Filled out during type inference,
    /// and is guarenteed to be Some afterward.
    pub typ: Option<Type>,

    /// A count of how many times was this variable referenced in the program.
    /// Used primarily for issuing unused warnings.
    pub uses: u32,
}

impl<'a> Locatable<'a> for DefinitionInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct TraitBindingId(pub usize);

/// TraitBindingIds are stored on ast::Variables and detail any
/// required_impls needed to compile the definitions
/// of these variables. These are filled out during type inference.
pub struct TraitBinding<'a> {
    pub required_impls: Vec<RequiredImpl>,
    pub location: Location<'a>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct TraitInfoId(pub usize);

/// Additional information on the definition of a trait.
/// The corresponding TraitInfoId is attatched to each
/// `ast::TraitDefinition` during name resolution.
///
/// Note that the builtin `Int a` trait as well as the builtin
/// member access family of traits also have their own TraitInfo.
#[derive(Debug)]
pub struct TraitInfo<'a> {
    pub name: String,

    /// The type arguments of this trait. These are the
    /// `a b c` in `trait Foo a b c -> d e f with ...`
    /// Note that all traits must have at least 1 type
    /// argument, otherwise there is no type to implement
    /// the trait for.
    pub typeargs: Vec<TypeVariableId>,

    /// The possibly-empty functional dependencies of this trait.
    /// These are the `d e f` in `trait Foo a b c -> d e f with ...`
    pub fundeps: Vec<TypeVariableId>,

    pub location: Location<'a>,

    /// The definitions included in this trait defintion.
    /// The term `defintion` is used somewhat loosely here
    /// since none of these functions/variables have bodies.
    /// They're merely declarations that impl definitions will
    /// later have to conform to.
    pub definitions: Vec<DefinitionInfoId>,

    /// The Ast node that defines this trait.
    /// A value of None means this trait was builtin to the compiler
    pub trait_node: Option<&'a mut TraitDefinition<'a>>,

    pub uses: u32,
}

impl<'a> TraitInfo<'a> {
    /// Member access traits are special in that they're automatically
    /// defined and implemented by the compiler.
    pub fn is_member_access(&self) -> bool {
        self.name.starts_with(".")
    }

    /// The `name` of a member access trait is `.field`
    /// where `field` is the name of the described field.
    /// E.g. `.name Person string` is a trait constraining
    /// the `Person` type to have a `name` field of type `string`.
    pub fn get_field_name(&self) -> &str {
        assert!(self.is_member_access());
        &self.name[1..]
    }
}

impl<'a> Locatable<'a> for TraitInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

/// An ImplScopeId is attached to an ast::Variable to remember
/// the impls that were in scope when it was used since scopes are
/// thrown away after name resolution but the impls in scope are still
/// needed during type inference.
/// TODO: The concept of an ImplScope is somewhat of a wart in the trait inference
/// algorithm. Getting rid of them would likely make it both cleaner and faster.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImplScopeId(pub usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ImplInfoId(pub usize);

/// Corresponds to a `ast::TraitImpl` node, carrying extra information
/// on it. These are filled out during name resolution.
#[derive(Debug)]
pub struct ImplInfo<'a> {
    pub trait_id: TraitInfoId,
    pub typeargs: Vec<Type>,
    pub location: Location<'a>,
    pub definitions: Vec<DefinitionInfoId>,
    pub given: Vec<RequiredTrait>,
    pub trait_impl: &'a mut TraitImpl<'a>,
}

/// Each `ast::Variable` node corresponds to a VariableId that identifies it,
/// filled out during name resolution. These are currently used to identify the
/// origin/callsites of traits for trait dispatch.
/// `TraitConstraints` are passed around during type inference carrying these so
/// that once they're finally resolved, the correct variable can be linked to the
/// correct impl definition.
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct VariableId(pub usize);

impl<'a> ModuleCache<'a> {
    pub fn new(project_directory: &'a Path) -> ModuleCache<'a> {
        let mut cache = ModuleCache {
            relative_roots: vec![project_directory.to_owned(), dirs::config_dir().unwrap().join("ante/stdlib")],
            int_trait: TraitInfoId(0), // Dummy value since we must have the cache to push a trait
            prelude_path: dirs::config_dir().unwrap().join("stdlib/prelude"),
            // Really wish you could do ..Default::default() for the remaining fields
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
            variable_nodes: Vec::default(),
        };

        let new_typevar = cache.next_type_variable_id(LetBindingLevel(std::usize::MAX));
        cache.push_trait_definition("Int".to_string(), vec![new_typevar], vec![], None, Location::builtin());
        cache
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
                fundeps: Vec<TypeVariableId>, trait_node: Option<&'a mut TraitDefinition<'a>>,
                location: Location<'a>) -> TraitInfoId
    {

        let id = self.trait_infos.len();
        self.trait_infos.push(TraitInfo {
            name,
            typeargs,
            fundeps,
            definitions: vec![],
            trait_node,
            location,
            uses: 0,
        });
        TraitInfoId(id)
    }

    pub fn push_trait_impl(&mut self, trait_id: TraitInfoId, typeargs: Vec<Type>,
            definitions: Vec<DefinitionInfoId>, trait_impl: &'a mut TraitImpl<'a>,
            given: Vec<RequiredTrait>, location: Location<'a>) -> ImplInfoId {

        let id = self.impl_infos.len();
        self.impl_infos.push(ImplInfo {
            trait_id,
            typeargs,
            definitions,
            location,
            given,
            trait_impl,
        });
        ImplInfoId(id)
    }

    pub fn push_impl_scope(&mut self) -> ImplScopeId {
        let id = self.impl_scopes.len();
        self.impl_scopes.push(vec![]);
        ImplScopeId(id)
    }

    pub fn push_trait_binding(&mut self, location: Location<'a>) -> TraitBindingId {
        let id = self.trait_bindings.len();
        self.trait_bindings.push(TraitBinding {
            required_impls: vec![],
            location,
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
                let id = self.push_trait_definition(trait_name, vec![collection_type], vec![field_type], None, Location::builtin());
                self.member_access_traits.insert(field_name.to_string(), id);
                id
            },
        }
    }

    pub fn push_variable_node(&mut self, name: &str) -> VariableId {
        let id = VariableId(self.variable_nodes.len());
        self.variable_nodes.push(name.to_string());
        id
    }
}
