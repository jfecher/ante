//! Mir Inlining passes:
//!
//! - [Mir::inline_small_functions] inlines functions under [MAX_INSTRUCTIONS] instructions. This
//!   pass is very important for performance when lowering generics via existentialization.
//! - [Mir::inline_higher_order_functions] inlines functions which are passed a statically known
//!   function value, before effects are lowered.
//!
//! Both are currently performed on `-O0` since they're important for existentialization and loop
//! performance.
use std::{borrow::Cow, sync::Arc};

use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    iterator_extensions::mapvec,
    mir::{
        Block, BlockId, ConstantEnvironment, Definition, DefinitionId, Definitions, GenericBindings, Instruction,
        InstructionId, Mir, TerminatorInstruction, Type, Value, is_generic,
    },
};

/// Functions with at most this many instructions are inlined by [Mir::inline_small_functions]
const MAX_INSTRUCTIONS: usize = 8;

/// Maximum number of recursive calls to do since inlining can expose more calls to inline.
/// This is just meant to be a quick pass in -O0 for existentialization so we keep this small.
const ROUNDS: usize = 2;

/// Callees with at most this many instructions may be inlined by
/// [Mir::inline_higher_order_functions] when a function value is statically known at the call site.
const HIGHER_ORDER_MAX_INSTRUCTIONS: usize = 64;

/// Rounds of [Mir::inline_higher_order_functions]. Each round may expose more known function values.
const HIGHER_ORDER_ROUNDS: usize = 4;

/// [Mir::inline_higher_order_functions] stops inlining into a definition once it grows past this
/// many instructions
const MAX_CALLER_INSTRUCTIONS: usize = 4000;

impl Mir {
    pub(crate) fn inline_small_functions(self) -> Mir {
        self.inline_in_rounds(
            ROUNDS,
            usize::MAX,
            |definition, mir| is_small(definition) && can_inline(definition, mir, MAX_INSTRUCTIONS),
            |_, _, _, _| true,
            false,
        )
    }

    /// Inline calls to (non-recursive, reasonably small) functions which are passed a known
    /// function value. Notably this lets [Mir::optimize_local_handlers] see effect calls as
    /// local, which is important for optimizing for loops.
    ///
    /// This runs before effects are lowered, so callees may still be generic.
    pub(crate) fn inline_higher_order_functions(self) -> Mir {
        let cycles = Cycles::new(&self);
        let should_inline = |caller: &Definition, inlinee: &Inlinee, arguments: &[Value], mir: &Mir| {
            let is_known = |value| is_known_function_value(caller, value, mir);
            let passed_known_function = arguments.iter().any(|argument| is_known(*argument))
                || matches!(inlinee.environment, Some(Environment::Value(environment)) if is_known(environment));
            // A Handle is as visible to [Mir::optimize_local_handlers] in the callee as it would
            // be in the caller, so inlining it would only duplicate it
            let has_handle = || {
                inlinee
                    .callee
                    .instructions
                    .values()
                    .any(|instruction| matches!(instruction, Instruction::Handle { .. }))
            };
            // Inlining a callee into a definition it refers to would create a cycle
            !caller.is_global()
                && !cycles.same(caller.id, inlinee.callee.id)
                && (passed_known_function || (is_small(inlinee.callee) && !has_handle()))
        };
        self.inline_in_rounds(
            HIGHER_ORDER_ROUNDS,
            MAX_CALLER_INSTRUCTIONS,
            |definition, mir| can_inline(definition, mir, HIGHER_ORDER_MAX_INSTRUCTIONS),
            should_inline,
            true,
        )
    }

