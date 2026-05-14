use serde::{Deserialize, Serialize};

use crate::type_inference::types::Type;

/// Contains only builtin items which can be redefined (are not keywords).
/// This includes most builtin types except for sized-integer and float types `I8`, `I16`, `U32`, etc.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Builtin {
    /// The Unit type (not value)
    Unit,
    /// The Char type
    Char,
    /// The Bool type
    Bool,
    /// The Ptr type constructor of kind `* -> *`
    Ptr,
    /// The Array type constructor of kind `U32 -> * -> *`. Applied as `Array n t`,
    /// where `n` is a type-level U32 length and `t` is the element type.
    Array,
    /// The bottom type
    Never,
    /// The core `intrinsic` function used in the stdlib as a placeholder for compiler intrinsics
    Intrinsic,
}

impl Builtin {
    /// Return the builtin of the same name, if there is one.
    ///
    /// This should only be implemented for builtins which should be exposed to each module
    /// automatically as if exposed by the Prelude.
    pub fn from_name(name: &str) -> Option<Builtin> {
        use Builtin::*;
        match name {
            "Unit" => Some(Unit),
            "Char" => Some(Char),
            "Bool" => Some(Bool),
            "Ptr" => Some(Ptr),
            "Array" => Some(Array),
            "Never" => Some(Never),
            // `Intrinsic` is excluded here since it should not be imported into
            // modules outside the stdlib
            _ => None,
        }
    }

    /// If this is a type, return its id.
    /// This will return [None] for values such as [Builtin::Intrinsic]
    pub fn as_type(self) -> Option<Type> {
        match self {
            Builtin::Unit => Some(Type::UNIT),
            Builtin::Char => Some(Type::CHAR),
            Builtin::Bool => Some(Type::BOOL),
            Builtin::Ptr => Some(Type::POINTER),
            Builtin::Array => Some(Type::ARRAY),
            Builtin::Never => Some(Type::NEVER),
            Builtin::Intrinsic => None,
        }
    }
}

impl std::fmt::Display for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Builtin::Unit => write!(f, "Unit"),
            Builtin::Char => write!(f, "Char"),
            Builtin::Bool => write!(f, "Bool"),
            Builtin::Ptr => write!(f, "Ptr"),
            Builtin::Array => write!(f, "Array"),
            Builtin::Never => write!(f, "Never"),
            Builtin::Intrinsic => write!(f, "intrinsic"),
        }
    }
}
