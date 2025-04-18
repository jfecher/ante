//! types/mod.rs - Unlike other modules for compiler passes,
//! the type inference compiler pass is defined in types/typechecker.rs
//! rather than the mod.rs file here. Instead, this file defines
//! the representation of `Type`s - which represent any Type in ante's
//! type system - and `TypeInfo`s - which hold more information about the
//! definition of a user-defined type.
use std::collections::BTreeMap;

use effects::Effect;

use crate::cache::{DefinitionInfoId, ModuleCache};
use crate::error::location::{Locatable, Location};
use crate::lexer::token::{FloatKind, IntegerKind};
use crate::util;
use crate::util::fmap;

use self::typeprinter::TypePrinter;
use crate::types::effects::EffectSet;

pub mod effects;
mod mutual_recursion;
pub mod pattern;
pub mod traitchecker;
pub mod traits;
pub mod typechecker;
pub mod typed;
pub mod typeprinter;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypeVariableId(pub usize);

/// Priority of operator on Types
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TypePriority(u8);

impl From<u8> for TypePriority {
    fn from(priority: u8) -> Self {
        Self(priority)
    }
}

impl std::fmt::Display for TypePriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "priority {}", self.0)
    }
}

impl TypePriority {
    pub const MAX: TypePriority = TypePriority(u8::MAX);
    pub const APP: TypePriority = TypePriority(4);
    pub const FORALL: TypePriority = TypePriority(3);
    pub const PAIR: TypePriority = TypePriority(2);
    pub const FUN: TypePriority = TypePriority(1);
}

/// Primitive types are the easy cases when unifying types.
/// They're equal simply if the other type is also the same PrimitiveType variant,
/// there is no recursion needed like with other Types. If the `Type`
/// enum forms a tree, then these are the leaf nodes.
///
/// A restriction from the cranelift backend enforces primitive
/// types must be of size <= a pointer size to be able to store them
/// unboxed when all other values are boxed.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PrimitiveType {
    IntegerType,             // : IntegerTag -> *
    IntegerTag(IntegerKind), // : IntegerTag
    FloatType,               // : FloatTag -> *
    FloatTag(FloatKind),     // : FloatTag
    CharType,                // : *
    BooleanType,             // : *
    UnitType,                // : *
    Ptr,                     // : * -> *
}

/// Function or closure types.
/// Functions with no environment (non-closures) are
/// represented with `environment = unit`. This allows
/// us to infer types for higher order functions that are
/// polymorphic over raw function types and closures.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub environment: Box<Type>,

    /// Expected to be a Type::Effects or Type::TypeVariable only
    pub effects: Box<Type>,
    pub has_varargs: bool,
}

