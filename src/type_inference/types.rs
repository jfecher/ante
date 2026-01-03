use std::{borrow::Cow, collections::BTreeMap, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    incremental::GetItem, iterator_extensions::mapvec, lexer::token::{FloatKind, IntegerKind}, name_resolution::{Origin, builtin::Builtin}, parser::{
        cst::{self, Mutability, Sharedness},
        ids::NameId,
    }, type_inference::generics::Generic, vecmap::VecMap
};

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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FunctionType {
    pub parameters: Vec<ParameterType>,
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
    /// TODO: This should be a struct type
    String,
    // * -> * -> *
    Pair,
    Int(IntegerKind),
    Float(FloatKind),
    Reference(Mutability, Sharedness),
}

/// Maps type variables to their bindings
pub type TypeBindings = BTreeMap<TypeVariableId, Type>;

pub type GenericSubstitutions = FxHashMap<Generic, Type>;

impl Type {
    pub const ERROR: Type = Type::Primitive(PrimitiveType::Error);
    pub const UNIT: Type = Type::Primitive(PrimitiveType::Unit);
    pub const BOOL: Type = Type::Primitive(PrimitiveType::Bool);
    pub const POINTER: Type = Type::Primitive(PrimitiveType::Pointer);
    pub const CHAR: Type = Type::Primitive(PrimitiveType::Char);
    pub const STRING: Type = Type::Primitive(PrimitiveType::String);
    pub const PAIR: Type = Type::Primitive(PrimitiveType::Pair);

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

    pub const REF: Type = Type::Primitive(PrimitiveType::Reference(Mutability::Immutable, Sharedness::Shared));
    pub const IMM: Type = Type::Primitive(PrimitiveType::Reference(Mutability::Immutable, Sharedness::Owned));
    pub const MUT: Type = Type::Primitive(PrimitiveType::Reference(Mutability::Mutable, Sharedness::Shared));
    pub const UNIQ: Type = Type::Primitive(PrimitiveType::Reference(Mutability::Mutable, Sharedness::Owned));

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

    pub fn primitive(primitive_type: super::types::PrimitiveType) -> Type {
        match primitive_type {
            super::types::PrimitiveType::Error => Type::ERROR,
            super::types::PrimitiveType::Unit => Type::UNIT,
            super::types::PrimitiveType::Bool => Type::BOOL,
            super::types::PrimitiveType::Pointer => Type::POINTER,
            super::types::PrimitiveType::Char => Type::CHAR,
            super::types::PrimitiveType::String => Type::STRING,
            super::types::PrimitiveType::Pair => Type::PAIR,
            super::types::PrimitiveType::Int(kind) => Self::integer(kind),
            super::types::PrimitiveType::Float(kind) => Self::float(kind),
            super::types::PrimitiveType::Reference(Mutability::Immutable, Sharedness::Shared) => Type::REF,
            super::types::PrimitiveType::Reference(Mutability::Immutable, Sharedness::Owned) => Type::IMM,
            super::types::PrimitiveType::Reference(Mutability::Mutable, Sharedness::Shared) => Type::MUT,
            super::types::PrimitiveType::Reference(Mutability::Mutable, Sharedness::Owned) => Type::UNIQ,
        }
    }

    pub fn reference(mutability: Mutability, sharedness: Sharedness) -> Type {
        match (mutability, sharedness) {
            (Mutability::Immutable, Sharedness::Shared) => Type::REF,
            (Mutability::Immutable, Sharedness::Owned) => Type::IMM,
            (Mutability::Mutable, Sharedness::Shared) => Type::MUT,
            (Mutability::Mutable, Sharedness::Owned) => Type::UNIQ,
        }
    }

    /// Convert this type to a string (without any coloring)
    pub fn to_string<Db>(&self, bindings: &TypeBindings, names: &VecMap<NameId, Arc<String>>, db: &Db) -> String
    where
        Db: DbGet<GetItem>,
    {
        self.display(bindings, names, db).to_string()
    }

