//! Turns calls to effect operations whose handler is visible in the same function into direct
//! jumps to (an inlined copy of) that handler.
//!
//! A `handle` expression is lowered to an [Instruction::Handle] whose body and handler cases are
//! lambdas. When the Handle's body and cases are statically known (typically after inlining),
//! this pass builds a replacement for the Handle:
//!
//! 1. The body lambda is copied into the replacement and the body's capability becomes a local value.
//! 2. Calls which are passed the capability are inlined, and nested Handles optimized, so that effect
//!    calls become local, i.e. `closure-call cap.i args`.
//! 3. Each local effect call is replaced with a copy of the handler case for that
//!    operation. For this to happen, `resume` needs to be called only in a tail position,
//!    or not at all.
//! 4. Each remaining call which is passed the capability but could not be inlined  is wrapped
//!    in a small residual Handle scoped to just that call. They are only created if they cost
//!    no more than the original Handle, see [Site::residuals_are_cheap].
use std::{borrow::Cow, sync::Arc};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    effect_lowering::Emitter,
    tail_resume_optimization::{Resumes, analyze_resumes},
};
use crate::{
    iterator_extensions::{mapvec, opt_mapvec},
    mir::{
        Block, BlockId, Definition, DefinitionId, FunctionType, GenericBindings, HandlerCase, Instruction,
        InstructionId, Mir, TerminatorInstruction, Type, Value, identity_bindings,
        inline::{
            BodyCopy, Cycles, Imports, Replacement, Substitution, can_copy_body, can_inline, copy_body, resolve_callee,
        },
        next_definition_id,
    },
};

/// Maximum number of calls inlined per Handle to make its effect operation calls local
const MAX_INLINES_PER_SITE: usize = 64;

/// Callees larger than this are not inlined to make effect operation calls local
const MAX_CALLEE_INSTRUCTIONS: usize = 400;

/// The most instructions the replacement of a Handle may have, including the Handles optimized
/// within it. Each Handle has its own budget.
/// TODO: Adjust, this is very aggressive
const MAX_SITE_INSTRUCTIONS: usize = 8000;

/// Stop optimizing Handles in a definition once it would grow past this many instructions
/// TODO: Adjust, this is very aggressive
const MAX_DEFINITION_INSTRUCTIONS: usize = 64000;

/// Maximum depth of the Handles optimized while optimizing an enclosing Handle, whether nested in
/// its body or in the handlers it copies
const MAX_NESTING: u32 = 8;

/// Maximum number of rounds over newly created definitions
const MAX_ROUNDS: usize = 4;

impl Mir {
    pub(crate) fn optimize_local_handlers(mut self) -> Self {
        let mut candidates: Vec<DefinitionId> =
            self.definitions.par_iter().filter(|(_, definition)| has_handle(definition)).map(|(id, _)| *id).collect();
        if candidates.is_empty() {
            return self;
        }

        // Inlining a function which may recurse would only expose another call to inline
        let cycles = Cycles::new(&self);
        let is_inlinable = |id: DefinitionId, definition: &Definition, mir: &Mir| {
            !cycles.is_recursive(id) && can_inline(definition, mir, MAX_CALLEE_INSTRUCTIONS)
        };
        let mut inlinable: FxHashSet<DefinitionId> = self
            .definitions
            .par_iter()
            .filter(|(id, definition)| is_inlinable(**id, definition, &self))
            .map(|(id, _)| *id)
            .collect();
        let mut bodies = FxHashSet::default();

        for _ in 0..MAX_ROUNDS {
            if candidates.is_empty() {
                break;
            }
            let context = Context { mir: &self, inlinable: &inlinable };
            let optimized: Vec<_> = candidates
                .par_iter()
                .filter_map(|id| {
                    let handles = optimize_handles(&self.definitions[id], &context);
                    (!handles.is_empty()).then_some((*id, handles))
                })
                .collect();

            candidates.clear();
            let plans = mapvec(optimized, |(id, handles)| {
                let replacements = mapvec(handles, |handle| {
                    bodies.extend(handle.bodies);
                    for definition in handle.new_definitions {
                        // Residual handlers may contain Handles copied from the original handlers
                        if has_handle(&definition) {
                            candidates.push(definition.id);
                        }
                        self.definitions.insert(definition.id, definition);
                    }
                    handle.replacement
                });
                (id, replacements)
            });

            for id in self.apply_replacements(plans, true) {
                if is_inlinable(id, &self.definitions[&id], &self) {
                    inlinable.insert(id);
                } else {
                    inlinable.remove(&id);
                }
            }
        }
        self.remove_unused_bodies(&bodies)
    }

