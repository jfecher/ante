//! Choosing where dynamic values are built. A dynamic value used only once, as a tuple field, as
//! the source of a transmute, or as the result, is built directly in the storage of that use so
//! it need not be copied there.

use rustc_hash::FxHashMap;

use crate::mir::{
    Definition, Instruction, InstructionId, TerminatorInstruction, Type, Value, existentialization::rewrite::Rewriter,
};

/// Where a dynamic value used only once should be built to prevent extra copies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Hint {
    /// Directly through the function's result pointer
    Result,
    /// Into this field of the dynamic tuple built by this instruction
    Field(InstructionId, usize),
    /// Into the storage of this transmute to a union containing its type
    Into(InstructionId),
}

impl Rewriter<'_> {
    /// Where the dynamic value `id` of type `typ` should be stored
    pub(super) fn storage(&mut self, id: InstructionId, typ: &Type) -> Value {
        if let Some(destination) = self.destination(id) {
            return destination;
        }
        let slot = self.builder.slot(typ);
        self.destinations.insert(id, slot);
        slot
    }

    /// The storage chosen for `id` by its hint, if it has one
    pub(super) fn destination(&mut self, id: InstructionId) -> Option<Value> {
        if let Some(destination) = self.destinations.get(&id) {
            return Some(*destination);
        }
        let destination = match *self.hints.get(&id)? {
            Hint::Result => self.result_slot?,
            Hint::Field(tuple, index) => {
                let tuple_type = self.old.instruction_result_type(tuple).clone();
                let base = self.storage(tuple, &tuple_type);
                let offset = self.builder.offsets(&tuple_type)[index];
                self.builder.offset(base, offset)
            },
            Hint::Into(target) => {
                let target_type = self.old.instruction_result_type(target).clone();
                self.storage(target, &target_type)
            },
        };
        self.destinations.insert(id, destination);
        Some(destination)
    }
}

/// Choose where each dynamic value with a single use is built
pub(super) fn destination_hints(
    old: &Definition, has_result_slot: bool, indirect: &impl Fn(&Type) -> bool,
) -> FxHashMap<InstructionId, Hint> {
    let mut uses: FxHashMap<InstructionId, u32> = FxHashMap::default();
    let mut count = |value: &Value| {
        if let Value::InstructionResult(id) = value {
            *uses.entry(*id).or_default() += 1;
        }
    };
    old.for_each_used_value(&mut count);
    let single_use = |value: &Value| match value {
        Value::InstructionResult(id) => (uses.get(id) == Some(&1)).then_some(*id),
        _ => None,
    };
    let dynamic = |id: InstructionId| indirect(old.instruction_result_type(id));

    let mut hints = FxHashMap::default();
    for (id, instruction) in old.instructions.iter() {
        match instruction {
            Instruction::MakeTuple(fields) if dynamic(id) => {
                for (index, field) in fields.iter().enumerate() {
                    if let Some(field) = single_use(field).filter(|field| dynamic(*field)) {
                        hints.insert(field, Hint::Field(id, index));
                    }
                }
            },
            Instruction::Transmute(value) if dynamic(id) => {
                let Some(source) = single_use(value).filter(|source| dynamic(*source)) else { continue };
                let source_type = old.instruction_result_type(source);
                let fits = match old.instruction_result_type(id) {
                    Type::Union(variants) => variants.iter().any(|variant| variant == source_type),
                    target => target == source_type,
                };
                if fits {
                    hints.insert(source, Hint::Into(id));
                }
            },
            _ => (),
        }
    }

    if has_result_slot {
        // Values returned by different blocks may all be built before the branch choosing which
        // is returned, and the fields hinted into them earlier still, so at most one value may
        // be built through the result pointer.
        let mut returned = old.blocks.values().filter_map(|block| match &block.terminator {
            Some(TerminatorInstruction::Return(value) | TerminatorInstruction::Result(value)) => {
                single_use(value).filter(|id| dynamic(*id))
            },
            _ => None,
        });
        if let (Some(id), None) = (returned.next(), returned.next()) {
            hints.insert(id, Hint::Result);
        }
    }
    hints
}
