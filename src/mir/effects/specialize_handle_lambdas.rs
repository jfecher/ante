//! The body and handler lambdas of each [Instruction::Handle] should share the generics of the
//! definition containing the Handle. This no longer holds once a function containing a Handle
//! is inlined into a caller since they're instantiated with the callee's bindings.
//!
//! This pass restores the invariant by specializing each lambda for the definition
//! containing the Handle.
use std::{borrow::Cow, sync::Arc};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};

use super::local_handler_optimization::has_handle;
use crate::mir::{
    BlockId, Definition, DefinitionId, GenericBindings, Instruction, InstructionId, Mir, Type, Value,
    identity_bindings,
    inline::{Substitution, instantiate},
    next_definition_id,
};

/// Guards against any unforeseen cycle of specializations
const MAX_SPECIALIZATION_DEPTH: u32 = 64;

impl Mir {
    pub(crate) fn specialize_handle_lambdas(mut self) -> Self {
        let mut worklist: Vec<DefinitionId> =
            self.definitions.par_iter().filter(|(_, definition)| has_handle(definition)).map(|(id, _)| *id).collect();

        // Lambdas kept by a Handle site as its body. Each site needs its own body since the
        // handle passes may clone and neutralize a site's body.
        let mut claimed: FxHashSet<DefinitionId> = FxHashSet::default();
        let mut visited: FxHashSet<DefinitionId> = FxHashSet::default();
        let mut depths: FxHashMap<DefinitionId, u32> = FxHashMap::default();
        // Built on first use since most programs need no specialization
        let mut references: Option<References> = None;

        while let Some(id) = worklist.pop() {
            let Some(definition) = self.definitions.get(&id) else { continue };
            // A definition is revisited after it is specialized in place, by which point its
            // Handles' bodies already belong to it
            let revisit = !visited.insert(id);
            let rewrites = find_rewrites(definition, &self, &mut claimed, revisit);
            if rewrites.is_empty() {
                continue;
            }
            let depth = depths.get(&id).copied().unwrap_or(0) + 1;
            assert!(
                depth <= MAX_SPECIALIZATION_DEPTH,
                "specialize_handle_lambdas: lambdas of `{}` are specialized more than {MAX_SPECIALIZATION_DEPTH} levels deep",
                definition.name,
            );

            let references = references.get_or_insert_with(|| References::new(&self));
            let mut definition = self.definitions.remove(&id).expect("definition vanished");
            references.remove(&definition);
            let mut local = References::of(&definition);
            let mut uses = use_counts(&definition);
            let mut instantiations = Vec::new();
            for rewrite in rewrites {
                // Once rewritten, the holder no longer uses the lambda. If nothing else does it
                // can be specialized in place.
                let exclusive = is_exclusive(&definition, rewrite.reference, &uses);
                if exclusive {
                    local.subtract(rewrite.lambda);
                }
                let in_place = exclusive && references.count(rewrite.lambda) + local.count(rewrite.lambda) == 0;
                let generic_count = definition.generic_count;
                let specialized =
                    self.specialize(rewrite.lambda, rewrite.bindings.as_ref(), generic_count, in_place, references);
                claimed.insert(specialized);
                depths.insert(specialized, depth);
                local.add(specialized);
                // A lambda specialized in place may have been visited already, when its own
                // Handles' lambdas still had its old generics
                if has_handle(&self.definitions[&specialized]) {
                    worklist.push(specialized);
                }
                apply_rewrite(
                    &mut definition,
                    &self.definitions[&specialized],
                    rewrite,
                    &mut instantiations,
                    &mut uses,
                );
            }
            // An `Instantiate` has no operands, so placing each at the start of the entry block
            // makes it available to every use
            definition.blocks[BlockId::ENTRY_BLOCK].instructions.splice(0..0, instantiations);
            // Drop the old references so the unspecialized lambdas are no longer used
            definition.remove_dead_instructions();
            definition.compact_instructions();
            references.add_references_of(&definition);
            self.definitions.insert(id, definition);
        }
        self
    }