    /// Run up to `rounds` rounds of inlining. Each call to a callee satisfying `is_eligible` is
    /// inlined if `should_inline` approves, until the caller has `max_caller_instructions`. If
    /// `cleanup`, each changed caller is cleaned up afterward, which is required if a callee has
    /// multiple blocks.
    fn inline_in_rounds(
        mut self, rounds: usize, max_caller_instructions: usize,
        is_eligible: impl Fn(&Definition, &Mir) -> bool + Sync,
        should_inline: impl Fn(&Definition, &Inlinee, &[Value], &Mir) -> bool + Sync, cleanup: bool,
    ) -> Mir {
        let mut eligible: FxHashSet<DefinitionId> = self
            .definitions
            .par_iter()
            .filter(|(_, definition)| is_eligible(definition, &self))
            .map(|(id, _)| *id)
            .collect();

        // After the first round only definitions that changed can have new calls to inline
        let mut changed: Option<Vec<DefinitionId>> = None;
        for _ in 0..rounds {
            let plan = |id: &DefinitionId| {
                let replacements =
                    plan_inlining(&self.definitions[id], &self, &eligible, max_caller_instructions, &should_inline);
                (!replacements.is_empty()).then_some((*id, replacements))
            };
            let plans: Vec<_> = match &changed {
                None => self.definitions.par_iter().filter_map(|(id, _)| plan(id)).collect(),
                Some(ids) => ids.par_iter().filter_map(plan).collect(),
            };
            if plans.is_empty() {
                break;
            }
            let ids = self.apply_replacements(plans, cleanup);

            // Inlining into a definition may change whether it is eligible itself
            let updates: Vec<_> = ids.par_iter().map(|id| (*id, is_eligible(&self.definitions[id], &self))).collect();
            for (id, is_eligible) in updates {
                if is_eligible {
                    eligible.insert(id);
                } else {
                    eligible.remove(&id);
                }
            }
            changed = Some(ids);
        }
        if changed.is_some() { self.remove_unreachable_functions() } else { self }
    }

    /// Apply each definition's replacements, which must be in block order. If `cleanup`, each
    /// definition is cleaned up afterward, see [Definition::cleanup_after_restructuring].
    /// Returns the id of each definition changed.
    pub(crate) fn apply_replacements(
        &mut self, plans: Vec<(DefinitionId, Vec<Replacement>)>, cleanup: bool,
    ) -> Vec<DefinitionId> {
        // Take each definition out of the Mir so they can be changed in parallel
        let mut changes = mapvec(plans, |(id, replacements)| {
            (self.definitions.remove(&id).expect("replacements for a missing definition"), replacements)
        });
        changes.par_iter_mut().for_each(|(definition, replacements)| {
            // From the end of each block so the positions of the remaining replacements stay valid
            for replacement in std::mem::take(replacements).into_iter().rev() {
                replacement.apply(definition);
            }
            if cleanup {
                definition.cleanup_after_restructuring();
            }
        });
        mapvec(changes, |(definition, _)| {
            let id = definition.id;
            self.definitions.insert(id, definition);
            id
        })
    }
}

/// Copy each callee to inline into `caller` into a replacement of its call, in block order
fn plan_inlining(
    caller: &Definition, mir: &Mir, eligible: &FxHashSet<DefinitionId>, max_caller_instructions: usize,
    should_inline: &impl Fn(&Definition, &Inlinee, &[Value], &Mir) -> bool,
) -> Vec<Replacement> {
    let mut size = caller.instructions.len();
    let mut replacements = Vec::new();
    for (block, block_data) in caller.blocks.iter() {
        for (index, id) in block_data.instructions.iter().enumerate() {
            if size > max_caller_instructions {
                return replacements;
            }
            let Some((inlinee, arguments)) = resolve_callee(caller, &caller.instructions[*id], mir) else { continue };
            if eligible.contains(&inlinee.callee.id) && should_inline(caller, &inlinee, arguments, mir) {
                let replacement = inlinee.replacement(caller, block, index, arguments, &mir.definitions);
                size += replacement.body.instructions.len();
                replacements.push(replacement);
            }
        }
    }
    replacements
}

fn is_small(definition: &Definition) -> bool {
    !definition.is_global()
        && definition.blocks.len() == 1
        && definition.instructions.len() <= MAX_INSTRUCTIONS
        && matches!(definition.entry_block().terminator, Some(TerminatorInstruction::Return(_)))
}

