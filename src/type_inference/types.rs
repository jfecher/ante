use std::{borrow::Cow, sync::LazyLock};

use inc_complete::DbGet;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::{DbHandle, GetItem, Resolve},
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    name_resolution::{Origin, ResolutionResult, builtin::Builtin},
    parser::{
        cst::{self, KindAnnotation, PlaceSegment, ReferenceKind},
        desugar_context::DesugarContext,
        get_item::{IMPL_GENERIC_DISPLAY_NAME, TraitDefinition},
        ids::{NameId, NameStore, TopLevelName},
    },
    shared_arc::Shared,
    type_inference::{
        TypeChecker,
        generics::Generic,
        get_type,
        kinds::Kind,
        places::Place,
        row::{
            Row, RowKind, canonicalize_row, construct_row, flatten_row_into, follow_row, is_open, sort_and_dedup_row,
        },
    },
};

/// Tracks the kind of each local type variable encountered while lowering a
/// `cst::Type` into a `Type`
pub(crate) type LocalKinds = BTreeMap<NameId, Kind>;

pub(crate) fn kind_from_annotation(kind: KindAnnotation) -> Kind {
    match kind {
        KindAnnotation::Type => Kind::Type,
        KindAnnotation::U32 => Kind::U32,
        KindAnnotation::Place => Kind::Place,
        KindAnnotation::Effect => Kind::Effect,
    }
}

