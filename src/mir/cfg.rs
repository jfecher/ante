//! Control-flow and dataflow utilities shared by inlining and the local handler optimization
use std::{borrow::Cow, sync::Arc};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    mir::{
        Block, BlockId, Definition, DefinitionId, EvidenceEntry, EvidenceItem, GenericBindings, Instruction,
        InstructionId, TerminatorInstruction, Type, Value, evidence,
    },
    parser::ids::TopLevelName,
};

/// Limits how deep `resolve` and its helpers may recurse so that we don't get stuck on
/// pathologically long chains
const MAX_RESOLVE_DEPTH: u32 = 64;

impl TerminatorInstruction {
    /// Invoke `f` on each block this terminator may transfer control to or names as a merge point
    pub(crate) fn for_each_block_mut(&mut self, mut f: impl FnMut(&mut BlockId)) {
        match self {
            TerminatorInstruction::Jmp((target, _)) => f(target),
            TerminatorInstruction::If { then, else_, end, .. } => {
                f(&mut then.0);
                f(&mut else_.0);
                f(end);
            },
            TerminatorInstruction::Switch { cases, else_, end, .. } => {
                cases.iter_mut().for_each(|(_, (target, _))| f(target));
                f(&mut else_.0);
                f(end);
            },
            TerminatorInstruction::Unreachable
            | TerminatorInstruction::Return(_)
            | TerminatorInstruction::Result(_) => {},
        }
    }

    /// Invoke `f` on each block this terminator may transfer control to (excluding merge points)
    pub(crate) fn for_each_successor(&self, mut f: impl FnMut(BlockId)) {
        match self {
            TerminatorInstruction::Jmp((target, _)) => f(*target),
            TerminatorInstruction::If { then, else_, .. } => {
                f(then.0);
                f(else_.0);
            },
            TerminatorInstruction::Switch { cases, else_, .. } => {
                cases.iter().for_each(|(_, (target, _))| f(*target));
                f(else_.0);
            },
            TerminatorInstruction::Unreachable
            | TerminatorInstruction::Return(_)
            | TerminatorInstruction::Result(_) => {},
        }
    }

    /// The merge block of an `If` or `Switch`, if any
    fn merge_block(&self) -> Option<BlockId> {
        match self {
            TerminatorInstruction::If { end, .. } | TerminatorInstruction::Switch { end, .. } => Some(*end),
            _ => None,
        }
    }
}

/// A function a value is statically known to be, see [Definition::known_function]
pub(crate) struct KnownFunction {
    pub(crate) id: DefinitionId,
    /// The bindings of the function's `Instantiate`, if any
    pub(crate) bindings: Option<Arc<GenericBindings>>,
    /// The environment, if the value is a closure of the function
    pub(crate) environment: Option<Value>,
    /// The `PackClosure` creating the closure, if any
    pub(crate) closure: Option<InstructionId>,
}

/// Instructions without side effects which may be removed if their result is unused
fn is_pure(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::Id(_)
            | Instruction::IndexTuple { .. }
            | Instruction::MakeTuple(_)
            | Instruction::MakeEvidence { .. }
            | Instruction::LookupEvidence { .. }
            | Instruction::RestEvidence { .. }
            | Instruction::PackClosure { .. }
            | Instruction::ClosureEnvironment(_)
            | Instruction::Instantiate(..)
            | Instruction::Capability
            // A slot can't be read if nothing points to it
            | Instruction::StackAlloc { .. }
    )
}

impl Definition {
    /// Add `instruction` with result type `typ` without placing it in any block
    pub(crate) fn add_instruction(&mut self, instruction: Instruction, typ: Type) -> InstructionId {
        let id = self.instructions.push(instruction);
        self.instruction_result_types.push_existing(id, typ);
        id
    }

    /// Append a new instruction to `block`
    pub(crate) fn push_instruction_in(&mut self, block: BlockId, instruction: Instruction, typ: Type) -> Value {
        let id = self.add_instruction(instruction, typ);
        self.blocks[block].instructions.push(id);
        Value::InstructionResult(id)
    }