    /// The body lambdas of optimized Handles are unused unless another Handle uses them too.
    /// Nothing would lower their `Capability`s so remove them. Without a `main` to find unused
    /// definitions from they are neutralized instead, as [super::tail_resume_optimization] does.
    fn remove_unused_bodies(self, bodies: &FxHashSet<DefinitionId>) -> Self {
        if bodies.is_empty() {
            return self;
        }
        if self.has_main() {
            return self.remove_unreachable_functions();
        }
        let used: FxHashSet<DefinitionId> = self
            .definitions
            .par_iter()
            .flat_map_iter(|(_, definition)| {
                let mut used = Vec::new();
                definition.for_each_referenced_definition(|id| {
                    if bodies.contains(&id) {
                        used.push(id);
                    }
                });
                used
            })
            .collect();
        let mut mir = self;
        for id in bodies.difference(&used) {
            if let Some(body) = mir.definitions.get_mut(id) {
                body.neutralize_capabilities();
            }
        }
        mir
    }
}

pub(super) fn has_handle(definition: &Definition) -> bool {
    !definition.is_global()
        && definition.block_instructions().any(|id| matches!(definition.instructions[id], Instruction::Handle { .. }))
}

struct Context<'mir> {
    mir: &'mir Mir,
    /// The callees which may be inlined to make effect operation calls local
    inlinable: &'mir FxHashSet<DefinitionId>,
}

/// A Handle which was optimized
struct OptimizedHandle {
    replacement: Replacement,
    /// Residual handle bodies and handlers created for the Handle
    new_definitions: Vec<Definition>,
    /// The body lambdas copied into the replacement, which may now be unused
    bodies: Vec<DefinitionId>,
}

/// Optimize each Handle in `definition`, returning the optimized ones in block order
fn optimize_handles(definition: &Definition, context: &Context) -> Vec<OptimizedHandle> {
    let handles: Vec<(BlockId, usize)> = definition
        .blocks
        .iter()
        .flat_map(|(block, block_data)| {
            let handles = block_data.instructions.iter().enumerate();
            handles
                .filter(|(_, id)| matches!(definition.instructions[**id], Instruction::Handle { .. }))
                .map(move |(index, _)| (block, index))
        })
        .collect();
    // Each Handle is optimized independently of the others
    let optimized: Vec<_> = handles
        .par_iter()
        .map(|(block, index)| optimize_handle(definition, *block, *index, context, 0, MAX_SITE_INSTRUCTIONS))
        .collect();

    let mut size = definition.instructions.len();
    let fits = |handle: &OptimizedHandle| {
        size += handle.replacement.body.instructions.len();
        size <= MAX_DEFINITION_INSTRUCTIONS
    };
    optimized.into_iter().flatten().take_while(fits).collect()
}

/// Try to optimize the Handle at `index` of `block` in `outer`, returning its replacement. The
/// replacement may have at most `budget` instructions.
fn optimize_handle(
    outer: &Definition, block: BlockId, index: usize, context: &Context, depth: u32, budget: usize,
) -> Option<OptimizedHandle> {
    let mir = context.mir;
    let handle = outer.blocks[block].instructions[index];
    let Instruction::Handle { body, cases } = &outer.instructions[handle] else { return None };
    let mut body = Lambda::resolve(outer, *body, mir)?;
    if body.definition.entry_block().parameter_types.len() != 1 + usize::from(body.environment.is_some()) {
        return None;
    }
    let mut cases = opt_mapvec(cases, |case| Case::analyze(outer, case, mir))?;

    // The lambdas' environments are values of `outer`, so the replacement imports them. Any
    // functions they hold must stay known so calls to them can be inlined.
    let mut work = Definition::new(outer.name.clone(), outer.id, outer.generic_count, Type::ERROR);
    let mut imports = Imports::default();
    for lambda in std::iter::once(&mut body).chain(cases.iter_mut().map(|case| &mut case.lambda)) {
        lambda.environment = lambda.environment.map(|environment| imports.import_known(&mut work, outer, environment));
    }
    let copy = body.copy_into(&mut work, BlockId::ENTRY_BLOCK, vec![Value::Unit], mir, |_, _, value| {
        TerminatorInstruction::Return(value)
    });
    let capabilities = body
        .definition
        .block_instructions()
        .filter(|id| matches!(body.definition.instructions[*id], Instruction::Capability))
        .map(|id| copy.instruction(id))
        .collect();

    let mut site = Site {
        context,
        depth,
        budget,
        work,
        capabilities,
        cases,
        result_type: outer.instruction_result_type(handle).clone(),
        new_definitions: Vec::new(),
        bodies: vec![body.definition.id],
        residual_handlers: None,
    };
    site.optimize()?;
    Some(OptimizedHandle {
        replacement: Replacement { block, index, body: site.work, arguments: imports.into_values() },
        new_definitions: site.new_definitions,
        bodies: site.bodies,
    })
}

/// A statically known lambda used as a Handle's body or handler
struct Lambda<'mir> {
    definition: &'mir Definition,
    bindings: Option<Arc<GenericBindings>>,
    environment: Option<Value>,
}