pub(crate) const NO_CLOSURE_ENV_STRING: &str = "NoClosureEnv";

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Type {
    /// Any primitive type which can be compared for unification via primitive equality
    Primitive(PrimitiveType),

    /// A user-supplied generic type. We don't want to bind over these like we do with type variables.
    Generic(Generic),

    /// We represent type variables with unique ids and an external bindings map instead of a
    /// `Arc<RwLock<..>>` or similar because these need to be compared for equality, serialized, and
    /// be performant. We want the faster insertion of a local BTreeMap compared to a thread-safe
    /// version so we use a BTreeMap internally then freeze it in an Arc when finished to be
    /// able to access it from other threads.
    Variable(TypeVariableId),
    Function(Shared<FunctionType>),
    Application(Shared<Type>, Shared<Vec<Type>>),
    UserDefined(Origin),

    /// A polytype such as `forall a. fn a -> a`.
    /// During unification the ordering of the type variables matters.
    /// `forall a b. (a, b)` will not unify with `forall b a. (a, b)`
    Forall(Shared<Vec<Generic>>, Shared<Type>),

    /// This is an internal type only created when handling closure environments.
    /// Most tuple types in source code refer to the `,` type defined in the prelude. While they
    /// could use this type instead, using a UserDefinedType for them lets us reuse the existing
    /// mechanisms to automatically define their constructor and retrieve their fields.
    Tuple(Shared<Vec<Type>>),

    /// A type-level U32 constant, used as the length parameter of [PrimitiveType::Array].
    /// Has [Kind::U32].
    U32(u32),

    /// An effects row, each effect is sorted & deduplicated
    ///
    /// A value of None corresponds to an empty effect set to avoid allocation
    Effects(Row<Type>),

    /// A single concrete place a reference may point to. Only ever appears as an entry
    /// inside a [Type::Places] row
    Place(Place),

    /// The set of places a reference may borrow from, sorted & deduplicated.
    /// Each entry is a [Type::Place] or generic when concrete or a [Type::Variable] when not.
    ///
    /// A value of None corresponds to a reference pointing to nothing
    Places(Row<Type>),

    /// A generic or unbound place followed by fields and `!`s, like `'a!`
    PlaceProjection(Shared<Type>, Shared<Vec<PlaceSegment>>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FunctionType {
    pub parameters: Vec<ParameterType>,

    /// Closures and functions are unified by all having an environment type.
    /// Free functions will have an environment of [Type::NO_CLOSURE_ENV] while closures will
    /// have other environment types and will be subject to closure conversion.
    pub environment: Type,

    pub return_type: Type,

    pub effects: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ParameterType {
    pub is_implicit: bool,
    pub typ: Type,
}

impl ParameterType {
    pub fn new(typ: Type, is_implicit: bool) -> ParameterType {
        ParameterType { typ, is_implicit }
    }

    pub fn explicit(typ: Type) -> ParameterType {
        ParameterType { typ, is_implicit: false }
    }

    pub fn implicit(typ: Type) -> ParameterType {
        ParameterType { typ, is_implicit: true }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PrimitiveType {
    Error,
    Unit,
    Bool,
    // * -> *
    Pointer,
    Char,
    Int(IntegerKind),
    Float(FloatKind),
    Reference(ReferenceKind),

    /// Built-in fixed-size unboxed array constructor of kind `U32 -> * -> *`.
    /// Applied form: `Type::Application(Type::ARRAY, [TypeLevelU32(n), t])`.
    Array,

    /// The bottom type
    Never,

    /// A special tag a closure's environment can be unified to, at
    /// which point it becomes a free function
    NoClosureEnv,

    /// The environment of a handler's `resume`, which is a pointer to its coroutine.
    ResumeEnv,

    /// The unique tag on each `impl`. It is a struct containing each of the impl's
    /// captures. Writing an impl's name in a type position refers to this type.
    ImplData(TopLevelName),
}

/// Maps type variables to their bindings
pub type TypeBindings = FxHashMap<TypeVariableId, Type>;

pub type GenericSubstitutions = FxHashMap<Generic, Type>;

impl Type {
    pub const ERROR: Type = Type::Primitive(PrimitiveType::Error);
    pub const UNIT: Type = Type::Primitive(PrimitiveType::Unit);
    pub const BOOL: Type = Type::Primitive(PrimitiveType::Bool);
    pub const POINTER: Type = Type::Primitive(PrimitiveType::Pointer);
    pub const CHAR: Type = Type::Primitive(PrimitiveType::Char);
    pub const NEVER: Type = Type::Primitive(PrimitiveType::Never);

    pub const I8: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::I8));
    pub const I16: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::I16));
    pub const I32: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::I32));
    pub const I64: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::I64));
    pub const ISZ: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::Isz));

    pub const U8: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::U8));
    pub const U16: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::U16));
    pub const U64: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::U64));
    pub const USZ: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::Usz));

    pub const F32: Type = Type::Primitive(PrimitiveType::Float(FloatKind::F32));
    pub const F64: Type = Type::Primitive(PrimitiveType::Float(FloatKind::F64));

    pub const REF: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Ref));
    pub const MUT: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Mut));
    pub const IMM: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Imm));
    pub const UNIQ: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Uniq));

    pub const ARRAY: Type = Type::Primitive(PrimitiveType::Array);

    pub const NO_CLOSURE_ENV: Type = Type::Primitive(PrimitiveType::NoClosureEnv);

    /// If this is an `App (UserDefined id) args`, return `App (UserDefined new_name) args`,
    /// otherwise return `UserDefined new_name`
    /// TODO: Remove
    pub fn retarget_user_defined(&self, new_name: TopLevelName) -> Type {
        match self {
            Type::Application(_, args) => {
                Type::Application(Shared::new(Type::UserDefined(Origin::TopLevelDefinition(new_name))), args.clone())
            },
            _ => Type::UserDefined(Origin::TopLevelDefinition(new_name)),
        }
    }

    pub fn integer(kind: crate::lexer::token::IntegerKind) -> Type {
        match kind {
            crate::lexer::token::IntegerKind::I8 => Type::I8,
            crate::lexer::token::IntegerKind::I16 => Type::I16,
            crate::lexer::token::IntegerKind::I32 => Type::I32,
            crate::lexer::token::IntegerKind::I64 => Type::I64,
            crate::lexer::token::IntegerKind::Isz => Type::ISZ,
            crate::lexer::token::IntegerKind::U8 => Type::U8,
            crate::lexer::token::IntegerKind::U16 => Type::U16,
            crate::lexer::token::IntegerKind::U32 => Type::Primitive(PrimitiveType::Int(IntegerKind::U32)),
            crate::lexer::token::IntegerKind::U64 => Type::U64,
            crate::lexer::token::IntegerKind::Usz => Type::USZ,
        }
    }

    pub fn float(kind: crate::lexer::token::FloatKind) -> Type {
        match kind {
            crate::lexer::token::FloatKind::F32 => Type::F32,
            crate::lexer::token::FloatKind::F64 => Type::F64,
        }
    }

    pub fn reference(kind: ReferenceKind) -> Type {
        Type::Primitive(PrimitiveType::Reference(kind))
    }

    /// Convert this type to a string (without any coloring)
    pub fn to_string<Db>(&self, bindings: &TypeBindings, names: &impl NameStore, db: &Db) -> String
    where
        Db: DbGet<GetItem>,
    {
        self.display(bindings, names, db).to_string()
    }

    pub fn display<'local, Db, Names>(
        &'local self, bindings: &'local TypeBindings, names: &'local Names, db: &'local Db,
    ) -> TypePrinter<'local, Db, Names>
    where
        Db: DbGet<GetItem>,
    {
        static EMPTY_SET: LazyLock<FxHashSet<TypeVariableId>> = LazyLock::new(Default::default);
        self.display_with_literal_vars(bindings, &EMPTY_SET, &EMPTY_SET, names, db)
    }

    /// Like [Self::display], but unbound variables in the given sets render as their
    /// literal default (I32 or F64) instead of `_`.
    pub fn display_with_literal_vars<'local, Db, Names>(
        &'local self, bindings: &'local TypeBindings, integer_literal_vars: &'local FxHashSet<TypeVariableId>,
        float_literal_vars: &'local FxHashSet<TypeVariableId>, names: &'local Names, db: &'local Db,
    ) -> TypePrinter<'local, Db, Names>
    where
        Db: DbGet<GetItem>,
    {
        TypePrinter {
            typ: self,
            bindings,
            integer_literal_vars,
            float_literal_vars,
            hide_environments: false,
            optional_args: OptionalArgs::Unbound,
            number_elided: false,
            names,
            db,
        }
    }

    /// Returns true if this is any of the primitive integer types I8, I16, .., Usz, etc.
    pub(crate) fn is_integer(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Int(_)))
    }

    /// True if this type is `Type::Primitive(PrimitiveType::Error)`
    pub(crate) fn is_error(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Error))
    }

    /// True if this is a place, a places row, or a projection of one
    pub(crate) fn is_place_row(&self) -> bool {
        matches!(self, Type::Places(_) | Type::Place(_) | Type::PlaceProjection(..))
    }

    /// Follow all of this type's type variable bindings so that we only return
    /// `Type::Variable` if the type variable is unbound. Note that this may still return
    /// a composite type such as `Type::Application` with bound type variables within.
    pub fn follow<'a>(mut self: &'a Self, bindings: &'a TypeBindings) -> &'a Type {
        // Arbitrary upper limit
        for _ in 0..1000 {
            match self {
                typ @ Type::Variable(id) => match bindings.get(id) {
                    Some(binding) => self = binding,
                    None => return typ,
                },
                other => return other,
            }
        }
        panic!("Infinite loop in follow_type!")
    }

    /// Follow two sets of bindings
    pub fn follow_two<'a>(mut self: &'a Self, one: &'a TypeBindings, two: &'a TypeBindings) -> Type {
        // Arbitrary upper limit
        for _ in 0..1000 {
            match self {
                typ @ Type::Variable(id) => {
                    if let Some(binding) = one.get(id) {
                        self = binding;
                    } else if let Some(binding) = two.get(id) {
                        self = binding;
                    } else {
                        return typ.clone();
                    }
                },
                other => return other.clone(),
            }
        }
        panic!("Infinite loop in follow_two!")
    }

    /// True if any type variable in this type is still unbound. Row tails of effect and place
    /// rows are ignored since open rows do not affect which implicits match a type.
    pub fn has_unbound_type_variables(&self, bindings: &TypeBindings) -> bool {
        match self.follow(bindings) {
            Type::Variable(_) => true,
            Type::Primitive(_)
            | Type::Generic(_)
            | Type::UserDefined(_)
            | Type::U32(_)
            | Type::Place(_)
            | Type::Effects(_)
            | Type::Places(_)
            | Type::PlaceProjection(..) => false,
            Type::Function(function) => {
                function.parameters.iter().any(|param| param.typ.has_unbound_type_variables(bindings))
                    || function.environment.has_unbound_type_variables(bindings)
                    || function.return_type.has_unbound_type_variables(bindings)
            },
            Type::Application(constructor, args) => {
                constructor.has_unbound_type_variables(bindings)
                    || args.iter().any(|arg| arg.has_unbound_type_variables(bindings))
            },
            Type::Forall(_, typ) => typ.has_unbound_type_variables(bindings),
            Type::Tuple(elements) => elements.iter().any(|element| element.has_unbound_type_variables(bindings)),
        }
    }

    /// Similar to [Self::follow] but will replace all bound type variables reachable within
    /// this type with their bindings if found. This is sometimes referred to as "zonking."
    pub fn follow_all(&self, bindings: &TypeBindings) -> Type {
        self.follow_all_two(bindings, &TypeBindings::default())
    }

    pub fn follow_all_two(&self, bindings: &TypeBindings, more_bindings: &TypeBindings) -> Type {
        self.follow_all_opt(bindings, more_bindings).unwrap_or_else(|| self.clone())
    }

    /// Returns `Some(new_type)` when a binding was substituted somewhere in this subtree, and `None`
    /// when the subtree is unchanged so the caller can reuse the original `Arc` instead of allocating.
    pub(super) fn follow_all_opt(&self, bindings: &TypeBindings, more_bindings: &TypeBindings) -> Option<Type> {
        match self {
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) | Type::U32(_) | Type::Place(_) => None,
            Type::Variable(id) => {
                let binding = bindings.get(id).or_else(|| more_bindings.get(id))?;
                Some(binding.follow_all_two(bindings, more_bindings))
            },
            Type::Function(function) => {
                let parameters = Self::follow_all_each(&function.parameters, |param| {
                    param
                        .typ
                        .follow_all_opt(bindings, more_bindings)
                        .map(|typ| ParameterType::new(typ, param.is_implicit))
                });

                let environment = function.environment.follow_all_opt(bindings, more_bindings);
                let return_type = function.return_type.follow_all_opt(bindings, more_bindings);
                let effects = function.effects.follow_all_opt(bindings, more_bindings);
                if parameters.is_none() && environment.is_none() && return_type.is_none() && effects.is_none() {
                    return None;
                }

                Some(Type::Function(Shared::new(FunctionType {
                    parameters: parameters.unwrap_or_else(|| function.parameters.clone()),
                    environment: environment.unwrap_or_else(|| function.environment.clone()),
                    return_type: return_type.unwrap_or_else(|| function.return_type.clone()),
                    effects: effects.unwrap_or_else(|| function.effects.clone()),
                })))
            },
            Type::Application(constructor, args) => {
                let new_constructor = constructor.follow_all_opt(bindings, more_bindings);
                let new_args = Self::follow_all_each(&args[..], |arg| arg.follow_all_opt(bindings, more_bindings));
                if new_constructor.is_none() && new_args.is_none() {
                    return None;
                }
                let constructor = new_constructor.map(Shared::new).unwrap_or_else(|| constructor.clone());
                let args = new_args.map(Shared::new).unwrap_or_else(|| args.clone());
                Some(Type::Application(constructor, args))
            },
            Type::Forall(generics, typ) => {
                let typ = typ.follow_all_opt(bindings, more_bindings)?;
                Some(Type::Forall(generics.clone(), Shared::new(typ)))
            },
            Type::Tuple(elements) => {
                let new_elements = Self::follow_all_each(elements, |t| t.follow_all_opt(bindings, more_bindings))?;
                Some(Type::Tuple(Shared::new(new_elements)))
            },
            Type::Effects(effects) => {
                let effects = Self::follow_all_each(effects.as_ref()?, |e| e.follow_all_opt(bindings, more_bindings))?;
                Some(Type::effects(&effects, bindings, more_bindings))
            },
            Type::Places(places) => {
                let places = Self::follow_all_each(places.as_ref()?, |p| p.follow_all_opt(bindings, more_bindings))?;
                Some(Type::places(&places, bindings, more_bindings))
            },
            Type::PlaceProjection(base, segments) => {
                let base = base.follow_all_opt(bindings, more_bindings)?;
                Some(Type::project_place(&base, segments, bindings, more_bindings))
            },
        }
    }

    /// Map `f` over `items`, cloning any element for which `f` returns `None`. Returns
    /// `Some(new_vec)` if at least one element changed, or `None` if none did.
    fn follow_all_each<T: Clone>(items: &[T], mut f: impl FnMut(&T) -> Option<T>) -> Option<Vec<T>> {
        let mut result: Option<Vec<T>> = None;
        for (i, item) in items.iter().enumerate() {
            match f(item) {
                Some(new) => result.get_or_insert_with(|| items[..i].to_vec()).push(new),
                None => {
                    if let Some(new_items) = result.as_mut() {
                        new_items.push(item.clone());
                    }
                },
            }
        }
        result
    }

    /// Similar to substitute, but substitutes `Type::Generic` instead of `Type::TypeVariable`
    pub fn substitute(&self, bindings_to_substitute: &GenericSubstitutions, bindings_in_scope: &TypeBindings) -> Type {
        self.substitute_opt(bindings_to_substitute, bindings_in_scope).unwrap_or_else(|| self.clone())
    }

    /// Returns `Some(new_type)` when a substitution changed something in this subtree, and `None`
    /// when the subtree is unchanged
    fn substitute_opt(
        &self, bindings_to_substitute: &GenericSubstitutions, bindings_in_scope: &TypeBindings,
    ) -> Option<Type> {
        // A composite reached by following a bound type variable must resolve to that binding rather
        // than reuse `self` (the variable), so it can never report "unchanged".
        let self_is_var = matches!(self, Type::Variable(_));

        match self.follow(bindings_in_scope) {
            Type::Primitive(_) | Type::UserDefined(_) | Type::U32(_) | Type::Place(_) => None,
            Type::Generic(generic) => bindings_to_substitute.get(generic).cloned(),
            Type::Variable(id) => bindings_to_substitute.get(&Generic::Inferred(*id)).cloned(),
            Type::Function(function) => {
                let parameters = Self::follow_all_each(&function.parameters, |param| {
                    param
                        .typ
                        .substitute_opt(bindings_to_substitute, bindings_in_scope)
                        .map(|typ| ParameterType::new(typ, param.is_implicit))
                });
                let environment = function.environment.substitute_opt(bindings_to_substitute, bindings_in_scope);
                let return_type = function.return_type.substitute_opt(bindings_to_substitute, bindings_in_scope);
                let effects = function.effects.substitute_opt(bindings_to_substitute, bindings_in_scope);
                if parameters.is_none()
                    && environment.is_none()
                    && return_type.is_none()
                    && effects.is_none()
                    && !self_is_var
                {
                    return None;
                }
                Some(Type::Function(Shared::new(FunctionType {
                    parameters: parameters.unwrap_or_else(|| function.parameters.clone()),
                    environment: environment.unwrap_or_else(|| function.environment.clone()),
                    return_type: return_type.unwrap_or_else(|| function.return_type.clone()),
                    effects: effects.unwrap_or_else(|| function.effects.clone()),
                })))
            },
            Type::Application(constructor, args) => {
                let new_constructor = constructor.substitute_opt(bindings_to_substitute, bindings_in_scope);
                let new_args = Self::follow_all_each(&args[..], |arg| {
                    arg.substitute_opt(bindings_to_substitute, bindings_in_scope)
                });
                if new_constructor.is_none() && new_args.is_none() && !self_is_var {
                    return None;
                }
                let constructor = new_constructor.map(Shared::new).unwrap_or_else(|| constructor.clone());
                let args = new_args.map(Shared::new).unwrap_or_else(|| args.clone());
                Some(Type::Application(constructor, args))
            },
            Type::Forall(generics, typ) => {
                // We need to remove any generics in `generics` that are in `bindings`,
                // but we wan't to avoid allocating a new map in the common case where there are
                // no conflicts.
                let mut bindings = Cow::Borrowed(bindings_to_substitute);

                for generic in generics.iter() {
                    if bindings.contains_key(generic) {
                        let mut new_bindings = bindings.into_owned();
                        new_bindings.remove(generic);
                        bindings = Cow::Owned(new_bindings);
                    }
                }
                Some(typ.substitute_opt(&bindings, bindings_in_scope).unwrap_or_else(|| (**typ).clone()))
            },
            Type::Tuple(elements) => {
                let new_elements =
                    Self::follow_all_each(elements, |t| t.substitute_opt(bindings_to_substitute, bindings_in_scope));
                if new_elements.is_none() && !self_is_var {
                    return None;
                }
                let elements = new_elements.unwrap_or_else(|| elements.to_vec());
                Some(Type::Tuple(Shared::new(elements)))
            },
            Type::Effects(effects) => {
                let new_effects = effects.as_ref().and_then(|effects| {
                    Self::follow_all_each(effects, |e| e.substitute_opt(bindings_to_substitute, bindings_in_scope))
                });
                if new_effects.is_none() && !self_is_var {
                    return None;
                }
                let entries = match &new_effects {
                    Some(new_effects) => new_effects.as_slice(),
                    None => effects.as_ref().map_or(&[][..], |effects| effects.as_slice()),
                };
                Some(Type::effects(entries, bindings_in_scope, &Default::default()))
            },
            Type::Places(places) => {
                let new_places = places.as_ref().and_then(|places| {
                    Self::follow_all_each(places, |p| p.substitute_opt(bindings_to_substitute, bindings_in_scope))
                });
                if new_places.is_none() && !self_is_var {
                    return None;
                }
                let entries = match &new_places {
                    Some(new_places) => new_places.as_slice(),
                    None => places.as_ref().map_or(&[][..], |places| places.as_slice()),
                };
                Some(Type::places(entries, bindings_in_scope, &Default::default()))
            },
            Type::PlaceProjection(base, segments) => {
                let new_base = base.substitute_opt(bindings_to_substitute, bindings_in_scope);
                if new_base.is_none() && !self_is_var {
                    return None;
                }
                let base = new_base.as_ref().unwrap_or(base.as_ref());
                Some(Type::project_place(base, segments, bindings_in_scope, &Default::default()))
            },
        }
    }

    /// Apply a [Type::Forall] to the given type arguments. The given arguments should
    /// be [Type]s in the current context, and the returned [Type] will be in the current
    /// context as well.
    ///
    /// If this type is not a [Type::Forall], nothing will be done aside from checking the
    /// argument count is zero.
    ///
    /// Panics if `arguments.len() != self.generics.len()`
    pub fn apply_type(&self, arguments: &[Type], bindings_in_scope: &TypeBindings) -> Type {
        // TODO: Re-add
        // assert_eq!(arguments.len(), self.generics.len());
        match self {
            Type::Forall(generics, typ) => {
                let substitutions =
                    generics.iter().zip(arguments).map(|(generic, argument)| (*generic, argument.clone())).collect();
                typ.substitute(&substitutions, bindings_in_scope)
            },
            other => other.clone(),
        }
    }

    /// If this type is `Type::Forall(_, typ)`, return `typ`.
    /// Otherwise, return the type as-is.
    pub fn ignore_forall(&self) -> &Self {
        match self {
            Type::Forall(_, typ) => typ,
            other => other,
        }
    }

    /// If this type is `Type::Forall(_, typ)`, return `typ` while avoiding cloning if possible.
    /// Otherwise, return the non-forall type as is.
    fn remove_forall(self) -> Type {
        match self {
            Type::Forall(_, typ) => Shared::unwrap_or_clone(typ),
            other => other,
        }
    }

    /// Convert an ast type to a Type as closely as possible.
    ///
    /// Issues error(s) if:
    /// - A type is not applied to the correct number of kinds of arguments
    /// - The final converted type is not of kind [Kind::Type]
    /// - A name [Origin] was used which does not point to a type
    pub(crate) fn from_cst_type(
        typ: &cst::Type, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32, local_kinds: &mut LocalKinds,
        insert_implicit_type_vars: bool, open_effects_by_default: bool,
    ) -> Type {
        Self::from_cst_type_with_kind(
            typ,
            Kind::Type,
            resolve,
            db,
            next_id,
            local_kinds,
            insert_implicit_type_vars,
            open_effects_by_default,
            Omissions::Fresh,
        )
    }

    /// Converts an ast type to a generalized Type, with any free type variables
    /// replaced with a `Type::Generic(Generic::Inferred(id))` - although the type will
    /// will not be wrapped in a `forall`.
    pub(crate) fn from_cst_type_generalized(
        typ: &cst::Type, resolve: &crate::name_resolution::ResolutionResult, db: &DbHandle,
        insert_implicit_type_vars: bool, open_effects_by_default: bool,
    ) -> Type {
        Self::generalized(|next_id| {
            let mut local_kinds = LocalKinds::default();
            Self::from_cst_type_with_kind(
                typ,
                Kind::Type,
                resolve,
                db,
                next_id,
                &mut local_kinds,
                insert_implicit_type_vars,
                open_effects_by_default,
                Omissions::Rigid,
            )
        })
    }

    /// Run `convert` with fresh ids from 0, then generalize the type variables it created
    pub(crate) fn generalized(convert: impl FnOnce(&mut u32) -> Type) -> Type {
        let mut next_id = 0;
        let typ = convert(&mut next_id);

        if next_id == 0 {
            // fast track - if no type variables were created, we have nothing to replace
            typ
        } else {
            typ.generalize(&TypeBindings::default()).remove_forall()
        }
    }

    /// Convert this [cst::Type] into a [Type] with the expected [Kind].
    /// Error if the converted [Kind] does not match the expected [Kind].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_cst_type_with_kind(
        typ: &cst::Type, expected: Kind, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32,
        local_kinds: &mut LocalKinds, insert_implicit_type_vars: bool, open_effects_by_default: bool,
        omissions: Omissions,
    ) -> Type {
        let mut visited = Vec::new();
        TypeConverter::new(
            resolve,
            db,
            next_id,
            local_kinds,
            insert_implicit_type_vars,
            open_effects_by_default,
            &mut visited,
        )
        .with_omissions(omissions)
        .convert_with_kind(typ, expected)
    }

    /// Converts the type of a constructor expression. A constructor's type may omit any of its type
    /// arguments, in which case it is applied to a fresh type variable for each. Type aliases are expanded.
    pub(crate) fn from_cst_constructor_type(
        typ: &cst::Type, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32,
    ) -> Type {
        let mut visited = Vec::new();
        let mut local_kinds = LocalKinds::default();
        let mut converter = TypeConverter::new(resolve, db, next_id, &mut local_kinds, true, true, &mut visited);
        let (typ, kind) = converter.convert(typ, None);
        let arity = kind.required_argument_count();
        if arity == 0 {
            return typ;
        }
        let args = mapvec(0..arity, |_| converter.next_type_variable());
        converter.apply_type_constructor(typ, &kind, args).0
    }

    /// Convert a definition's function type; a top-level `signature`'s elided parts convert as `omissions` says
    pub(crate) fn from_cst_function(
        function: &cst::FunctionType, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32,
        open_effects_by_default: bool, signature: bool, omissions: Omissions,
    ) -> Type {
        let mut local_kinds = LocalKinds::default();
        let mut visited = Vec::new();
        let elision = if signature { Elision::Signature } else { Elision::Infer };
        TypeConverter::new(resolve, db, next_id, &mut local_kinds, true, open_effects_by_default, &mut visited)
            .with_omissions(omissions)
            .convert_function(function, elision, None)
    }

    /// Convert a top-level definition's type annotation; its function type's elided parameter
    /// places convert as `omissions` says
    pub(crate) fn from_cst_annotation(
        typ: &cst::Type, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32, open_effects_by_default: bool,
        omissions: Omissions,
    ) -> Type {
        let mut local_kinds = LocalKinds::default();
        let mut visited = Vec::new();
        TypeConverter::new(resolve, db, next_id, &mut local_kinds, true, open_effects_by_default, &mut visited)
            .with_omissions(omissions)
            .convert_annotation(typ)
    }

    /// Convert an effects clause into an effect row [Type].
    pub(crate) fn from_cst_effects_clause(
        effects: Option<&[cst::Type]>, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32,
        local_kinds: &mut LocalKinds, insert_implicit_type_vars: bool, open_effects_by_default: bool,
    ) -> Type {
        let mut visited = Vec::new();
        TypeConverter::new(
            resolve,
            db,
            next_id,
            local_kinds,
            insert_implicit_type_vars,
            open_effects_by_default,
            &mut visited,
        )
        .convert_effects_clause(effects, None)
    }
}

