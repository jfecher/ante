use std::collections::BTreeMap;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    lexer::token::{FloatKind, IntegerKind},
    parser::cst::{Mutability, Sharedness},
    type_inference::{
        type_id::TypeId,
        types::{PrimitiveType, Type},
    },
    vecmap::VecMap,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TypeContext {
    id_to_type: VecMap<TypeId, Type>,
    type_to_id: BTreeMap<Type, TypeId>,
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeContext {
    /// Create a new type context, initially populated with primitive
    /// types. These are pre-populated so we can refer to statically-known
    /// ids instead of requiring a lookup whenever constructing a primitive type.
    pub fn new() -> Self {
        let mut id_to_type = VecMap::default();
        let mut type_to_id = FxHashMap::default();
        let mut insert = |typ| {
            let id = id_to_type.push(Type::Primitive(typ));
            type_to_id.insert(id, Type::Primitive(typ));
            id
        };

        let error = insert(PrimitiveType::Error);
        let unit = insert(PrimitiveType::Unit);
        let bool = insert(PrimitiveType::Bool);
        let pointer = insert(PrimitiveType::Pointer);
        let char = insert(PrimitiveType::Char);
        let string = insert(PrimitiveType::String);
        let pair = insert(PrimitiveType::Pair);
        let i8 = insert(PrimitiveType::Int(IntegerKind::I8));
        let i16 = insert(PrimitiveType::Int(IntegerKind::I16));
        let i32 = insert(PrimitiveType::Int(IntegerKind::I32));
        let i64 = insert(PrimitiveType::Int(IntegerKind::I64));
        let isz = insert(PrimitiveType::Int(IntegerKind::Isz));
        let u8 = insert(PrimitiveType::Int(IntegerKind::U8));
        let u16 = insert(PrimitiveType::Int(IntegerKind::U16));
        let u32 = insert(PrimitiveType::Int(IntegerKind::U32));
        let u64 = insert(PrimitiveType::Int(IntegerKind::U64));
        let usz = insert(PrimitiveType::Int(IntegerKind::Usz));
        let f32 = insert(PrimitiveType::Float(FloatKind::F32));
        let f64 = insert(PrimitiveType::Float(FloatKind::F64));
        let ref_ = insert(PrimitiveType::Reference(Mutability::Immutable, Sharedness::Shared));
        let ref_own = insert(PrimitiveType::Reference(Mutability::Immutable, Sharedness::Owned));
        let ref_mut = insert(PrimitiveType::Reference(Mutability::Mutable, Sharedness::Shared));
        let ref_mut_own = insert(PrimitiveType::Reference(Mutability::Mutable, Sharedness::Owned));

        assert_eq!(error, TypeId::ERROR);
        assert_eq!(unit, TypeId::UNIT);
        assert_eq!(bool, TypeId::BOOL);
        assert_eq!(pointer, TypeId::POINTER);
        assert_eq!(char, TypeId::CHAR);
        assert_eq!(string, TypeId::STRING);
        assert_eq!(pair, TypeId::PAIR);
        assert_eq!(i8, TypeId::I8);
        assert_eq!(i16, TypeId::I16);
        assert_eq!(i32, TypeId::I32);
        assert_eq!(i64, TypeId::I64);
        assert_eq!(isz, TypeId::ISZ);
        assert_eq!(u8, TypeId::U8);
        assert_eq!(u16, TypeId::U16);
        assert_eq!(u32, TypeId::U32);
        assert_eq!(u64, TypeId::U64);
        assert_eq!(usz, TypeId::USZ);
        assert_eq!(f32, TypeId::F32);
        assert_eq!(f64, TypeId::F64);
        assert_eq!(ref_, TypeId::REF);
        assert_eq!(ref_own, TypeId::REF_OWN);
        assert_eq!(ref_mut, TypeId::REF_MUT);
        assert_eq!(ref_mut_own, TypeId::REF_MUT_OWN);

        Self { id_to_type, type_to_id: Default::default() }
    }

    pub fn get_type(&self, id: TypeId) -> &Type {
        &self.id_to_type[id]
    }

    pub fn get_or_insert_type(&mut self, typ: Type) -> TypeId {
        if let Some(id) = self.type_to_id.get(&typ) {
            return *id;
        }

        let next_id = self.id_to_type.push(typ.clone());
        self.type_to_id.insert(typ, next_id);
        next_id
    }
}