/// Any type in ante. Note that a trait is not a type. Traits are
/// relations between 1 or more types rather than being types themselves.
///
/// NOTE: PartialEq and Hash impls here are somewhat unsafe since any
/// type variables will not have access to the cache to follow their bindings.
/// Thus, PartialEq/Hash may think two types aren't equal when they otherwise
/// would be. For this reason, these impls are currently only used after
/// following all type bindings via `follow_bindings` or a similar function.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Type {
    /// int, char, bool, etc
    Primitive(PrimitiveType),

    /// Any function type (including closures)
    /// Note that all functions in ante take at least 1 argument.
    Function(FunctionType),

    /// Any stand-in type e.g. `a` in `Vec a`. The original names are
    /// translated into unique TypeVariableIds during name resolution.
    /// Each TypeVariableId is either Bound or Unbound in the ModuleCache.
    /// Bound type variables should be treated as equal to what they're bound
    /// to. Unbound type variables may stand in for any type. During type
    /// inference, the `unify` function may bind unbound type variables
    /// into bound type variables when asserting two types are equal.
    TypeVariable(TypeVariableId),

    /// Any user defined type defined via the `type` keyword
    /// These have a unique UserDefinedTypeId which points to
    /// additional information about the contents of the type
    /// not needed for most type checking.
    UserDefined(TypeInfoId),

    /// Any type in the form `constructor arg1 arg2 ... argN`
    TypeApplication(Box<Type>, Vec<Type>),

    /// A region-allocated reference to some data.
    /// Contains a region variable that is unified with other refs during type
    /// inference. All these refs will be allocated in the same region.
    Ref { mutability: Box<Type>, sharedness: Box<Type>, lifetime: Box<Type> },

    /// A (row-polymorphic) struct type. Unlike normal rho variables,
    /// the type variable used here replaces the entire type if bound.
    /// This makes it so we don't have to remember previous types to combine
    /// when traversing bindings.
    Struct(BTreeMap<String, Type>, TypeVariableId),

    /// Effects are not the same kind (*) as most Type variants, but
    /// are included in it since they are still valid in a type position
    /// most notably when substituting type variables for effects.
    Effects(EffectSet),

    /// Tags are any type which isn't a valid type by itself but may be inside
    /// a larger type. For example, `shared` is not a type, but a polymorphic
    /// reference's type variable may resolve to a shared reference.
    Tag(TypeTag),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TypeTag {
    // References can be polymorphic in their ownership or mutability.
    // When they are, they hold type variables which later resolve to one
    // of these variants.
    Owned,
    Shared,
    Mutable,
    Immutable,
}

#[derive(Debug, Clone)]
pub enum GeneralizedType {
    /// A non-generic type
    MonoType(Type),

    /// A generic type in the form `forall vars. typ`.
    /// These are used internally to indicate polymorphic
    /// type variables for let-polymorphism. There is no syntax to
    /// specify these explicitly in ante code. Each type variable in
    /// the Vec is polymorphic in the Box<Type>. This differentiates
    /// generic terms from normal terms whose types are
    /// just type variables of unknown types yet to be inferenced.
    PolyType(Vec<TypeVariableId>, Type),
}

impl Type {
    pub const UNIT: Type = Type::Primitive(PrimitiveType::UnitType);

    pub fn polymorphic_int(variable: TypeVariableId) -> Type {
        let int = Box::new(Type::Primitive(PrimitiveType::IntegerType));
        let kind = Type::TypeVariable(variable);
        Type::TypeApplication(int, vec![kind])
    }

    pub fn int(kind: IntegerKind) -> Type {
        let int = Box::new(Type::Primitive(PrimitiveType::IntegerType));
        let kind = Type::Primitive(PrimitiveType::IntegerTag(kind));
        Type::TypeApplication(int, vec![kind])
    }

    pub fn polymorphic_float(variable: TypeVariableId) -> Type {
        let int = Box::new(Type::Primitive(PrimitiveType::FloatType));
        let kind = Type::TypeVariable(variable);
        Type::TypeApplication(int, vec![kind])
    }

    pub fn float(kind: FloatKind) -> Type {
        let int = Box::new(Type::Primitive(PrimitiveType::FloatType));
        let kind = Type::Primitive(PrimitiveType::FloatTag(kind));
        Type::TypeApplication(int, vec![kind])
    }

    pub fn is_pair_type(&self) -> bool {
        self == &Type::UserDefined(PAIR_TYPE)
    }

    pub fn is_polymorphic_int_type(&self) -> bool {
        self == &Type::Primitive(PrimitiveType::IntegerType)
    }

    pub fn is_polymorphic_float_type(&self) -> bool {
        self == &Type::Primitive(PrimitiveType::FloatType)
    }

    pub fn is_reference_type(&self) -> bool {
        matches!(self, Type::Ref { .. })
    }

    pub fn is_unit(&self, cache: &ModuleCache<'_>) -> bool {
        match self {
            Type::Primitive(PrimitiveType::UnitType) => true,
            Type::TypeVariable(id) => match &cache.type_bindings[id.0] {
                TypeBinding::Bound(typ) => typ.is_unit(cache),
                TypeBinding::Unbound(..) => false,
            },
            _ => false,
        }
    }

