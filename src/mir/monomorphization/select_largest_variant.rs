//! A pass separate to but related to monomorphization.
//!
//! Traverses the Mir replacing each union type with the largest variant in the union,
//! according to the current target machine.

use crate::{
    incremental::TargetPointerSize,
    mir::{Definition, Instruction, IntConstant, Mir, Type, Value},
};
use inc_complete::DbGet;
use std::sync::Arc;

impl Mir {
    /// Replace each union type used with the largest variant of that type.
    pub(super) fn select_largest_variants<Db>(mut self, db: &Db) -> Self
    where
        Db: DbGet<TargetPointerSize>,
    {
        let ptr_size = TargetPointerSize.get(db);

        //self.definitions.par_iter_mut().for_each(|(_, definition)| definition.select_largest_variants(ptr_size));
        self.definitions.iter_mut().for_each(|(_, definition)| definition.select_largest_variants(ptr_size));
        self.externals.values_mut().for_each(|extern_| extern_.typ.select_largest_variants(ptr_size));
        self
    }
}

impl Definition {
    /// Replaces each union type with the largest variant of that union
    ///
    /// `ptr_size` should be the size of a pointer in bytes.
    fn select_largest_variants(&mut self, ptr_size: u32) {
        self.typ.select_largest_variants(ptr_size);

        for typ in self.instruction_result_types.values_mut() {
            typ.select_largest_variants(ptr_size);
        }

        for block in self.blocks.values_mut() {
            for parameter in block.parameter_types.iter_mut() {
                parameter.select_largest_variants(ptr_size);
            }
        }

        // Resolve SizeOf / ArrayLen of concrete types to Usz constants now that all types are concrete.
        for instruction in self.instructions.values_mut() {
            if let Instruction::SizeOf(typ) = instruction {
                typ.select_largest_variants(ptr_size);
                let size = typ.size_in_bytes(ptr_size) as usize;
                *instruction = Instruction::Id(Value::Integer(IntConstant::Usz(size)));
            } else if let Instruction::ArrayLen(typ) = instruction {
                typ.select_largest_variants(ptr_size);
                let Type::Array { length, .. } = typ else {
                    unreachable!("ArrayLen on non-Array type after monomorphization: {typ}")
                };
                let Type::U32(n) = length.as_ref() else {
                    unreachable!("ArrayLen with non-constant length after monomorphization: {length}")
                };
                *instruction = Instruction::Id(Value::Integer(IntConstant::Usz(*n as usize)));
            } else if let Instruction::StackAllocUninit(typ) = instruction {
                typ.select_largest_variants(ptr_size);
            }
        }
    }
}

impl Type {
    fn contains_union(&self) -> bool {
        match self {
            Type::Primitive(_) | Type::Generic(_) | Type::U32(_) => false,
            Type::Union(_) => true,
            Type::Tuple(fields) => fields.iter().any(Type::contains_union),
            Type::Array { length: _, element } => element.contains_union(),
            Type::Function(function) => {
                function.parameters.iter().any(Type::contains_union)
                    || function.environment.contains_union()
                    || function.return_type.contains_union()
            },
        }
    }

    fn select_largest_variants(&mut self, ptr_size: u32) {
        if self.contains_union() {
            match self {
                Type::Primitive(_) | Type::Generic(_) | Type::U32(_) => unreachable!(),
                Type::Tuple(items) => {
                    let items = Arc::make_mut(items);
                    items.iter_mut().for_each(|typ| typ.select_largest_variants(ptr_size));
                },
                Type::Array { length: _, element } => {
                    // Length is a TypeLevelU32 by this point (monomorphization is done) so no
                    // recursion needed for it.
                    let element = Arc::make_mut(element);
                    element.select_largest_variants(ptr_size);
                },
                Type::Function(function) => {
                    let function = Arc::make_mut(function);
                    function.parameters.iter_mut().for_each(|typ| typ.select_largest_variants(ptr_size));
                    function.environment.select_largest_variants(ptr_size);
                    function.return_type.select_largest_variants(ptr_size);
                },
                Type::Union(variants) => {
                    let variants = Arc::make_mut(variants);
                    variants.iter_mut().for_each(|typ| typ.select_largest_variants(ptr_size));

                    let largest = Self::find_largest_variant(variants, ptr_size);
                    *self = largest;
                },
            }
        }
    }

    fn find_largest_variant(variants: &[Type], ptr_size: u32) -> Type {
        match variants.len() {
            0 => Type::UNIT,
            1 => variants[0].clone(),
            _ => variants.iter().max_by_key(|typ| typ.size_in_bytes(ptr_size)).unwrap().clone(),
        }
    }
}