/// True if `callee` may be inlined into another function
pub(crate) fn can_inline(callee: &Definition, mir: &Mir, max_instructions: usize) -> bool {
    callee.instructions.len() <= max_instructions
        && can_copy_body(callee)
        && !callee.instructions.values().any(|instruction| matches!(instruction, Instruction::Capability))
        && !refers_to_itself(callee)
        // Bare references to generic definitions are made explicit when inlining (see
        // [copy_body]), which requires the callee to have generics of its own to substitute
        && (callee.generic_count > 0 || callee.implicit_instantiations(&mir.definitions).is_empty())
}

/// Referring to itself would inline forever
fn refers_to_itself(callee: &Definition) -> bool {
    let mut found = false;
    callee.for_each_referenced_definition(|id| found |= id == callee.id);
    found
}

/// True if [copy_body] can copy `definition`
pub(crate) fn can_copy_body(definition: &Definition) -> bool {
    !definition.is_global()
        && !entry_is_jump_target(definition)
        && definition.blocks.values().all(|block| block.terminator.is_some())
}

fn entry_is_jump_target(definition: &Definition) -> bool {
    definition.blocks.values().any(|block| {
        let mut found = false;
        if let Some(terminator) = &block.terminator {
            terminator.for_each_successor(|target| found |= target == BlockId::ENTRY_BLOCK);
        }
        found
    })
}

/// The cycles in the graph of definitions referring to each other
pub(crate) struct Cycles {
    /// The strongly connected component of each definition
    components: FxHashMap<DefinitionId, u32>,
    /// Whether each component is a cycle: it has multiple definitions or one referring to itself
    is_cycle: Vec<bool>,
}

impl Cycles {
    /// Inlining never creates a cycle (a caller already refers to each callee inlined into it),
    /// so these remain valid as long as no new definitions are inlined
    pub(crate) fn new(mir: &Mir) -> Self {
        let ids: Vec<DefinitionId> = mir.definitions.keys().copied().collect();
        let indices: FxHashMap<DefinitionId, u32> =
            ids.iter().enumerate().map(|(index, id)| (*id, index as u32)).collect();
        let edges: Vec<Vec<u32>> = ids
            .par_iter()
            .map(|id| {
                let mut edges = Vec::new();
                mir.definitions[id].for_each_referenced_definition(|other| edges.extend(indices.get(&other)));
                edges.sort_unstable();
                edges.dedup();
                edges
            })
            .collect();
        let (components, is_cycle) = strongly_connected_components(&edges);
        Self { components: ids.into_iter().zip(components).collect(), is_cycle }
    }

    /// True if `a` and `b` may (transitively) refer to each other
    pub(crate) fn same(&self, a: DefinitionId, b: DefinitionId) -> bool {
        matches!((self.components.get(&a), self.components.get(&b)), (Some(a), Some(b)) if a == b)
    }

    /// True if `id` may (transitively) refer to itself
    pub(crate) fn is_recursive(&self, id: DefinitionId) -> bool {
        self.components.get(&id).is_some_and(|component| self.is_cycle[*component as usize])
    }
}

