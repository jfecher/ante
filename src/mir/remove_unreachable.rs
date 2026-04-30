//! A MIR pass that drops every definition not transitively reachable from `main`.
use rustc_hash::FxHashSet;

use crate::mir::{DefinitionId, Instruction, Mir, Value};

impl Mir {
    pub(crate) fn remove_unreachable_functions(mut self) -> Self {
        let seeds: Vec<DefinitionId> =
            self.definitions.iter().filter(|(_, def)| def.name.as_str() == "main").map(|(id, _)| *id).collect();

        // No main found, leave the MIR alone
        if seeds.is_empty() {
            return self;
        }

        let reachable = reachable_from(&self, seeds);
        self.definitions.retain(|id, _| reachable.contains(id));
        self
    }
}

fn reachable_from(mir: &Mir, seeds: Vec<DefinitionId>) -> FxHashSet<DefinitionId> {
    let mut reachable = FxHashSet::default();
    let mut worklist = seeds;

    while let Some(id) = worklist.pop() {
        if !reachable.insert(id) {
            continue;
        }

        let Some(def) = mir.definitions.get(&id) else {
            continue;
        };

        let enqueue = |called: DefinitionId, worklist: &mut Vec<DefinitionId>| {
            if mir.definitions.contains_key(&called) && !reachable.contains(&called) {
                worklist.push(called);
            }
        };

        for instruction in def.instructions.values() {
            instruction.for_each_value(|value| {
                if let Value::Definition(called) = value {
                    enqueue(*called, &mut worklist);
                }
            });
            // `for_each_value` exposes Values but misses DefinitionIds that are carried
            // directly by some instructions. Visit those explicitly.
            match instruction {
                Instruction::Instantiate(id, _) => enqueue(*id, &mut worklist),
                Instruction::Perform { effect_op, .. } => enqueue(*effect_op, &mut worklist),
                Instruction::Handle { cases, .. } => {
                    for case in cases {
                        enqueue(case.effect_op, &mut worklist);
                    }
                },
                _ => (),
            }
        }

        for block in def.blocks.values() {
            if let Some(terminator) = &block.terminator {
                terminator.for_each_value(|value| {
                    if let Value::Definition(called) = value {
                        enqueue(*called, &mut worklist);
                    }
                });
            }
        }
    }

    reachable
}