impl<'mir> Lambda<'mir> {
    fn resolve(outer: &Definition, value: Value, mir: &'mir Mir) -> Option<Self> {
        let function = outer.known_function(value)?;
        let definition = mir.definitions.get(&function.id)?;
        let well_formed = function.id != outer.id
            && definition.typ.is_closure() == function.environment.is_some()
            && can_copy_body(definition);
        well_formed.then_some(Lambda { definition, bindings: function.bindings, environment: function.environment })
    }

    fn substitution<'a>(&'a self, mir: &'a Mir) -> Option<Substitution<'a>> {
        self.bindings.as_ref().map(|bindings| Substitution { bindings, definitions: &mir.definitions })
    }

    /// Copy the lambda into the end of `block` of `work`, passing it `arguments` and its environment
    fn copy_into(
        &self, work: &mut Definition, block: BlockId, mut arguments: Vec<Value>, mir: &Mir,
        on_return: impl FnMut(&mut Definition, BlockId, Value) -> TerminatorInstruction,
    ) -> BodyCopy {
        arguments.extend(self.environment);
        copy_body(work, Cow::Borrowed(self.definition), self.substitution(mir), block, &arguments, on_return)
    }
}

/// A handler case whose resumes are all in tail position
struct Case<'mir> {
    effect_op: DefinitionId,
    lambda: Lambda<'mir>,
    op_arg_count: usize,
    resumes: Resumes,
}

impl<'mir> Case<'mir> {
    fn analyze(outer: &Definition, case: &HandlerCase, mir: &'mir Mir) -> Option<Self> {
        let lambda = Lambda::resolve(outer, case.handler, mir)?;
        let parameter_count = lambda.definition.entry_block().parameter_types.len();
        let op_arg_count = parameter_count.checked_sub(1 + usize::from(lambda.environment.is_some()))?;
        let resumes = analyze_resumes(lambda.definition, op_arg_count)?;
        Some(Case { effect_op: case.effect_op, lambda, op_arg_count, resumes })
    }
}

/// A Handle being optimized
struct Site<'a, 'mir> {
    context: &'a Context<'mir>,
    depth: u32,
    /// The most instructions `work` may grow to
    budget: usize,
    /// The Handle's replacement, which returns the Handle's result
    work: Definition,
    capabilities: Vec<InstructionId>,
    cases: Vec<Case<'mir>>,
    result_type: Type,
    new_definitions: Vec<Definition>,
    bodies: Vec<DefinitionId>,
    /// The residual handler of each case and its type, shared by each residual handle
    residual_handlers: Option<Vec<(DefinitionId, Type)>>,
}

/// How the capability of the Handle being optimized is used
struct Uses {
    /// The capabilities and every value carrying one. Rewriting the uses below creates no new
    /// carriers, so this remains accurate while they are rewritten.
    carriers: FxHashSet<InstructionId>,
    /// The position of each effect operation call and the index of its operation (and handler case)
    local: Vec<(BlockId, usize, usize)>,
    /// The position of each call or Handle passed the capability which cannot be inlined
    escaping: Vec<(BlockId, usize)>,
}

impl Site<'_, '_> {
    fn optimize(&mut self) -> Option<()> {
        let uses = self.make_operations_local()?;
        if !uses.escaping.is_empty() && (uses.local.is_empty() || !self.residuals_are_cheap(&uses.escaping)) {
            // Nothing would be gained over the original Handle
            return None;
        }

        // Rewrite from the end of each block so the positions of the remaining uses stay valid
        let local = uses.local.iter().map(|(block, index, operation)| (*block, *index, Some(*operation)));
        let mut rewrites: Vec<_> =
            local.chain(uses.escaping.iter().map(|(block, index)| (*block, *index, None))).collect();
        rewrites.sort_unstable_by_key(|(block, index, _)| std::cmp::Reverse((usize::from(*block), *index)));
        let rewritten = self.work.instructions.len();
        let mut residuals = Vec::new();
        for (block, index, operation) in rewrites {
            match operation {
                Some(operation) => self.jump_to_handler(block, index, operation)?,
                None => residuals.push(self.wrap_in_residual_handle(block, index, &uses.carriers)?),
            }
        }

        // The capability must now be unused. Projections out of evidence or tuples holding it may
        // still refer to it until they are folded.
        self.work.fold_known_values();
        let live = self.work.live_instructions();
        if self.capabilities.iter().any(|capability| live.contains(capability)) {
            return None;
        }
        if self.optimize_copied_handles(rewritten, &residuals) {
            self.work.remove_dead_instructions();
        } else {
            self.work.remove_dead_instructions_given(&live);
        }
        self.work.simplify_cfg();
        Some(())
    }