/// Tarjan's algorithm. Returns the component of each node, and whether each component is a cycle.
/// `edges` must be sorted.
fn strongly_connected_components(edges: &[Vec<u32>]) -> (Vec<u32>, Vec<bool>) {
    const UNVISITED: u32 = u32::MAX;
    let count = edges.len();
    let mut index = vec![UNVISITED; count];
    let mut lowlink = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut components = vec![UNVISITED; count];
    let mut is_cycle = Vec::new();
    let mut stack = Vec::new();
    let mut next_index = 0;
    // Each node being visited along with the index of its next edge to follow
    let mut path: Vec<(usize, usize)> = Vec::new();

    macro_rules! visit {
        ($node: expr) => {{
            let node = $node;
            index[node] = next_index;
            lowlink[node] = next_index;
            next_index += 1;
            stack.push(node);
            on_stack[node] = true;
            path.push((node, 0));
        }};
    }

    for root in 0..count {
        if index[root] != UNVISITED {
            continue;
        }
        visit!(root);
        while let Some(top) = path.last_mut() {
            let node = top.0;
            if let Some(&next) = edges[node].get(top.1) {
                top.1 += 1;
                let next = next as usize;
                if index[next] == UNVISITED {
                    visit!(next);
                } else if on_stack[next] {
                    lowlink[node] = lowlink[node].min(index[next]);
                }
                continue;
            }
            path.pop();
            if let Some(&(parent, _)) = path.last() {
                lowlink[parent] = lowlink[parent].min(lowlink[node]);
            }
            if lowlink[node] == index[node] {
                let component = is_cycle.len() as u32;
                let mut size = 0;
                loop {
                    let member = stack.pop().expect("component root is on the stack");
                    on_stack[member] = false;
                    components[member] = component;
                    size += 1;
                    if member == node {
                        break;
                    }
                }
                is_cycle.push(size > 1 || edges[node].binary_search(&(node as u32)).is_ok());
            }
        }
    }
    (components, is_cycle)
}

/// A callee to inline
pub(crate) struct Inlinee<'a> {
    pub(crate) callee: &'a Definition,
    /// `None` if the callee shares the caller's generics
    bindings: Option<Arc<GenericBindings>>,

    /// The closure environment, if any
    environment: Option<Environment>,
}

enum Environment {
    Value(Value),
    /// A constant transmuted to the environment's type
    /// TODO: These complicate things, we should initalize globals before main instead of forcing
    /// them all into constants
    Transmuted(Value, Type),
}

impl Inlinee<'_> {
    /// Copy the callee into a replacement of the call at `index` of `block` in `caller`, which
    /// passes it `arguments`
    pub(crate) fn replacement(
        &self, caller: &Definition, block: BlockId, index: usize, arguments: &[Value], definitions: &Definitions,
    ) -> Replacement {
        let mut body = Definition::new(self.callee.name.clone(), caller.id, caller.generic_count, Type::ERROR);
        let mut imports = Imports::default();
        let mut parameters = mapvec(arguments, |argument| imports.import(&mut body, caller, *argument));
        if self.callee.entry_block().parameter_types.len() == arguments.len() + 1 {
            parameters.push(match &self.environment {
                Some(Environment::Value(value)) => imports.import(&mut body, caller, *value),
                Some(Environment::Transmuted(constant, typ)) => {
                    body.push_instruction_in(BlockId::ENTRY_BLOCK, Instruction::Transmute(*constant), typ.clone())
                },
                None => panic!("inlining a closure without its environment"),
            });
        }
        let substitution = self.bindings.as_ref().map(|bindings| Substitution { bindings, definitions });
        copy_body(
            &mut body,
            Cow::Borrowed(self.callee),
            substitution,
            BlockId::ENTRY_BLOCK,
            &parameters,
            |_, _, value| TerminatorInstruction::Return(value),
        );
        Replacement { block, index, body, arguments: imports.into_values() }
    }
}

/// Resolve the function called by `instruction`, if statically known. Returns the callee to
/// inline along with the call's arguments.
pub(crate) fn resolve_callee<'a, 'i>(
    caller: &Definition, instruction: &'i Instruction, mir: &'a Mir,
) -> Option<(Inlinee<'a>, &'i [Value])> {
    let (function, arguments) = match instruction {
        Instruction::Call { function, arguments } => (*function, arguments),
        Instruction::CallClosure { closure, arguments } => (*closure, arguments),
        _ => return None,
    };
    let (id, bindings, environment) = if let Some(known) = caller.known_function(function) {
        (known.id, known.bindings, known.environment.map(Environment::Value))
    } else if let Value::InstructionResult(result) = caller.resolve(function)
        && let Instruction::IndexTuple { tuple, index } = &caller.instructions[result]
    {
        dictionary_method(caller, &caller.resolve(*tuple), *index, mir)?
    } else {
        return None;
    };
    let callee = mir.definitions.get(&id)?;
    if id == caller.id {
        return None;
    }
    let expected = callee.entry_block().parameter_types.len();
    let has_environment_parameter = expected == arguments.len() + 1;
    if expected != arguments.len() && !(has_environment_parameter && environment.is_some()) {
        return None;
    }
    Some((Inlinee { callee, bindings, environment }, arguments))
}