    pub fn display<'local, Db>(
        &'local self, bindings: &'local TypeBindings, names: &'local VecMap<NameId, Arc<String>>, db: &'local Db,
    ) -> TypePrinter<'local, Db>
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
    pub fn follow_type<'a>(mut self: &'a Self, bindings: &'a TypeBindings) -> &'a Type {
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

    /// Similar to substitute, but substitutes `Type::Generic` instead of `Type::TypeVariable`
    pub fn substitute_generics(&self, bindings_to_substitute: &GenericSubstitutions, bindings_in_scope: &TypeBindings) -> Type {
        match self.follow_type(bindings_in_scope) {
            Type::Primitive(_) | Type::Variable(_) | Type::UserDefined(_) => self.clone(),
            Type::Generic(generic) => match bindings_to_substitute.get(generic) {
                Some(binding) => binding.clone(),
                None => self.clone(),
            },
            Type::Function(function) => {
                let function = function.clone();
                let parameters = mapvec(&function.parameters, |param| {
                    let typ = param.typ.substitute_generics(bindings_to_substitute, bindings_in_scope);
                    ParameterType::new(typ, param.is_implicit)
                });
                let return_type = function.return_type.substitute_generics(bindings_to_substitute, bindings_in_scope);
                let effects = function.effects.substitute_generics(bindings_to_substitute, bindings_in_scope);
                Type::Function(Arc::new(FunctionType { parameters, return_type, effects }))
            },
            Type::Application(constructor, args) => {
                let (constructor, args) = (constructor.clone(), args.clone());
                let constructor = constructor.substitute_generics(bindings_to_substitute, bindings_in_scope);
                let args = mapvec(args.iter(), |arg| arg.substitute_generics(bindings_to_substitute, bindings_in_scope));
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
                typ.substitute_generics(&bindings, bindings_in_scope)
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
                typ.substitute_generics(&substitutions, bindings_in_scope)
            }
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

    /// Convert an ast type to a Type as closely as possible.
    /// This method does not emit any errors and relies on name resolution
    /// to emit errors when resolving types.
    /// Convert the given Origin to a type, issuing an error if the origin is not a type
    pub(crate) fn from_cst_type(typ: &cst::Type, resolve: &crate::name_resolution::ResolutionResult) -> Type {
        match typ {
            crate::parser::cst::Type::Integer(kind) => match kind {
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
            },
            crate::parser::cst::Type::Float(kind) => match kind {
                FloatKind::F32 => Type::F32,
                FloatKind::F64 => Type::F64,
            },
            crate::parser::cst::Type::String => Type::STRING,
            crate::parser::cst::Type::Char => Type::CHAR,
            crate::parser::cst::Type::Named(path) => {
                // TODO: is `current_resolve` sufficient or do we need the [ExtendedTopLevelContext]?
                let origin = resolve.path_origins.get(path).copied();
                Self::convert_origin_to_type(origin, Type::UserDefined)
            },
            crate::parser::cst::Type::Variable(name) => {
                // TODO: is `current_resolve` sufficient or do we need the [ExtendedTopLevelContext]?
                let origin = resolve.name_origins.get(name).copied();
                Self::convert_origin_to_type(origin, |origin| Type::Generic(Generic::Named(origin)))
            },
            crate::parser::cst::Type::Function(function) => {
                let parameters = mapvec(&function.parameters, |param| {
                    let typ = Self::from_cst_type(&param.typ, resolve);
                    ParameterType::new(typ, param.is_implicit)
                });
                let return_type = Self::from_cst_type(&function.return_type, resolve);
                // TODO: Effects
                let effects = Type::UNIT;
                Type::Function(Arc::new(FunctionType { parameters, return_type, effects }))
            },
            crate::parser::cst::Type::Error => Type::ERROR,
            crate::parser::cst::Type::Unit => Type::UNIT,
            crate::parser::cst::Type::Pair => Type::PAIR,
            crate::parser::cst::Type::Application(f, args) => {
                let f = Self::from_cst_type(f, resolve);
                let args = mapvec(args, |typ| Self::from_cst_type(typ, resolve));
                Type::Application(Arc::new(f), Arc::new(args))
            },
            crate::parser::cst::Type::Reference(mutability, sharedness) => {
                Type::Primitive(PrimitiveType::Reference(*mutability, *sharedness))
            },
        }
    }

    fn convert_origin_to_type(origin: Option<Origin>, make_type: impl FnOnce(Origin) -> Type) -> Type {
        match origin {
            Some(Origin::Builtin(builtin)) => {
                match builtin {
                    Builtin::Unit => Type::UNIT,
                    Builtin::Int => Type::ERROR, // TODO: Polymorphic integers
                    Builtin::Char => Type::CHAR,
                    Builtin::Float => Type::ERROR, // TODO: Polymorphic floats
                    Builtin::String => Type::STRING,
                    Builtin::Ptr => Type::POINTER,
                    Builtin::PairType => Type::PAIR,
                    Builtin::PairConstructor => {
                        // TODO: Error
                        Type::ERROR
                    },
                }
            },
            Some(origin) => {
                if !origin.may_be_a_type() {
                    // TODO: Error
                }
                make_type(origin)
            },
            // Assume name resolution has already issued an error for this case
            None => Type::ERROR,
        }
    }

    /// Generalize a type, making it generic. Any holes in the type become generic types.
    pub fn generalize(&self, bindings: &TypeBindings) -> Type {
        let free_vars = self.free_vars(&bindings);

        if free_vars.is_empty() {
            self.clone()
        } else {
            let substitutions = free_vars.iter().map(|var| (*var, Type::Generic(Generic::Inferred(*var)))).collect();
            let free_vars = mapvec(free_vars, Generic::Inferred);
            let typ = self.substitute(&substitutions, bindings);
            Type::Forall(Arc::new(free_vars), Arc::new(typ))
        }
    }

    pub fn substitute(&self, substitutions: &TypeBindings, bindings: &TypeBindings) -> Type {
        match self.follow_type(bindings) {
            Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) => self.clone(),
            Type::Variable(id) => match substitutions.get(id) {
                Some(binding) => binding.clone(),
                None => self.clone(),
            },
            Type::Function(function) => {
                let function = function.clone();
                let parameters = mapvec(&function.parameters, |param| {
                    let typ = param.typ.substitute(substitutions, bindings);
                    ParameterType::new(typ, param.is_implicit)
                });
                let return_type = function.return_type.substitute(substitutions, bindings);
                let effects = function.effects.substitute(substitutions, bindings);
                Type::Function(Arc::new(FunctionType { parameters, return_type, effects }))
            },
            Type::Application(constructor, args) => {
                let (constructor, args) = (constructor.clone(), args.clone());
                let constructor = constructor.substitute(substitutions, bindings);
                let args = mapvec(args.iter(), |arg| arg.substitute(substitutions, bindings));
                Type::Application(Arc::new(constructor), Arc::new(args))
            },
            Type::Forall(generics, typ) => {
                // We need to remove any generics in `generics` that are in `bindings`,
                // but we wan't to avoid allocating a new map in the common case where there are
                // no conflicts.
                let mut new_substitutions = Cow::Borrowed(substitutions);

                for generic in generics.iter() {
                    if let Generic::Inferred(id) = generic {
                        if new_substitutions.contains_key(id) {
                            let mut new_bindings = new_substitutions.into_owned();
                            new_bindings.remove(id);
                            new_substitutions = Cow::Owned(new_bindings);
                        }
                    }
                }
                let typ = typ.clone();
                typ.substitute(&new_substitutions, bindings)
            },
        }
    }

    /// Return the list of unbound type variables within this type
    pub fn free_vars(&self, bindings: &TypeBindings) -> Vec<TypeVariableId> {
        fn free_vars_helper(typ: &Type, bindings: &TypeBindings, free_vars: &mut Vec<TypeVariableId>) {
            match typ.follow_type(bindings) {
                Type::Primitive(_) | Type::Generic(_) | Type::UserDefined(_) => (),
                Type::Variable(id) => {
                    // The number of free vars is expected to remain too small so we're
                    // not too worried about asymptotic behavior. It is more important we
                    // maintain the ordering of insertion.
                    if !free_vars.contains(id) {
                        free_vars.push(*id);
                    }
                },
                Type::Function(function) => {
                    for parameter in &function.parameters {
                        free_vars_helper(&parameter.typ, bindings, free_vars);
                    }
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
                    free_vars.retain(|id| !generics.contains(&Generic::Inferred(*id)));
                },
            }
        }

        let mut free_vars = Vec::new();
        free_vars_helper(self, bindings, &mut free_vars);
        free_vars
    }
}

pub struct TypePrinter<'a, Db> {
    typ: &'a Type,
    bindings: &'a TypeBindings,
    names: &'a VecMap<NameId, Arc<String>>,
    db: &'a Db,
}