    /// Optimize the Handles in the handlers copied in place of effect operation calls, which are
    /// the instructions from `rewritten` on other than `residuals`. They cannot use the capability.
    /// Returns true if any were optimized.
    fn optimize_copied_handles(&mut self, rewritten: usize, residuals: &[InstructionId]) -> bool {
        let end = self.work.instructions.len();
        let mut changed = false;
        let mut block = 0;
        while block < self.work.blocks.len() {
            let mut index = 0;
            while index < self.work.blocks[BlockId::from(block)].instructions.len() {
                let id = self.work.blocks[BlockId::from(block)].instructions[index];
                let is_copied_handle = matches!(self.work.instructions[id], Instruction::Handle { .. })
                    && (rewritten..end).contains(&usize::from(id))
                    && !residuals.contains(&id);
                match is_copied_handle.then(|| self.optimize_nested(BlockId::from(block), index)).flatten() {
                    // The new code at `index` has no copied Handles of its own
                    Some(replacement) => {
                        replacement.apply(&mut self.work);
                        changed = true;
                    },
                    None => index += 1,
                }
            }
            block += 1;
        }
        changed
    }

    /// Optimize the Handle at `index` of `block`, returning its replacement if successful
    fn optimize_nested(&mut self, block: BlockId, index: usize) -> Option<Replacement> {
        let size = self.work.instructions.len();
        if self.depth >= MAX_NESTING || size >= self.budget {
            return None;
        }
        let nested = optimize_handle(&self.work, block, index, self.context, self.depth + 1, self.budget - size)?;
        self.new_definitions.extend(nested.new_definitions);
        self.bodies.extend(nested.bodies);
        Some(nested.replacement)
    }

    /// Inline calls passed the capability and optimize nested Handles until every use of the
    /// capability is a local effect operation call or a call which cannot be inlined
    fn make_operations_local(&mut self) -> Option<Uses> {
        let mut carriers = Carriers::new(&self.work, self.capabilities.clone());
        let mut inlines = 0;
        // Visit each instruction once, including the inlined ones so they can be inlined into too
        let mut block = 0;
        while block < self.work.blocks.len() {
            let mut index = 0;
            while index < self.work.blocks[BlockId::from(block)].instructions.len() {
                let first_new = self.work.instructions.len();
                match self.expand(BlockId::from(block), index, &carriers, &mut inlines) {
                    // The code which replaced the instruction is now at `index`
                    Some(replaced) => carriers.update(&self.work, replaced, first_new),
                    None => index += 1,
                }
            }
            block += 1;
        }
        carriers.classify(&self.work)
    }

    /// Inline the call or optimize the Handle at `index` of `block` if possible, returning the
    /// id of the instruction replaced
    fn expand(
        &mut self, block: BlockId, index: usize, carriers: &Carriers, inlines: &mut usize,
    ) -> Option<InstructionId> {
        let mir = self.context.mir;
        let id = self.work.blocks[block].instructions[index];
        let replacement = match &self.work.instructions[id] {
            // Optimizing a nested Handle whose handlers use the capability can make their uses of
            // it local effect operation calls
            Instruction::Handle { .. } => self.optimize_nested(block, index)?,
            instruction @ (Instruction::Call { .. } | Instruction::CallClosure { .. }) => {
                let work = &self.work;
                let candidate = *inlines < MAX_INLINES_PER_SITE
                    && work.instructions.len() < self.budget
                    && carriers.local_operation(work, instruction).is_none()
                    && carriers.any_operand_carries(work, instruction);
                if !candidate {
                    return None;
                }
                let (inlinee, arguments) = resolve_callee(work, instruction, mir)?;
                if !self.context.inlinable.contains(&inlinee.callee.id) {
                    return None;
                }
                *inlines += 1;
                inlinee.replacement(work, block, index, arguments, &mir.definitions)
            },
            _ => return None,
        };
        replacement.apply(&mut self.work);
        Some(id)
    }

    /// True if residual handles for the calls at `escaping` would cost no more than the original Handle
    fn residuals_are_cheap(&self, escaping: &[(BlockId, usize)]) -> bool {
        // Residual handles of tail-resumptive handlers become direct calls. Otherwise each needs a
        // coroutine or a setjmp, which the original Handle needed only once.
        self.cases.iter().all(|case| case.resumes.is_tail_resumptive())
            || matches!(escaping, [(block, _)] if !self.work.is_in_loop(*block))
    }

    /// Replace the effect operation call at `index` of `block` with a copy of its handler.
    /// Resumes jump back to the code after the call, other returns return from the Handle.
    fn jump_to_handler(&mut self, block: BlockId, index: usize, operation: usize) -> Option<()> {
        let case = self.cases.get(operation)?;
        let call = self.work.blocks[block].instructions[index];
        let Instruction::CallClosure { arguments, .. } = &mut self.work.instructions[call] else { return None };
        if arguments.len() != case.op_arg_count {
            return None;
        }
        let mut arguments = std::mem::take(arguments);
        let continuation = self.work.split_at_instruction(block, index);

        // `resume` is only used by the calls rewritten below, so any placeholder will do
        arguments.push(Value::Unit);
        let copy = case.lambda.copy_into(&mut self.work, block, arguments, self.context.mir, |_, _, value| {
            TerminatorInstruction::Return(value)
        });
        for (resume_block, _, argument) in truncate_at_resumes(&mut self.work, case, &copy, false)? {
            self.work.blocks[resume_block].terminator = Some(TerminatorInstruction::jmp(continuation, argument));
        }
        Some(())
    }