/// The function stored in a constant global. This is usually a trait impl method
fn dictionary_method(
    caller: &Definition, tuple: &Value, index: u32, mir: &Mir,
) -> Option<(DefinitionId, Option<Arc<GenericBindings>>, Option<Environment>)> {
    let (global, outer) = caller.definition_of(*tuple)?;
    let outer = outer.unwrap_or_default();
    let global = mir.definitions.get(&global)?;
    if !global.is_global() || global.blocks.len() != 1 || outer.len() != global.generic_count as usize {
        return None;
    }
    let (function, environment) = global.constant_function_field(index)?;
    let (function, bindings) = match global.definition_of(function)? {
        (id, None) if !is_generic(&mir.definitions, &id) => (id, None),
        (id, Some(bindings)) => (id, Some(Arc::new(mapvec(bindings.iter(), |binding| binding.substitute(&outer))))),
        _ => return None,
    };
    let environment = match environment {
        None => None,
        Some(ConstantEnvironment::Constant(constant)) => Some(Environment::Value(constant)),
        Some(ConstantEnvironment::Transmuted(constant, id)) => {
            Some(Environment::Transmuted(constant, global.instruction_result_type(id).substitute(&outer)))
        },
        Some(ConstantEnvironment::Runtime) => return None,
    };
    Some((function, bindings, environment))
}

/// True if `value` is statically known to be a function (possibly a closure) or a constant
/// global containing functions, such as a trait dictionary.
fn is_known_function_value(caller: &Definition, value: Value, mir: &Mir) -> bool {
    let contains_function = |id: &DefinitionId| {
        mir.definitions.get(id).is_some_and(|definition| match &definition.typ {
            Type::Function(_) => true,
            Type::Tuple(fields) => fields.iter().any(|field| matches!(field, Type::Function(_))),
            _ => false,
        })
    };
    match caller.resolve(value) {
        Value::Definition(id) => contains_function(&id),
        Value::InstructionResult(id) => match &caller.instructions[id] {
            Instruction::Instantiate(id, _) => contains_function(id),
            Instruction::PackClosure { function, .. } => is_known_function_value(caller, *function, mir),
            _ => false,
        },
        _ => false,
    }
}

/// Code to replace an instruction with. Its body is built separately from the definition
/// containing the instruction, then moved into it by [Replacement::apply].
pub(crate) struct Replacement {
    pub(crate) block: BlockId,
    pub(crate) index: usize,
    /// Code computing the replaced instruction's result, which it returns. Its entry block's
    /// parameters are [Imports].
    pub(crate) body: Definition,
    /// The value passed as each of `body`'s parameters
    pub(crate) arguments: Vec<Value>,
}

impl Replacement {
    /// Move the body into `definition` in place of the instruction it replaces. The instruction
    /// becomes an `Id` of the body's result so its uses remain valid.
    pub(crate) fn apply(self, definition: &mut Definition) {
        let Replacement { block, index, body, arguments } = self;
        let replaced = definition.blocks[block].instructions[index];
        let single_block =
            body.blocks.len() == 1 && matches!(body.entry_block().terminator, Some(TerminatorInstruction::Return(_)));
        if single_block {
            // Splice the body in just before the replaced instruction
            let rest = definition.blocks[block].instructions.split_off(index);
            let terminator = definition.blocks[block].terminator.take();
            let mut result = Value::Unit;
            copy_body(definition, Cow::Owned(body), None, block, &arguments, |_, _, value| {
                result = value;
                TerminatorInstruction::Unreachable
            });
            definition.instructions[replaced] = Instruction::Id(result);
            definition.blocks[block].instructions.extend(rest);
            definition.blocks[block].terminator = terminator;
        } else {
            let continuation = definition.split_at_instruction(block, index);
            copy_body(definition, Cow::Owned(body), None, block, &arguments, |_, _, value| {
                TerminatorInstruction::jmp(continuation, value)
            });
        }
    }
}