    /// Specialize `lambda` with `bindings` for a definition with `generic_count` generics,
    /// returning the specialized lambda. If `in_place`, `lambda` itself is specialized.
    fn specialize(
        &mut self, lambda: DefinitionId, bindings: Option<&Arc<GenericBindings>>, generic_count: u32, in_place: bool,
        references: &mut References,
    ) -> DefinitionId {
        let Some(bindings) = bindings else {
            // Only a copy is needed
            if in_place {
                return lambda;
            }
            let copy = self.definitions[&lambda].clone_with_id(next_definition_id());
            references.add_references_of(&copy);
            let id = copy.id;
            self.definitions.insert(id, copy);
            return id;
        };
        let (definition, id) = if in_place {
            let definition = self.definitions.remove(&lambda).expect("specialized lambda is missing");
            references.remove(&definition);
            (Cow::Owned(definition), lambda)
        } else {
            (Cow::Borrowed(&self.definitions[&lambda]), next_definition_id())
        };
        let substitution = Substitution { bindings, definitions: &self.definitions };
        let specialized = instantiate(definition, id, substitution, generic_count);
        references.add_references_of(&specialized);
        self.definitions.insert(id, specialized);
        id
    }
}

/// How many times each definition is referred to, see [Definition::for_each_referenced_definition]
#[derive(Default)]
struct References(FxHashMap<DefinitionId, usize>);

impl References {
    fn new(mir: &Mir) -> Self {
        let add = |mut references: References, (_, definition): (&DefinitionId, &Definition)| {
            references.add_references_of(definition);
            references
        };
        let merge = |mut a: References, b: References| {
            for (id, count) in b.0 {
                *a.0.entry(id).or_default() += count;
            }
            a
        };
        mir.definitions.par_iter().fold(References::default, add).reduce(References::default, merge)
    }

    fn of(definition: &Definition) -> Self {
        let mut references = Self::default();
        references.add_references_of(definition);
        references
    }

    fn add_references_of(&mut self, definition: &Definition) {
        definition.for_each_referenced_definition(|id| self.add(id));
    }

    fn remove(&mut self, definition: &Definition) {
        definition.for_each_referenced_definition(|id| self.subtract(id));
    }

    fn add(&mut self, id: DefinitionId) {
        *self.0.entry(id).or_default() += 1;
    }

    fn subtract(&mut self, id: DefinitionId) {
        if let Some(count) = self.0.get_mut(&id) {
            *count = count.saturating_sub(1);
        }
    }

    fn count(&self, id: DefinitionId) -> usize {
        self.0.get(&id).copied().unwrap_or(0)
    }
}

/// The number of uses of each instruction of `definition`
fn use_counts(definition: &Definition) -> Vec<usize> {
    let mut uses = vec![0; definition.instructions.len()];
    definition.for_each_used_value(|value| {
        if let Value::InstructionResult(id) = value {
            uses[usize::from(*id)] += 1;
        }
    });
    uses
}

/// True if `reference` is a reference to a lambda used only once, by the
/// instruction holding it, so rewriting the holder leaves the lambda unused here
fn is_exclusive(definition: &Definition, mut reference: Value, uses: &[usize]) -> bool {
    loop {
        let Value::InstructionResult(id) = reference else { return matches!(reference, Value::Definition(_)) };
        if uses[usize::from(id)] != 1 {
            return false;
        }
        match &definition.instructions[id] {
            Instruction::Id(value) => reference = *value,
            instruction => return matches!(instruction, Instruction::Instantiate(..)),
        }
    }
}

/// A reference to a lambda in a Handle with bindings other than its parent's identity bindings
struct Rewrite {
    /// The instruction holding the reference: a `PackClosure` or the `Handle` itself
    holder: InstructionId,
    /// The value referencing the lambda within `holder`
    reference: Value,
    lambda: DefinitionId,
    bindings: Option<Arc<GenericBindings>>,
}