    /// Replace the call (or Handle) at `index` of `block`, which is passed the capability but
    /// could not be inlined, with a Handle of just that call:
    ///
    /// ```text
    /// x_slot = alloca uninit; exit_slot = alloca uninit; exited = alloca false
    /// handle (fn () -> x_slot := call ..)
    /// | op args.. -> handler with `return v` replaced by `exit_slot := v; exited := true`
    /// if exited.* then return exit_slot.* else continue with x = x_slot.*
    /// ```
    ///
    /// `exit_slot` and `exited` are omitted if no handler can return without resuming. Returns the
    /// new Handle.
    fn wrap_in_residual_handle(
        &mut self, block: BlockId, index: usize, carriers: &FxHashSet<InstructionId>,
    ) -> Option<InstructionId> {
        let call = self.work.blocks[block].instructions[index];
        let (tree, free) = residual_dependencies(&self.work, call, carriers);
        let free_types = opt_mapvec(&free, |value| self.work.local_value_type(value).cloned())?;
        let exits = !self.cases.iter().all(|case| case.resumes.only_resumes);
        let handlers = match self.residual_handlers.take() {
            Some(handlers) => handlers,
            None => self.make_residual_handlers(exits)?,
        };

        let call_type = self.work.instruction_result_type(call).clone();
        let call_instruction = std::mem::replace(&mut self.work.instructions[call], Instruction::Id(Value::Unit));
        let body =
            residual_body(&self.work, call_instruction, &call_type, &tree, &free, free_types, &self.capabilities);

        let work = &mut self.work;
        let rest = work.split_block_after(block, index);
        work.blocks[block].instructions.pop();

        let x_slot = work.push_instruction_in(block, Instruction::StackAllocUninit(call_type), Type::POINTER);
        let exit = exits.then(|| {
            let result = Instruction::StackAllocUninit(self.result_type.clone());
            let exit_slot = work.push_instruction_in(block, result, Type::POINTER);
            let exited = work.push_instruction_in(block, Instruction::StackAlloc(Value::Bool(false)), Type::POINTER);
            (exit_slot, exited)
        });

        let mut body_environment = free;
        body_environment.push(x_slot);
        let body_closure = emit_function(work, block, body.id, &body.typ, body_environment);

        let handler_cases = mapvec(self.cases.iter().zip(&handlers), |(case, (handler, typ))| {
            // The environment of the original case closure is still available here
            let exit = exit.into_iter().flat_map(|(exit_slot, exited)| [exit_slot, exited]);
            let environment = case.lambda.environment.into_iter().chain(exit).collect();
            HandlerCase { effect_op: case.effect_op, handler: emit_function(work, block, *handler, typ, environment) }
        });
        let handle = Instruction::Handle { body: body_closure, cases: handler_cases };
        let Value::InstructionResult(handle) = work.push_instruction_in(block, handle, Type::UNIT) else {
            unreachable!()
        };

        work.blocks[block].terminator = Some(match exit {
            Some((exit_slot, exited)) => {
                let exited = work.push_instruction_in(block, Instruction::Deref(exited), Type::BOOL);
                let exit_block = work.blocks.push(Block::new(Vec::new()));
                let exit_value =
                    work.push_instruction_in(exit_block, Instruction::Deref(exit_slot), self.result_type.clone());
                work.blocks[exit_block].terminator = Some(TerminatorInstruction::Return(exit_value));
                TerminatorInstruction::if_(exited, exit_block, rest, rest)
            },
            None => TerminatorInstruction::jmp_no_args(rest),
        });

        // The original call's result is now read from `x_slot`
        work.instructions[call] = Instruction::Deref(x_slot);
        work.blocks[rest].instructions.insert(0, call);

        self.new_definitions.push(body);
        self.residual_handlers = Some(handlers);
        Some(handle)
    }

    /// Create the residual handler of each case, see [residual_handler]
    fn make_residual_handlers(&mut self, exits: bool) -> Option<Vec<(DefinitionId, Type)>> {
        let handlers = opt_mapvec(&self.cases, |case| residual_handler(&self.work, case, exits, self.context.mir))?;
        Some(mapvec(handlers, |handler| {
            let handler_and_type = (handler.id, handler.typ.clone());
            self.new_definitions.push(handler);
            handler_and_type
        }))
    }
}

/// Truncate the copy (made by `copy`) of each of `case`'s resume blocks at its resume call, which
/// is kept if `keep_call`. Returns each truncated block with its copied resume call and argument.
fn truncate_at_resumes(
    definition: &mut Definition, case: &Case, copy: &BodyCopy, keep_call: bool,
) -> Option<Vec<(BlockId, InstructionId, Value)>> {
    opt_mapvec(&case.resumes.calls, |(block, call, argument)| {
        let (block, call) = (copy.block(*block), copy.instruction(*call));
        let position = definition.blocks[block].instructions.iter().rposition(|id| *id == call)?;
        definition.blocks[block].instructions.truncate(position + usize::from(keep_call));
        Some((block, call, copy.value(*argument)))
    })
}

