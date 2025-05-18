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

    pub fn is_unsigned(self) -> bool {
        use IntegerKind::*;
        match self {
            I8 | I16 | I32 | I64 | Isz => false,
            U8 | U16 | U32 | U64 | Usz => true,
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
    Continuation,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub is_varargs: bool,
}

impl FunctionType {
    pub fn new(parameters: Vec<Type>, return_type: Type) -> Self {
        Self { parameters, return_type: Box::new(return_type), is_varargs: false }
    }
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
    pub fn unit() -> Self {
        Type::Primitive(PrimitiveType::Unit)
    }

    pub fn continuation() -> Self {
        Type::Primitive(PrimitiveType::Continuation)
    }

    pub fn pointer() -> Self {
        Type::Primitive(PrimitiveType::Pointer)
    }

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
                PrimitiveType::Pointer | PrimitiveType::Continuation => std::mem::size_of::<*const i8>() as u64,
            },
            Type::Function(_) => std::mem::size_of::<*const i8>() as u64,
            Type::Tuple(elements) => elements.iter().map(|element| element.size_in_bytes()).sum(),
        }
    }

    /// The type of the Builtin::ContinuationInit
    pub fn continuation_init_type() -> FunctionType {
        // mco_coro* mco_coro_init(void(*f)(mco_coro*));
        let init_function_arg = Type::Function(FunctionType::new(vec![Type::continuation()], Type::unit()));
        FunctionType::new(vec![init_function_arg], Type::continuation())
    }

    /// The type of the Builtin::ContinuationIsSuspended
    pub fn continuation_is_suspended_type() -> FunctionType {
        // char mco_coro_is_suspended(mco_coro*, k);
        FunctionType::new(vec![Type::continuation()], Type::Primitive(PrimitiveType::Boolean))
    }

    /// The type of the Builtin::ContinuationPush
    pub fn continuation_push_type() -> FunctionType {
        // void mco_coro_push(mco_coro* k, const void* data, size_t data_size);
        let usz = Type::Primitive(PrimitiveType::Integer(IntegerKind::Usz));
        FunctionType::new(vec![Type::continuation(), Type::pointer(), usz], Type::unit())
    }

    /// The type of the Builtin::ContinuationPop
    pub fn continuation_pop_type() -> FunctionType {
        // void mco_coro_pop(mco_coro* k, void* data, size_t data_size);
        let usz = Type::Primitive(PrimitiveType::Integer(IntegerKind::Usz));
        FunctionType::new(vec![Type::continuation(), Type::pointer(), usz], Type::unit())
    }

    /// The type of the Builtin::ContinuationSuspend
    pub fn continuation_suspend_type() -> FunctionType {
        // void mco_coro_suspend(mco_coro* k);
        FunctionType::new(vec![Type::continuation()], Type::unit())
    }

    /// The type of the Builtin::ContinuationResume
    pub fn continuation_resume_type() -> FunctionType {
        // void mco_coro_resume(mco_coro* k);
        FunctionType::new(vec![Type::continuation()], Type::unit())
    }

    /// The type of the Builtin::ContinuationFree
    pub fn continuation_free_type() -> FunctionType {
        // void mco_coro_free(mco_coro* k);
        FunctionType::new(vec![Type::continuation()], Type::unit())
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Primitive(p) => match p {
                PrimitiveType::Integer(kind) => kind.fmt(f),
                PrimitiveType::Float(kind) => kind.fmt(f),
                PrimitiveType::Char => write!(f, "Char"),
                PrimitiveType::Boolean => write!(f, "Bool"),
                PrimitiveType::Unit => write!(f, "Unit"),
                PrimitiveType::Pointer => write!(f, "Ptr"),
                PrimitiveType::Continuation => write!(f, "Cont"),
            },
            Type::Function(function) => write!(f, "({})", function),
            Type::Tuple(elems) => {
                let elems = fmap(elems, ToString::to_string);
                write!(f, "{{{}}}", elems.join(", "))
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
