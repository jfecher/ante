use std::{
    borrow::Cow,
    num::NonZeroUsize,
    sync::{Arc, LazyLock},
};

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
        cst::{self, KindAnnotation, ReferenceKind},
        get_item::IMPLICIT_EFFECT_NAME,
        ids::{NameId, NameStore, TopLevelName},
    },
    type_inference::{TypeChecker, generics::Generic, kinds::Kind},
};

/// Tracks the kind of each local type variable encountered while lowering a
/// `cst::Type` into a `Type`
pub(crate) type LocalKinds = BTreeMap<NameId, Kind>;

pub(crate) fn kind_from_annotation(kind: KindAnnotation) -> Kind {
    match kind {
        KindAnnotation::Type => Kind::Type,
        KindAnnotation::U32 => Kind::U32,
        KindAnnotation::Lifetime => Kind::Lifetime,
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
    Function(Arc<FunctionType>),
    Application(Arc<Type>, Arc<Vec<Type>>),
    UserDefined(Origin),

    /// A polytype such as `forall a. fn a -> a`.
    /// During unification the ordering of the type variables matters.
    /// `forall a b. (a, b)` will not unify with `forall b a. (a, b)`
    Forall(Arc<Vec<Generic>>, Arc<Type>),

    /// This is an internal type only created when handling closure environments.
    /// Most tuple types in source code refer to the `,` type defined in the prelude. While they
    /// could use this type instead, using a UserDefinedType for them lets us reuse the existing
    /// mechanisms to automatically define their constructor and retrieve their fields.
    Tuple(Arc<Vec<Type>>),

    /// A type-level U32 constant, used as the length parameter of [PrimitiveType::Array].
    /// Has [Kind::U32].
    U32(u32),

    /// An effects row: sorted & deduplicated effects, plus an optional tail to extend the row.
    Effects(Arc<Vec<Type>>, Option<Arc<Type>>),
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
    fn follow_all_opt(&self, bindings: &TypeBindings, more_bindings: &TypeBindings) -> Option<Type> {
        match self {
            Type::Primitive(_) | Type::Generic(Generic::Named(_)) | Type::UserDefined(_) | Type::U32(_) => None,
            Type::Generic(Generic::Inferred(id)) | Type::Variable(id) => {
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

                Some(Type::Function(Arc::new(FunctionType {
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
                let constructor = new_constructor.map(Arc::new).unwrap_or_else(|| constructor.clone());
                let args = new_args.map(Arc::new).unwrap_or_else(|| args.clone());
                Some(Type::Application(constructor, args))
            },
            Type::Forall(generics, typ) => {
                for generic in generics.iter() {
                    if let Generic::Inferred(id) = generic {
                        assert!(!bindings.contains_key(id));
                        assert!(!more_bindings.contains_key(id));
                    }
                }

                let typ = typ.follow_all_opt(bindings, more_bindings)?;
                Some(Type::Forall(generics.clone(), Arc::new(typ)))
            },
            Type::Tuple(elements) => {
                let new_elements = Self::follow_all_each(elements, |t| t.follow_all_opt(bindings, more_bindings))?;
                Some(Type::Tuple(Arc::new(new_elements)))
            },
            Type::Effects(list, tail) => {
                let new_list = Self::follow_all_each(list, |t| t.follow_all_opt(bindings, more_bindings));
                let new_tail = tail.as_ref().and_then(|t| t.follow_all_opt(bindings, more_bindings));
                if new_list.is_none() && new_tail.is_none() {
                    return None;
                }
                let list = new_list.unwrap_or_else(|| (**list).clone());
                let tail = new_tail.or_else(|| tail.as_ref().map(|t| (**t).clone()));
                Some(Type::effects(list, tail))
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
            Type::Primitive(_) | Type::UserDefined(_) | Type::U32(_) => None,
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
                Some(Type::Function(Arc::new(FunctionType {
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
                let constructor = new_constructor.map(Arc::new).unwrap_or_else(|| constructor.clone());
                let args = new_args.map(Arc::new).unwrap_or_else(|| args.clone());
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
                Some(Type::Tuple(Arc::new(elements)))
            },
            Type::Effects(list, tail) => {
                let new_list =
                    Self::follow_all_each(list, |t| t.substitute_opt(bindings_to_substitute, bindings_in_scope));
                let new_tail = tail.as_ref().and_then(|t| t.substitute_opt(bindings_to_substitute, bindings_in_scope));
                if new_list.is_none() && new_tail.is_none() && !self_is_var {
                    return None;
                }
                let list = new_list.unwrap_or_else(|| (**list).clone());
                let tail = new_tail.or_else(|| tail.as_ref().map(|t| (**t).clone()));
                Some(Type::effects(list, tail))
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
            Type::Forall(_, typ) => Arc::unwrap_or_clone(typ),
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
        )
    }

    /// Converts an ast type to a generalized Type, with any free type variables
    /// replaced with a `Type::Generic(Generic::Inferred(id))` - although the type will
    /// will not be wrapped in a `forall`.
    pub(crate) fn from_cst_type_generalized(
        typ: &cst::Type, resolve: &crate::name_resolution::ResolutionResult, db: &DbHandle,
        insert_implicit_type_vars: bool, open_effects_by_default: bool,
    ) -> Type {
        let mut next_id = 0;
        let mut local_kinds = LocalKinds::default();
        let typ = Self::from_cst_type_with_kind(
            typ,
            Kind::Type,
            resolve,
            db,
            &mut next_id,
            &mut local_kinds,
            insert_implicit_type_vars,
            open_effects_by_default,
        );

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
    fn from_cst_type_with_kind(
        typ: &cst::Type, expected: Kind, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32,
        local_kinds: &mut LocalKinds, insert_implicit_type_vars: bool, open_effects_by_default: bool,
    ) -> Type {
        let mut visited = Vec::new();
        TypeConverter::new(resolve, db, next_id, local_kinds, insert_implicit_type_vars, open_effects_by_default, &mut visited)
            .convert_with_kind(typ, expected)
    }

    /// Returns a tuple of:
    /// - The converted type
    /// - The kind of the converted type
    ///
    /// Does not error if the returned type is not of kind [Kind::Type].
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_cst_type_helper(
        typ: &cst::Type, expected: Option<&Kind>, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32,
        local_kinds: &mut LocalKinds, insert_implicit_type_vars: bool, open_effects_by_default: bool,
    ) -> (Type, Kind) {
        let mut visited = Vec::new();
        TypeConverter::new(resolve, db, next_id, local_kinds, insert_implicit_type_vars, open_effects_by_default, &mut visited)
            .convert(typ, expected)
    }

    /// Convert an effects clause into an effect row [Type].
    pub(crate) fn from_cst_effects_clause(
        effects: Option<&[cst::Type]>, resolve: &ResolutionResult, db: &DbHandle, next_id: &mut u32,
        local_kinds: &mut LocalKinds, insert_implicit_type_vars: bool, open_effects_by_default: bool,
    ) -> Type {
        let mut visited = Vec::new();
        TypeConverter::new(resolve, db, next_id, local_kinds, insert_implicit_type_vars, open_effects_by_default, &mut visited)
            .convert_effects_clause(effects)
    }
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
}

impl<'a, 'b> TypeConverter<'a, 'b> {
    fn new(
        resolve: &'a ResolutionResult, db: &'a DbHandle<'b>, next_id: &'a mut u32, local_kinds: &'a mut LocalKinds,
        insert_implicit_type_vars: bool, open_effects_by_default: bool, visited: &'a mut Vec<TopLevelName>,
    ) -> Self {
        TypeConverter { resolve, db, next_id, local_kinds, insert_implicit_type_vars, open_effects_by_default, visited }
    }

    /// Convert `typ` and error if its [Kind] does not unify with `expected`.
    fn convert_with_kind(&mut self, typ: &cst::Type, expected: Kind) -> Type {
        let location = typ.location.clone();
        let (typ, kind) = self.convert(typ, Some(&expected));
        if !expected.unifies(&kind) {
            self.db.accumulate(Diagnostic::ExpectedKind { actual: kind, expected, location });
        }
        typ
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
                let (typ, kind) =
                    Type::convert_origin_to_type(origin, self.db, &typ.location, self.local_kinds, Type::UserDefined);

                // Expand a type alias if necessary
                if kind == Kind::Type
                    && let Type::UserDefined(Origin::TopLevelDefinition(name)) = typ
                    && let Some(expanded) = self.expand_alias(name, &[])
                {
                    (expanded, Kind::Type)
                } else {
                    (typ, kind)
                }
            },
            crate::parser::cst::TypeKind::Variable(name) | crate::parser::cst::TypeKind::Lifetime(name) => {
                let origin = self.resolve.name_origins.get(name).copied();
                if let (Some(expected), Some(Origin::Local(name_id))) = (expected, origin) {
                    self.local_kinds.entry(name_id).or_insert_with(|| expected.clone());
                }
                Type::convert_origin_to_type(origin, self.db, &typ.location, self.local_kinds, |origin| {
                    Type::Generic(Generic::Named(origin))
                })
            },
            crate::parser::cst::TypeKind::Function(function) => {
                let parameters = mapvec(&function.parameters, |param| {
                    let typ = self.convert_with_kind(&param.typ, Kind::Type);
                    ParameterType::new(typ, param.is_implicit)
                });
                let environment = match function.environment.as_ref() {
                    Some(environment) => self.convert_with_kind(environment, Kind::Type),
                    None => Type::NO_CLOSURE_ENV,
                };
                let return_type = self.convert_with_kind(&function.return_type, Kind::Type);
                let effects = self.convert_effects_clause(function.effects.as_deref());

                let f = Type::Function(Arc::new(FunctionType { parameters, environment, return_type, effects }));
                (f, Kind::Type)
            },
            crate::parser::cst::TypeKind::Error => (Type::ERROR, Kind::Error),
            crate::parser::cst::TypeKind::Unit => (Type::UNIT, Kind::Type),
            crate::parser::cst::TypeKind::Application(f, args) => {
                let (f, f_kind) = self.convert(f, None);

                if !f_kind.accepts_n_arguments(args.len()) {
                    let expected = f_kind.required_argument_count();
                    let location = typ.location.clone();
                    self.db.accumulate(Diagnostic::FunctionArgCountMismatch { actual: args.len(), expected, location });
                    return (Type::ERROR, Kind::Type);
                }

                let result_kind = f_kind.result_kind();

                let converted_args = mapvec(args.iter().enumerate(), |(i, arg)| {
                    let expected_kind = f_kind.get_nth_parameter_kind(i);
                    self.convert_with_kind(arg, expected_kind)
                });

                assert!(!converted_args.is_empty());

                // Expand a generic type alias if necessary
                if let Type::UserDefined(Origin::TopLevelDefinition(name)) = &f
                    && let Some(expanded) = self.expand_alias(*name, &converted_args)
                {
                    return (expanded, Kind::Type);
                }

                let typ = Type::Application(Arc::new(f), Arc::new(converted_args));
                (typ, result_kind)
            },
            crate::parser::cst::TypeKind::Reference(kind) => (
                Type::Primitive(PrimitiveType::Reference(*kind)),
                Kind::TypeConstructorComplex { params: vec![Kind::Lifetime, Kind::Type], result: Box::new(Kind::Type) },
            ),
            crate::parser::cst::TypeKind::NoClosureEnv => (Type::NO_CLOSURE_ENV, Kind::Type),
            crate::parser::cst::TypeKind::Pointer => (
                Type::POINTER,
                Kind::TypeConstructorSimple { arity: NonZeroUsize::new(1).unwrap(), result: Box::new(Kind::Type) },
            ),
            crate::parser::cst::TypeKind::Tuple(elements) => {
                let elements = mapvec(elements, |t| self.convert_with_kind(t, Kind::Type));
                (Type::Tuple(Arc::new(elements)), Kind::Type)
            },
            crate::parser::cst::TypeKind::Hole if self.insert_implicit_type_vars => {
                let typ = Type::Variable(TypeVariableId(*self.next_id));
                *self.next_id += 1;
                (typ, Kind::Type)
            },
            crate::parser::cst::TypeKind::Hole => {
                self.db.accumulate(Diagnostic::HoleCantBeUsed { location: typ.location.clone() });
                (Type::ERROR, Kind::Error)
            },
            // TODO: There is a separate check in type definition bodies to reject these.
            // Rework it to be more similar to the HoleCantBeUsed above.
            crate::parser::cst::TypeKind::ImplicitLifetime => {
                let typ = Type::Variable(TypeVariableId(*self.next_id));
                *self.next_id += 1;
                (typ, Kind::Lifetime)
            },
            crate::parser::cst::TypeKind::IntegerConstant(v) => (Type::U32(*v), Kind::U32),
            crate::parser::cst::TypeKind::Forall(generics, body) => {
                for param in generics {
                    let kind = param.kind.map(kind_from_annotation).unwrap_or(Kind::Type);
                    self.local_kinds.insert(param.name, kind);
                }
                self.convert(body, expected)
            },
        }
    }

    /// Convert an effects clause into an effect row; `None` is open or closed per `self.open_effects_by_default`.
    fn convert_effects_clause(&mut self, effects: Option<&[cst::Type]>) -> Type {
        match effects {
            None if self.open_effects_by_default => {
                let tail = Type::Variable(TypeVariableId(*self.next_id));
                *self.next_id += 1;
                Type::effects(Vec::new(), Some(tail))
            },
            None => Type::pure(),
            Some(list) => {
                let mut tail = None;
                let mut concrete = Vec::new();
                for entry in list {
                    if matches!(entry.kind, crate::parser::cst::TypeKind::Variable(_)) {
                        if tail.is_some() {
                            self.db.accumulate(Diagnostic::MultipleEffectRowVariables {
                                location: entry.location.clone(),
                            });
                        }
                        tail = Some(self.convert_with_kind(entry, Kind::Effect));
                    } else {
                        concrete.push(self.convert_with_kind(entry, Kind::Effect));
                    }
                }
                Type::effects(concrete, tail)
            },
        }
    }

    /// If `name` refers to a type alias, return its body type with `args` substituted
    /// for the alias's generic parameters, with any aliases referenced within the body expanded
    /// transparently as well. Returns `None` if `name` is not a type alias.
    ///
    /// Emits [Diagnostic::RecursiveTypeAlias] if the type alias is infinitely recursive.
    fn expand_alias(&mut self, name: TopLevelName, args: &[Type]) -> Option<Type> {
        let (item, ctx) = GetItem(name.top_level_item).get(self.db);
        let cst::TopLevelItemKind::TypeDefinition(definition) = &item.kind else {
            return None;
        };
        let cst::TypeDefinitionBody::Alias(body) = &definition.body else {
            return None;
        };

        if self.visited.contains(&name) {
            let typ = ctx[definition.name].to_string();
            let location = ctx.name_location(definition.name).clone();
            self.db.accumulate(Diagnostic::RecursiveTypeAlias { typ, location });
            return Some(Type::ERROR);
        }
        self.visited.push(name);

        let resolve = Resolve(name.top_level_item).get(self.db);
        let mut local_kinds = TypeChecker::local_kinds_from_generics(&definition.generics);
        let (body_type, _) =
            TypeConverter::new(&resolve, self.db, self.next_id, &mut local_kinds, false, false, self.visited)
                .convert(body, Some(&Kind::Type));

        let result = if definition.generics.is_empty() {
            body_type
        } else {
            let generics_and_args = definition.generics.iter().zip(args);
            let make_generic = |name| Generic::Named(Origin::Local(name));
            let substitutions = generics_and_args.map(|(param, arg)| (make_generic(param.name), arg.clone()));
            body_type.substitute(&substitutions.collect(), &TypeBindings::default())
        };

        self.visited.pop();
        Some(result)
    }
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
                Builtin::Ptr => (
                    Type::POINTER,
                    Kind::TypeConstructorSimple { arity: NonZeroUsize::new(1).unwrap(), result: Box::new(Kind::Type) },
                ),
                Builtin::Array => (
                    Type::ARRAY,
                    Kind::TypeConstructorComplex { params: vec![Kind::U32, Kind::Type], result: Box::new(Kind::Type) },
                ),
                Builtin::Never => (Type::NEVER, Kind::Type),
                Builtin::Intrinsic => (Type::ERROR, Kind::Error),
            },
            Some(origin @ Origin::TopLevelDefinition(type_name)) => {
                let (item, ctx) = GetItem(type_name.top_level_item).get(db);
                match &item.kind {
                    cst::TopLevelItemKind::TypeDefinition(definition) => {
                        let kind = crate::definition_collection::kind_of_type_definition(definition);
                        (make_type(origin), kind)
                    },
                    _ => {
                        let name = item.kind.name().to_string(ctx.as_ref());
                        db.accumulate(Diagnostic::NotAType { name, location: location.clone() });
                        (Type::ERROR, Kind::Type)
                    },
                }
            },
            Some(origin @ Origin::Local(name)) => {
                let kind = local_kinds.get(&name).cloned().unwrap_or(Kind::Type);
                (make_type(origin), kind)
            },
            Some(origin) => (make_type(origin), Kind::Type),
            // Assume name resolution has already issued an error for this case
            None => (Type::ERROR, Kind::Error),
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
            Type::Forall(Arc::new(free_vars), Arc::new(typ))
        }
    }

    /// Return the list of unbound type variables or generics within this type
    pub fn free_vars(&self, bindings: &TypeBindings) -> Vec<Generic> {
        fn free_vars_helper(typ: &Type, bindings: &TypeBindings, free_vars: &mut Vec<Generic>) {
            match typ.follow(bindings) {
                Type::Primitive(_) | Type::UserDefined(_) => (),
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
                Type::Function(function) => {
                    for parameter in &function.parameters {
                        free_vars_helper(&parameter.typ, bindings, free_vars);
                    }
                    free_vars_helper(&function.environment, bindings, free_vars);
                    free_vars_helper(&function.return_type, bindings, free_vars);
                    free_vars_helper(&function.effects, bindings, free_vars);
                },
                Type::Application(constructor, args) => {
                    free_vars_helper(constructor, bindings, free_vars);
                    for arg in args.iter() {
                        free_vars_helper(arg, bindings, free_vars);
                    }
                },
                Type::Forall(generics, typ) => {
                    free_vars_helper(typ, bindings, free_vars);

                    // Remove any free variable contained within `generics`.
                    // This is technically incorrect in the case any of these variables appeared in
                    // `free_vars` before the previous call to `free_vars_helper(_, typ, _)`, but
                    // we expect scoping rules to prevent these cases.
                    free_vars.retain(|generic| !generics.contains(generic));
                },
                Type::Tuple(elements) => {
                    for element in elements.iter() {
                        free_vars_helper(element, bindings, free_vars);
                    }
                },
                Type::U32(_) => (),
                Type::Effects(list, tail) => {
                    for effect in list.iter() {
                        free_vars_helper(effect, bindings, free_vars);
                    }
                    if let Some(tail) = tail {
                        free_vars_helper(tail, bindings, free_vars);
                    }
                },
            }
        }

        let mut free_vars = Vec::new();
        free_vars_helper(self, bindings, &mut free_vars);
        free_vars
    }

    /// Return the list of unbound type variables within this type.
    /// Unlike [Self::free_vars], this excludes [Type::Generic]s within the type and it returns a set.
    pub fn free_unification_vars(&self, bindings: &TypeBindings) -> FxHashSet<TypeVariableId> {
        self.free_vars(bindings).into_iter().filter_map(Generic::as_inferred).collect::<FxHashSet<_>>()
    }

    /// Counts every occurrence of `target` within this type, unlike [Self::free_unification_vars] which dedups into a set.
    pub fn count_unification_var_occurrences(&self, target: TypeVariableId, bindings: &TypeBindings) -> usize {
        fn helper(typ: &Type, target: TypeVariableId, bindings: &TypeBindings, count: &mut usize) {
            match typ.follow(bindings) {
                Type::Primitive(_) | Type::UserDefined(_) | Type::Generic(_) | Type::U32(_) => (),
                Type::Variable(id) => {
                    if *id == target {
                        *count += 1;
                    }
                },
                Type::Function(function) => {
                    for parameter in &function.parameters {
                        helper(&parameter.typ, target, bindings, count);
                    }
                    helper(&function.environment, target, bindings, count);
                    helper(&function.return_type, target, bindings, count);
                    helper(&function.effects, target, bindings, count);
                },
                Type::Application(constructor, args) => {
                    helper(constructor, target, bindings, count);
                    for arg in args.iter() {
                        helper(arg, target, bindings, count);
                    }
                },
                Type::Forall(_, typ) => helper(typ, target, bindings, count),
                Type::Tuple(elements) => {
                    for element in elements.iter() {
                        helper(element, target, bindings, count);
                    }
                },
                Type::Effects(list, tail) => {
                    for effect in list.iter() {
                        helper(effect, target, bindings, count);
                    }
                    if let Some(tail) = tail {
                        helper(tail, target, bindings, count);
                    }
                },
            }
        }
        let mut count = 0;
        helper(self, target, bindings, &mut count);
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
    pub fn as_application(&self) -> Option<(&Arc<Type>, &Arc<Vec<Type>>)> {
        match self {
            Type::Application(constructor, args) => Some((constructor, args)),
            _ => None,
        }
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

    /// Construct a canonicalized effect row by following the tail and deduplicating entries.
    ///
    /// Entries sort by their effect head only, stably: a full-type order would reorder
    /// same-head entries under substitution (`Send a, Send b` vs their instantiations),
    /// silently changing the evidence layout.
    pub(crate) fn effects(mut list: Vec<Type>, mut tail: Option<Type>) -> Type {
        while let Some(Type::Effects(inner_list, inner_tail)) = tail {
            list.extend(inner_list.iter().cloned());
            tail = inner_tail.as_ref().map(|t| (**t).clone());
        }
        list.sort_by_key(|effect| effect.effect_head().copied());
        let mut deduped = Vec::with_capacity(list.len());
        for effect in list {
            if !deduped.contains(&effect) {
                deduped.push(effect);
            }
        }
        Type::Effects(Arc::new(deduped), tail.map(Arc::new))
    }

    /// The effect constructor an effect-row entry refers to, ignoring its type arguments.
    pub(crate) fn effect_head(&self) -> Option<&Origin> {
        self.as_user_defined().or_else(|| self.as_application()?.0.as_user_defined())
    }

    /// An empty, closed effect row
    pub fn pure() -> Type {
        Type::Effects(Arc::new(Vec::new()), None)
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
    bindings: &'a TypeBindings,
    /// Unbound variables in these sets render as their literal default (I32/F64) instead of `_`.
    integer_literal_vars: &'a FxHashSet<TypeVariableId>,
    float_literal_vars: &'a FxHashSet<TypeVariableId>,
    /// In error messages we omit the `[env]` block and instead write `=>` for functions
    /// with an environment. The typed AST dump keeps the env.
    hide_environments: bool,
    names: &'a Names,
    db: &'a Db,
}

impl<'a, Db, Names> TypePrinter<'a, Db, Names> {
    pub fn hiding_environments(mut self) -> Self {
        self.hide_environments = true;
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
    fn fmt_type(&self, typ: &Type, parenthesize: bool, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match typ {
            Type::Primitive(primitive_type) => write!(f, "{primitive_type}"),
            Type::UserDefined(origin) => self.fmt_type_origin(*origin, f),
            Type::Generic(Generic::Named(origin)) => self.fmt_type_origin(*origin, f),
            Type::Generic(Generic::Inferred(id)) => write!(f, "g{id}"),
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
                self.fmt_type(&function.return_type, false, f)?;
                self.fmt_effects_suffix(&function.effects, f)
            }),
            Type::Application(constructor, args) => try_parenthesize(parenthesize, f, |f| {
                // Hack: If the constructor formats to `,` then print it infix
                let constructor = constructor.follow(self.bindings);
                let is_pair = if let Type::UserDefined(Origin::TopLevelDefinition(name)) = constructor {
                    let (item, context) = GetItem(name.top_level_item).get(self.db);
                    if let cst::ItemName::Single(name) = item.kind.name() {
                        context[name].as_str() == ","
                    } else {
                        unreachable!()
                    }
                } else {
                    false
                };

                if is_pair && args.len() == 2 {
                    self.fmt_type(&args[0], true, f)?;
                    write!(f, ", ")?;
                    self.fmt_type(&args[1], true, f)
                } else {
                    let skip_implicit_lifetime = args.len() == 2
                        && matches!(constructor.follow(self.bindings), Type::Primitive(PrimitiveType::Reference(_)))
                        && matches!(args[0].follow(self.bindings), Type::Variable(_));

                    self.fmt_type(constructor, true, f)?;
                    let start = if skip_implicit_lifetime { 1 } else { 0 };
                    for arg in args[start..].iter() {
                        write!(f, " ")?;
                        self.fmt_type(arg, true, f)?;
                    }
                    Ok(())
                }
            }),
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
            Type::Effects(list, tail) => self.fmt_effect_list(list, tail, f),
        }
    }

    fn fmt_effect_list(
        &self, list: &[Type], tail: &Option<Arc<Type>>, f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        let (full_list, final_tail) = self.flatten_effect_row(list, tail);
        self.fmt_flat_effect_list(&full_list, &final_tail, f)
    }

    fn flatten_effect_row(&self, list: &[Type], tail: &Option<Arc<Type>>) -> (Vec<Type>, Option<Type>) {
        let mut full_list = list.to_vec();
        let final_tail = tail.as_deref().and_then(|tail| self.flatten_effects_tail(tail.clone(), &mut full_list));
        full_list.sort();
        full_list.dedup();
        (full_list, final_tail)
    }

    fn fmt_flat_effect_list(
        &self, list: &[Type], tail: &Option<Type>, f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        for (i, effect) in list.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            self.fmt_type(effect, true, f)?;
        }
        if let Some(tail) = tail {
            // An open, unconstrained row tail adds no information for the reader, so it's omitted.
            if !self.is_omittable_effect_tail(tail) {
                if !list.is_empty() {
                    write!(f, ", ")?;
                }
                self.fmt_type(tail, true, f)?;
            }
        }
        Ok(())
    }

    fn is_omittable_effect_tail(&self, tail: &Type) -> bool {
        match tail {
            Type::Variable(_) => true,
            Type::Generic(Generic::Named(Origin::Local(name_id))) => {
                self.names.try_get_name(*name_id).is_some_and(|n| n.as_str() == IMPLICIT_EFFECT_NAME)
            },
            _ => false,
        }
    }

    fn flatten_effects_tail(&self, mut tail: Type, list: &mut Vec<Type>) -> Option<Type> {
        loop {
            match tail.follow(self.bindings).clone() {
                Type::Effects(inner_list, inner_tail) => {
                    list.extend(inner_list.iter().cloned());
                    match inner_tail {
                        Some(next) => tail = (*next).clone(),
                        None => return None,
                    }
                },
                other => return Some(other),
            }
        }
    }

    fn fmt_effects_suffix(&self, effects: &Type, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match effects.follow(self.bindings) {
            Type::Variable(_) => Ok(()),
            Type::Effects(list, tail) => {
                let (full_list, final_tail) = self.flatten_effect_row(list, tail);
                match &final_tail {
                    Some(Type::Variable(_)) if full_list.is_empty() => Ok(()),
                    None if full_list.is_empty() => write!(f, " pure"),
                    _ => {
                        write!(f, " can ")?;
                        self.fmt_flat_effect_list(&full_list, &final_tail, f)
                    },
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
            Origin::TopLevelDefinition(id) => {
                let (item, context) = GetItem(id.top_level_item).get(self.db);
                if let cst::ItemName::Single(name) = item.kind.name() {
                    write!(f, "{}", context[name])
                } else {
                    unreachable!()
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