/// The name error messages use for the generic `generic` of `definition`. A trait's unspellable
/// impl generic is given a spellable name instead.
pub(crate) fn generic_display_name(
    definition: &cst::TypeDefinition, generic: usize, context: &DesugarContext,
) -> String {
    if Some(generic) == definition.impl_generic_index() {
        IMPL_GENERIC_DISPLAY_NAME.to_string()
    } else {
        context[definition.generics[generic].name].to_string()
    }
}

/// What to do when a type hole is encountered
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) enum Omissions {
    /// Each is a fresh type variable
    Fresh,
    /// Each is a unique rigid [Generic::Elided]
    Rigid,
    /// Omitting a type argument is an error
    Error,
}

/// Converts a [cst::Type] to a type-checker [Type].
struct TypeConverter<'a, 'b> {
    resolve: &'a ResolutionResult,
    db: &'a DbHandle<'b>,
    next_id: &'a mut u32,
    local_kinds: &'a mut LocalKinds,
    insert_implicit_type_vars: bool,

    /// Whether an omitted `can` clause converts to an open effect row instead of a closed one.
    open_effects_by_default: bool,

    /// The stack of type aliases currently being expanded, used to detect recursive aliases
    visited: &'a mut Vec<TopLevelName>,

    /// What an elided place converts to here
    elided_place: ElidedPlace,

    /// What each part left out of a type converts to
    omissions: Omissions,

    /// True while converting a parameter of a top-level signature, where each [cst::TypeKind::RigidHole]
    /// converts as [Omissions] says
    in_signature_parameter: bool,
}

/// Which elision rules a function type's parameters follow
#[derive(Clone, Copy, PartialEq, Eq)]
enum Elision {
    /// Elided places are fresh variables
    Infer,
    /// A top-level annotation's elided parameter places convert as [Omissions] says
    Annotation,
    /// A top-level lambda's signature also names places after parameters and shares bare parameters' effects
    Signature,
}

/// What an elided place converts to
enum ElidedPlace {
    /// A fresh place variable
    Infer,
    /// A place in a signature's parameter: the parameter's own place first, if any, then places
    /// converted as [Omissions] says
    Parameter(Option<Type>),
    /// A place in a return type, which refers to the function's only input place
    Return(Type),
}

impl<'a, 'b> TypeConverter<'a, 'b> {
    fn new(
        resolve: &'a ResolutionResult, db: &'a DbHandle<'b>, next_id: &'a mut u32, local_kinds: &'a mut LocalKinds,
        insert_implicit_type_vars: bool, open_effects_by_default: bool, visited: &'a mut Vec<TopLevelName>,
    ) -> Self {
        TypeConverter {
            resolve,
            db,
            next_id,
            local_kinds,
            insert_implicit_type_vars,
            open_effects_by_default,
            visited,
            elided_place: ElidedPlace::Infer,
            omissions: Omissions::Fresh,
            in_signature_parameter: false,
        }
    }

    fn with_omissions(mut self, omissions: Omissions) -> Self {
        self.omissions = omissions;
        self
    }

    /// Convert `typ` and error if its [Kind] does not unify with `expected`.
    fn convert_with_kind(&mut self, typ: &cst::Type, expected: Kind) -> Type {
        let (converted, kind) = self.convert(typ, Some(&expected));
        self.check_kind(kind, expected, &typ.location);
        converted
    }