/// The values carrying the capabilities of the Handle being optimized
struct Carriers {
    capabilities: Vec<InstructionId>,
    carriers: FxHashSet<InstructionId>,
    /// The instructions using each instruction, to find the carriers a change may create
    users: Vec<Vec<InstructionId>>,
}

impl Carriers {
    fn new(work: &Definition, capabilities: Vec<InstructionId>) -> Self {
        let carriers = capabilities.iter().copied().collect();
        let mut this = Self { capabilities, carriers, users: Vec::new() };
        let ids: Vec<_> = work.block_instructions().collect();
        this.add_users(work, &ids);
        this.propagate(work, ids);
        this
    }

    /// Find the new carriers after `replaced` was replaced with the instructions from `first_new` on
    fn update(&mut self, work: &Definition, replaced: InstructionId, first_new: usize) {
        let changed: Vec<_> =
            std::iter::once(replaced).chain((first_new..work.instructions.len()).map(InstructionId::from)).collect();
        self.add_users(work, &changed);
        self.propagate(work, changed);
    }

    fn add_users(&mut self, work: &Definition, ids: &[InstructionId]) {
        self.users.resize_with(work.instructions.len(), Vec::new);
        for id in ids {
            work.instructions[*id].for_each_value(|value| {
                if let Value::InstructionResult(operand) = value {
                    self.users[usize::from(*operand)].push(*id);
                }
            });
        }
    }

    /// Check whether each of `worklist` carries the capability, and the users of each which does
    fn propagate(&mut self, work: &Definition, mut worklist: Vec<InstructionId>) {
        while let Some(id) = worklist.pop() {
            if !self.carriers.contains(&id) && self.instruction_carries(work, id) {
                self.carriers.insert(id);
                worklist.extend_from_slice(&self.users[usize::from(id)]);
            }
        }
    }

    fn instruction_carries(&self, work: &Definition, id: InstructionId) -> bool {
        match &work.instructions[id] {
            Instruction::Id(value) => self.carries(work, *value),
            Instruction::IndexTuple { tuple: operand, .. } | Instruction::LookupEvidence { evidence: operand, .. } => {
                let resolved = work.resolve(Value::InstructionResult(id));
                let value = if resolved != Value::InstructionResult(id) { resolved } else { *operand };
                self.carries(work, value)
            },
            instruction @ (Instruction::MakeTuple(_)
            | Instruction::MakeEvidence { .. }
            | Instruction::PackClosure { .. }) => self.any_operand_carries(work, instruction),
            _ => false,
        }
    }

    fn carries(&self, work: &Definition, value: Value) -> bool {
        matches!(work.resolve(value), Value::InstructionResult(id) if self.carriers.contains(&id))
    }

    fn any_operand_carries(&self, work: &Definition, instruction: &Instruction) -> bool {
        let mut found = false;
        instruction.for_each_value(|value| found |= self.carries(work, *value));
        found
    }

    /// If `instruction` calls an effect operation of the capability, returns the operation's index
    fn local_operation(&self, work: &Definition, instruction: &Instruction) -> Option<usize> {
        let Instruction::CallClosure { closure, arguments } = instruction else { return None };
        let Value::InstructionResult(function) = work.resolve(*closure) else { return None };
        let Instruction::IndexTuple { tuple, index } = &work.instructions[function] else { return None };
        let Value::InstructionResult(capability) = work.resolve(*tuple) else { return None };
        let is_local =
            self.capabilities.contains(&capability) && !arguments.iter().any(|value| self.carries(work, *value));
        is_local.then_some(*index as usize)
    }

    /// Classify every use of a carrier, returning `None` if any cannot be handled
    fn classify(self, work: &Definition) -> Option<Uses> {
        let (mut local, mut escaping) = (Vec::new(), Vec::new());
        for (block_id, block) in work.blocks.iter() {
            for (index, id) in block.instructions.iter().enumerate() {
                let instruction = &work.instructions[*id];
                // A projection which is not a carrier extracts something other than the capability
                let is_projection = matches!(
                    instruction,
                    Instruction::Id(_) | Instruction::IndexTuple { .. } | Instruction::LookupEvidence { .. }
                );
                if self.carriers.contains(id) || is_projection || !self.any_operand_carries(work, instruction) {
                    continue;
                }
                if let Some(operation) = self.local_operation(work, instruction) {
                    local.push((block_id, index, operation));
                } else if matches!(
                    instruction,
                    Instruction::Call { .. } | Instruction::CallClosure { .. } | Instruction::Handle { .. }
                ) {
                    escaping.push((block_id, index));
                } else {
                    return None;
                }
            }
            if let Some(terminator) = &block.terminator {
                let mut found = false;
                terminator.for_each_value(|value| found |= self.carries(work, *value));
                if found {
                    return None;
                }
            }
        }
        Some(Uses { carriers: self.carriers, local, escaping })
    }
}

