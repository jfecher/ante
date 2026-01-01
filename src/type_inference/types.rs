use std::{collections::BTreeMap, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    incremental::GetItem,
    lexer::token::{FloatKind, IntegerKind},
    name_resolution::Origin,
    parser::{
        cst::{self, Mutability, Sharedness},
        ids::NameId,
    },
    type_inference::generics::Generic,
    vecmap::VecMap,
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
