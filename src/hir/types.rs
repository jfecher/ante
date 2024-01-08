use std::rc::Rc;

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
    pub effects: Vec<super::Effect>,
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
    pub fn unit() -> Self {
        Type::Primitive(PrimitiveType::Unit)
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
        write!(f, "{}", self.return_type)?;

        if !self.effects.is_empty() {
            let effects = fmap(&self.effects, ToString::to_string);
            write!(f, " can {}", effects.join(", "))?;
        }

        Ok(())
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

pub trait Typed {
    fn get_type(&self) -> Type;
}

impl Typed for super::Ast {
    fn get_type(&self) -> Type {
        crate::hir::dispatch_on_hir!(self, Typed::get_type)
    }
}

impl Typed for super::Literal {
    fn get_type(&self) -> Type {
        match self {
            super::Literal::Integer(_, kind) => kind.get_type(),
            super::Literal::Float(_, kind) => kind.get_type(),
            super::Literal::CString(_) => Type::Primitive(PrimitiveType::Pointer),
            super::Literal::Char(_) => Type::Primitive(PrimitiveType::Char),
            super::Literal::Bool(_) => Type::Primitive(PrimitiveType::Boolean),
            super::Literal::Unit => Type::Primitive(PrimitiveType::Unit),
        }
    }
}

impl Typed for super::IntegerKind {
    fn get_type(&self) -> Type {
        Type::Primitive(PrimitiveType::Integer(*self))
    }
}

impl Typed for super::FloatKind {
    fn get_type(&self) -> Type {
        Type::Primitive(PrimitiveType::Float(*self))
    }
}

impl Typed for super::Variable {
    fn get_type(&self) -> Type {
        self.typ.as_ref().clone()
    }
}

impl<T> Typed for Rc<T> where T: Typed {
    fn get_type(&self) -> Type {
        self.as_ref().get_type()
    }
}

impl Typed for super::Lambda {
    fn get_type(&self) -> Type {
        Type::Function(self.typ.clone())
    }
}

impl Typed for super::FunctionCall {
    fn get_type(&self) -> Type {
        self.function_type.return_type.as_ref().clone()
    }
}

impl Typed for super::Definition {
    fn get_type(&self) -> Type {
        Type::Primitive(PrimitiveType::Unit)
    }
}

impl Typed for super::If {
    fn get_type(&self) -> Type {
        self.result_type.clone()
    }
}

impl Typed for super::Match {
    fn get_type(&self) -> Type {
        self.result_type.clone()
    }
}

impl Typed for super::Return {
    fn get_type(&self) -> Type {
        self.typ.clone()
    }
}

impl Typed for super::Sequence {
    fn get_type(&self) -> Type {
        match self.statements.last() {
            Some(last) => last.get_type(),
            None => Type::Primitive(PrimitiveType::Unit),
        }
    }
}

impl Typed for super::Extern {
    fn get_type(&self) -> Type {
        self.typ.clone()
    }
}

impl Typed for super::Assignment {
    fn get_type(&self) -> Type {
        Type::Primitive(PrimitiveType::Unit)
    }
}

impl Typed for super::MemberAccess {
    fn get_type(&self) -> Type {
        self.typ.clone()
    }
}

impl Typed for super::Tuple {
    fn get_type(&self) -> Type {
        Type::Tuple(fmap(&self.fields, |field| field.get_type()))
    }
}

impl Typed for super::ReinterpretCast {
    fn get_type(&self) -> Type {
        self.target_type.clone()
    }
}

impl Typed for super::Builtin {
    fn get_type(&self) -> Type {
        match self {
            super::Builtin::AddInt(lhs, _) => lhs.get_type(),
            super::Builtin::AddFloat(lhs, _) => lhs.get_type(),
            super::Builtin::SubInt(lhs, _) => lhs.get_type(),
            super::Builtin::SubFloat(lhs, _) => lhs.get_type(),
            super::Builtin::MulInt(lhs, _) => lhs.get_type(),
            super::Builtin::MulFloat(lhs, _) => lhs.get_type(),
            super::Builtin::DivSigned(lhs, _) => lhs.get_type(),
            super::Builtin::DivUnsigned(lhs, _) => lhs.get_type(),
            super::Builtin::DivFloat(lhs, _) => lhs.get_type(),
            super::Builtin::ModSigned(lhs, _) => lhs.get_type(),
            super::Builtin::ModUnsigned(lhs, _) => lhs.get_type(),
            super::Builtin::ModFloat(lhs, _) => lhs.get_type(),
            super::Builtin::LessSigned(lhs, _) => lhs.get_type(),
            super::Builtin::LessUnsigned(lhs, _) => lhs.get_type(),
            super::Builtin::LessFloat(lhs, _) => lhs.get_type(),
            super::Builtin::EqInt(lhs, _) => lhs.get_type(),
            super::Builtin::EqFloat(lhs, _) => lhs.get_type(),
            super::Builtin::EqChar(lhs, _) => lhs.get_type(),
            super::Builtin::EqBool(lhs, _) => lhs.get_type(),
            super::Builtin::SignExtend(_, typ) => typ.clone(),
            super::Builtin::ZeroExtend(_, typ) => typ.clone(),
            super::Builtin::SignedToFloat(_, typ) => typ.clone(),
            super::Builtin::UnsignedToFloat(_, typ) => typ.clone(),
            super::Builtin::FloatToSigned(_, typ) => typ.clone(),
            super::Builtin::FloatToUnsigned(_, typ) => typ.clone(),
            super::Builtin::FloatPromote(_, typ) => typ.clone(),
            super::Builtin::FloatDemote(_, typ) => typ.clone(),
            super::Builtin::BitwiseAnd(lhs, _) => lhs.get_type(),
            super::Builtin::BitwiseOr(lhs, _) => lhs.get_type(),
            super::Builtin::BitwiseXor(lhs, _) => lhs.get_type(),
            super::Builtin::BitwiseNot(lhs) => lhs.get_type(),
            super::Builtin::Truncate(_, typ) => typ.clone(),
            super::Builtin::Deref(_, typ) => typ.clone(),
            super::Builtin::Offset(lhs, _, _) => lhs.get_type(),
            super::Builtin::Transmute(_, typ) => typ.clone(),
            super::Builtin::StackAlloc(_) => Type::Primitive(PrimitiveType::Pointer),
        }
    }
}

impl Typed for super::Effect {
    fn get_type(&self) -> Type {
        self.typ.clone()
    }
}

impl Typed for super::Handle {
    fn get_type(&self) -> Type {
        self.result_type.clone()
    }
}