/// Collect the carrier instructions `call` (transitively) uses, in dependency order, along with
/// every other value they and `call` use which is local to `work`.
fn residual_dependencies(
    work: &Definition, call: InstructionId, carriers: &FxHashSet<InstructionId>,
) -> (Vec<InstructionId>, Vec<Value>) {
    let mut tree = Vec::new();
    let mut visited = FxHashSet::default();
    let mut free = Vec::new();

    /// Whether `operand` must be copied rather than passed in as a free value
    fn must_copy(
        work: &Definition, operand: InstructionId, carriers: &FxHashSet<InstructionId>, in_handle: bool,
    ) -> bool {
        carriers.contains(&operand)
            || (in_handle
                && matches!(work.instructions[operand], Instruction::PackClosure { .. } | Instruction::Instantiate(..)))
    }

    fn visit(
        work: &Definition, id: InstructionId, carriers: &FxHashSet<InstructionId>, tree: &mut Vec<InstructionId>,
        visited: &mut FxHashSet<InstructionId>, free: &mut Vec<Value>,
    ) {
        // The handle passes require a Handle's body and handlers to be visible `PackClosure`s
        // of known functions, so those are copied along with a Handle
        let is_handle = matches!(work.instructions[id], Instruction::Handle { .. });
        let is_closure = matches!(work.instructions[id], Instruction::PackClosure { .. });
        let mut operands = Vec::new();
        work.instructions[id].for_each_value(|value| operands.push(*value));
        for (index, value) in operands.into_iter().enumerate() {
            // Only the function of a copied PackClosure needs copying, not its environment
            let in_handle = is_handle || (is_closure && index == 0 && visited.contains(&id));
            match value {
                Value::InstructionResult(operand) if must_copy(work, operand, carriers, in_handle) => {
                    if visited.insert(operand) {
                        visit(work, operand, carriers, tree, visited, free);
                        tree.push(operand);
                    }
                },
                Value::InstructionResult(_) | Value::Parameter(..) => {
                    if !free.contains(&value) {
                        free.push(value);
                    }
                },
                _ => (),
            }
        }
    }
    visit(work, call, carriers, &mut tree, &mut visited, &mut free);
    (tree, free)
}

/// Emit `function` of type `typ`, which shares `work`'s generics, as a closure with an
/// environment tuple of `environment`, or as just the function if there is no environment
fn emit_function(
    work: &mut Definition, block: BlockId, function: DefinitionId, typ: &Type, environment: Vec<Value>,
) -> Value {
    let bindings = identity_bindings(work.generic_count);
    let mut emitter = Emitter::in_block(work, block);
    let function_value = emitter.emit_definition_value(function, typ.clone(), bindings);
    if environment.is_empty() {
        return function_value;
    }
    let Type::Function(function_type) = typ else { unreachable!("emit_function: {typ} is not a function") };
    let environment = emitter.push_instruction(Instruction::MakeTuple(environment), function_type.environment.clone());
    emitter.push_instruction(Instruction::PackClosure { function: function_value, environment }, typ.clone())
}

/// `fn () [free.., x_slot] -> Unit` which recreates `call` (and the carriers it uses) with its own
/// capability, storing the result in `x_slot`.
fn residual_body(
    work: &Definition, call: Instruction, call_type: &Type, tree: &[InstructionId], free: &[Value],
    free_types: Vec<Type>, capabilities: &[InstructionId],
) -> Definition {
    let name = Arc::new(format!("{}_residual_handle_body", work.name));
    let mut body = Definition::new(name, next_definition_id(), work.generic_count, Type::ERROR);
    let entry = BlockId::ENTRY_BLOCK;
    let environment = Value::Parameter(entry, 1);

    let mut values = FxHashMap::default();
    for (index, (value, typ)) in free.iter().zip(&free_types).enumerate() {
        let field = Instruction::IndexTuple { tuple: environment, index: index as u32 };
        values.insert(*value, body.push_instruction_in(entry, field, typ.clone()));
    }

    let mut environment_types = free_types;
    environment_types.push(Type::POINTER);
    let environment_type = Type::tuple(environment_types);
    body.typ = Type::Function(Arc::new(FunctionType {
        parameters: vec![Type::UNIT],
        environment: environment_type.clone(),
        return_type: Type::UNIT,
    }));
    body.blocks[entry].parameter_types = vec![Type::UNIT, environment_type];
    let x_slot_field = Instruction::IndexTuple { tuple: environment, index: free.len() as u32 };
    let x_slot = body.push_instruction_in(entry, x_slot_field, Type::POINTER);
    let capability_type = work.instruction_result_type(capabilities[0]).clone();
    let capability = body.push_instruction_in(entry, Instruction::Capability, capability_type);
    for id in capabilities {
        values.insert(Value::InstructionResult(*id), capability);
    }

    let map = |instruction: &mut Instruction, values: &FxHashMap<Value, Value>| {
        instruction.for_each_value_mut(|value| {
            if let Some(new) = values.get(value) {
                *value = *new;
            }
        });
    };
    // The carriers may have other uses in `work`, so they are copied
    for id in tree.iter().filter(|id| !capabilities.contains(id)) {
        let mut instruction = work.instructions[*id].clone();
        map(&mut instruction, &values);
        let new = body.push_instruction_in(entry, instruction, work.instruction_result_type(*id).clone());
        values.insert(Value::InstructionResult(*id), new);
    }
    let mut call = call;
    map(&mut call, &values);
    let result = body.push_instruction_in(entry, call, call_type.clone());
    body.push_instruction_in(entry, Instruction::Store { pointer: x_slot, value: result }, Type::UNIT);
    body.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::Unit));
    body
}

