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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
    Tuple(TupleId, Vec<Type>),
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
