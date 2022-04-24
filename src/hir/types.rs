use crate::util::fmap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntegerKind {
    I8, I16, I32, I64, Isz,
    U8, U16, U32, U64, Usz,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PrimitiveType {
    IntegerType(IntegerKind), // : *
    FloatType,                // : *
    CharType,                 // : *
    BooleanType,              // : *
    UnitType,                 // : *
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub is_varargs: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TupleId(pub usize);

/// A HIR type representation.
/// Removes all references to generics and user-defined types.
/// Union variants are also absent, being represented by a struct
/// value and a cast to a different struct type of the largest variant.
#[derive(Debug, Clone, Eq, Ord, PartialOrd, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Function(FunctionType),
    Pointer(Box<Type>),

    /// Tuples have a TypeId to allow for struct recursion
    Tuple(Option<TupleId>, Vec<Type>),
}

impl Type {
    pub fn into_function(self) -> Option<FunctionType> {
        match self {
            Type::Function(f) => Some(f),
            _ => None,
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Primitive(l), Self::Primitive(r)) => l == r,
            (Self::Function(l), Self::Function(r)) => l == r,
            (Self::Pointer(l), Self::Pointer(r)) => l == r,
            (Self::Tuple(l_id, _), Self::Tuple(r_id, _)) => l_id == r_id,
            _ => false,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Primitive(p) => {
                match p {
                    PrimitiveType::IntegerType(kind) => kind.fmt(f),
                    PrimitiveType::FloatType => write!(f, "float"),
                    PrimitiveType::CharType => write!(f, "char"),
                    PrimitiveType::BooleanType => write!(f, "bool"),
                    PrimitiveType::UnitType => write!(f, "unit"),
                }
            },
            Type::Function(function) => function.fmt(f),
            Type::Pointer(_elem) => write!(f, "(Ptr _)"),
            Type::Tuple(_, elems) => {
                let elems = fmap(elems, ToString::to_string);
                write!(f, "{{ {} }}", elems.join(", "))
            },
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
            IntegerKind::I8  => write!(f, "i8"),
            IntegerKind::I16 => write!(f, "i16"),
            IntegerKind::I32 => write!(f, "i32"),
            IntegerKind::I64 => write!(f, "i64"),
            IntegerKind::Isz => write!(f, "isz"),
            IntegerKind::U8  => write!(f, "u8"),
            IntegerKind::U16 => write!(f, "u16"),
            IntegerKind::U32 => write!(f, "u32"),
            IntegerKind::U64 => write!(f, "u64"),
            IntegerKind::Usz => write!(f, "usz"),
        }
    }
}
