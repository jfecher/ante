use serde::{Deserialize, Serialize};

/// Contains only builtin items which can be redefined (are not keywords).
/// This includes most builtin types except for sized-integer and float types `I8`, `I16`, `U32`, etc.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Builtin {
    Unit,
    Int,
    Char,
    Float,
    String,
    Ptr,
    PairType,
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