    pub fn is_union_constructor<'a>(&'a self, cache: &'a ModuleCache<'_>) -> bool {
        self.union_constructor_variants(cache).is_some()
    }

    /// Returns Some(variants) if this is a union type constructor or union type itself.
    pub fn union_constructor_variants<'a>(
        &'a self, cache: &'a ModuleCache<'_>,
    ) -> Option<&'a Vec<TypeConstructor<'a>>> {
        use Type::*;
        match self {
            Primitive(_) => None,
            Ref { .. } => None,
            Function(function) => function.return_type.union_constructor_variants(cache),
            TypeApplication(typ, _) => typ.union_constructor_variants(cache),
            UserDefined(id) => cache.type_infos[id.0].union_variants(),
            TypeVariable(_) => unreachable!("Constructors should always have concrete types"),
            Struct(_, _) => None,
            Effects(_) => None,
            Tag(_) => None,
        }
    }

    pub fn priority(&self, cache: &ModuleCache<'_>) -> TypePriority {
        use Type::*;
        match self {
            Primitive(_) | UserDefined(_) | Struct(_, _) | Tag(_) => TypePriority::MAX,
            TypeVariable(id) => match &cache.type_bindings[id.0] {
                TypeBinding::Bound(typ) => typ.priority(cache),
                TypeBinding::Unbound(..) => TypePriority::MAX,
            },
            Function(_) => TypePriority::FUN,
            TypeApplication(ctor, args) if ctor.is_polymorphic_int_type() || ctor.is_polymorphic_float_type() => {
                if matches!(cache.follow_typebindings_shallow(&args[0]), Type::TypeVariable(_)) {
                    // type variable is unbound variable
                    TypePriority::APP
                } else {
                    // type variable is bound (polymorphic int)
                    TypePriority::MAX
                }
            },
            TypeApplication(ctor, _) => {
                if ctor.is_pair_type() {
                    TypePriority::PAIR
                } else {
                    TypePriority::APP
                }
            },
            Ref { .. } => TypePriority::APP,
            Effects(_) => unimplemented!("Type::priority for Effects"),
        }
    }

    /// Pretty-print each type with each typevar substituted for a, b, c, etc.
    pub fn display<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> typeprinter::TypePrinter<'a, 'b> {
        let typ = GeneralizedType::MonoType(self.clone());
        TypePrinter::display_type(typ, cache)
    }

    /// Like display but show the real unique TypeVariableId for each typevar instead
    #[allow(dead_code)]
    pub fn debug<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> typeprinter::TypePrinter<'a, 'b> {
        let typ = GeneralizedType::MonoType(self.clone());
        TypePrinter::debug_type(typ, cache)
    }

    /// Apply a function recursively to this type
    pub fn traverse(&self, cache: &ModuleCache, mut f: impl FnMut(&Type)) {
        self.traverse_rec(cache, &mut f)
    }

    fn traverse_rec(&self, cache: &ModuleCache, f: &mut impl FnMut(&Type)) {
        f(self);
        match self {
            Type::Primitive(_) => (),
            Type::UserDefined(_) => (),
            Type::Tag(_) => (),

            Type::Function(function) => {
                for parameter in &function.parameters {
                    parameter.traverse_rec(cache, f)
                }
                function.environment.traverse_rec(cache, f);
                function.return_type.traverse_rec(cache, f);
            },
            Type::TypeVariable(id) => match &cache.type_bindings[id.0] {
                TypeBinding::Bound(binding) => binding.traverse_rec(cache, f),
                TypeBinding::Unbound(_, _) => (),
            },
            Type::Ref { sharedness, mutability, lifetime } => {
                sharedness.traverse_rec(cache, f);
                mutability.traverse_rec(cache, f);
                lifetime.traverse_rec(cache, f);
            },
            Type::TypeApplication(constructor, args) => {
                constructor.traverse_rec(cache, f);
                for arg in args {
                    arg.traverse_rec(cache, f);
                }
            },
            Type::Effects(effects) => {
                if let Some(replacement) = effects.extension {
                    if let TypeBinding::Bound(binding) = &cache.type_bindings[replacement.0] {
                        return binding.traverse_rec(cache, f);
                    }
                }
                for (_, effect_args) in &effects.effects {
                    for arg in effect_args {
                        arg.traverse_rec(cache, f);
                    }
                }
            },
            Type::Struct(fields, id) => {
                if let TypeBinding::Bound(binding) = &cache.type_bindings[id.0] {
                    return binding.traverse_rec(cache, f);
                }
                for typ in fields.values() {
                    typ.traverse_rec(cache, f);
                }
            },
        }
    }

    // Like traverse, but do not follow type variable links
    pub fn traverse_no_follow(&self, mut f: impl FnMut(&Type)) {
        self.traverse_no_follow_rec(&mut f)
    }

    fn traverse_no_follow_rec(&self, f: &mut impl FnMut(&Type)) {
        f(self);
        match self {
            Type::Primitive(_) => (),
            Type::UserDefined(_) => (),
            Type::TypeVariable(_) => (),
            Type::Tag(_) => (),

            Type::Function(function) => {
                for parameter in &function.parameters {
                    parameter.traverse_no_follow_rec(f)
                }
                function.environment.traverse_no_follow_rec(f);
                function.return_type.traverse_no_follow_rec(f);
                function.effects.traverse_no_follow_rec(f);
            },
            Type::TypeApplication(constructor, args) => {
                constructor.traverse_no_follow_rec(f);
                for arg in args {
                    arg.traverse_no_follow_rec(f);
                }
            },
            Type::Effects(effects) => {
                for (_, effect_args) in &effects.effects {
                    for arg in effect_args {
                        arg.traverse_no_follow_rec(f);
                    }
                }
            },
            Type::Struct(fields, _) => {
                for typ in fields.values() {
                    typ.traverse_no_follow_rec(f);
                }
            },
            Type::Ref { sharedness, mutability, lifetime: _ } => {
                sharedness.traverse_no_follow_rec(f);
                mutability.traverse_no_follow_rec(f);
            },
        }
    }

    /// Try to create a string from this type without following any type variables
    /// or referencing any names of UserDefined types (as both of these would require a ModuleCache).
    /// This should be used for debugging only when you have no access to a ModuleCache
    #[allow(unused)]
    pub fn approx_to_string(&self) -> String {
        match self {
            Type::Primitive(p) => format!("{}", p),
            Type::Function(f) => {
                let params = fmap(&f.parameters, |param| param.approx_to_string());
                let env = f.environment.approx_to_string();
                let effects = f.effects.approx_to_string();
                let ret = f.return_type.approx_to_string();
                format!("({} ={}> {} {})", params.join(" -> "), env, ret, effects)
            },
            Type::TypeVariable(id) => format!("tv{}", id.0),
            Type::UserDefined(id) => format!("T{}", id.0),
            Type::TypeApplication(constructor, args) => {
                let constructor = constructor.approx_to_string();
                let args = fmap(args, |arg| arg.approx_to_string());
                format!("({} {})", constructor, args.join(" "))
            },
            Type::Ref { sharedness, mutability, lifetime } => {
                let shared = sharedness.approx_to_string();
                let mutable = mutability.approx_to_string();
                let lifetime = lifetime.approx_to_string();
                format!("{}{} '{}", mutable, shared, lifetime)
            },
            Type::Struct(fields, id) => {
                let fields = fmap(fields, |(name, typ)| format!("{}: {}", name, typ.approx_to_string()));
                format!("{{ {}, ..tv{} }}", fields.join(", "), id.0)
            },
            Type::Effects(set) => {
                if set.effects.is_empty() {
                    if let Some(replacement) = set.extension {
                        format!("can tv{}", replacement.0)
                    } else {
                        "pure".to_string()
                    }
                } else {
                    let effects = fmap(&set.effects, |(id, args)| {
                        let args = fmap(args, |arg| arg.approx_to_string());
                        format!("e{} {}", id.0, args.join(" "))
                    });
                    let mut effects = format!("can {}", effects.join(", "));
                    if let Some(replacement) = set.extension {
                        effects = format!("{effects}, ..tv{}", replacement.0)
                    }
                    effects
                }
            },
            Type::Tag(tag) => tag.to_string(),
        }
    }

    /// Converts the given type into an EffectSet.
    /// Panics if it is not a Type::Effects
    fn as_effect_set(&self) -> Vec<Effect> {
        match self {
            Type::Effects(effects) => effects.effects.clone(),
            _ => panic!("as_effect_set called on non-effect type"),
        }
    }

    pub fn flatten_effects(&self, cache: &ModuleCache) -> EffectSet {
        match self {
            Type::TypeVariable(type_variable_id) => match &cache.type_bindings[type_variable_id.0] {
                TypeBinding::Bound(typ) => typ.flatten_effects(cache),
                TypeBinding::Unbound(..) => EffectSet::new(Vec::new(), Some(*type_variable_id)),
            },
            Type::Effects(effect_set) => effect_set.flatten(cache),
            other => panic!("flatten_effects expected effects, found {}", other.debug(cache)),
        }
    }
}