/// The values of an enclosing definition which a [Replacement] body uses. Each is a parameter of
/// the body's entry block.
#[derive(Default)]
pub(crate) struct Imports {
    values: Vec<Value>,
    /// The copy of each instruction copied by [Imports::import_known]
    copies: FxHashMap<InstructionId, Value>,
}

/// How many instructions deep [Imports::import_known] copies
const MAX_IMPORT_DEPTH: u32 = 16;

impl Imports {
    /// Like [Imports::import], except the pure instructions computing `value` are copied into
    /// the end of `body`'s entry block so that it can still be resolved statically, see
    /// [Definition::resolve].
    pub(crate) fn import_known(&mut self, body: &mut Definition, outer: &Definition, value: Value) -> Value {
        self.import_known_with_depth(body, outer, value, 0)
    }

    fn import_known_with_depth(
        &mut self, body: &mut Definition, outer: &Definition, value: Value, depth: u32,
    ) -> Value {
        let Value::InstructionResult(id) = value else { return self.import(body, outer, value) };
        if let Some(copy) = self.copies.get(&id) {
            return *copy;
        }
        let instruction = &outer.instructions[id];
        let copyable = matches!(
            instruction,
            Instruction::Id(_)
                | Instruction::IndexTuple { .. }
                | Instruction::MakeTuple(_)
                | Instruction::MakeEvidence { .. }
                | Instruction::LookupEvidence { .. }
                | Instruction::PackClosure { .. }
                | Instruction::Instantiate(..)
        );
        if !copyable || depth >= MAX_IMPORT_DEPTH {
            return self.import(body, outer, value);
        }
        let mut instruction = instruction.clone();
        instruction.for_each_value_mut(|operand| {
            *operand = self.import_known_with_depth(body, outer, *operand, depth + 1);
        });
        let typ = outer.instruction_result_type(id).clone();
        let copy = body.push_instruction_in(BlockId::ENTRY_BLOCK, instruction, typ);
        self.copies.insert(id, copy);
        copy
    }

    /// The value of `body` standing for `outer`'s `value`. `body`'s entry block may have no
    /// parameters other than imports.
    pub(crate) fn import(&mut self, body: &mut Definition, outer: &Definition, value: Value) -> Value {
        if !matches!(value, Value::InstructionResult(_) | Value::Parameter(..)) {
            return value;
        }
        let index = self.values.iter().position(|import| *import == value).unwrap_or_else(|| {
            let typ = outer.local_value_type(&value).cloned().unwrap_or(Type::ERROR);
            body.blocks[BlockId::ENTRY_BLOCK].parameter_types.push(typ);
            self.values.push(value);
            self.values.len() - 1
        });
        Value::Parameter(BlockId::ENTRY_BLOCK, index as u32)
    }

    pub(crate) fn into_values(self) -> Vec<Value> {
        self.values
    }
}

/// Generic bindings for [copy_body] to substitute into the body it copies
#[derive(Clone, Copy)]
pub(crate) struct Substitution<'a> {
    pub(crate) bindings: &'a Arc<GenericBindings>,
    /// To find the generic definitions the body instantiates implicitly
    pub(crate) definitions: &'a Definitions,
}

/// The callee-to-caller mapping of values and blocks produced by [copy_body]
pub(crate) struct BodyCopy {
    /// The copy of each instruction, by callee instruction index
    instructions: Vec<Option<InstructionId>>,
    /// The copy of each block, by callee block index
    blocks: Vec<BlockId>,
    parameters: Vec<Value>,
    /// The `Instantiate` made for each generic definition the callee instantiated implicitly
    instantiations: Vec<(DefinitionId, InstructionId)>,
}

