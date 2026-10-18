//! Lowers [Type::Evidence] and the evidence instructions to plain tuples once monomorphization
//! has substituted every row generic, so each capability has a fixed index in canonical order.
use std::sync::Arc;

use crate::{
    iterator_extensions::mapvec,
    mir::{BlockId, Definition, EffectKey, EvidenceEntry, FunctionType, Instruction, InstructionId, Mir, Type, Value},
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
    if !matches!(definition.instructions[id], Instruction::LookupEvidence { .. } | Instruction::MakeEvidence { .. }) {
        return;
    }
    match std::mem::replace(&mut definition.instructions[id], Instruction::MakeTuple(Vec::new())) {
        Instruction::LookupEvidence { evidence, key } => {
            let index = evidence_index(&evidence_entries(definition, &evidence), &key)
                .unwrap_or_else(|| panic!("the evidence `{evidence}` holds no capability for {key}"));
            definition.instructions[id] = Instruction::IndexTuple { tuple: evidence, index };
        },
        Instruction::MakeEvidence { capabilities, rest } => {
            let Type::Evidence(entries) = definition.instruction_result_type(id).clone() else {
                panic!("MakeEvidence does not have an evidence type");
            };
            let rest = rest.map(|rest| (rest, evidence_entries(definition, &rest)));
            let values = mapvec(Type::evidence_capabilities(&entries), |(key, capability)| {
                if let Some((_, value)) = capabilities.iter().find(|(candidate, _)| candidate == key) {
                    return *value;
                }
                let (rest, index) = rest
                    .as_ref()
                    .and_then(|(rest, entries)| Some((*rest, evidence_index(entries, key)?)))
                    .unwrap_or_else(|| panic!("no evidence holds the capability {key} for a MakeEvidence"));
                let new_id = definition.instructions.push(Instruction::IndexTuple { tuple: rest, index });
                definition.instruction_result_types.push_existing(new_id, capability.clone());
                before.push(new_id);
                Value::InstructionResult(new_id)
            });
            definition.instructions[id] = Instruction::MakeTuple(values);
        },
        _ => unreachable!(),
    }
}

/// The entries of the evidence value `evidence`
fn evidence_entries(definition: &Definition, evidence: &Value) -> Arc<Vec<EvidenceEntry>> {
    match definition.local_value_type(evidence) {
        Some(Type::Evidence(entries)) => entries.clone(),
        _ => panic!("`{evidence}` is not an evidence value"),
    }
}

/// The canonical index of `key` within evidence with `entries`
fn evidence_index(entries: &[EvidenceEntry], key: &EffectKey) -> Option<u32> {
    Type::evidence_capabilities(entries).position(|(candidate, _)| candidate == key).map(|index| index as u32)
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
