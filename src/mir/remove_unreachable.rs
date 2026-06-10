//! A MIR pass that drops every definition not transitively reachable from `main`.
use rustc_hash::FxHashSet;

use crate::mir::{DefinitionId, Mir};

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

        def.for_each_referenced_definition(|called| {
            if mir.definitions.contains_key(&called) && !reachable.contains(&called) {
                worklist.push(called);
            }
        });
    }

    reachable
}