    /// Error if `actual` does not unify with `expected`
    fn check_kind(&self, actual: Kind, expected: Kind, location: &Location) {
        if !expected.unifies(&actual) {
            self.db.accumulate(Diagnostic::ExpectedKind { actual, expected, location: location.clone() });
        }
    }

    /// Convert a [cst::Type] into a [Type]. Returns the converted type and its kind.
    /// Does not error if the result is not of kind [Kind::Type].
    fn convert(&mut self, typ: &cst::Type, expected: Option<&Kind>) -> (Type, Kind) {
        match &typ.kind {
            crate::parser::cst::TypeKind::Integer(kind) => {
                let typ = match kind {
                    IntegerKind::I8 => Type::I8,
                    IntegerKind::I16 => Type::I16,
                    IntegerKind::I32 => Type::I32,
                    IntegerKind::I64 => Type::I64,
                    IntegerKind::Isz => Type::ISZ,
                    IntegerKind::U8 => Type::U8,
                    IntegerKind::U16 => Type::U16,
                    IntegerKind::U32 => Type::Primitive(PrimitiveType::Int(IntegerKind::U32)),
                    IntegerKind::U64 => Type::U64,
                    IntegerKind::Usz => Type::USZ,
                };
                (typ, Kind::Type)
            },
            crate::parser::cst::TypeKind::Float(kind) => match kind {
                FloatKind::F32 => (Type::F32, Kind::Type),
                FloatKind::F64 => (Type::F64, Kind::Type),
            },
            crate::parser::cst::TypeKind::Char => (Type::CHAR, Kind::Type),
            crate::parser::cst::TypeKind::Named(path) => {
                let origin = self.resolve.path_origins.get(path).copied();
                let location = &typ.location;
                let (typ, kind) =
                    Type::convert_origin_to_type(origin, self.db, location, self.local_kinds, Type::UserDefined);

                if let Type::UserDefined(Origin::TopLevelDefinition(name)) = typ {
                    // A bare type with all-default parameters gets them, unless it is an application head or constructor
                    if expected.is_some()
                        && kind.required_argument_count() != 0
                        && self.required_parameter_count(name) == Some(0)
                    {
                        return self.convert_application(typ, kind, &[], location);
                    }

                    // Expand a type or effect alias if necessary
                    if kind == Kind::Type
                        && let Some(expanded) = self.expand_type_alias(name, &[])
                    {
                        return (expanded, Kind::Type);
                    } else if kind == Kind::Effect
                        && let Some(expanded) = self.expand_effect_alias(name, &[])
                    {
                        return (expanded, Kind::Effect);
                    }
                }
                (typ, kind)
            },
            crate::parser::cst::TypeKind::Variable(name) => self.convert_type_variable(*name, expected, &typ.location),
            crate::parser::cst::TypeKind::Place(paths) => {
                let none = TypeBindings::default();
                let entries = mapvec(paths, |path| {
                    let (root, kind) = self.convert_type_variable(path.root, Some(&Kind::Place), &typ.location);
                    self.check_kind(kind, Kind::Place, &typ.location);
                    Type::project_place(&root, &path.segments, &none, &none)
                });
                let place =
                    if let [entry] = entries.as_slice() { entry.clone() } else { Type::places(&entries, &none, &none) };
                (place, Kind::Place)
            },
            crate::parser::cst::TypeKind::Function(function) => {
                (self.convert_function(function, Elision::Infer, None), Kind::Type)
            },
            crate::parser::cst::TypeKind::Error => (Type::ERROR, Kind::Error),
            crate::parser::cst::TypeKind::Unit => (Type::UNIT, Kind::Type),
            crate::parser::cst::TypeKind::Application(f, args) => {
                let (f, f_kind) = self.convert(f, None);
                self.convert_application(f, f_kind, args, &typ.location)
            },
            crate::parser::cst::TypeKind::Reference(kind) => {
                (Type::Primitive(PrimitiveType::Reference(*kind)), Kind::from_args(vec![Kind::Place, Kind::Type]))
            },
            crate::parser::cst::TypeKind::NoClosureEnv => (Type::NO_CLOSURE_ENV, Kind::Type),
            crate::parser::cst::TypeKind::Tuple(elements) => {
                let elements = mapvec(elements, |t| self.convert_with_kind(t, Kind::Type));
                (Type::Tuple(Shared::new(elements)), Kind::Type)
            },
            crate::parser::cst::TypeKind::RigidHole if self.in_signature_parameter => (self.next_omitted(), Kind::Type),
            crate::parser::cst::TypeKind::Hole | crate::parser::cst::TypeKind::RigidHole
                if self.insert_implicit_type_vars =>
            {
                (self.next_omitted(), Kind::Type)
            },
            crate::parser::cst::TypeKind::Hole | crate::parser::cst::TypeKind::RigidHole => {
                self.db.accumulate(Diagnostic::HoleCantBeUsed { location: typ.location.clone() });
                (Type::ERROR, Kind::Error)
            },
            // TODO: There is a separate check in type definition bodies to reject these.
            // Rework it to be more similar to the HoleCantBeUsed above.
            crate::parser::cst::TypeKind::ImplicitPlace => {
                let place = match &mut self.elided_place {
                    ElidedPlace::Infer => self.next_type_variable(),
                    ElidedPlace::Parameter(own) => own.take().unwrap_or_else(|| self.next_omitted()),
                    ElidedPlace::Return(place) => place.clone(),
                };
                (place, Kind::Place)
            },
            crate::parser::cst::TypeKind::IntegerConstant(v) => (Type::U32(*v), Kind::U32),
            crate::parser::cst::TypeKind::Pure => (Type::pure(), Kind::Effect),
            crate::parser::cst::TypeKind::EffectUnion(operands) => {
                (self.convert_effect_row_entries(operands, None), Kind::Effect)
            },
            crate::parser::cst::TypeKind::Forall(generics, body) => {
                self.declare_forall_kinds(generics);
                self.convert(body, expected)
            },
        }
    }

    /// Record the kind of each generic in an explicit `forall`
    fn declare_forall_kinds(&mut self, generics: &cst::Generics) {
        for param in generics {
            let kind = param.kind.map(kind_from_annotation).unwrap_or(Kind::Type);
            self.local_kinds.insert(param.name, kind);
        }
    }

    /// Convert a top-level definition's type annotation; its function type's elided parameter places are rigid
    fn convert_annotation(&mut self, typ: &cst::Type) -> Type {
        match &typ.kind {
            cst::TypeKind::Function(function) => self.convert_function(function, Elision::Annotation, None),
            cst::TypeKind::Forall(generics, body) => {
                self.declare_forall_kinds(generics);
                self.convert_annotation(body)
            },
            _ => self.convert_with_kind(typ, Kind::Type),
        }
    }

    /// Convert a type variable or place name, recording its kind as `expected` if it has none yet
    fn convert_type_variable(&mut self, name: NameId, expected: Option<&Kind>, location: &Location) -> (Type, Kind) {
        let origin = self.resolve.name_origins.get(&name).copied();
        if let (Some(expected), Some(Origin::Local(name_id))) = (expected, origin) {
            self.local_kinds.entry(name_id).or_insert_with(|| expected.clone());
        }
        Type::convert_origin_to_type(origin, self.db, location, self.local_kinds, |origin| {
            Type::Generic(Generic::Named(origin))
        })
    }

    /// Apply the type constructor `f` to `args`, filling in the defaults of any omitted trailing arguments
    fn convert_application(&mut self, f: Type, f_kind: Kind, args: &[cst::Type], location: &Location) -> (Type, Kind) {
        let arity = f_kind.required_argument_count();
        let name = match &f {
            Type::UserDefined(Origin::TopLevelDefinition(name)) => Some(*name),
            _ => None,
        };

        if !f_kind.accepts_n_arguments(args.len()) {
            let required = name.and_then(|name| self.required_parameter_count(name)).unwrap_or(arity);
            if !(required..arity).contains(&args.len()) {
                let expected = if args.len() < required { required } else { arity };
                let location = location.clone();
                self.db.accumulate(Diagnostic::FunctionArgCountMismatch { actual: args.len(), expected, location });
                return (Type::ERROR, Kind::Type);
            }
        }

        let mut converted_args = mapvec(args.iter().enumerate(), |(i, arg)| {
            let expected_kind = f_kind.get_nth_parameter_kind(i);
            self.convert_with_kind(arg, expected_kind)
        });

        if let Some(name) = name
            && converted_args.len() < arity
        {
            self.push_default_arguments(name, &f_kind, &mut converted_args, location);
        }

        assert!(!converted_args.is_empty());
        self.apply_type_constructor(f, &f_kind, converted_args)
    }

    /// Apply `constructor` to `args`, expanding any type or effect alias.
    fn apply_type_constructor(&mut self, constructor: Type, kind: &Kind, args: Vec<Type>) -> (Type, Kind) {
        // Expand a generic type or effect alias if necessary
        if let Type::UserDefined(Origin::TopLevelDefinition(name)) = &constructor {
            if let Some(expanded) = self.expand_type_alias(*name, &args) {
                return (expanded, Kind::Type);
            } else if let Some(expanded) = self.expand_effect_alias(*name, &args) {
                return (expanded, Kind::Effect);
            }
        }

        let typ = Type::Application(Shared::new(constructor), Shared::new(args));
        (typ, kind.result_kind())
    }

    /// The minimum number of arguments `name` accepts. Returns `None` if it is not a type definition
    fn required_parameter_count(&self, name: TopLevelName) -> Option<usize> {
        let (item, _) = GetItem(name.top_level_item).get(self.db);
        let cst::TopLevelItemKind::TypeDefinition(definition) = &item.kind else {
            return None;
        };
        Some(definition.required_generic_count())
    }

    /// Push the defaults of `name`'s parameters after `args`
    fn push_default_arguments(&mut self, name: TopLevelName, kind: &Kind, args: &mut Vec<Type>, location: &Location) {
        let (item, ctx) = GetItem(name.top_level_item).get(self.db);
        let cst::TopLevelItemKind::TypeDefinition(definition) = &item.kind else {
            return;
        };

        for (i, param) in definition.generics.iter().enumerate().skip(args.len()) {
            let default = param.default.as_ref().expect("omitted parameters should all have defaults");

            let cant_infer = !self.insert_implicit_type_vars || self.omissions == Omissions::Error;
            let typ = if cant_infer && default.contains_hole() {
                let typ = ctx[definition.name].to_string();
                let param = generic_display_name(definition, i, &ctx);
                self.db.accumulate(Diagnostic::DefaultHoleCantBeUsed { typ, param, location: location.clone() });
                Type::ERROR
            } else {
                let implicit = self.insert_implicit_type_vars;
                let kind = kind.get_nth_parameter_kind(i);
                self.expand_alias(name, &ctx, definition, args, implicit, |converter| {
                    converter.convert_with_kind(default, kind)
                })
            };
            args.push(typ);
        }
    }

