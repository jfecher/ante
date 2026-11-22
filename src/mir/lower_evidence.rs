//! Lowers [Type::Evidence] and the evidence instructions to plain tuples. This requires every row
//! generic to already be substituted, either by monomorphization or by [row
//! specialization](super::row_specialization).
//!
//! After this pass, evidence is a tuple holding the capability of each slot in the same order as
//! its type (see [evidence]), so each slot is just an index into the tuple.
use std::sync::Arc;

use crate::{
    iterator_extensions::mapvec,
    mir::{
        BlockId, Definition, EvidenceEntry, EvidenceItem, FunctionType, Instruction, InstructionId, Mir, Type, Value,
        evidence,
    },
    parser::ids::TopLevelName,
};

impl Mir {
    pub fn lower_evidence(mut self) -> Mir {
        for definition in self.definitions.values_mut() {
            lower_definition(definition);
        }
        self
    }
}

fn lower_definition(definition: &mut Definition) {
    // Instructions are rewritten first since they index into the still-unlowered evidence types
    for block_id in (0..definition.blocks.len()).map(BlockId::from) {
        let instruction_ids = std::mem::take(&mut definition.blocks[block_id].instructions);
        let mut lowered = Vec::with_capacity(instruction_ids.len());
        for id in instruction_ids {
            lower_instruction(definition, id, &mut lowered);
            lowered.push(id);
        }
        definition.blocks[block_id].instructions = lowered;
    }

    definition.typ = lower_type(&definition.typ);
    definition.for_each_type_mut(|typ| *typ = lower_type(typ));
}

/// Rewrites the evidence instruction `id` in place, pushing any new instructions it needs onto `before`
fn lower_instruction(definition: &mut Definition, id: InstructionId, before: &mut Vec<InstructionId>) {
    let instruction = std::mem::replace(&mut definition.instructions[id], Instruction::Id(Value::Unit));
    let lowered = match instruction {
        Instruction::LookupEvidence { evidence, key, prefix } => {
            let held = value_type(definition, evidence);
            let ahead = count(evidence::entries(&prefix), key.effect);
            slot(evidence, &held, key.effect, ahead)
        },
        // Take each slot of the row generic. These come after any slots for the same effect in
        // `prefix`.
        Instruction::RestEvidence { evidence, prefix } => {
            let result = definition.instruction_result_type(id).clone();
            let held = value_type(definition, evidence);
            let prefix = evidence::entries(&prefix);
            let result = evidence::entries(&result);
            let slots = mapvec(Type::evidence_capabilities(result).enumerate(), |(i, (key, capability))| {
                let ahead = count(prefix, key.effect) + count(&result[..i], key.effect);
                let slot = slot(evidence, &held, key.effect, ahead);
                add(definition, before, slot, capability.clone())
            });
            Instruction::MakeTuple(slots)
        },
        Instruction::MakeEvidence(items) => {
            let mut slots = Vec::new();
            for item in items {
                match item {
                    EvidenceItem::Capability(key, value) => slots.push((key.effect, value)),
                    EvidenceItem::Splice(spliced) => {
                        let held = value_type(definition, spliced);
                        let capabilities = Type::evidence_capabilities(evidence::entries(&held));
                        for (index, (key, capability)) in capabilities.enumerate() {
                            let slot = Instruction::IndexTuple { tuple: spliced, index: index as u32 };
                            slots.push((key.effect, add(definition, before, slot, capability.clone())));
                        }
                    },
                }
            }
            evidence::sort_by_effect(&mut slots, |(effect, _)| Some(*effect));
            Instruction::MakeTuple(mapvec(slots, |(_, value)| value))
        },
        other => other,
    };
    definition.instructions[id] = lowered;
}

fn count(entries: &[EvidenceEntry], effect: TopLevelName) -> usize {
    evidence::count(entries, effect).expect("every row generic is substituted before evidence is lowered")
}

fn add(definition: &mut Definition, before: &mut Vec<InstructionId>, instruction: Instruction, typ: Type) -> Value {
    let id = definition.add_instruction(instruction, typ);
    before.push(id);
    Value::InstructionResult(id)
}

/// Returns the evidence type of the value `evidence`
fn value_type(definition: &Definition, evidence: Value) -> Type {
    definition.local_value_type(&evidence).unwrap_or_else(|| panic!("`{evidence}` is not an evidence value")).clone()
}

/// Returns an instruction extracting the capability for `effect` from `evidence` (of type `held`),
/// skipping the first `ahead` capabilities for that effect
fn slot(evidence: Value, held: &Type, effect: TopLevelName, ahead: usize) -> Instruction {
    let index = evidence::slot_index(evidence::entries(held), effect, ahead)
        .unwrap_or_else(|| panic!("the evidence `{evidence}` holds no capability {ahead} for effect {effect}"));
    Instruction::IndexTuple { tuple: evidence, index: index as u32 }
}

fn lower_type(typ: &Type) -> Type {
    match typ {
        Type::Evidence(entries) => {
            Type::tuple(mapvec(Type::evidence_capabilities(entries), |(_, capability)| lower_type(capability)))
        },
        Type::Tuple(fields) => Type::tuple(mapvec(fields.iter(), lower_type)),
        Type::Union(variants) => Type::union(mapvec(variants.iter(), lower_type)),
        Type::Array { length, element } => Type::array_with_length(lower_type(length), lower_type(element)),
        Type::Function(function) => Type::Function(Arc::new(FunctionType {
            parameters: mapvec(&function.parameters, lower_type),
            environment: lower_type(&function.environment),
            return_type: lower_type(&function.return_type),
        })),
        Type::Primitive(_) | Type::U32(_) | Type::Generic(_) => typ.clone(),
    }
}