/// `case`'s handler with type `fn op_args.. (resume: fn r -> Unit) [env..] -> Unit`, whose
/// resumes return the result of resuming directly. If `exits`, `exit_slot` and `exited` follow the
/// original environment (if any) in its environment, and each `return v` not via `resume` stores
/// `v` to `exit_slot` and sets `exited`. Otherwise each return must be via `resume`.
fn residual_handler(work: &Definition, case: &Case, exits: bool, mir: &Mir) -> Option<Definition> {
    let original = case.lambda.definition;
    let bindings = case.lambda.bindings.as_deref();
    let entry_types = &original.entry_block().parameter_types;
    let mut parameters = mapvec(&entry_types[..case.op_arg_count], |typ| typ.substitute_optional(bindings));
    let Type::Function(resume_type) = entry_types.get(case.op_arg_count)?.substitute_optional(bindings) else {
        return None;
    };
    parameters.push(Type::Function(Arc::new(FunctionType { return_type: Type::UNIT, ..(*resume_type).clone() })));

    let original_environment_type = case.lambda.environment.map(|environment| mir.type_of_value(&environment, work));
    let mut environment_types: Vec<Type> = original_environment_type.iter().cloned().collect();
    if exits {
        environment_types.extend([Type::POINTER, Type::POINTER]);
    }
    let is_closure = !environment_types.is_empty();
    let environment_type = if is_closure { Type::tuple(environment_types) } else { Type::NO_CLOSURE_ENV };
    let typ = Type::Function(Arc::new(FunctionType {
        parameters: parameters.clone(),
        environment: environment_type.clone(),
        return_type: Type::UNIT,
    }));

    let name = Arc::new(format!("{}_residual_handler", original.name));
    let mut handler = Definition::new(name, next_definition_id(), work.generic_count, typ);
    let entry = BlockId::ENTRY_BLOCK;
    if is_closure {
        parameters.push(environment_type);
    }
    handler.blocks[entry].parameter_types = parameters;
    let environment = Value::Parameter(entry, case.op_arg_count as u32 + 1);

    // The environment holds the original environment (if any), then `exit_slot` and `exited`
    let original_environment = original_environment_type
        .map(|typ| handler.push_instruction_in(entry, Instruction::IndexTuple { tuple: environment, index: 0 }, typ));
    let field = |index: u32| index + u32::from(original_environment.is_some());
    let exit = exits.then(|| {
        let mut push_field = |index| {
            let instruction = Instruction::IndexTuple { tuple: environment, index: field(index) };
            handler.push_instruction_in(entry, instruction, Type::POINTER)
        };
        (push_field(0), push_field(1))
    });

    let mut arguments = mapvec(0..=case.op_arg_count as u32, |index| Value::Parameter(entry, index));
    arguments.extend(original_environment);
    let substitution = case.lambda.substitution(mir);
    let copy =
        copy_body(&mut handler, Cow::Borrowed(original), substitution, entry, &arguments, |handler, block, value| {
            let Some((exit_slot, exited)) = exit else {
                // Every return is a resume's, which are rewritten below
                return TerminatorInstruction::Unreachable;
            };
            handler.push_instruction_in(block, Instruction::Store { pointer: exit_slot, value }, Type::UNIT);
            let exited = Instruction::Store { pointer: exited, value: Value::Bool(true) };
            handler.push_instruction_in(block, exited, Type::UNIT);
            TerminatorInstruction::Return(Value::Unit)
        });
    // Resumes return the result of resuming directly
    for (resume_block, resume_call, _) in truncate_at_resumes(&mut handler, case, &copy, true)? {
        handler.instruction_result_types[resume_call] = Type::UNIT;
        handler.blocks[resume_block].terminator =
            Some(TerminatorInstruction::Return(Value::InstructionResult(resume_call)));
    }
    handler.cleanup_after_restructuring();
    Some(handler)
}
