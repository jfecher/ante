//! Type infos, the `(size, align, id)` of a type, and the type info tables passed to callees. Type
//! ids hash a type's structure so every definition computes the same id for the same type.

use std::{hash::BuildHasher, sync::Arc};

use rustc_hash::FxBuildHasher;

use crate::{
    iterator_extensions::mapvec,
    lexer::token::IntegerKind,
    mir::{
        DefinitionId, EffectKey, Instruction, Type, Value,
        existentialization::{
            builder::{
                At, FunctionBuilder, Op,
                arithmetic::{u64_value, usz_value, word_value},
            },
            convention::type_info_type,
            resolve::Bindings,
        },
    },
};

const TYPE_ID_PRIME: u64 = 0x100000001b3;

impl FunctionBuilder<'_> {
    pub(super) fn generic_size_align(&mut self, generic: u32) -> (Op, Op) {
        (Op::Value(self.generic_word(generic, 0)), Op::Value(self.generic_word(generic, 1)))
    }

    /// Field `field` of the `(size, align, id)` type info of `generic`, read from the type info table
    fn generic_word(&mut self, generic: u32, field: usize) -> Value {
        let Ok(index) = self.needed.binary_search(&generic) else {
            panic!(
                "existentialization: `{}` needs the type info of '{generic} but was not given it",
                self.function.name
            )
        };
        self.table_field(index * 3 + field)
    }

    /// The size, alignment, and id of `typ`
    pub(super) fn type_info_ops(&mut self, typ: &Type) -> [Op; 3] {
        let (size, align) = match typ {
            Type::U32(length) => (Op::Const(*length as u64), Op::Const(1)),
            Type::Generic(generic) => self.generic_size_align(generic.0),
            _ => {
                let layout = self.layout(typ);
                (layout.size, self.add(layout.align_mask, Op::Const(1)))
            },
        };
        [size, align, self.type_id(typ)]
    }

    /// The type info of `typ`: its size, alignment, and id
    pub(crate) fn type_info(&mut self, typ: &Type) -> Value {
        if let Some(info) = self.cache.type_infos.get(typ) {
            return *info;
        }
        let [size, align, id] = self.type_info_ops(typ);
        let fields = vec![usz_value(size), usz_value(align), u64_value(id)];
        let info = self.emit_prologue(Instruction::MakeTuple(fields), type_info_type());
        self.cache.type_infos.insert(typ.clone(), info);
        info
    }

    fn combine(&mut self, hash: Op, value: Op) -> Op {
        let u64_type = Type::int(IntegerKind::U64);
        let mixed = self.binary(hash, value, |a, b| a ^ b, Instruction::BitwiseXor, u64_type.clone());
        self.binary(mixed, Op::Const(TYPE_ID_PRIME), u64::wrapping_mul, Instruction::MulInt, u64_type)
    }

    fn combine_all<'t>(&mut self, seed: u64, types: impl IntoIterator<Item = &'t Type>) -> Op {
        let mut hash = Op::Const(seed);
        for typ in types {
            let id = self.type_id(typ);
            hash = self.combine(hash, id);
        }
        hash
    }

    /// A runtime identity for `typ`, the same whichever definition computes it
    fn type_id(&mut self, typ: &Type) -> Op {
        if let Some(id) = self.cache.type_ids.get(typ) {
            return *id;
        }
        let id = match typ {
            Type::Generic(generic) => Op::Value(self.generic_word(generic.0, 2)),
            Type::Primitive(primitive) => Op::Const(FxBuildHasher.hash_one(primitive)),
            Type::U32(n) => self.combine(Op::Const(1), Op::Const(*n as u64)),
            Type::Evidence(_) => Op::Const(2),
            Type::Tuple(fields) => self.combine_all(3 ^ fields.len() as u64, fields.iter()),
            Type::Union(variants) => self.combine_all(4 ^ variants.len() as u64, variants.iter()),
            Type::Array { length, element } => self.combine_all(5 ^ 2, [&**length, &**element]),
            Type::Function(function) => {
                let types = function.parameters.iter().chain([&function.environment, &function.return_type]);
                self.combine_all(6 ^ (function.parameters.len() as u64 + 2), types)
            },
        };
        self.cache.type_ids.insert(typ.clone(), id);
        id
    }

    /// The runtime key identifying an effect within evidence
    pub(crate) fn effect_key(&mut self, key: &EffectKey) -> Value {
        u64_value(self.combine_all(FxBuildHasher.hash_one(&key.effect), &key.args))
    }

    /// A pointer to the type info table to pass to `callee`, if it takes one.
    /// With every type known it is a constant, otherwise it is filled in by the prologue.
    pub(super) fn metadata_argument(&mut self, callee: DefinitionId, bindings: &Bindings) -> Option<Value> {
        let needed = self.shared.needed(callee);
        if needed.is_empty() {
            return None;
        }
        // A type info table holding a prefix of another's precomputed layouts is laid out as a prefix of it
        let keys = self.shared.keys(callee);
        if let Some(table) = &self.table
            && matches!(bindings, Bindings::Identity)
            && needed == self.needed
            && table.keys.starts_with(keys)
        {
            return Some(table.pointer);
        }
        let mut words = Vec::with_capacity(needed.len() * 3 + keys.len() * 2);
        let uses_ids = self.shared.uses_ids(callee);
        for generic in needed {
            let [size, align, id] = self.type_info_ops(&bindings.get(*generic));
            words.extend([size, align, if uses_ids { id } else { Op::Const(0) }]);
        }
        for key in keys {
            let key = bindings.substitute(key);
            let (layout, offsets) = match &key {
                Type::Tuple(_) | Type::Function(_) => self.tuple_layout(&key),
                _ => (self.layout(&key), Arc::new(Vec::new())),
            };
            words.extend([layout.size, layout.align_mask]);
            words.extend(offsets.iter());
        }
        if let Some(metadata) = self.cache.metadata_arguments.get(&words) {
            return Some(*metadata);
        }

        let table = self.shared.types.table(needed.len(), keys);
        let fields = mapvec(words.iter().zip(&table.fields), |(word, typ)| word_value(*word, typ));
        let metadata = if words.iter().all(|word| matches!(word, Op::Const(_))) {
            let global = self.shared.static_table(Type::tuple(table.fields), fields);
            self.emit_prologue(Instruction::GlobalAddress(global), Type::POINTER)
        } else {
            let slot = self.emit_prologue(Instruction::StackAllocUninit(Type::tuple(table.fields)), Type::POINTER);
            for (field, offset) in fields.into_iter().zip(table.offsets) {
                self.store_word_at(At::Prologue, slot, Op::Const(offset.into()), field);
            }
            slot
        };
        self.cache.metadata_arguments.insert(words, metadata);
        Some(metadata)
    }
}