impl BodyCopy {
    /// The caller's copy of the callee's `value`
    pub(crate) fn value(&self, value: Value) -> Value {
        match value {
            Value::InstructionResult(id) => match self.instructions.get(usize::from(id)).copied().flatten() {
                Some(copy) => Value::InstructionResult(copy),
                None => {
                    debug_assert!(false, "copied a use of {value}, which is in no block");
                    value
                },
            },
            Value::Parameter(block, index) if block == BlockId::ENTRY_BLOCK => {
                self.parameters.get(index as usize).copied().unwrap_or(value)
            },
            Value::Parameter(block, index) => match self.blocks.get(usize::from(block)) {
                Some(block) => Value::Parameter(*block, index),
                None => value,
            },
            Value::Definition(id) => match self.instantiations.iter().find(|(target, _)| *target == id) {
                Some((_, instantiation)) => Value::InstructionResult(*instantiation),
                None => value,
            },
            _ => value,
        }
    }

    /// The caller's copy of the callee's `block`
    pub(crate) fn block(&self, block: BlockId) -> BlockId {
        self.blocks[usize::from(block)]
    }

    /// The caller's copy of the callee's instruction `id`
    pub(crate) fn instruction(&self, id: InstructionId) -> InstructionId {
        self.instructions[usize::from(id)].expect("instruction in no block was not copied")
    }

    /// Map `value` of the callee into the caller. A generic definition the callee instantiates
    /// implicitly is instantiated explicitly with the bindings of `instantiate` instead.
    fn map(&mut self, value: &mut Value, caller: &mut Definition, instantiate: Option<Substitution>) {
        let Value::Definition(id) = *value else {
            *value = self.value(*value);
            return;
        };
        let Some(Substitution { bindings, definitions }) = instantiate.filter(|s| is_generic(s.definitions, &id))
        else {
            return;
        };
        let instantiation = match self.instantiations.iter().find(|(target, _)| *target == id) {
            Some((_, instantiation)) => *instantiation,
            None => {
                let typ = definitions[&id].typ.substitute(bindings);
                let instantiation = caller.add_instruction(Instruction::Instantiate(id, bindings.clone()), typ);
                self.instantiations.push((id, instantiation));
                instantiation
            },
        };
        *value = Value::InstructionResult(instantiation);
    }
}