impl std::fmt::Display for TypeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeTag::Owned => write!(f, "owned"),
            TypeTag::Shared => write!(f, "shared"),
            TypeTag::Mutable => write!(f, "!"),
            TypeTag::Immutable => write!(f, "&"),
        }
    }
}

impl GeneralizedType {
    /// Pretty-print each type with each typevar substituted for a, b, c, etc.
    #[allow(dead_code)]
    pub fn display<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> typeprinter::TypePrinter<'a, 'b> {
        TypePrinter::display_type(self.clone(), cache)
    }

    /// Like display but show the real unique TypeVariableId for each typevar instead
    #[allow(dead_code)]
    pub fn debug<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> typeprinter::TypePrinter<'a, 'b> {
        TypePrinter::debug_type(self.clone(), cache)
    }

    pub fn find_all_typevars(&self, polymorphic_only: bool, cache: &ModuleCache) -> Vec<TypeVariableId> {
        match self {
            GeneralizedType::MonoType(typ) => util::dedup(typechecker::find_all_typevars(typ, polymorphic_only, cache)),
            GeneralizedType::PolyType(typevars, typ) => {
                if polymorphic_only {
                    typevars.clone()
                } else {
                    let mut vars = typevars.clone();
                    vars.append(&mut typechecker::find_all_typevars(typ, polymorphic_only, cache));
                    util::dedup(vars)
                }
            },
        }
    }