    /// Convert a function type, following `elision`'s rules for its parameters
    fn convert_function(
        &mut self, function: &cst::FunctionType, elision: Elision, implicit_effects: Option<Type>,
    ) -> Type {
        // TODO: Places a callee provides (callback parameters, `can` clauses) need a `forall` on their function type
        let signature = elision == Elision::Signature;

        // Parameters without a `can` clause share the signature's row variable, or a fresh one
        let row_variable = function.effects.iter().flatten().find(|e| matches!(e.kind, cst::TypeKind::Variable(_)));
        let has_bare_parameter = function.parameters.iter().any(|param| param.typ.is_function_without_effects());
        let parameter_row = (signature && has_bare_parameter).then(|| match row_variable {
            Some(variable) => self.convert_with_kind(variable, Kind::Effect),
            None => self.next_omitted(),
        });

        let outer_in_signature_parameter = self.in_signature_parameter;
        self.in_signature_parameter |= elision != Elision::Infer;
        let parameters = mapvec(&function.parameters, |param| {
            let typ = match &param.typ.kind {
                cst::TypeKind::Function(bare) if signature && bare.effects.is_none() => {
                    self.convert_function(bare, Elision::Infer, parameter_row.clone())
                },
                _ if elision != Elision::Infer => {
                    let place = ElidedPlace::Parameter(self.parameter_place(param));
                    self.convert_with_place(&param.typ, place)
                },
                _ => self.convert_with_place(&param.typ, ElidedPlace::Infer),
            };
            ParameterType::new(typ, param.is_implicit)
        });
        self.in_signature_parameter = outer_in_signature_parameter;
        let environment = match function.environment.as_ref() {
            Some(environment) => self.convert_with_place(environment, ElidedPlace::Infer),
            None => Type::NO_CLOSURE_ENV,
        };

        let return_place = self.elided_return_place(function, &parameters, &environment);
        let return_type = self.convert_with_place(&function.return_type, return_place);

        // A bare parameter's effects are its signature's row variable, added to the signature's clause if fresh
        let implicit_effects = match function.effects {
            Some(_) => parameter_row.filter(|_| row_variable.is_none()),
            None => implicit_effects,
        };
        let effects = self.with_elided_place(ElidedPlace::Infer, |this| {
            this.convert_effects_clause(function.effects.as_deref(), implicit_effects)
        });
        Type::Function(Shared::new(FunctionType { parameters, environment, return_type, effects }))
    }

    /// Convert `typ` with `place` for its elided places
    fn convert_with_place(&mut self, typ: &cst::Type, place: ElidedPlace) -> Type {
        self.with_elided_place(place, |this| this.convert_with_kind(typ, Kind::Type))
    }

    /// Run `convert` with `place` for elided places
    fn with_elided_place<T>(&mut self, place: ElidedPlace, convert: impl FnOnce(&mut Self) -> T) -> T {
        let outer_place = std::mem::replace(&mut self.elided_place, place);
        let result = convert(self);
        self.elided_place = outer_place;
        result
    }

    /// What elided places in `function`'s return type refer to: its only input place, if it has one
    fn elided_return_place(
        &mut self, function: &cst::FunctionType, parameters: &[ParameterType], environment: &Type,
    ) -> ElidedPlace {
        if !function.return_type.has_elided_place() {
            return ElidedPlace::Infer;
        }

        let mut places = Vec::new();
        for typ in parameters.iter().map(|parameter| &parameter.typ).chain(std::iter::once(environment)) {
            collect_input_places(typ, &mut places);
        }

        match places.as_slice() {
            [] => ElidedPlace::Infer,
            [place] => ElidedPlace::Return(place.clone()),
            _ => {
                let location = function.return_type.location.clone();
                self.db.accumulate(Diagnostic::AmbiguousElidedPlace { location });
                ElidedPlace::Infer
            },
        }
    }

    /// The place of a signature's `parameter`, named after it
    fn parameter_place(&mut self, parameter: &cst::ParameterType) -> Option<Type> {
        let name = parameter.name.filter(|_| parameter.typ.has_elided_place())?;
        Some(self.convert_type_variable(name, Some(&Kind::Place), &parameter.typ.location).0)
    }

    /// Convert an effects clause into an effect row with `implicit` added.
    /// An omitted clause without `implicit` is open or closed per `self.open_effects_by_default`.
    fn convert_effects_clause(&mut self, effects: Option<&[cst::Type]>, implicit: Option<Type>) -> Type {
        match (effects, implicit) {
            (None, None) if self.open_effects_by_default => {
                Type::Effects(Some(Shared::new(vec![self.next_type_variable()])))
            },
            (None, None) => Type::pure(),
            (list, implicit) => self.convert_effect_row_entries(list.unwrap_or_default(), implicit),
        }
    }

    /// Convert a list of effect type entries, plus `implicit` if given, into an effect row
    fn convert_effect_row_entries(&mut self, list: &[cst::Type], implicit: Option<Type>) -> Type {
        let mut effects = mapvec(list, |e| self.convert_with_kind(e, Kind::Effect));
        effects.extend(implicit);
        Type::effects(&effects, &Default::default(), &Default::default())
    }

    fn next_type_variable_id(&mut self) -> TypeVariableId {
        let id = TypeVariableId(*self.next_id);
        *self.next_id += 1;
        id
    }

    fn next_type_variable(&mut self) -> Type {
        Type::Variable(self.next_type_variable_id())
    }

    /// A hole or a part left out of a top-level signature's parameter, see [Omissions]
    fn next_omitted(&mut self) -> Type {
        match self.omissions {
            Omissions::Rigid => self.next_elided_generic(),
            Omissions::Fresh | Omissions::Error => self.next_type_variable(),
        }
    }

    fn next_elided_generic(&mut self) -> Type {
        Type::Generic(Generic::Elided(self.next_type_variable_id()))
    }

    /// If `name` refers to a type alias, return its body type with `args` substituted
    /// for the alias's generic parameters. Returns `None` if `name` is not a type alias.
    ///
    /// Emits [Diagnostic::RecursiveTypeAlias] if the type alias is infinitely recursive.
    fn expand_type_alias(&mut self, name: TopLevelName, args: &[Type]) -> Option<Type> {
        let (item, ctx) = GetItem(name.top_level_item).get(self.db);
        let cst::TopLevelItemKind::TypeDefinition(definition) = &item.kind else {
            return None;
        };
        let cst::TypeDefinitionBody::Alias(body) = &definition.body else {
            return None;
        };
        Some(self.expand_alias(name, &ctx, definition, args, false, |converter| {
            converter.convert(body, Some(&Kind::Type)).0
        }))
    }

