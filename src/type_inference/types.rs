use std::{borrow::Cow, num::NonZeroUsize, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::{DbHandle, GetItem},
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    name_resolution::{Origin, builtin::Builtin},
    parser::{
        cst::{self, ReferenceKind},
        ids::NameStore,
    },
    type_inference::{generics::Generic, kinds::Kind},
};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FunctionType {
    pub parameters: Vec<ParameterType>,

    /// Closures and functions are unified by all having an environment type.
    /// Free functions will have an environment of [Type::NO_CLOSURE_ENV] while closures will
    /// have other environment types and will be subject to closure conversion.
    pub environment: Type,

    pub return_type: Type,
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
    pub const U32: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::U32));
    pub const U64: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::U64));
    pub const USZ: Type = Type::Primitive(PrimitiveType::Int(IntegerKind::Usz));

    pub const F32: Type = Type::Primitive(PrimitiveType::Float(FloatKind::F32));
    pub const F64: Type = Type::Primitive(PrimitiveType::Float(FloatKind::F64));

    pub const REF: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Ref));
    pub const MUT: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Mut));
    pub const IMM: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Imm));
    pub const UNIQ: Type = Type::Primitive(PrimitiveType::Reference(ReferenceKind::Uniq));

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
            crate::lexer::token::IntegerKind::U32 => Type::U32,
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
        TypePrinter { typ: self, bindings, names, db }
    }

    /// Returns true if this is any of the primitive integer types I8, I16, .., Usz, etc.
    pub(crate) fn is_integer(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Int(_)))
    }

    /// Follow all of this type's type variable bindings so that we only return
    /// `Type::Variable` if the type variable is unbound. Note that this may still return
    /// a composite type such as `Type::Application` with bound type variables within.
    pub fn follow<'a>(mut self: &'a Self, bindings: &'a TypeBindings) -> &'a Type {
        // Arbitrary upper limit
        for _ in 0..1000 {
            match self {
                typ @ Type::Variable(id) => match bindings.get(&id) {
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
                    if let Some(binding) = one.get(&id) {
                        self = binding;
                    } else if let Some(binding) = two.get(&id) {
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
        match self {
            Type::Primitive(_) | Type::Generic(Generic::Named(_)) => self.clone(),
            Type::Generic(Generic::Inferred(id)) => {
                if let Some(binding) = bindings.get(id) {
                    binding.follow_all_two(bindings, more_bindings)
                } else if let Some(binding) = more_bindings.get(id) {
                    binding.follow_all_two(bindings, more_bindings)
                } else {
                    Type::Generic(Generic::Inferred(*id))
                }
            },
            Type::Variable(id) => {
                if let Some(binding) = bindings.get(id) {
                    binding.follow_all_two(bindings, more_bindings)
                } else if let Some(binding) = more_bindings.get(id) {
                    binding.follow_all_two(bindings, more_bindings)
                } else {
                    Type::Variable(*id)
                }
            },
            Type::Function(function) => {
                let parameters = mapvec(function.parameters.iter(), |param| {
                    let typ = param.typ.follow_all_two(bindings, more_bindings);
                    ParameterType::new(typ, param.is_implicit)
                });

                Type::Function(Arc::new(FunctionType {
                    parameters,
                    environment: function.environment.follow_all_two(bindings, more_bindings),
                    return_type: function.return_type.follow_all_two(bindings, more_bindings),
                }))
            },
            Type::Application(constructor, args) => {
                let constructor = Arc::new(constructor.follow_all_two(bindings, more_bindings));
                let args = Arc::new(mapvec(args.iter(), |arg| arg.follow_all_two(bindings, more_bindings)));
                Type::Application(constructor, args)
            },
            Type::UserDefined(origin) => Type::UserDefined(*origin),
            Type::Forall(generics, typ) => {
                for generic in generics.iter() {
                    if let Generic::Inferred(id) = generic {
                        assert!(!bindings.contains_key(id));
                        assert!(!more_bindings.contains_key(id));
                    }
                }

                let typ = Arc::new(typ.follow_all_two(bindings, more_bindings));
                Type::Forall(generics.clone(), typ)
            },
            Type::Tuple(elements) => {
                Type::Tuple(Arc::new(mapvec(elements.iter(), |t| t.follow_all_two(bindings, more_bindings))))
            },
        }
    }

    /// Similar to substitute, but substitutes `Type::Generic` instead of `Type::TypeVariable`
    pub fn substitute(&self, bindings_to_substitute: &GenericSubstitutions, bindings_in_scope: &TypeBindings) -> Type {
        match self.follow(bindings_in_scope) {
            Type::Primitive(_) | Type::UserDefined(_) => self.clone(),
            Type::Generic(generic) => match bindings_to_substitute.get(generic) {
                Some(binding) => binding.clone(),
                None => self.clone(),
            },
            Type::Variable(id) => match bindings_to_substitute.get(&Generic::Inferred(*id)) {
                Some(binding) => binding.clone(),
                None => self.clone(),
            },
            Type::Function(function) => {
                let function = function.clone();
                let parameters = mapvec(&function.parameters, |param| {
                    let typ = param.typ.substitute(bindings_to_substitute, bindings_in_scope);
                    ParameterType::new(typ, param.is_implicit)
                });
                let environment = function.environment.substitute(bindings_to_substitute, bindings_in_scope);
                let return_type = function.return_type.substitute(bindings_to_substitute, bindings_in_scope);
                Type::Function(Arc::new(FunctionType { parameters, environment, return_type }))
            },
            Type::Application(constructor, args) => {
                let (constructor, args) = (constructor.clone(), args.clone());
                let constructor = constructor.substitute(bindings_to_substitute, bindings_in_scope);
                let args = mapvec(args.iter(), |arg| arg.substitute(bindings_to_substitute, bindings_in_scope));
                Type::Application(Arc::new(constructor), Arc::new(args))
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
                let typ = typ.clone();
                typ.substitute(&bindings, bindings_in_scope)
            },
            Type::Tuple(elements) => Type::Tuple(Arc::new(mapvec(elements.iter(), |t| {
                t.substitute(bindings_to_substitute, bindings_in_scope)
            }))),
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
    pub(crate) fn ignore_forall(&self) -> &Self {
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
        typ: &cst::Type, resolve: &crate::name_resolution::ResolutionResult, db: &DbHandle, next_id: &mut u32,
        insert_implicit_type_vars: bool,
    ) -> Type {
        Self::from_cst_type_with_kind(typ, Kind::Type, resolve, db, next_id, insert_implicit_type_vars)
    }

    /// Converts an ast type to a generalized Type, with any free type variables
    /// replaced with a `Type::Generic(Generic::Inferred(id))` - although the type will
    /// will not be wrapped in a `forall`.
    pub(crate) fn from_cst_type_generalized(
        typ: &cst::Type, resolve: &crate::name_resolution::ResolutionResult, db: &DbHandle,
        insert_implicit_type_vars: bool,
    ) -> Type {
        let mut next_id = 0;
        let typ = Self::from_cst_type_with_kind(typ, Kind::Type, resolve, db, &mut next_id, insert_implicit_type_vars);

        if next_id == 0 {
            // fast track - if no type variables were created, we have nothing to replace
            typ
        } else {
            typ.generalize(&TypeBindings::default()).remove_forall()
        }
    }

    /// Convert this [cst::Type] into a [Type] with the expected [Kind].
    /// Error if the converted [Kind] does not match the expected [Kind].
    fn from_cst_type_with_kind(
        typ: &cst::Type, expected: Kind, resolve: &crate::name_resolution::ResolutionResult, db: &DbHandle,
        next_id: &mut u32, insert_implicit_type_vars: bool,
    ) -> Type {
        let location = typ.location.clone();
        let (typ, kind) = Type::from_cst_type_helper(typ, resolve, db, next_id, insert_implicit_type_vars, true);
        if !expected.unifies(&kind) {
            db.accumulate(Diagnostic::ExpectedKind { actual: kind, expected, location });
        }
        typ
    }

    /// Returns a tuple of:
    /// - The converted type
    /// - The kind of the converted type
    ///
    /// Does not error if the returned type is not of kind [Kind::Type]
    ///
    /// `wrap_bare_ability`: when true (the default for any "use" position), a bare
    /// ability with no explicit args (e.g. `Fail`) is automatically applied to a fresh
    /// env type variable so it can stand alone as a `Kind::Type`. The Application
    /// branch passes `false` when recursing into its `f` position, since `Fail env`
    /// needs to see the raw `AbilityConstructor` kind to accept the explicit env arg.
    fn from_cst_type_helper(
        typ: &cst::Type, resolve: &crate::name_resolution::ResolutionResult, db: &DbHandle, next_id: &mut u32,
        insert_implicit_type_vars: bool, wrap_bare_ability: bool,
    ) -> (Type, Kind) {
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
                    IntegerKind::U32 => Type::U32,
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
                let origin = resolve.path_origins.get(path).copied();
                let (named_type, kind) =
                    Self::convert_origin_to_type(origin, db, &typ.location, Type::UserDefined);

                // A bare ability with no explicit args (e.g. `Fail`) still needs its implicit
                // `[env]` arg before it can be used as a regular type. The Application branch
                // handles this for `Fail a` / `Eq t` when the env is missing; we apply the same
                // insertion at the leaf so nested uses like `fn Fail -> Unit` also work.
                if wrap_bare_ability
                    && let Kind::AbilityConstructor(explicit_kinds) = &kind
                    && explicit_kinds.is_empty()
                {
                    let env = if insert_implicit_type_vars {
                        let fresh_env = Type::Variable(TypeVariableId(*next_id));
                        *next_id += 1;
                        fresh_env
                    } else {
                        db.accumulate(Diagnostic::TraitTypeCantBeUsed { location: typ.location.clone() });
                        Type::ERROR
                    };
                    let applied = Type::Application(Arc::new(named_type), Arc::new(vec![env]));
                    (applied, Kind::Type)
                } else {
                    (named_type, kind)
                }
            },
            crate::parser::cst::TypeKind::Variable(name) => {
                let origin = resolve.name_origins.get(name).copied();
                Self::convert_origin_to_type(origin, db, &typ.location, |origin| Type::Generic(Generic::Named(origin)))
            },
            crate::parser::cst::TypeKind::Function(function) => {
                let parameters = mapvec(&function.parameters, |param| {
                    let typ = Self::from_cst_type(&param.typ, resolve, db, next_id, insert_implicit_type_vars);
                    ParameterType::new(typ, param.is_implicit)
                });
                let environment = if let Some(environment) = function.environment.as_ref() {
                    Self::from_cst_type(environment, resolve, db, next_id, insert_implicit_type_vars)
                } else {
                    Type::NO_CLOSURE_ENV
                };
                let return_type =
                    Self::from_cst_type(&function.return_type, resolve, db, next_id, insert_implicit_type_vars);

                let f = Type::Function(Arc::new(FunctionType { parameters, environment, return_type }));
                (f, Kind::Type)
            },
            crate::parser::cst::TypeKind::Error => (Type::ERROR, Kind::Error),
            crate::parser::cst::TypeKind::Unit => (Type::UNIT, Kind::Type),
            crate::parser::cst::TypeKind::Application(f, args) => {
                // The `f` of `f args` may be a bare ability like `Fail` whose env is being
                // supplied explicitly by the args list. Don't auto-wrap it here — the Application
                // case below handles the AbilityConstructor kind itself.
                let (f, f_kind) =
                    Self::from_cst_type_helper(f, resolve, db, next_id, insert_implicit_type_vars, false);

                if !f_kind.accepts_n_arguments(args.len()) {
                    let expected = f_kind.required_argument_count();
                    let location = typ.location.clone();
                    db.accumulate(Diagnostic::FunctionArgCountMismatch { actual: args.len(), expected, location });
                    return (Type::ERROR, Kind::Type);
                }

                let mut converted_args = mapvec(args.iter().enumerate(), |(i, arg)| {
                    let expected_kind = f_kind.get_nth_parameter_kind(i);
                    Self::from_cst_type_with_kind(arg, expected_kind, resolve, db, next_id, insert_implicit_type_vars)
                });

                // Automatically insert a fresh type variable for the implicit env parameter
                // when an AbilityConstructor is applied without the optional env argument.
                if let Kind::AbilityConstructor(kinds) = &f_kind {
                    if converted_args.len() == kinds.len() {
                        if insert_implicit_type_vars {
                            let fresh_env = Type::Variable(TypeVariableId(*next_id));
                            *next_id += 1;
                            converted_args.push(fresh_env);
                        } else {
                            db.accumulate(Diagnostic::TraitTypeCantBeUsed { location: typ.location.clone() });
                            converted_args.push(Type::ERROR);
                        }
                    }
                }

                assert!(!converted_args.is_empty());
                let typ = Type::Application(Arc::new(f), Arc::new(converted_args));
                (typ, Kind::Type)
            },
            crate::parser::cst::TypeKind::Reference(kind) => (
                Type::Primitive(PrimitiveType::Reference(*kind)),
                Kind::TypeConstructorSimple(NonZeroUsize::new(1).unwrap()),
            ),
            crate::parser::cst::TypeKind::NoClosureEnv => (Type::NO_CLOSURE_ENV, Kind::Type),
            crate::parser::cst::TypeKind::Pointer => {
                (Type::POINTER, Kind::TypeConstructorSimple(NonZeroUsize::new(1).unwrap()))
            },
            crate::parser::cst::TypeKind::Tuple(elements) => {
                let elements =
                    mapvec(elements, |t| Self::from_cst_type(t, resolve, db, next_id, insert_implicit_type_vars));
                (Type::Tuple(Arc::new(elements)), Kind::Type)
            },
            crate::parser::cst::TypeKind::Hole if insert_implicit_type_vars => {
                let typ = Type::Variable(TypeVariableId(*next_id));
                *next_id += 1;
                (typ, Kind::Type)
            },
            crate::parser::cst::TypeKind::Hole => {
                db.accumulate(Diagnostic::HoleCantBeUsed { location: typ.location.clone() });
                (Type::ERROR, Kind::Error)
            },
        }
    }

    fn convert_origin_to_type(
        origin: Option<Origin>, db: &DbHandle, location: &Location, make_type: impl FnOnce(Origin) -> Type,
    ) -> (Type, Kind) {
        match origin {
            Some(Origin::Builtin(builtin)) => match builtin {
                Builtin::Unit => (Type::UNIT, Kind::Type),
                Builtin::Char => (Type::CHAR, Kind::Type),
                Builtin::Bool => (Type::BOOL, Kind::Type),
                Builtin::Ptr => (Type::POINTER, Kind::TypeConstructorSimple(NonZeroUsize::new(1).unwrap())),
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
            Some(origin) => (make_type(origin), Kind::Type),
            // Assume name resolution has already issued an error for this case
            None => (Type::ERROR, Kind::Error),
        }
    }

    /// Generalize a type, making it generic. Any holes in the type become generic types.
    pub fn generalize(&self, bindings: &TypeBindings) -> Type {
        let free_vars = self.free_vars(&bindings);

        if free_vars.is_empty() {
            self.clone()
        } else {
            let substitutions = free_vars.iter().map(|var| (*var, Type::Generic(*var))).collect();
            let typ = self.substitute(&substitutions, bindings);
            Type::Forall(Arc::new(free_vars), Arc::new(typ))
        }
    }

    /// Return the list of unbound type variables within this type
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
                    if !free_vars.contains(&generic) {
                        free_vars.push(*generic);
                    }
                },
                Type::Function(function) => {
                    for parameter in &function.parameters {
                        free_vars_helper(&parameter.typ, bindings, free_vars);
                    }
                    free_vars_helper(&function.environment, bindings, free_vars);
                    free_vars_helper(&function.return_type, bindings, free_vars);
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
            }
        }

        let mut free_vars = Vec::new();
        free_vars_helper(self, bindings, &mut free_vars);
        free_vars
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
    fn reference_constructor(&self, bindings: &TypeBindings) -> Option<ReferenceKind> {
        match self.follow(bindings) {
            Type::Primitive(PrimitiveType::Reference(kind)) => Some(*kind),
            _ => None,
        }
    }

    /// If this is a reference type, return the reference kind and its element type
    pub fn reference_element(&self, bindings: &TypeBindings) -> Option<(ReferenceKind, Type)> {
        match self.follow(bindings) {
            Type::Application(constructor, args) if !args.is_empty() => {
                constructor.reference_constructor(bindings).map(|kind| (kind, args[0].clone()))
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
            Type::Application(constructor, args)
                if constructor.pointer_constructor(bindings)
                    || constructor.reference_constructor(bindings).is_some() =>
            {
                args.get(0)
            },
            _ => None,
        }
    }
}

pub struct TypePrinter<'a, Db, Names> {
    typ: &'a Type,
    bindings: &'a TypeBindings,
    names: &'a Names,
    db: &'a Db,
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

                if *function.environment.follow(&self.bindings) != Type::NO_CLOSURE_ENV {
                    write!(f, " [")?;
                    self.fmt_type(&function.environment, false, f)?;
                    write!(f, "]")?;
                }

                write!(f, " -> ")?;
                self.fmt_type(&function.return_type, false, f)
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
                    let display_args = if self.is_ability_constructor(constructor) {
                        &args[..args.len().saturating_sub(1)]
                    } else {
                        args.as_slice()
                    };
                    self.fmt_type(constructor, true, f)?;
                    for arg in display_args.iter() {
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
        }
    }

    fn is_ability_constructor(&self, constructor: &Type) -> bool {
        if let Type::UserDefined(Origin::TopLevelDefinition(id)) = constructor.follow(self.bindings) {
            let (item, _ctx) = GetItem(id.top_level_item).get(self.db);
            if let cst::TopLevelItemKind::TypeDefinition(definition) = &item.kind {
                return definition.is_ability;
            }
        }
        false
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