impl<Db> std::fmt::Display for TypePrinter<'_, Db>
where
    Db: DbGet<GetItem>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.fmt_type(self.typ, false, f)
    }
}

impl<Db> TypePrinter<'_, Db>
where
    Db: DbGet<GetItem>,
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
            Type::Function(function) => {
                try_parenthesize(parenthesize, f, |f| {
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
                    write!(f, " -> ")?;
                    self.fmt_type(&function.return_type, false, f)
                })
            },
            Type::Application(constructor, args) => {
                try_parenthesize(parenthesize, f, |f| {
                if **constructor == Type::PAIR && args.len() == 2 {
                    self.fmt_type(&args[0], true, f)?;
                    write!(f, ", ")?;
                    self.fmt_type(&args[1], true, f)
                } else {
                    self.fmt_type(constructor, true, f)?;
                    for arg in args.iter() {
                        write!(f, " ")?;
                        self.fmt_type(arg, true, f)?;
                    }
                        Ok(())
                }
                })
            },
            Type::Forall(generics, typ) => {
                try_parenthesize(parenthesize, f, |f| {
                    write!(f, "forall")?;
                    for generic in generics.iter() {
                        write!(f, " {generic}")?;
                    }
                    write!(f, ". ")?;
                    self.fmt_type(typ, parenthesize, f)
                })
            },
        }
    }

    fn fmt_type_origin(&self, origin: Origin, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match origin {
            Origin::TopLevelDefinition(id) => {
                let (item, context) = GetItem(id.top_level_item).get(self.db);
                if let cst::ItemName::Single(name) = item.kind.name() {
                    write!(f, "{}", context.names[name])
                } else {
                    unreachable!()
                }
            },
            Origin::Local(name) => write!(f, "{}", self.names[name]),
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
            PrimitiveType::String => write!(f, "String"),
            PrimitiveType::Char => write!(f, "Char"),
            PrimitiveType::Pair => write!(f, ","),
            PrimitiveType::Reference(mutability, Sharedness::Shared) => write!(f, "{mutability}"),
            PrimitiveType::Reference(mutability, Sharedness::Owned) => write!(f, "{mutability}own"),
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
