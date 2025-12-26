use serde::{Deserialize, Serialize};

use crate::type_inference::types::Type;

/// Contains only builtin items which can be redefined (are not keywords).
/// This includes most builtin types except for sized-integer and float types `I8`, `I16`, `U32`, etc.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Builtin {
    /// The Unit type (not value)
    Unit,
    /// The polymorphic Int type
    Int,
    /// The Char type
    Char,
    /// The polymorphic Float type
    Float,
    /// The String type
    String,
    /// The Ptr type constructor of kind `* -> *`
    Ptr,
    /// The Pair type constructor of kind `* -> * -> *`
    PairType,
    /// The Pair value constructor with type `fn a b -> (a, b)`
    PairConstructor,
}

impl Builtin {
    /// Return the builtin of the same name, if there is one.
    /// An `is_type` disambiguator is required to distinguish between
    /// the pair type `,` and the value-level pair constructor `,`.
    pub fn from_name(name: &str, is_type: bool) -> Option<Builtin> {
        use Builtin::*;
        match name {
            "Unit" => Some(Unit),
            "Int" => Some(Int),
            "Char" => Some(Char),
            "Float" => Some(Float),
            "String" => Some(String),
            "Ptr" => Some(Ptr),
            "," if is_type => Some(PairType),
            "," => Some(PairConstructor),
            _ => None,
        }
    }

    /// If this is a type, return its id.
    /// This will return [None] for values such as [Builtin::PairConstructor]
    pub fn as_type(self) -> Option<Type> {
        match self {
            Builtin::Unit => Some(Type::UNIT),
            Builtin::Int => None,
            Builtin::Char => Some(Type::CHAR),
            Builtin::Float => None,
            Builtin::String => Some(Type::STRING),
            Builtin::Ptr => Some(Type::POINTER),
            Builtin::PairType => Some(Type::PAIR),
            Builtin::PairConstructor => None,
        }
    }

    /// If this builtin is a value constructor, return the type it constructs (not applied to any
    /// arguments e.g. `Pair` for the pair type instead of `Pair a b`) as well as the index of the
    /// constructor.
    /// Currently all built-in types only define one constructor so the index is always zero.
    /// Returns [None] if this is not a value constructor.
    pub fn constructor(self) -> Option<(Type, usize)> {
        match self {
            Builtin::Unit
            | Builtin::Int
            | Builtin::Char
            | Builtin::Float
            | Builtin::String
            | Builtin::Ptr
            | Builtin::PairType => None,

            Builtin::PairConstructor => Some((Type::PAIR, 0)),
        }
    }

    /// Returns the fields of this builtin type, if it has any (that are meant to be publically visible).
    ///
    /// Currently only the PairType has publically visible fields.
    pub fn fields(self, generic_args: Vec<Type>) -> Option<Vec<Type>> {
        (self == Builtin::PairType && generic_args.len() == 2).then_some(generic_args)
    }
}

impl std::fmt::Display for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Builtin::Unit => write!(f, "Unit"),
            Builtin::Int => write!(f, "Int"),
            Builtin::Char => write!(f, "Char"),
            Builtin::Float => write!(f, "Float"),
            Builtin::String => write!(f, "String"),
            Builtin::Ptr => write!(f, "Ptr"),
            Builtin::PairType => write!(f, ","),
            Builtin::PairConstructor => write!(f, ","),
        }
    }
}