    /// If `name` refers to an effect alias, return its expansion with `args` substituted
    /// for the alias's generic parameters. Returns `None` if `name` is not an effect alias.
    ///
    /// Emits [Diagnostic::RecursiveTypeAlias] if the effect alias is infinitely recursive.
    fn expand_effect_alias(&mut self, name: TopLevelName, args: &[Type]) -> Option<Type> {
        let (item, ctx) = GetItem(name.top_level_item).get(self.db);
        let cst::TopLevelItemKind::TypeDefinition(definition) = &item.kind else {
            return None;
        };
        let cst::TypeDefinitionBody::EffectAlias(effects) = &definition.body else {
            return None;
        };
        Some(self.expand_alias(name, &ctx, definition, args, false, |converter| {
            converter.convert_effects_clause(Some(effects), None)
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn expand_alias(
        &mut self, name: TopLevelName, ctx: &DesugarContext, definition: &cst::TypeDefinition, args: &[Type],
        insert_implicit_type_vars: bool, convert_body: impl FnOnce(&mut TypeConverter) -> Type,
    ) -> Type {
        if self.visited.contains(&name) {
            let typ = ctx[definition.name].to_string();
            let location = ctx.name_location(definition.name).clone();
            self.db.accumulate(Diagnostic::RecursiveTypeAlias { typ, location });
            return Type::ERROR;
        }
        self.visited.push(name);

        let resolve = Resolve(name.top_level_item).get(self.db);
        let mut local_kinds = TypeChecker::local_kinds_from_generics(&definition.generics);
        let mut converter = TypeConverter::new(
            &resolve,
            self.db,
            self.next_id,
            &mut local_kinds,
            insert_implicit_type_vars,
            false,
            self.visited,
        )
        .with_omissions(self.omissions);

        let body_type = convert_body(&mut converter);

        let result = if definition.generics.is_empty() {
            body_type
        } else {
            let generics_and_args = definition.generics.iter().zip(args);
            let make_generic = |name| Generic::Named(Origin::Local(name));
            let substitutions = generics_and_args.map(|(param, arg)| (make_generic(param.name), arg.clone()));
            body_type.substitute(&substitutions.collect(), &TypeBindings::default())
        };

        self.visited.pop();
        result
    }
}

/// Collect the distinct place of each reference in `typ`, except within function types
fn collect_input_places(typ: &Type, places: &mut Vec<Type>) {
    if let Some(place) = typ.reference_places(&TypeBindings::default())
        && !places.contains(&place)
    {
        places.push(place);
    }
    match typ {
        Type::Application(_, args) => args.iter().for_each(|arg| collect_input_places(arg, places)),
        Type::Tuple(elements) => elements.iter().for_each(|element| collect_input_places(element, places)),
        _ => (),
    }
}

/// Expand an effect alias to `IO` into a row like `Fs, Net`.
/// Returns `None` if `name` is not an effect alias.
pub(crate) fn expand_effect_alias(db: &DbHandle, name: TopLevelName, next_id: &mut u32) -> Option<Type> {
    // TODO: This is a lot of setup just to call one function
    let mut visited = Vec::new();
    let resolve = Resolve(name.top_level_item).get(db);
    let mut local_kinds = LocalKinds::default();
    let mut converter = TypeConverter::new(&resolve, db, next_id, &mut local_kinds, false, false, &mut visited);
    converter.expand_effect_alias(name, &[])
}

impl Type {
    fn convert_origin_to_type(
        origin: Option<Origin>, db: &DbHandle, location: &Location, local_kinds: &LocalKinds,
        make_type: impl FnOnce(Origin) -> Type,
    ) -> (Type, Kind) {
        match origin {
            Some(Origin::Builtin(builtin)) => match builtin {
                Builtin::Unit => (Type::UNIT, Kind::Type),
                Builtin::Char => (Type::CHAR, Kind::Type),
                Builtin::Bool => (Type::BOOL, Kind::Type),
                Builtin::Ptr => (Type::POINTER, Kind::from_args(vec![Kind::Type])),
                Builtin::Array => (Type::ARRAY, Kind::from_args(vec![Kind::U32, Kind::Type])),
                Builtin::Never => (Type::NEVER, Kind::Type),
                Builtin::Intrinsic => (Type::ERROR, Kind::Error),
            },
            Some(origin @ Origin::TopLevelDefinition(type_name)) => {
                let (item, ctx) = GetItem(type_name.top_level_item).get(db);
                if let cst::TopLevelItemKind::TypeDefinition(definition) = &item.kind {
                    return (make_type(origin), crate::definition_collection::kind_of_type_definition(definition));
                }

                // An impl's name in a type position refers to its data type, a type
                // constructor over each of the impl's parameter types.
                if let cst::TopLevelItemKind::Definition(definition) = &item.kind
                    && definition.impl_info.is_some()
                    && let resolve = Resolve(type_name.top_level_item).get(db)
                    && get_type::trait_impl_of(definition, &ctx, &resolve, db).is_some()
                {
                    let kind = Kind::from_args(vec![Kind::Type; definition.impl_parameters(ctx.as_ref()).len()]);
                    (Type::Primitive(PrimitiveType::ImplData(type_name)), kind)
                } else {
                    let name = item.kind.name().to_string(ctx.as_ref());
                    db.accumulate(Diagnostic::NotAType { name, location: location.clone() });
                    (Type::ERROR, Kind::Type)
                }
            },
            Some(origin @ Origin::Local(name)) => {
                let kind = local_kinds.get(&name).cloned().unwrap_or(Kind::Type);
                (make_type(origin), kind)
            },
            // Assume name resolution has already issued an error for these cases
            Some(Origin::TraitMember(_) | Origin::EffectOperation { .. }) | None => (Type::ERROR, Kind::Error),
            Some(origin) => (make_type(origin), Kind::Type),
        }
    }

    /// Generalize a type, making it generic. Any holes in the type become generic types.
    pub fn generalize(&self, bindings: &TypeBindings) -> Type {
        let free_vars = self.free_vars(bindings);

        if free_vars.is_empty() {
            self.clone()
        } else {
            let substitutions = free_vars.iter().map(|var| (*var, Type::Generic(*var))).collect();
            let typ = self.substitute(&substitutions, bindings);
            Type::Forall(Shared::new(free_vars), Shared::new(typ))
        }
    }

    /// Instantiates only `Generic::Inferred` generics with fresh type variables.
    /// Also returns a forall-wrapped type with each new type variable.
    pub(crate) fn open_inferred_generics(&self, next_id: &mut u32) -> (Type, Type) {
        let (generics, body) = match self {
            Type::Forall(generics, body) => (generics.as_slice(), body.as_ref()),
            other => return (other.clone(), other.clone()),
        };

        let place_kinded = body.place_kinded_generics();
        let mut new_generics = Vec::with_capacity(generics.len());
        let mut seed_substitutions = GenericSubstitutions::default();
        let mut declared_substitutions = GenericSubstitutions::default();
        for generic in generics.iter() {
            match generic {
                Generic::Named(_) => new_generics.push(*generic),
                // Elided generics stay rigid but are renumbered since each signature numbers them
                // locally from zero and they would otherwise collide with those of other signatures.
                Generic::Elided(_) => {
                    let new_generic = Generic::Elided(TypeVariableId(*next_id));
                    *next_id += 1;
                    new_generics.push(new_generic);
                    seed_substitutions.insert(*generic, Type::Generic(new_generic));
                    declared_substitutions.insert(*generic, Type::Generic(new_generic));
                },
                Generic::Inferred(_) => {
                    let id = TypeVariableId(*next_id);
                    *next_id += 1;
                    let new_generic = Generic::Inferred(id);
                    new_generics.push(new_generic);
                    let seed = if place_kinded.contains(generic) {
                        Type::places(&[Type::Variable(id)], &TypeBindings::default(), &TypeBindings::default())
                    } else {
                        Type::Variable(id)
                    };
                    seed_substitutions.insert(*generic, seed);
                    declared_substitutions.insert(*generic, Type::Generic(new_generic));
                },
            }
        }

        let seeded = if seed_substitutions.is_empty() {
            body.clone()
        } else {
            body.substitute(&seed_substitutions, &TypeBindings::default())
        };
        let declared_body = if declared_substitutions.is_empty() {
            body.clone()
        } else {
            body.substitute(&declared_substitutions, &TypeBindings::default())
        };
        (seeded, Type::Forall(Shared::new(new_generics), Shared::new(declared_body)))
    }

    /// Collect each [Type::Generic] with [Kind::Place].
    /// FIXME: This is approximate until we track the kind of type variables
    pub(crate) fn place_kinded_generics(&self) -> FxHashSet<Generic> {
        let mut out = FxHashSet::default();
        self.collect_place_kinded_generics(&mut out);
        out
    }

    fn collect_place_kinded_generics(&self, out: &mut FxHashSet<Generic>) {
        match self {
            Type::Application(constructor, args) => {
                if constructor.reference_constructor(&TypeBindings::default()).is_some() {
                    let place = match args.first() {
                        Some(Type::PlaceProjection(base, _)) => Some(base.as_ref()),
                        other => other,
                    };
                    if let Some(Type::Generic(g)) = place {
                        out.insert(*g);
                    }
                }
                for arg in args.iter() {
                    arg.collect_place_kinded_generics(out);
                }
            },
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    element.collect_place_kinded_generics(out);
                }
            },
            Type::Function(f) => {
                for parameter in &f.parameters {
                    parameter.typ.collect_place_kinded_generics(out);
                }
                f.environment.collect_place_kinded_generics(out);
                f.return_type.collect_place_kinded_generics(out);
            },
            Type::Forall(_, body) => body.collect_place_kinded_generics(out),
            Type::Primitive(_)
            | Type::Generic(_)
            | Type::Variable(_)
            | Type::UserDefined(_)
            | Type::U32(_)
            | Type::Effects(_)
            | Type::Place(_)
            | Type::Places(_)
            | Type::PlaceProjection(..) => (),
        }
    }

    /// Collect all unbound effects in every effect row within this type.
    pub(crate) fn unbound_effect_entries(&self, bindings: &TypeBindings) -> Vec<TypeVariableId> {
        fn go(typ: &Type, bindings: &TypeBindings, ends: &mut Vec<TypeVariableId>) {
            let followed = typ.follow(bindings);
            match followed {
                Type::Effects(_) => {
                    for effect in followed.effect_entries() {
                        match effect.follow(bindings) {
                            Type::Variable(id) => ends.push(*id),
                            nested => go(nested, bindings, ends),
                        }
                    }
                },
                Type::Function(function) => {
                    for parameter in &function.parameters {
                        go(&parameter.typ, bindings, ends);
                    }
                    go(&function.return_type, bindings, ends);
                    go(&function.effects, bindings, ends);
                },
                Type::Application(constructor, args) => {
                    go(constructor, bindings, ends);
                    for arg in args.iter() {
                        go(arg, bindings, ends);
                    }
                },
                Type::Tuple(elements) => {
                    for element in elements.iter() {
                        go(element, bindings, ends);
                    }
                },
                Type::Forall(_, inner) => go(inner, bindings, ends),
                _ => (),
            }
        }

        let mut ends = Vec::new();
        go(self, bindings, &mut ends);
        ends
    }

    /// Walk every subterm of this type, although does not recur into [Type::Forall].
    fn for_each_subterm(&self, bindings: &TypeBindings, f: &mut impl FnMut(&Type, &TypeBindings)) {
        let typ = self.follow(bindings);
        f(typ, bindings);
        match typ {
            Type::Primitive(_)
            | Type::UserDefined(_)
            | Type::Variable(_)
            | Type::Generic(_)
            | Type::U32(_)
            | Type::Place(_) => (),
            Type::Function(function) => {
                for parameter in &function.parameters {
                    parameter.typ.for_each_subterm(bindings, f);
                }
                function.environment.for_each_subterm(bindings, f);
                function.return_type.for_each_subterm(bindings, f);
                function.effects.for_each_subterm(bindings, f);
            },
            Type::Application(constructor, args) => {
                constructor.for_each_subterm(bindings, f);
                for arg in args.iter() {
                    arg.for_each_subterm(bindings, f);
                }
            },
            // Bound generics need different treatment per-caller (e.g. `free_vars` filters
            // them back out after recursing), so `f` is responsible for recursing into `typ`.
            Type::Forall(..) => (),
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    element.for_each_subterm(bindings, f);
                }
            },
            Type::Effects(effects) => {
                if let Some(effects) = effects.as_ref() {
                    for effect in effects.iter() {
                        effect.for_each_subterm(bindings, f);
                    }
                }
            },
            Type::Places(places) => {
                if let Some(places) = places.as_ref() {
                    for place in places.iter() {
                        place.for_each_subterm(bindings, f);
                    }
                }
            },
            Type::PlaceProjection(base, _) => base.for_each_subterm(bindings, f),
        }
    }

    /// Return the list of unbound type variables or generics within this type
    pub fn free_vars(&self, bindings: &TypeBindings) -> Vec<Generic> {
        let mut free_vars = Vec::new();

        // `for_each_subterm` doesn't recur into Type::Forall, so do it manually here
        fn go(typ: &Type, bindings: &TypeBindings, free_vars: &mut Vec<Generic>) {
            typ.for_each_subterm(bindings, &mut |typ, bindings| match typ {
                Type::Variable(id) => {
                    // The number of free vars is expected to remain too small so we're
                    // not too worried about asymptotic behavior. It is more important we
                    // maintain the ordering of insertion.
                    let generic = Generic::Inferred(*id);
                    if !free_vars.contains(&generic) {
                        free_vars.push(generic);
                    }
                },
                Type::Generic(generic) => {
                    if !free_vars.contains(generic) {
                        free_vars.push(*generic);
                    }
                },
                Type::Forall(generics, inner) => {
                    go(inner, bindings, free_vars);
                    free_vars.retain(|generic| !generics.contains(generic));
                },
                _ => (),
            });
        }

        go(self, bindings, &mut free_vars);
        free_vars
    }

    /// Return the list of unbound type variables within this type.
    /// Unlike [Self::free_vars], this excludes [Type::Generic]s within the type and it returns a set.
    pub fn free_unification_vars(&self, bindings: &TypeBindings) -> FxHashSet<TypeVariableId> {
        self.free_vars(bindings).into_iter().filter_map(Generic::as_inferred).collect::<FxHashSet<_>>()
    }