    pub fn is_union_constructor<'a>(&'a self, cache: &'a ModuleCache<'_>) -> bool {
        self.remove_forall().is_union_constructor(cache)
    }

    pub fn remove_forall(&self) -> &Type {
        match self {
            GeneralizedType::MonoType(typ) => typ,
            GeneralizedType::PolyType(_, typ) => typ,
        }
    }

    pub fn into_monotype(self) -> Type {
        match self {
            GeneralizedType::MonoType(typ) => typ,
            GeneralizedType::PolyType(_, _) => unreachable!(),
        }
    }

    pub fn as_monotype(&self) -> &Type {
        match self {
            GeneralizedType::MonoType(typ) => typ,
            GeneralizedType::PolyType(_, _) => unreachable!(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct LetBindingLevel(pub usize);

/// The initial LetBindingLevel used in nameresolution and typechecking.
/// This must be at least 1 since typechecker::infer_ast will set the CURRENT_LEVEL
/// to INITIAL_LEVEL - 1 when finishing type checking main to differentiate between
/// traits used within main and traits propagated up into main's signature.
/// Since the later case is an error (all traits must be resolved by the point
/// we finish typechecking) typechecker::infer_ast and typechecker::should_propagate
/// use this INITIAL_LEVEL - 1 to distinguish between the two cases. Note that since
/// at each ast::Definition the current LetBindingLevel is incremented when recursing
/// inside and decremented after finishing, this distinction is equivalent to if we
/// manually forced users to wrap their program in the following:
/// ```ante
/// main () = ...
/// ```
/// See okmij.org/ftp/ML/generalization.html for more information on the levels
/// algorithm used in the typechecker.
pub const INITIAL_LEVEL: usize = 1;

/// A given TypeVariableId is either bound to some type
/// or is unbound and has a given LetBindingLevel as its lifetime.
/// This LetBindingLevel is used to determine which type variables
/// can be generalized.
#[derive(Debug)]
pub enum TypeBinding {
    Bound(Type),
    Unbound(LetBindingLevel, Kind),
}

#[derive(Debug)]
pub struct TypeConstructor<'a> {
    #[allow(unused)]
    pub name: String,
    pub args: Vec<Type>,
    pub id: DefinitionInfoId,
    #[allow(unused)]
    pub location: Location<'a>,
}

#[derive(Debug)]
pub struct Field<'a> {
    pub name: String,
    pub field_type: Type,
    #[allow(unused)]
    pub location: Location<'a>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct TypeInfoId(pub usize);

/// The string type is semi builtin in that it isn't a primitive type
/// but all string literals will nevertheless have the type `string`
/// even if the prelude isn't imported into scope.
pub const STRING_TYPE: TypeInfoId = TypeInfoId(0);

/// The pair type is another semi builtin type. Its constructor (,)
/// is also visible whether or not the prelude is imported.
/// It is somewhat special in that it is the only type defined with
/// an operator for its name, but it is otherwise a normal struct type.
pub const PAIR_TYPE: TypeInfoId = TypeInfoId(1);

#[derive(Debug)]
pub enum TypeInfoBody<'a> {
    Union(Vec<TypeConstructor<'a>>),
    Struct(Vec<Field<'a>>),
    Alias(Type),
    Unknown,
}

/// Holds additional information for a given `type T = ...` definition.
#[derive(Debug)]
pub struct TypeInfo<'a> {
    pub args: Vec<TypeVariableId>,
    pub name: String,
    pub body: TypeInfoBody<'a>,
    pub uses: u32,
    pub location: Location<'a>,
}

impl<'a> Locatable<'a> for TypeInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

impl<'a> TypeInfo<'a> {
    pub fn union_variants(&self) -> Option<&Vec<TypeConstructor>> {
        match &self.body {
            TypeInfoBody::Union(variants) => Some(variants),
            _ => None,
        }
    }

    pub fn find_field<'b>(&'b self, field_name: &str) -> Option<(u32, &'b Field<'b>)> {
        match &self.body {
            TypeInfoBody::Struct(fields) => fields
                .iter()
                .enumerate()
                .find(|(_, field)| field.name == field_name)
                .map(|(i, field)| (i as u32, field)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Kind {
    /// usize is the number of type arguments it takes before it returns a type of kind *.
    /// For example, the kind Normal(2) : * -> * -> *
    #[allow(unused)]
    Normal(usize),

    /// A higher order kind where each element in the Vec is an argument. For example, the kind:
    /// HigherOrder(vec![ Normal(0), HigherOrder(vec![ Normal(0), Normal(1) ]), Normal(1) ])
    /// has kind: * -> (* -> (* -> *)) -> (* -> *)
    #[allow(dead_code)]
    HigherOrder(Vec<Kind>),
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::IntegerType => write!(f, "Int"),
            PrimitiveType::IntegerTag(tag) => write!(f, "{}", tag),
            PrimitiveType::FloatType => write!(f, "Float"),
            PrimitiveType::FloatTag(tag) => write!(f, "{}", tag),
            PrimitiveType::CharType => write!(f, "char"),
            PrimitiveType::BooleanType => write!(f, "bool"),
            PrimitiveType::UnitType => write!(f, "unit"),
            PrimitiveType::Ptr => write!(f, "Ptr"),
        }
    }
}