    /// Insert a new instruction into `block` at `index`
    pub(crate) fn insert_instruction_at(
        &mut self, block: BlockId, index: usize, instruction: Instruction, typ: Type,
    ) -> Value {
        let id = self.add_instruction(instruction, typ);
        self.blocks[block].instructions.insert(index, id);
        Value::InstructionResult(id)
    }

    /// The id of each instruction in a block, in block order
    pub(crate) fn block_instructions(&self) -> impl Iterator<Item = InstructionId> + '_ {
        self.blocks.values().flat_map(|block| block.instructions.iter().copied())
    }

    /// Move the instructions after `index` along with the terminator of `block` into a new block
    /// without parameters, returning it. `block` is left without a terminator.
    pub(crate) fn split_block_after(&mut self, block: BlockId, index: usize) -> BlockId {
        let rest: Vec<_> = self.blocks[block].instructions.drain(index + 1..).collect();
        let terminator = self.blocks[block].terminator.take();
        let new_block = self.blocks.push(Block::new(Vec::new()));
        self.blocks[new_block].instructions = rest;
        self.blocks[new_block].terminator = terminator;
        new_block
    }

    /// Split `block` at the instruction at `index`, which is removed from `block`. Every following
    /// instruction and the terminator move into a new block with one parameter of the removed
    /// instruction's result type. The removed instruction becomes an `Id` of that parameter at the
    /// start of the new block so existing uses of its result remain valid.
    ///
    /// Returns the new block. `block` is left without a terminator.
    pub(crate) fn split_at_instruction(&mut self, block: BlockId, index: usize) -> BlockId {
        let new_block = self.split_block_after(block, index);
        let id = self.blocks[block].instructions.pop().expect("split_at_instruction: index out of bounds");
        let typ = self.instruction_result_types[id].clone();
        self.blocks[new_block].parameter_types.push(typ);
        self.instructions[id] = Instruction::Id(Value::Parameter(new_block, 0));
        self.blocks[new_block].instructions.insert(0, id);
        new_block
    }

    /// Resolve `value` through any `Id`s, `IndexTuple`s of `MakeTuple`s, `LookupEvidence`s
    /// of `MakeEvidence`s, and `Deref`s of immutable slots, returning the value it is statically
    /// known to be.
    pub(crate) fn resolve(&self, value: Value) -> Value {
        self.resolve_with_depth(value, 0)
    }

    fn resolve_with_depth(&self, value: Value, depth: u32) -> Value {
        // Guard against pathological chains
        if depth > MAX_RESOLVE_DEPTH {
            return value;
        }
        let value = self.follow_ids(value);
        let Value::InstructionResult(id) = value else { return value };
        match &self.instructions[id] {
            Instruction::IndexTuple { tuple, index } => {
                if let Value::InstructionResult(tuple) = self.resolve_with_depth(*tuple, depth + 1)
                    && let Instruction::MakeTuple(fields) = &self.instructions[tuple]
                    && let Some(field) = fields.get(*index as usize)
                {
                    return self.resolve_with_depth(*field, depth + 1);
                }
                value
            },
            Instruction::LookupEvidence { evidence, key, prefix } => {
                match self.known_slot(*evidence, key.effect, evidence::entries(prefix), depth) {
                    KnownSlot::Capability(capability) => self.resolve_with_depth(capability, depth + 1),
                    KnownSlot::In(..) => value,
                }
            },
            Instruction::RestEvidence { evidence, prefix } => {
                match self.known_rest(id, *evidence, evidence::entries(prefix), depth) {
                    Some(rest) => self.resolve_with_depth(rest, depth + 1),
                    None => value,
                }
            },
            Instruction::MakeEvidence(items) => match self.repacked_evidence(id, items, depth) {
                Some(source) => self.resolve_with_depth(source, depth + 1),
                None => value,
            },
            // Immutable slots are never stored to after they're created, so anywhere the slot can
            // be read from will still see the value it was created with
            Instruction::Deref(pointer) => {
                let pointer = self.resolve_with_depth(*pointer, depth + 1);
                if let Some(stored) = self.immutable_slot_value(pointer, depth) {
                    return stored;
                }
                // This is a field of a known tuple stored in an immutable slot. E.g. a method of a
                // dictionary which was passed by pointer.
                if let Value::InstructionResult(pointer) = pointer
                    && let Instruction::GetFieldPtr { struct_ptr, index, .. } = &self.instructions[pointer]
                    && let slot = self.resolve_with_depth(*struct_ptr, depth + 1)
                    && let Some(Value::InstructionResult(tuple)) = self.immutable_slot_value(slot, depth)
                    && let Instruction::MakeTuple(fields) = &self.instructions[tuple]
                    && let Some(field) = fields.get(*index as usize)
                {
                    return self.resolve_with_depth(*field, depth + 1);
                }
                value
            },
            _ => value,
        }
    }

    /// Returns the value the immutable slot `slot` was created with, or `None` if it isn't an
    /// immutable slot. Expects `slot` to already be resolved.
    fn immutable_slot_value(&self, slot: Value, depth: u32) -> Option<Value> {
        let Value::InstructionResult(slot) = slot else { return None };
        match &self.instructions[slot] {
            Instruction::StackAlloc { value: stored, immutable: true } => {
                Some(self.resolve_with_depth(*stored, depth + 1))
            },
            _ => None,
        }
    }

    /// Try to find where the capability for `effect` in the slot of `evidence` after `prefix` comes
    /// from. This looks through any known `MakeEvidence` and `RestEvidence` instructions along the
    /// way.
    fn known_slot<'a>(
        &'a self, evidence: Value, effect: TopLevelName, prefix: &'a [EvidenceEntry], depth: u32,
    ) -> KnownSlot<'a> {
        let unknown = || KnownSlot::In(evidence, Cow::Borrowed(prefix));
        let Value::InstructionResult(id) = self.resolve_with_depth(evidence, depth + 1) else { return unknown() };
        if depth > MAX_RESOLVE_DEPTH {
            return unknown();
        }
        match &self.instructions[id] {
            // The slot we're looking for also comes after everything ahead of the row generic
            Instruction::RestEvidence { evidence: inner, prefix: inner_prefix } => {
                let ahead = [evidence::entries(inner_prefix), prefix].concat();
                self.known_slot(*inner, effect, &ahead, depth + 1).into_owned()
            },
            Instruction::MakeEvidence(items) => {
                let Some(mut ahead) = evidence::count(prefix, effect) else { return unknown() };
                for item in items {
                    match item {
                        EvidenceItem::Capability(key, value) if key.effect == effect => {
                            if ahead == 0 {
                                return KnownSlot::Capability(*value);
                            }
                            ahead -= 1;
                        },
                        EvidenceItem::Capability(..) => (),
                        EvidenceItem::Splice(spliced) => {
                            let Some(Type::Evidence(entries)) = self.local_value_type(spliced) else {
                                return unknown();
                            };
                            let Some(held) = evidence::count(entries, effect) else { return unknown() };
                            if ahead < held {
                                let slot = evidence::slot_index(entries, effect, ahead).unwrap();
                                return self.known_slot(*spliced, effect, &entries[..slot], depth + 1);
                            }
                            ahead -= held;
                        },
                    }
                }
                unknown()
            },
            _ => unknown(),
        }
    }

    /// If the `MakeEvidence` at `id` only rebuilds a single evidence value with every slot in its
    /// original position, return that value. This is what a function builds when calling itself
    /// with its own row.
    fn repacked_evidence(&self, id: InstructionId, items: &[EvidenceItem], depth: u32) -> Option<Value> {
        let mut source = None;
        for (index, item) in items.iter().enumerate() {
            let (EvidenceItem::Capability(_, value) | EvidenceItem::Splice(value)) = item;
            let Value::InstructionResult(read) = self.resolve_with_depth(*value, depth + 1) else { return None };
            let (evidence, prefix) = match (item, &self.instructions[read]) {
                (EvidenceItem::Capability(..), Instruction::LookupEvidence { evidence, prefix, .. }) => {
                    (evidence, prefix)
                },
                (EvidenceItem::Splice(_), Instruction::RestEvidence { evidence, prefix })
                    if self.instruction_result_type(read).is_single_rest() =>
                {
                    (evidence, prefix)
                },
                _ => return None,
            };
            // This only reads the slot at `index` if everything ahead of both slots is the same
            let held = evidence::entries(self.local_value_type(evidence)?);
            let prefix = evidence::entries(prefix);
            let evidence = self.follow_ids(*evidence);
            if prefix.len() != index || !held.starts_with(prefix) || *source.get_or_insert(evidence) != evidence {
                return None;
            }
        }
        let source = source?;
        (self.local_value_type(&source) == Some(self.instruction_result_type(id))).then_some(source)
    }

    /// If the `RestEvidence` at `id` reads a single row generic out of a known `MakeEvidence`,
    /// return the value the `MakeEvidence` used for that generic. This only works while the generic
    /// is unsubstituted. Afterward, the instruction reads every slot the generic was bound to,
    /// which isn't any single value.
    fn known_rest(&self, id: InstructionId, evidence: Value, prefix: &[EvidenceEntry], depth: u32) -> Option<Value> {
        let result = self.instruction_result_type(id);
        if !result.is_single_rest() {
            return None;
        }
        let (source, ahead) = self.spliced_rest(evidence, prefix, depth)?;
        (ahead.is_empty() && self.local_value_type(&source) == Some(result)).then_some(source)
    }

    /// If `evidence` is a known `MakeEvidence`, return the value it was built from which holds the
    /// row generic after `prefix`, along with the entries ahead of that row generic within the
    /// returned value.
    fn spliced_rest(&self, evidence: Value, prefix: &[EvidenceEntry], depth: u32) -> Option<(Value, &[EvidenceEntry])> {
        let Value::InstructionResult(id) = self.resolve_with_depth(evidence, depth + 1) else { return None };
        let Instruction::MakeEvidence(items) = &self.instructions[id] else { return None };
        let mut ahead = evidence::rest_count(prefix);
        for item in items {
            let EvidenceItem::Splice(spliced) = item else { continue };
            let Some(Type::Evidence(entries)) = self.local_value_type(spliced) else { return None };
            let held = evidence::rest_count(entries);
            if ahead < held {
                let index = evidence::rest_index(entries, ahead)?;
                return Some((*spliced, &entries[..index]));
            }
            ahead -= held;
        }
        None
    }

    /// Returns the innermost evidence value we know holds the row generic after `prefix` in
    /// `evidence`
    pub(crate) fn rest_source(&self, evidence: Value, prefix: &[EvidenceEntry]) -> Value {
        let (mut evidence, mut prefix) = (evidence, Cow::Borrowed(prefix));
        for _ in 0..MAX_RESOLVE_DEPTH {
            if let Some((source, ahead)) = self.spliced_rest(evidence, &prefix, 0) {
                (evidence, prefix) = (source, Cow::Borrowed(ahead));
            } else if let Value::InstructionResult(id) = self.resolve(evidence)
                && let Instruction::RestEvidence { evidence: inner, prefix: inner_prefix } = &self.instructions[id]
            {
                (evidence, prefix) = (*inner, Cow::Owned([evidence::entries(inner_prefix), &prefix].concat()));
            } else {
                break;
            }
        }
        evidence
    }

    /// The function `value` is statically known to be, if any. See [KnownFunction].
    pub(crate) fn known_function(&self, value: Value) -> Option<KnownFunction> {
        let value = self.resolve(value);
        let (function, environment, closure) = match value {
            Value::InstructionResult(id) => match &self.instructions[id] {
                Instruction::PackClosure { function, environment } => {
                    (self.resolve(*function), Some(*environment), Some(id))
                },
                _ => (value, None, None),
            },
            _ => (value, None, None),
        };
        let (id, bindings) = self.definition_of(function)?;
        Some(KnownFunction { id, bindings, environment, closure })
    }

    /// True if `block` can be reached again after it is executed
    pub(crate) fn is_in_loop(&self, block: BlockId) -> bool {
        self.reachable_from(block).contains(&block)
    }

    /// Every block reachable from `block`'s successors. Includes `block` only if it's in a loop.
    pub(crate) fn reachable_from(&self, block: BlockId) -> FxHashSet<BlockId> {
        let mut visited = FxHashSet::default();
        let mut stack = Vec::new();
        let push_successors = |block: BlockId, stack: &mut Vec<BlockId>| {
            if let Some(terminator) = &self.blocks[block].terminator {
                terminator.for_each_successor(|target| stack.push(target));
            }
        };
        push_successors(block, &mut stack);
        while let Some(next) = stack.pop() {
            if visited.insert(next) {
                push_successors(next, &mut stack);
            }
        }
        visited
    }

    /// Replace each instruction which [Definition::resolve]s to another value with an `Id` of it.
    pub(crate) fn fold_known_values(&mut self) {
        let ids: Vec<_> = self.block_instructions().collect();
        for id in ids {
            let this = Value::InstructionResult(id);
            let folded = match &self.instructions[id] {
                Instruction::LookupEvidence { evidence, key, prefix } => {
                    match self.known_slot(*evidence, key.effect, evidence::entries(prefix), 0) {
                        KnownSlot::Capability(capability) => Some(Instruction::Id(self.resolve(capability))),
                        // If we know this lookup really reads a slot of some other evidence, read
                        // from that evidence directly
                        KnownSlot::In(source, ahead) if source != *evidence => {
                            let prefix = Type::Evidence(Arc::new(ahead.into_owned()));
                            Some(Instruction::LookupEvidence { evidence: source, key: key.clone(), prefix })
                        },
                        KnownSlot::In(..) => None,
                    }
                },
                Instruction::IndexTuple { .. }
                | Instruction::RestEvidence { .. }
                | Instruction::MakeEvidence(_)
                | Instruction::Deref(_) => Some(Instruction::Id(self.resolve(this))),
                _ => None,
            };
            if let Some(folded) = folded
                && !matches!(folded, Instruction::Id(resolved) if resolved == this)
            {
                self.instructions[id] = folded;
            }
        }
    }

    /// Instructions (transitively) used by a terminator or by an instruction in a block satisfying `is_root`
    fn used_instructions(&self, is_root: impl Fn(&Instruction) -> bool) -> FxHashSet<InstructionId> {
        let mut stack = Vec::new();
        let push_operand = |value: &Value, stack: &mut Vec<InstructionId>| {
            if let Value::InstructionResult(id) = value {
                stack.push(*id);
            }
        };
        for block in self.blocks.values() {
            stack.extend(block.instructions.iter().copied().filter(|id| is_root(&self.instructions[*id])));
            if let Some(terminator) = &block.terminator {
                terminator.for_each_value(|value| push_operand(value, &mut stack));
            }
        }
        let mut used = FxHashSet::default();
        while let Some(id) = stack.pop() {
            if used.insert(id) {
                self.instructions[id].for_each_value(|value| push_operand(value, &mut stack));
            }
        }
        used
    }

    /// Instructions in blocks which are (transitively) used by an impure instruction or terminator
    pub(crate) fn live_instructions(&self) -> FxHashSet<InstructionId> {
        self.used_instructions(|instruction| !is_pure(instruction))
    }

    /// Remove pure instructions whose results are unused
    pub(crate) fn remove_dead_instructions(&mut self) {
        let live = self.live_instructions();
        self.remove_dead_instructions_given(&live);
    }

    /// Remove each instruction not in `live`, the result of [Definition::live_instructions]
    pub(crate) fn remove_dead_instructions_given(&mut self, live: &FxHashSet<InstructionId>) {
        for block in self.blocks.values_mut() {
            block.instructions.retain(|id| live.contains(id));
        }
    }

    /// Replace every use of `find` with `replace`
    pub(crate) fn replace_value(&mut self, find: Value, replace: Value) {
        self.for_each_used_value_mut(|value| {
            if *value == find {
                *value = replace;
            }
        });
    }

    /// Merge each block ending in an unconditional jump to a block with no other predecessors
    /// into that block, then remove any blocks which are unreachable. Merge points of `If`s
    /// and `Switch`es count as reachable since [Definition::topological_sort] visits them.
    pub(crate) fn simplify_cfg(&mut self) {
        self.merge_jump_chains();
        self.remove_unreachable_blocks();
    }

    fn merge_jump_chains(&mut self) {
        // Merging a block's only predecessor into it leaves the predecessors of every other
        // block, and the set of merge points, unchanged
        let mut predecessors: FxHashMap<BlockId, usize> = FxHashMap::default();
        let mut merge_points = FxHashSet::default();
        for block in self.blocks.values() {
            if let Some(terminator) = &block.terminator {
                terminator.for_each_successor(|target| *predecessors.entry(target).or_default() += 1);
                merge_points.extend(terminator.merge_block());
            }
        }

        // Each merged block's parameter maps to its argument. These are substituted all at once
        // afterward rather than rescanning the definition for each merge.
        let mut substitutions: FxHashMap<Value, Value> = FxHashMap::default();
        for block in self.blocks.iter().map(|(id, _)| id).collect::<Vec<_>>() {
            while let Some(TerminatorInstruction::Jmp((target, argument))) = &self.blocks[block].terminator {
                let (target, argument) = (*target, *argument);
                let mergeable = target != block
                    && target != BlockId::ENTRY_BLOCK
                    && predecessors.get(&target) == Some(&1)
                    && !merge_points.contains(&target)
                    && self.blocks[target].parameter_types.len() == usize::from(argument.is_some());
                if !mergeable {
                    break;
                }
                if let Some(argument) = argument {
                    substitutions.insert(Value::Parameter(target, 0), argument);
                }
                let instructions = std::mem::take(&mut self.blocks[target].instructions);
                let terminator = self.blocks[target].terminator.take();
                self.blocks[target].parameter_types.clear();
                // Leave the now-empty target unreachable so it is removed later
                self.blocks[target].terminator = Some(TerminatorInstruction::Unreachable);
                self.blocks[block].instructions.extend(instructions);
                self.blocks[block].terminator = terminator;
            }
        }
        if !substitutions.is_empty() {
            self.for_each_used_value_mut(|value| {
                // An argument may itself be the parameter of a merged block
                while let Some(new) = substitutions.get(value) {
                    *value = *new;
                }
            });
        }
    }

    /// Remove blocks which are unreachable from the entry block. Merge blocks of reachable `If`s
    /// and `Switch`es are always kept since [Definition::topological_sort] visits them, but if
    /// they are themselves unreachable they are emptied: their instructions may refer to values
    /// of removed blocks.
    pub(crate) fn remove_unreachable_blocks(&mut self) {
        let mut reachable = FxHashSet::default();
        let mut stack = vec![BlockId::ENTRY_BLOCK];
        while let Some(block) = stack.pop() {
            if !reachable.insert(block) {
                continue;
            }
            if let Some(terminator) = &self.blocks[block].terminator {
                terminator.for_each_successor(|target| stack.push(target));
            }
        }
        let stubs: FxHashSet<BlockId> = reachable
            .iter()
            .filter_map(|block| self.blocks[*block].terminator.as_ref()?.merge_block())
            .filter(|block| !reachable.contains(block))
            .collect();
        for stub in &stubs {
            self.blocks[*stub].instructions.clear();
            self.blocks[*stub].terminator = Some(TerminatorInstruction::Unreachable);
        }
        reachable.extend(stubs);
        if reachable.len() == self.blocks.len() {
            return;
        }

        let mut mapping: FxHashMap<BlockId, BlockId> = FxHashMap::default();
        for (old_id, block) in std::mem::take(&mut self.blocks) {
            if reachable.contains(&old_id) {
                mapping.insert(old_id, self.blocks.push(block));
            }
        }
        self.remap_blocks(&mapping);
    }

    /// Rename every block reference according to `mapping`. Values referring to unmapped
    /// blocks (which must be dead) are left as-is.
    fn remap_blocks(&mut self, mapping: &FxHashMap<BlockId, BlockId>) {
        self.for_each_used_value_mut(|value| {
            if let Value::Parameter(block, _) = value
                && let Some(new) = mapping.get(block)
            {
                *block = *new;
            }
        });
        for terminator in self.blocks.values_mut().filter_map(|block| block.terminator.as_mut()) {
            terminator.for_each_block_mut(|block| *block = mapping[block]);
        }
    }

    /// Drop every instruction which is neither in a block nor (transitively) used by one, then
    /// renumber the remaining instructions densely, each after every instruction it uses. Some
    /// passes visit instructions in id order and rely on this, see e.g. monomorphization.
    pub(crate) fn compact_instructions(&mut self) {
        let order = self.operands_first_order();
        let unchanged = order.len() == self.instructions.len()
            && order.iter().enumerate().all(|(index, id)| usize::from(*id) == index);
        if unchanged {
            return;
        }

        let mut old_instructions: Vec<_> = std::mem::take(&mut self.instructions).into_iter().map(Some).collect();
        let mut old_types: Vec<_> = std::mem::take(&mut self.instruction_result_types).into_iter().map(Some).collect();
        let mut mapping = vec![None; old_instructions.len()];
        for old in order {
            let (_, instruction) = old_instructions[usize::from(old)].take().expect("instruction ordered twice");
            let (_, typ) = old_types[usize::from(old)].take().expect("instruction ordered twice");
            mapping[usize::from(old)] = Some(self.add_instruction(instruction, typ));
        }
        let map = |id: &mut InstructionId| *id = mapping[usize::from(*id)].expect("use of a removed instruction");
        self.for_each_used_value_mut(|value| {
            if let Value::InstructionResult(id) = value {
                map(id);
            }
        });
        for block in self.blocks.values_mut() {
            block.instructions.iter_mut().for_each(map);
        }
    }

    /// Every instruction in a block or (transitively) used by one, in block order except that each
    /// instruction comes after every instruction it uses
    fn operands_first_order(&self) -> Vec<InstructionId> {
        let mut visited = vec![false; self.instructions.len()];
        let mut order = Vec::with_capacity(self.instructions.len());
        // `true` once an instruction's operands are ordered, so it can be ordered itself
        let mut stack: Vec<(InstructionId, bool)> = Vec::new();
        let mut visit = |root: InstructionId, order: &mut Vec<InstructionId>| {
            stack.push((root, false));
            while let Some((id, operands_ordered)) = stack.pop() {
                if operands_ordered {
                    order.push(id);
                    continue;
                }
                if std::mem::replace(&mut visited[usize::from(id)], true) {
                    continue;
                }
                stack.push((id, true));
                self.instructions[id].for_each_value(|value| {
                    if let Value::InstructionResult(operand) = value
                        && !visited[usize::from(*operand)]
                    {
                        stack.push((*operand, false));
                    }
                });
            }
        };
        for block in self.blocks.values() {
            for id in &block.instructions {
                visit(*id, &mut order);
            }
            if let Some(terminator) = &block.terminator {
                terminator.for_each_value(|value| {
                    if let Value::InstructionResult(id) = value {
                        visit(*id, &mut order);
                    }
                });
            }
        }
        order
    }

    /// Run the cleanups needed after restructuring a definition's blocks
    pub(crate) fn cleanup_after_restructuring(&mut self) {
        self.fold_known_values();
        self.remove_dead_instructions();
        self.simplify_cfg();
        self.compact_instructions();
    }
}

/// Describes where the capability in an evidence slot comes from, as far as we can tell statically
enum KnownSlot<'a> {
    Capability(Value),
    /// The capability is in the slot of this evidence value which comes after these entries
    In(Value, Cow<'a, [EvidenceEntry]>),
}

impl KnownSlot<'_> {
    fn into_owned(self) -> KnownSlot<'static> {
        match self {
            KnownSlot::Capability(capability) => KnownSlot::Capability(capability),
            KnownSlot::In(evidence, ahead) => KnownSlot::In(evidence, Cow::Owned(ahead.into_owned())),
        }
    }
}
