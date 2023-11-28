use crate::{lexer::token::FloatKind, util::fmap};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntegerKind {
    I8,
    I16,
    I32,
    I64,
    Isz,
    U8,
    U16,
    U32,
    U64,
    Usz,
}

impl IntegerKind {
    pub fn size_in_bits(self) -> u64 {
        use IntegerKind::*;
        match self {
            I8 | U8 => 8,
            I16 | U16 => 16,
            I32 | U32 => 32,
            I64 | U64 => 64,
            Isz | Usz => std::mem::size_of::<*const i8>() as u64 * 8,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PrimitiveType {
    Integer(IntegerKind),
    Float(FloatKind),
    Char,
    Boolean,
    Unit,
    Pointer, // An opaque pointer type
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub is_varargs: bool,
}

/// A HIR type representation.
/// Removes all references to generics and user-defined types.
/// Union variants are also absent, being represented by a struct
/// value and a cast to a different struct type of the largest variant.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Function(FunctionType),

    /// Tuples have a TypeId to allow for struct recursion
    Tuple(Vec<Type>),
}

impl Type {
    pub fn into_function(self) -> Option<FunctionType> {
        match self {
            Type::Function(f) => Some(f),
            _ => None,
        }
    }

    pub fn size_in_bytes(&self) -> u64 {
        match self {
            Type::Primitive(p) => match p {
                PrimitiveType::Integer(integer) => integer.size_in_bits() / 8,
                PrimitiveType::Float(float) => match float {
                    FloatKind::F32 => 4,
                    FloatKind::F64 => 8,
                },
                PrimitiveType::Char => 1,
                PrimitiveType::Boolean => 1,
                PrimitiveType::Unit => 1,
                PrimitiveType::Pointer => std::mem::size_of::<*const i8>() as u64,
            },
            Type::Function(_) => panic!("Tried to take size of a function type"), // Functions technically do not have a size, only pointers do
            Type::Tuple(elements) => elements.iter().map(|element| element.size_in_bytes()).sum(),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Primitive(p) => write!(f, "{p}"),
            Type::Function(function) => write!(f, "({})", function),
            Type::Tuple(elems) => {
                let elems = fmap(elems, ToString::to_string);
                write!(f, "{{{}}}", elems.join(", "))
            },
        }
    }
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::Integer(kind) => kind.fmt(f),
            PrimitiveType::Float(kind) => kind.fmt(f),
            PrimitiveType::Char => write!(f, "Char"),
            PrimitiveType::Boolean => write!(f, "Bool"),
            PrimitiveType::Unit => write!(f, "Unit"),
            PrimitiveType::Pointer => write!(f, "Ptr"),
        }
    }
}

impl std::fmt::Display for FunctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for param in &self.parameters {
            write!(f, "{} -> ", param)?;
        }
        if self.is_varargs {
            write!(f, "... -> ")?;
        }
        write!(f, "{}", self.return_type)
    }
}

impl std::fmt::Display for IntegerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegerKind::I8 => write!(f, "i8"),
            IntegerKind::I16 => write!(f, "i16"),
            IntegerKind::I32 => write!(f, "i32"),
            IntegerKind::I64 => write!(f, "i64"),
            IntegerKind::Isz => write!(f, "isz"),
            IntegerKind::U8 => write!(f, "u8"),
            IntegerKind::U16 => write!(f, "u16"),
            IntegerKind::U32 => write!(f, "u32"),
            IntegerKind::U64 => write!(f, "u64"),
            IntegerKind::Usz => write!(f, "usz"),
        }
    }
}