/// Copy every block of `callee` into `caller`, moving its contents if it is owned. The callee's
/// entry block instructions are appended to `entry` (which must not have a terminator yet) and its
/// parameters are replaced with `parameters`. Every other callee block is copied into a fresh
/// block. Each `Return(value)` in the callee is replaced with the terminator returned by
/// `on_return`, which is given the caller block containing the return and the copied value.
/// `on_return` may push additional instructions into that block.
///
/// If `substitution` is given its bindings are substituted into the callee's types, and any bare
/// reference to a generic definition (implicitly instantiated with the callee's own generics) is
/// instantiated with them explicitly.
pub(crate) fn copy_body(
    caller: &mut Definition, mut callee: Cow<Definition>, substitution: Option<Substitution>, entry: BlockId,
    parameters: &[Value], mut on_return: impl FnMut(&mut Definition, BlockId, Value) -> TerminatorInstruction,
) -> BodyCopy {
    let bindings = substitution.map(|substitution| &**substitution.bindings);
    let entry_start = caller.blocks[entry].instructions.len();
    let (mut copy, block_count, instantiate) = {
        let callee: &Definition = &callee;
        assert_eq!(parameters.len(), callee.entry_block().parameter_types.len(), "copy_body: parameter count mismatch");
        let mut blocks = Vec::with_capacity(callee.blocks.len());
        blocks.push(entry);
        for block in callee.blocks.values().skip(1) {
            let parameter_types = mapvec(&block.parameter_types, |typ| typ.substitute_optional(bindings));
            blocks.push(caller.blocks.push(Block::new(parameter_types)));
        }

        // Allocate every copy first, in the callee's order, since a block may use values of later blocks
        let mut in_block = vec![false; callee.instructions.len()];
        callee.block_instructions().for_each(|id| in_block[usize::from(id)] = true);
        let instructions = mapvec(in_block, |in_block| {
            in_block.then(|| caller.add_instruction(Instruction::Id(Value::Unit), Type::UNIT))
        });
        for (block, copy) in callee.blocks.values().zip(&blocks) {
            let copies = block.instructions.iter().map(|id| instructions[usize::from(*id)].unwrap());
            caller.blocks[*copy].instructions.extend(copies);
        }
        let instantiate = substitution.filter(|_| callee.generic_count > 0);
        let copy = BodyCopy { instructions, blocks, parameters: parameters.to_vec(), instantiations: Vec::new() };
        (copy, callee.blocks.len(), instantiate)
    };

    for old in 0..copy.instructions.len() {
        let Some(new) = copy.instructions[old] else { continue };
        let (mut instruction, typ) = take_instruction(&mut callee, InstructionId::from(old));
        instruction.for_each_value_mut(|value| copy.map(value, caller, instantiate));
        if let Some(bindings) = bindings {
            instruction.for_each_type_mut(|typ| *typ = typ.substitute(bindings));
        }
        caller.instructions[new] = instruction;
        caller.instruction_result_types[new] = match bindings {
            Some(bindings) => typ.substitute(bindings),
            None => typ,
        };
    }

    for old in 0..block_count {
        let new_block = copy.blocks[old];
        let terminator = match take_terminator(&mut callee, BlockId::from(old)) {
            TerminatorInstruction::Return(mut value) => {
                copy.map(&mut value, caller, instantiate);
                on_return(caller, new_block, value)
            },
            mut terminator => {
                terminator.for_each_value_mut(|value| copy.map(value, caller, instantiate));
                terminator.for_each_block_mut(|block| *block = copy.blocks[usize::from(*block)]);
                terminator
            },
        };
        caller.blocks[new_block].terminator = Some(terminator);
    }

    // An `Instantiate` has no operands, so placing each before the callee's entry block
    // instructions makes it available to every copied instruction
    let instantiations = copy.instantiations.iter().map(|(_, instantiation)| *instantiation);
    caller.blocks[entry].instructions.splice(entry_start..entry_start, instantiations);
    copy
}

/// `definition` (moved if owned) with the id `id` and `substitution`'s bindings substituted for its
/// generics, of which it then has `generic_count`
pub(crate) fn instantiate(
    definition: Cow<Definition>, id: DefinitionId, substitution: Substitution, generic_count: u32,
) -> Definition {
    let bindings = &**substitution.bindings;
    let mut instance = Definition::new(definition.name.clone(), id, generic_count, definition.typ.substitute(bindings));
    let entry = &definition.entry_block().parameter_types;
    let parameters = mapvec(0..entry.len() as u32, |index| Value::Parameter(BlockId::ENTRY_BLOCK, index));
    instance.blocks[BlockId::ENTRY_BLOCK].parameter_types = mapvec(entry, |typ| typ.substitute(bindings));
    copy_body(&mut instance, definition, Some(substitution), BlockId::ENTRY_BLOCK, &parameters, |_, _, value| {
        TerminatorInstruction::Return(value)
    });
    instance
}

/// Take the instruction `id` from `definition` along with its result type, copying it if borrowed
fn take_instruction(definition: &mut Cow<Definition>, id: InstructionId) -> (Instruction, Type) {
    match definition {
        Cow::Borrowed(definition) => {
            (definition.instructions[id].clone(), definition.instruction_result_types[id].clone())
        },
        Cow::Owned(definition) => (
            std::mem::replace(&mut definition.instructions[id], Instruction::Id(Value::Unit)),
            std::mem::replace(&mut definition.instruction_result_types[id], Type::UNIT),
        ),
    }
}

/// Take the terminator of `block` from `definition`, copying it if borrowed
fn take_terminator(definition: &mut Cow<Definition>, block: BlockId) -> TerminatorInstruction {
    let terminator = match definition {
        Cow::Borrowed(definition) => definition.blocks[block].terminator.clone(),
        Cow::Owned(definition) => definition.blocks[block].terminator.take(),
    };
    terminator.expect("copy_body: block without a terminator")
}