fn find_rewrites(
    definition: &Definition, mir: &Mir, claimed: &mut FxHashSet<DefinitionId>, revisit: bool,
) -> Vec<Rewrite> {
    let mut rewrites = Vec::new();
    for id in definition.block_instructions() {
        let Instruction::Handle { body, cases } = &definition.instructions[id] else { continue };
        let values = std::iter::once((*body, true)).chain(cases.iter().map(|case| (case.handler, false)));
        for (value, is_body) in values {
            let Some(function) = definition.known_function(value) else { continue };
            let Some(lambda) = mir.definitions.get(&function.id) else { continue };
            let (holder, reference) = match function.closure {
                Some(closure) => match &definition.instructions[closure] {
                    Instruction::PackClosure { function, .. } => (closure, *function),
                    _ => unreachable!("known_function returned a closure which is not a PackClosure"),
                },
                None => (id, value),
            };
            let identity = function
                .bindings
                .as_ref()
                .is_none_or(|bindings| is_identity(bindings, definition.generic_count, lambda.generic_count));
            // Only bodies must be unique to a site; they are mutated in place by effect lowering.
            // A lambda specialized for a site is unique to it already.
            if identity && (!is_body || revisit || claimed.insert(function.id)) {
                continue;
            }
            rewrites.push(Rewrite { holder, reference, lambda: function.id, bindings: function.bindings });
        }
    }
    rewrites
}

fn is_identity(bindings: &GenericBindings, parent_generic_count: u32, lambda_generic_count: u32) -> bool {
    lambda_generic_count == parent_generic_count
        && bindings.iter().enumerate().all(|(index, binding)| *binding == Type::generic(index as u32))
}

/// Make `rewrite.holder` refer to `specialized` with the parent's identity bindings. Any new
/// `Instantiate` is added to `instantiations` to be placed in a block.
fn apply_rewrite(
    definition: &mut Definition, specialized: &Definition, rewrite: Rewrite, instantiations: &mut Vec<InstructionId>,
    uses: &mut [usize],
) {
    let new_reference = match identity_bindings(definition.generic_count) {
        Some(bindings) if specialized.generic_count == definition.generic_count => {
            let instantiate = Instruction::Instantiate(specialized.id, bindings);
            let id = definition.add_instruction(instantiate, specialized.typ.clone());
            instantiations.push(id);
            Value::InstructionResult(id)
        },
        _ => Value::Definition(specialized.id),
    };
    let mut replaced = 0;
    let mut replace = |value: &mut Value| {
        if *value == rewrite.reference {
            *value = new_reference;
            replaced += 1;
        }
    };
    match &mut definition.instructions[rewrite.holder] {
        Instruction::PackClosure { function, .. } => replace(function),
        Instruction::Handle { body, cases } => {
            replace(body);
            cases.iter_mut().for_each(|case| replace(&mut case.handler));
        },
        _ => unreachable!(),
    }
    if let Value::InstructionResult(old) = rewrite.reference
        && let Some(uses) = uses.get_mut(usize::from(old))
    {
        *uses = uses.saturating_sub(replaced);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::mir::{BlockId, TerminatorInstruction};

    #[test]
    fn rewrites_handle_body_reached_through_id() {
        let entry = BlockId::ENTRY_BLOCK;
        let mut lambda = Definition::new(Arc::new("lambda".to_string()), next_definition_id(), 0, Type::UNIT);
        lambda.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::Unit));
        let lambda_id = lambda.id;

        let mut parent = Definition::new(Arc::new("parent".to_string()), next_definition_id(), 0, Type::UNIT);
        let body = parent.push_instruction_in(entry, Instruction::Id(Value::Definition(lambda_id)), Type::UNIT);
        let handle = parent.push_instruction_in(entry, Instruction::Handle { body, cases: Vec::new() }, Type::UNIT);
        parent.blocks[entry].terminator = Some(TerminatorInstruction::Return(handle));
        let parent_id = parent.id;
        let Value::InstructionResult(handle) = handle else { unreachable!() };

        let mut mir = Mir::default();
        mir.definitions.insert(lambda_id, lambda);
        mir.definitions.insert(parent_id, parent);

        let mut claimed = FxHashSet::from_iter([lambda_id]);
        let rewrites = find_rewrites(&mir.definitions[&parent_id], &mir, &mut claimed, false);
        assert_eq!(rewrites.len(), 1);

        let mut references = References::new(&mir);
        let mut definition = mir.definitions.remove(&parent_id).unwrap();
        let mut uses = use_counts(&definition);
        for rewrite in rewrites {
            let new_id = mir.specialize(rewrite.lambda, None, definition.generic_count, false, &mut references);
            apply_rewrite(&mut definition, &mir.definitions[&new_id], rewrite, &mut Vec::new(), &mut uses);

            let Instruction::Handle { body, .. } = &definition.instructions[handle] else { unreachable!() };
            assert_eq!(definition.definition_of(*body).map(|(id, _)| id), Some(new_id));
        }
    }
}