    /// Counts every occurrence of `target` within this type, unlike [Self::free_unification_vars] which dedups into a set.
    pub fn count_unification_var_occurrences(&self, target: TypeVariableId, bindings: &TypeBindings) -> usize {
        fn go(typ: &Type, target: TypeVariableId, bindings: &TypeBindings, count: &mut usize) {
            typ.for_each_subterm(bindings, &mut |typ, bindings| match typ {
                Type::Variable(id) => {
                    if *id == target {
                        *count += 1;
                    }
                },
                Type::Forall(_, inner) => go(inner, target, bindings, count),
                _ => (),
            });
        }
        let mut count = 0;
        go(self, target, bindings, &mut count);
        count
    }

    /// If this is a function, return its return type. Otherwise return None.
    pub(crate) fn return_type(&self) -> Option<&Type> {
        match self {
            Type::Function(function) => Some(&function.return_type),
            _ => None,
        }
    }

    /// If this is a type application, return the constructor and arguments
    pub fn as_application(&self) -> Option<(&Shared<Type>, &Shared<Vec<Type>>)> {
        match self {
            Type::Application(constructor, args) => Some((constructor, args)),
            _ => None,
        }
    }

    /// The type constructor at the head of this type, along with its arguments if it is applied
    fn head_and_args<'a>(&'a self, bindings: &'a TypeBindings) -> (&'a Type, Option<&'a [Type]>) {
        match self.follow(bindings) {
            Type::Application(constructor, args) => (constructor.follow(bindings), Some(args.as_slice())),
            other => (other, None),
        }
    }

    /// The top-level definition this type names, along with its arguments if it is applied
    pub(crate) fn definition_head<'a>(
        &'a self, bindings: &'a TypeBindings,
    ) -> Option<(TopLevelName, Option<&'a [Type]>)> {
        match self.head_and_args(bindings) {
            (Type::UserDefined(Origin::TopLevelDefinition(name)), args) => Some((*name, args)),
            _ => None,
        }
    }

    /// If this is a trait dictionary type, return the trait along with its arguments if it is applied
    pub(crate) fn trait_application<'a>(
        &'a self, bindings: &'a TypeBindings, db: &impl DbGet<GetItem>,
    ) -> Option<(TraitDefinition, Option<&'a [Type]>)> {
        let (name, args) = self.definition_head(bindings)?;
        Some((TraitDefinition::get(name, db)?, args))
    }

    /// The impl type argument of a trait dictionary type, see [cst::TypeDefinitionKind::Trait]
    pub(crate) fn impl_arg<'a>(&'a self, bindings: &'a TypeBindings, db: &impl DbGet<GetItem>) -> Option<&'a Type> {
        let (trait_, args) = self.trait_application(bindings, db)?;
        args?.get(trait_.impl_generic_index())
    }

    /// If this is user-defined, return the origin
    pub fn as_user_defined(&self) -> Option<&Origin> {
        match self {
            Type::UserDefined(origin) => Some(origin),
            _ => None,
        }
    }

    /// `Some(kind)` if the passed type is a reference type constructor.
    /// Note that this returns `None` for `Application(Reference, _)`,
    /// it is only `Some` for references directly.
    pub(super) fn reference_constructor(&self, bindings: &TypeBindings) -> Option<ReferenceKind> {
        match self.follow(bindings) {
            Type::Primitive(PrimitiveType::Reference(kind)) => Some(*kind),
            _ => None,
        }
    }

    /// If this is a reference type, return the reference kind and its element type
    pub fn reference_element(&self, bindings: &TypeBindings) -> Option<(ReferenceKind, Type)> {
        match self.follow(bindings) {
            Type::Application(constructor, args) if args.len() >= 2 => {
                constructor.reference_constructor(bindings).map(|kind| (kind, args[1].clone()))
            },
            _ => None,
        }
    }

    /// `true` if this type is the `Pointer` primitive constructor.
    fn pointer_constructor(&self, bindings: &TypeBindings) -> bool {
        matches!(self.follow(bindings), Type::Primitive(PrimitiveType::Pointer))
    }

    /// If this is `Ptr t`, return its element type `t`.
    pub fn pointer_element(&self, bindings: &TypeBindings) -> Option<Type> {
        match self.follow(bindings) {
            Type::Application(constructor, args) if !args.is_empty() && constructor.pointer_constructor(bindings) => {
                Some(args[0].clone())
            },
            _ => None,
        }
    }

    pub fn reference_or_pointer_element<'a>(&'a self, bindings: &'a TypeBindings) -> Option<&'a Type> {
        match self.follow(bindings) {
            Type::Application(constructor, args) => {
                if constructor.reference_constructor(bindings).is_some() {
                    args.get(1)
                } else if constructor.pointer_constructor(bindings) {
                    args.first()
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Flatten every effect reachable from `effects` into `found`
    pub(crate) fn flatten_effects_into(
        effects: &[Type], found: &mut Vec<Type>, bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) {
        flatten_row_into(RowKind::Effects, effects, found, bindings, more_bindings);
    }

    /// Flatten, follow, sort, and deduplicate `effects` by exact equality rather than unification
    pub(crate) fn canonicalize_effects(
        effects: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) -> Vec<Type> {
        canonicalize_row(RowKind::Effects, effects, bindings, more_bindings)
    }

    /// Zonk each entry in place
    pub(crate) fn follow_effects(effects: &mut [Type], bindings: &TypeBindings, more_bindings: &TypeBindings) {
        follow_row(effects, bindings, more_bindings);
    }

    /// Sort and deduplicate the given effect set. Entries must already be zonked.
    pub(crate) fn sort_and_dedup_effects(effects: &mut Vec<Type>) {
        sort_and_dedup_row(RowKind::Effects, effects);
    }

    /// Construct a canonicalized effect row by following & deduplicating entries.
    pub(crate) fn effects(list: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings) -> Type {
        construct_row(RowKind::Effects, list, bindings, more_bindings)
    }

    /// The data type of the impl `name`, applied to the types of the impl's parameters if it has any
    pub(crate) fn impl_data(name: TopLevelName, parameter_types: Vec<Type>) -> Type {
        let impl_data = Type::Primitive(PrimitiveType::ImplData(name));
        if parameter_types.is_empty() {
            impl_data
        } else {
            Type::Application(Shared::new(impl_data), Shared::new(parameter_types))
        }
    }

    /// If this is a type application, return it with its argument at `index` replaced with `arg`
    pub(crate) fn with_arg(&self, index: usize, arg: Type) -> Option<Type> {
        let Type::Application(constructor, args) = self else { return None };
        let mut args = args.as_ref().clone();
        *args.get_mut(index)? = arg;
        Some(Type::Application(constructor.clone(), Shared::new(args)))
    }

    /// If this is the data type of an impl, return that impl's name, see [Type::impl_data]
    pub(crate) fn impl_data_name(&self, bindings: &TypeBindings) -> Option<TopLevelName> {
        match self.head_and_args(bindings) {
            (Type::Primitive(PrimitiveType::ImplData(name)), _) => Some(*name),
            _ => None,
        }
    }

    /// The effect constructor an effect-row entry refers to, ignoring its type arguments.
    pub(crate) fn effect_head(&self, bindings: &TypeBindings, more_bindings: &TypeBindings) -> Option<Origin> {
        match self.follow_two(bindings, more_bindings) {
            Type::UserDefined(origin) => Some(origin),
            Type::Application(constructor, _) => match constructor.follow_two(bindings, more_bindings) {
                Type::UserDefined(origin) => Some(origin),
                _ => None,
            },
            _ => None,
        }
    }

    /// If this is [Type::Effects], return the inner effects. Otherwise return an empty slice.
    pub(crate) fn effect_entries(&self) -> &[Type] {
        match self {
            Type::Effects(Some(effects)) => effects,
            _ => &[],
        }
    }

    /// An empty, closed effect row
    pub fn pure() -> Type {
        Type::Effects(None)
    }

    /// If this is a `Array n t`, return its element type `t`.
    pub fn array_element(&self, bindings: &TypeBindings) -> Option<Type> {
        match self.follow(bindings) {
            Type::Application(constructor, args) if args.len() == 2 => match constructor.follow(bindings) {
                Type::Primitive(PrimitiveType::Array) => Some(args[1].clone()),
                _ => None,
            },
            _ => None,
        }
    }
}

pub struct TypePrinter<'a, Db, Names> {
    typ: &'a Type,
    pub(super) bindings: &'a TypeBindings,
    /// Unbound variables in these sets render as their literal default (I32/F64) instead of `_`.
    integer_literal_vars: &'a FxHashSet<TypeVariableId>,
    float_literal_vars: &'a FxHashSet<TypeVariableId>,
    /// In error messages we omit the `[env]` block and instead write `=>` for functions
    /// with an environment. The typed AST dump keeps the env.
    hide_environments: bool,
    /// Which trailing arguments of a type constructor are omitted
    optional_args: OptionalArgs,
    /// Print each [Generic::Elided] with its id rather than as `_`, to tell them apart
    number_elided: bool,
    pub(super) names: &'a Names,
    db: &'a Db,
}

/// Which trailing arguments of a type constructor a [TypePrinter] omits. Only arguments whose
/// parameter defaults to `_`, such as a trait's impl argument, are ever omitted.
#[derive(Copy, Clone, PartialEq, Eq)]
enum OptionalArgs {
    /// Omit unbound type variables, which would also be omitted when writing the type
    Unbound,
    /// Also omit rigid [Generic::Elided] arguments, which can't be written. Used in error messages.
    Unknown,
    /// Also omit each trait's impl argument. Used in error messages comparing two types whose
    /// impl arguments aren't what distinguishes them.
    UnknownAndImpls,
}

impl<'a, Db, Names> TypePrinter<'a, Db, Names> {
    pub fn hiding_environments(mut self) -> Self {
        self.hide_environments = true;
        self
    }

    pub fn omitting_unknown_args(mut self) -> Self {
        self.optional_args = OptionalArgs::Unknown;
        self
    }

    pub fn omitting_unknown_and_impl_args(mut self) -> Self {
        self.optional_args = OptionalArgs::UnknownAndImpls;
        self
    }

    pub fn numbering_elided_generics(mut self) -> Self {
        self.number_elided = true;
        self
    }
}

impl<Db, Names> std::fmt::Display for TypePrinter<'_, Db, Names>
where
    Db: DbGet<GetItem>,
    Names: NameStore,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.fmt_type(self.typ, false, f)
    }
}

