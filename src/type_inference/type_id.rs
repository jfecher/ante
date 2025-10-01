use std::sync::Arc;

use inc_complete::DbGet;
use serde::{Deserialize, Serialize};

use crate::{
    incremental::GetItem, parser::{cst::{Mutability, Sharedness}, ids::NameId}, type_inference::{type_context::TypeContext, types::TypeBindings}, vecmap::VecMap
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TypeId(u32);

impl From<TypeId> for usize {
    fn from(value: TypeId) -> Self {
        value.0 as usize
    }
}

impl From<usize> for TypeId {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

impl TypeId {
    pub const ERROR: TypeId = TypeId(0);
    pub const UNIT: TypeId = TypeId(1);
    pub const BOOL: TypeId = TypeId(2);
    pub const POINTER: TypeId = TypeId(3);
    pub const CHAR: TypeId = TypeId(4);
    pub const STRING: TypeId = TypeId(5);
    pub const PAIR: TypeId = TypeId(6);

    pub const I8: TypeId = TypeId(7);
    pub const I16: TypeId = TypeId(8);
    pub const I32: TypeId = TypeId(9);
    pub const I64: TypeId = TypeId(10);
    pub const ISZ: TypeId = TypeId(11);

    pub const U8: TypeId = TypeId(12);
    pub const U16: TypeId = TypeId(13);
    pub const U32: TypeId = TypeId(14);
    pub const U64: TypeId = TypeId(15);
    pub const USZ: TypeId = TypeId(16);

    pub const F32: TypeId = TypeId(17);
    pub const F64: TypeId = TypeId(18);

    pub const REF: TypeId = TypeId(19);
    pub const REF_OWN: TypeId = TypeId(20);
    pub const REF_MUT: TypeId = TypeId(21);
    pub const REF_MUT_OWN: TypeId = TypeId(22);

    pub fn integer(kind: crate::lexer::token::IntegerKind) -> TypeId {
        match kind {
            crate::lexer::token::IntegerKind::I8 => TypeId::I8,
            crate::lexer::token::IntegerKind::I16 => TypeId::I16,
            crate::lexer::token::IntegerKind::I32 => TypeId::I32,
            crate::lexer::token::IntegerKind::I64 => TypeId::I64,
            crate::lexer::token::IntegerKind::Isz => TypeId::ISZ,
            crate::lexer::token::IntegerKind::U8 => TypeId::U8,
            crate::lexer::token::IntegerKind::U16 => TypeId::U16,
            crate::lexer::token::IntegerKind::U32 => TypeId::U32,
            crate::lexer::token::IntegerKind::U64 => TypeId::U64,
            crate::lexer::token::IntegerKind::Usz => TypeId::USZ,
        }
    }

    pub fn float(kind: crate::lexer::token::FloatKind) -> TypeId {
        match kind {
            crate::lexer::token::FloatKind::F32 => TypeId::F32,
            crate::lexer::token::FloatKind::F64 => TypeId::F64,
        }
    }

    pub(crate) fn primitive(primitive_type: super::types::PrimitiveType) -> TypeId {
        match primitive_type {
            super::types::PrimitiveType::Error => TypeId::ERROR,
            super::types::PrimitiveType::Unit => TypeId::UNIT,
            super::types::PrimitiveType::Bool => TypeId::BOOL,
            super::types::PrimitiveType::Pointer => TypeId::POINTER,
            super::types::PrimitiveType::Char => TypeId::CHAR,
            super::types::PrimitiveType::String => TypeId::STRING,
            super::types::PrimitiveType::Pair => TypeId::PAIR,
            super::types::PrimitiveType::Int(kind) => Self::integer(kind),
            super::types::PrimitiveType::Float(kind) => Self::float(kind),
            super::types::PrimitiveType::Reference(Mutability::Immutable, Sharedness::Shared) => TypeId::REF,
            super::types::PrimitiveType::Reference(Mutability::Immutable, Sharedness::Owned) => TypeId::REF_OWN,
            super::types::PrimitiveType::Reference(Mutability::Mutable, Sharedness::Shared) => TypeId::REF_MUT,
            super::types::PrimitiveType::Reference(Mutability::Mutable, Sharedness::Owned) => TypeId::REF_MUT_OWN,
        }
    }

    /// Convert this type to a string (without any coloring)
    pub fn to_string<Db>(
        self, context: &TypeContext, bindings: &TypeBindings, names: &VecMap<NameId, Arc<String>>, db: &Db,
    ) -> String where Db: DbGet<GetItem> {
        context.get_type(self).display(bindings, context, names, db).to_string()
    }
}