impl<Db, Names> TypePrinter<'_, Db, Names>
where
    Db: DbGet<GetItem>,
    Names: NameStore,
{
    pub(super) fn fmt_type(&self, typ: &Type, parenthesize: bool, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match typ {
            Type::Primitive(PrimitiveType::ImplData(name)) => {
                let (_, context) = GetItem(name.top_level_item).get(self.db);
                write!(f, "{}", context[name.local_name_id])
            },
            Type::Primitive(primitive_type) => write!(f, "{primitive_type}"),
            Type::UserDefined(origin) => self.fmt_type_origin(*origin, f),
            Type::Generic(Generic::Named(origin)) => self.fmt_type_origin(*origin, f),
            Type::Generic(Generic::Inferred(id)) => write!(f, "g{id}"),
            Type::Generic(generic @ Generic::Elided(_)) if self.number_elided => write!(f, "{generic}"),
            Type::Generic(Generic::Elided(_)) => write!(f, "_"),
            Type::Variable(id) => {
                if let Some(binding) = self.bindings.get(id) {
                    self.fmt_type(binding, parenthesize, f)
                } else if self.integer_literal_vars.contains(id) {
                    self.fmt_type(&Type::I32, parenthesize, f)
                } else if self.float_literal_vars.contains(id) {
                    self.fmt_type(&Type::F64, parenthesize, f)
                } else {
                    write!(f, "_")
                }
            },
            Type::Function(function) => try_parenthesize(parenthesize, f, |f| {
                write!(f, "fn")?;
                for parameter in &function.parameters {
                    write!(f, " ")?;
                    if parameter.is_implicit {
                        write!(f, "{{")?;
                        self.fmt_type(&parameter.typ, false, f)?;
                        write!(f, "}}")?;
                    } else {
                        self.fmt_type(&parameter.typ, true, f)?;
                    }
                }

                let has_env = *function.environment.follow(self.bindings) != Type::NO_CLOSURE_ENV;

                if self.hide_environments {
                    write!(f, "{}", if has_env { " => " } else { " -> " })?;
                } else {
                    if has_env {
                        write!(f, " [")?;
                        self.fmt_type(&function.environment, false, f)?;
                        write!(f, "]")?;
                    }
                    write!(f, " -> ")?;
                }
                let returns_fn = matches!(function.return_type.follow(self.bindings), Type::Function(_));
                self.fmt_type(&function.return_type, returns_fn, f)?;
                self.fmt_effects_suffix(&function.effects, f)
            }),
            Type::Application(constructor, args) => {
                let constructor = constructor.follow(self.bindings);
                let item = match constructor {
                    Type::UserDefined(Origin::TopLevelDefinition(name)) => {
                        Some(GetItem(name.top_level_item).get(self.db))
                    },
                    _ => None,
                };
                let definition = match item.as_ref().map(|(item, _)| &item.kind) {
                    Some(cst::TopLevelItemKind::TypeDefinition(definition)) => Some(definition),
                    _ => None,
                };
                let shown = args.len() - self.omittable_trailing_args(definition, args);

                // A type whose arguments are all omitted is written like a type without any
                try_parenthesize(parenthesize && shown != 0, f, |f| {
                    // Hack: If the constructor formats to `,` then print it infix
                    let is_pair = item.as_ref().is_some_and(|(item, context)| match item.kind.name() {
                        cst::ItemName::Single(name) => context[name].as_str() == ",",
                        _ => unreachable!(),
                    });

                    if is_pair && args.len() == 2 {
                        self.fmt_type(&args[0], true, f)?;
                        write!(f, ", ")?;
                        self.fmt_type(&args[1], true, f)
                    } else {
                        let is_reference = args.len() == 2
                            && matches!(
                                constructor.follow(self.bindings),
                                Type::Primitive(PrimitiveType::Reference(_))
                            );
                        let generics = definition.map_or(&[][..], |definition| definition.generics.as_slice());

                        self.fmt_type(constructor, true, f)?;
                        for (i, arg) in args[..shown].iter().enumerate() {
                            let place_param =
                                generics.get(i).is_some_and(|param| param.kind == Some(KindAnnotation::Place));
                            if (is_reference && i == 0) || place_param || arg.follow(self.bindings).is_place_row() {
                                self.fmt_place_arg(arg, is_reference, f)?;
                            } else {
                                write!(f, " ")?;
                                self.fmt_type(arg, true, f)?;
                            }
                        }
                        Ok(())
                    }
                })
            },
            Type::Forall(generics, typ) => try_parenthesize(parenthesize, f, |f| {
                write!(f, "forall")?;
                for generic in generics.iter() {
                    write!(f, " {generic}")?;
                }
                write!(f, ". ")?;
                self.fmt_type(typ, parenthesize, f)
            }),
            Type::Tuple(elements) => try_parenthesize(parenthesize, f, |f| {
                for (i, element) in elements.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    // TODO: Improve parenthesization here
                    self.fmt_type(element, true, f)?;
                }
                Ok(())
            }),
            Type::U32(n) => write!(f, "{n}"),
            Type::Effects(effects) => self.fmt_effects(effects, parenthesize, f),
            Type::Place(atom) => write!(f, "{}", atom.display_name(self.names)),
            Type::Places(_) => {
                let entries = self.printed_places(typ).unwrap_or_default();
                if entries.is_empty() { write!(f, "()") } else { self.fmt_place_entries(&entries, f) }
            },
            Type::PlaceProjection(base, segments) => {
                self.fmt_type(base, false, f)?;
                write!(f, "{}", cst::Segments(segments))
            },
        }
    }

    /// The number of trailing arguments of a type defined by `definition` to omit when printing,
    /// see [OptionalArgs]. Only an argument whose parameter defaults to `_` may be omitted.
    fn omittable_trailing_args(&self, definition: Option<&cst::TypeDefinition>, args: &[Type]) -> usize {
        let Some(definition) = definition else { return 0 };
        let omit_impls = self.optional_args == OptionalArgs::UnknownAndImpls;
        let impl_index =
            definition.impl_generic_index().filter(|_| omit_impls && args.len() == definition.generics.len());

        let omittable = |i: usize| {
            definition.generics.get(i).is_some_and(|param| param.defaults_to_hole())
                && (Some(i) == impl_index || self.is_unknown_arg(&args[i]))
        };
        (0..args.len()).rev().take_while(|i| omittable(*i)).count()
    }

    /// True if `arg` is an omittable argument which is not known, see [OptionalArgs]
    fn is_unknown_arg(&self, arg: &Type) -> bool {
        match arg.follow(self.bindings) {
            Type::Variable(_) => true,
            Type::Generic(Generic::Elided(_)) => self.optional_args != OptionalArgs::Unbound,
            _ => false,
        }
    }

    fn fmt_effects(
        &self, effects: &Option<Shared<Vec<Type>>>, parenthesize: bool, f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        let effects = self.canonicalize(effects);
        match effects.as_slice() {
            [] => write!(f, "pure"),
            [effect] => self.fmt_type(effect, parenthesize, f),
            _ => try_parenthesize(parenthesize, f, |f| self.fmt_non_empty_effects(&effects, f)),
        }
    }

    /// Canonicalize a row for printing, showing any number of open ends as one `_`
    fn canonicalize(&self, effects: &Option<Shared<Vec<Type>>>) -> Vec<Type> {
        let effects = effects.as_deref().map_or(&[][..], Vec::as_slice);
        let mut effects = Type::canonicalize_effects(effects, self.bindings, &Default::default());
        // Open ends are sorted last
        if let Some(first_open) = effects.iter().position(is_open) {
            effects.truncate(first_open + 1);
        }
        effects
    }

    /// Print each entry of an already-canonicalized, non-empty row
    fn fmt_non_empty_effects(&self, effects: &[Type], f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for (i, effect) in effects.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            self.fmt_type(effect, false, f)?;
        }
        Ok(())
    }

    fn fmt_effects_suffix(&self, effects: &Type, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match effects.follow(self.bindings) {
            Type::Variable(_) => Ok(()),
            Type::Effects(effects) => {
                let effects = self.canonicalize(effects);
                if effects.is_empty() {
                    write!(f, " is pure")
                } else {
                    write!(f, " can ")?;
                    self.fmt_non_empty_effects(&effects, f)
                }
            },
            other => {
                write!(f, " can ")?;
                self.fmt_type(other, true, f)
            },
        }
    }

    fn fmt_type_origin(&self, origin: Origin, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match origin {
            Origin::TopLevelDefinition(id)
            | Origin::TraitMember(crate::name_resolution::TraitMember { name: id, .. })
            | Origin::EffectOperation { name: id, .. } => {
                let (item, context) = GetItem(id.top_level_item).get(self.db);
                match &item.kind {
                    // A variant type
                    cst::TopLevelItemKind::TypeDefinition(def) if id.local_name_id != def.name => {
                        write!(f, "{}.{}", context[def.name], context[id.local_name_id])
                    },
                    _ => {
                        if let cst::ItemName::Single(name) = item.kind.name() {
                            write!(f, "{}", context[name])
                        } else {
                            unreachable!()
                        }
                    },
                }
            },
            Origin::Local(name) => {
                if let Some(name) = self.names.try_get_name(name) {
                    write!(f, "{name}")
                } else {
                    write!(f, "#name-not-in-context")
                }
            },
            Origin::TypeResolution => write!(f, "TypeResolution"),
            Origin::Builtin(builtin) => write!(f, "{builtin}"),
        }
    }
}

fn try_parenthesize(
    parenthesize: bool, f: &mut std::fmt::Formatter, func: impl FnOnce(&mut std::fmt::Formatter) -> std::fmt::Result,
) -> std::fmt::Result {
    if parenthesize {
        write!(f, "(")?;
    }
    func(f)?;
    if parenthesize {
        write!(f, ")")?;
    }
    Ok(())
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::Error => write!(f, "(error)"),
            PrimitiveType::Unit => write!(f, "Unit"),
            PrimitiveType::Bool => write!(f, "Bool"),
            PrimitiveType::Pointer => write!(f, "Ptr"),
            PrimitiveType::Int(kind) => write!(f, "{kind}"),
            PrimitiveType::Float(kind) => write!(f, "{kind}"),
            PrimitiveType::Char => write!(f, "Char"),
            PrimitiveType::Never => write!(f, "Never"),
            PrimitiveType::Reference(kind) => write!(f, "{kind}"),
            PrimitiveType::Array => write!(f, "Array"),
            PrimitiveType::NoClosureEnv => write!(f, "{NO_CLOSURE_ENV_STRING}"),
            PrimitiveType::ResumeEnv => write!(f, "ResumeEnv"),
            PrimitiveType::ImplData(_) => write!(f, "(impl)"),
        }
    }
}

impl std::fmt::Display for ReferenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceKind::Ref => write!(f, "ref"),
            ReferenceKind::Mut => write!(f, "mut"),
            ReferenceKind::Imm => write!(f, "imm"),
            ReferenceKind::Uniq => write!(f, "uniq"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TypeVariableId(pub u32);

impl std::fmt::Display for TypeVariableId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
