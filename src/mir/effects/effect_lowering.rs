//! Lower [crate::mir::Instruction::Handle] and [crate::mir::Instruction::Perform]
//! into aminicoro primitives.
//!
//! Each Handler expression lowers into:
//! - An init function wrapping the handled expression
//! - Creating the coroutine via `mco_coro_init` with the init function
//! - Creating a `drive` function to resume the coroutine, which gets passed
//!   as the capability value.
//!
//! Each effect performed uses `mco_coro_push` to push the relevant arguments
//! to the effect, followed by the tag of the effect performed, before calling
//! the driver function which will resume the handler, match on the effect,
//! and dispatch to the relevant handler branch.
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    iterator_extensions::mapvec,
    lexer::token::IntegerKind,
    mir::{
        Block, BlockId, Definition, DefinitionId, FunctionType, GenericBindings, HandlerCase, Instruction,
        InstructionId, IntConstant, Mir, PrimitiveType, TerminatorInstruction, Type, Value, next_definition_id,
    },
};

struct AminicoroFn {
    name: &'static str,
    typ: Type,
}

struct AminicoroFns {
    init: AminicoroFn,
    free: AminicoroFn,
    is_suspended: AminicoroFn,
    push: AminicoroFn,
    pop: AminicoroFn,
    suspend: AminicoroFn,
    resume: AminicoroFn,
    get_user_data: AminicoroFn,
    running: AminicoroFn,
    bytes_stored: AminicoroFn,
    transfer: AminicoroFn,
}

fn ptr_fn(parameters: Vec<Type>, return_type: Type) -> Type {
    Type::Function(Arc::new(FunctionType { parameters, environment: Type::NO_CLOSURE_ENV, return_type }))
}

fn aminicoro_fns() -> AminicoroFns {
    let ptr = || Type::POINTER;
    let u8_t = || Type::int(IntegerKind::U8);
    let usize_t = || Type::int(IntegerKind::Usz);

    AminicoroFns {
        init: AminicoroFn { name: "mco_coro_init", typ: ptr_fn(vec![ptr(), ptr()], ptr()) },
        free: AminicoroFn { name: "mco_coro_free", typ: ptr_fn(vec![ptr()], u8_t()) },
        is_suspended: AminicoroFn { name: "mco_coro_is_suspended", typ: ptr_fn(vec![ptr()], Type::BOOL) },
        push: AminicoroFn { name: "mco_coro_push", typ: ptr_fn(vec![ptr(), ptr(), usize_t()], u8_t()) },
        pop: AminicoroFn { name: "mco_coro_pop", typ: ptr_fn(vec![ptr(), ptr(), usize_t()], u8_t()) },
        suspend: AminicoroFn { name: "mco_coro_suspend", typ: ptr_fn(vec![ptr()], u8_t()) },
        resume: AminicoroFn { name: "mco_coro_resume", typ: ptr_fn(vec![ptr()], u8_t()) },
        get_user_data: AminicoroFn { name: "mco_coro_get_user_data", typ: ptr_fn(vec![ptr()], ptr()) },
        running: AminicoroFn { name: "mco_coro_running", typ: ptr_fn(vec![], ptr()) },
        bytes_stored: AminicoroFn { name: "mco_coro_bytes_stored", typ: ptr_fn(vec![ptr()], usize_t()) },
        transfer: AminicoroFn { name: "mco_coro_transfer", typ: ptr_fn(vec![ptr(), ptr(), usize_t()], u8_t()) },
    }
}

impl Mir {
    pub(crate) fn lower_effects(mut self) -> Self {
        if !contains_effects(&self) {
            return self;
        }
        let fns = aminicoro_fns();
        let op_index = build_op_index(&self);
        let context = Context { mco: &fns, op_index: &op_index };

        let definition_ids: Vec<DefinitionId> = self.definitions.keys().copied().collect();

        // Pass 1: rewrite every Handle into wrapper free functions + a per-Handle drive function.
        for id in &definition_ids {
            rewrite_sites_in_definition(&mut self, *id, context, collect_handle_sites, rewrite_single_handle);
        }

        // Pass 2: rewrite every Perform into IndexTuple + CallClosure on the capability.
        for id in self.definitions.keys().copied().collect::<Vec<_>>() {
            rewrite_sites_in_definition(&mut self, id, context, collect_perform_sites, rewrite_single_perform);
        }

        self
    }
}

/// Drives a Handle-rewriting optimization, also processing body clones it creates along the way.
pub(super) fn run_handle_optimization_worklist(
    mir: &mut Mir,
    mut optimize_definition: impl FnMut(&mut Mir, DefinitionId, &mut FxHashSet<DefinitionId>) -> Vec<DefinitionId>,
) {
    let mut worklist: Vec<DefinitionId> = mir.definitions.keys().copied().collect();
    let mut seen: FxHashSet<DefinitionId> = worklist.iter().copied().collect();
    let mut dead_bodies = FxHashSet::default();
    while let Some(id) = worklist.pop() {
        if !mir.definitions.contains_key(&id) || dead_bodies.contains(&id) {
            continue;
        }
        for new_id in optimize_definition(mir, id, &mut dead_bodies) {
            if seen.insert(new_id) {
                worklist.push(new_id);
            }
        }
    }
}

fn contains_effects(mir: &Mir) -> bool {
    mir.definitions.values().any(|definition| {
        definition
            .instructions
            .values()
            .any(|instruction| matches!(instruction, Instruction::Perform { .. } | Instruction::Handle { .. }))
    })
}

/// Mapping from an effect-op DefinitionId to its position within its parent effect.
type OpIndex = FxHashMap<DefinitionId, u32>;

fn build_op_index(mir: &Mir) -> OpIndex {
    let mut index: OpIndex = FxHashMap::default();
    for definition in mir.definitions.values() {
        for instruction in definition.instructions.values() {
            if let Instruction::Handle { cases, .. } = instruction {
                for (i, case) in cases.iter().enumerate() {
                    let i = u32::try_from(i).expect("effect with more than u32::MAX ops");
                    let prev = index.insert(case.effect_op, i);
                    if let Some(prev) = prev {
                        debug_assert_eq!(prev, i, "effect op index disagreement across Handle sites");
                    }
                }
            }
        }
    }
    // Merge in entries recovered from Handles that the tail-resume optimization removed before
    // this pass ran. These are op→index pairs that lower_effects would have built itself if the
    // Handles were still around. The position is uniform for an op (it's the op's slot within
    // its effect's declaration), so a debug-only sanity check matches the same constraint the
    // Handle-walking loop above enforces.
    for (op, idx) in &mir.preserved_op_indices {
        let prev = index.insert(*op, *idx);
        if let Some(prev) = prev {
            debug_assert_eq!(prev, *idx, "op_index disagreement between live Handle and preserved entry");
        }
    }
    index
}

#[derive(Clone, Copy)]
struct Context<'local> {
    mco: &'local AminicoroFns,
    op_index: &'local OpIndex,
}

fn is_zero_sized(typ: &Type) -> bool {
    match typ {
        Type::Primitive(PrimitiveType::Unit | PrimitiveType::NoClosureEnv) => true,
        Type::Tuple(fields) => fields.iter().all(is_zero_sized),
        _ => false,
    }
}

pub(super) enum EmitTarget<'local> {
    Block(BlockId),
    Pending(&'local mut Vec<InstructionId>),
}

pub(super) struct Emitter<'local> {
    pub(super) definition: &'local mut Definition,
    target: EmitTarget<'local>,
}

impl<'local> Emitter<'local> {
    pub(super) fn in_block(definition: &'local mut Definition, block: BlockId) -> Self {
        Self { definition, target: EmitTarget::Block(block) }
    }

    pub(super) fn pending(definition: &'local mut Definition, pending: &'local mut Vec<InstructionId>) -> Self {
        Self { definition, target: EmitTarget::Pending(pending) }
    }

    pub(super) fn push_instruction(&mut self, instruction: Instruction, result_type: Type) -> Value {
        let id = self.definition.instructions.push(instruction);
        self.definition.instruction_result_types.push_existing(id, result_type);
        self.append_instruction(id);
        Value::InstructionResult(id)
    }

    fn reuse_instruction(&mut self, id: InstructionId, instruction: Instruction, result_type: Type) -> Value {
        self.definition.instructions[id] = instruction;
        self.definition.instruction_result_types[id] = result_type;
        self.append_instruction(id);
        Value::InstructionResult(id)
    }

    fn append_instruction(&mut self, id: InstructionId) {
        match &mut self.target {
            EmitTarget::Block(block) => self.definition.blocks[*block].instructions.push(id),
            EmitTarget::Pending(pending) => pending.push(id),
        }
    }

    fn call_extern(&mut self, function: &AminicoroFn, arguments: Vec<Value>) -> Value {
        let function_value =
            self.push_instruction(Instruction::Extern(function.name.to_string()), function.typ.clone());
        let Type::Function(function_type) = &function.typ else { panic!("McoFn.typ must be a Function") };
        let return_type = function_type.return_type.clone();
        self.push_instruction(Instruction::Call { function: function_value, arguments }, return_type)
    }

    fn emit_size_of(&mut self, typ: &Type) -> Value {
        self.push_instruction(Instruction::SizeOf(typ.clone()), Type::int(IntegerKind::Usz))
    }

    /// Returns `Value::Definition(target_id)` when `bindings` is None; otherwise emits
    /// an `Instruction::Instantiate(target_id, bindings)` and returns its result Value.
    pub(super) fn emit_definition_value(
        &mut self, target_id: DefinitionId, target_typ: Type, bindings: Option<Arc<GenericBindings>>,
    ) -> Value {
        if let Some(bindings) = bindings {
            self.push_instruction(Instruction::Instantiate(target_id, bindings), target_typ)
        } else {
            Value::Definition(target_id)
        }
    }

    fn push_bytes(&mut self, mco: &AminicoroFns, coro: Value, value: Value, value_type: &Type) {
        if is_zero_sized(value_type) {
            return;
        }
        if let Type::Tuple(fields) = value_type {
            for (index, field_type) in fields.iter().enumerate() {
                let field = self.push_instruction(
                    Instruction::IndexTuple { tuple: value, index: index as u32 },
                    field_type.clone(),
                );
                self.push_bytes(mco, coro, field, field_type);
            }
            return;
        }
        let slot = self.push_instruction(Instruction::StackAlloc(value), Type::POINTER);
        let size = self.emit_size_of(value_type);
        self.call_extern(&mco.push, vec![coro, slot, size]);
    }

    fn pop_bytes(&mut self, mco: &AminicoroFns, coro: Value, value_type: &Type) -> Value {
        if is_zero_sized(value_type) {
            return Value::Unit;
        }
        if let Type::Tuple(fields) = value_type {
            let mut popped = mapvec(fields.iter().rev(), |field_type| self.pop_bytes(mco, coro, field_type));
            popped.reverse();
            return self.push_instruction(Instruction::MakeTuple(popped), value_type.clone());
        }
        // Allocate a slot sized to value_type. We use StackAllocUninit  instead of
        // StackAlloc(zeroed_value) so the slot is correctly sized even when
        // value_type is a generic that mono will later replace with a larger concrete type.
        let slot = self.push_instruction(Instruction::StackAllocUninit(value_type.clone()), Type::POINTER);
        let size = self.emit_size_of(value_type);
        self.call_extern(&mco.pop, vec![coro, slot, size]);
        self.push_instruction(Instruction::Deref(slot), value_type.clone())
    }
}

/// Walk every instruction in `definition`, calling `extract` on each. Sites are
/// collected per block and reversed within each block so later splices don't
/// invalidate earlier indices.
fn collect_sites<S>(
    definition: &Definition, mut extract: impl FnMut(BlockId, usize, InstructionId) -> Option<S>,
) -> Vec<S> {
    let mut sites = Vec::new();
    for (block_id, block) in definition.blocks.iter() {
        let start = sites.len();
        for (index, instruction_id) in block.instructions.iter().enumerate() {
            if let Some(site) = extract(block_id, index, *instruction_id) {
                sites.push(site);
            }
        }
        sites[start..].reverse();
    }
    sites
}

fn rewrite_sites_in_definition<S>(
    mir: &mut Mir, definition_id: DefinitionId, context: Context, collect: impl FnOnce(&Definition) -> Vec<S>,
    mut rewrite: impl FnMut(&mut Mir, DefinitionId, S, Context),
) {
    let Some(definition) = mir.definitions.get(&definition_id) else { return };
    let sites = collect(definition);
    for site in sites {
        rewrite(mir, definition_id, site, context);
    }
}

/// Reverses the pop order since aminicoro's channel is LIFO.
fn pop_operation_arguments(emitter: &mut Emitter, context: Context, coro: Value, arg_types: &[Type]) -> Vec<Value> {
    let mut popped = mapvec(arg_types.iter().rev(), |typ| emitter.pop_bytes(context.mco, coro, typ));
    popped.reverse();
    popped
}

struct PerformSite {
    block: BlockId,
    index: usize,
    id: InstructionId,
    op: DefinitionId,
    arguments: Vec<Value>,
}

fn collect_perform_sites(definition: &Definition) -> Vec<PerformSite> {
    collect_sites(definition, |block, index, id| {
        if let Instruction::Perform { effect_op, arguments } = &definition.instructions[id] {
            Some(PerformSite { block, index, id, op: *effect_op, arguments: arguments.clone() })
        } else {
            None
        }
    })
}

/// Replace `Perform { op, args.., cap }` with:
///     wrapper = IndexTuple(cap, op_index)
///     result  = CallClosure(wrapper, op_args..)
/// reusing the original Perform's InstructionId for the result so existing
/// users keep working.
fn rewrite_single_perform(mir: &mut Mir, definition_id: DefinitionId, site: PerformSite, context: Context) {
    let PerformSite { block, index, id: original_id, op, arguments } = site;

    let return_type = mir.definitions[&definition_id].instruction_result_types[original_id].clone();

    let op_index =
        *context.op_index.get(&op).unwrap_or_else(|| panic!("effect_lowering: effect op {op:?} has no op-index entry"));

    // Capability is the implicit trailing argument, appended by implicit-arg resolution.
    let (cap_value, op_args) =
        arguments.split_last().unwrap_or_else(|| panic!("effect_lowering: Perform {op:?} has no arguments"));
    let cap_value = *cap_value;
    let op_args: Vec<Value> = op_args.to_vec();

    // Resolved before the mutable borrow below so an external cap_value can still be looked up.
    let definition_ref = &mir.definitions[&definition_id];
    let cap_type = definition_ref.type_of_value(&cap_value, &mir.externals, &mir.definitions);
    let op_arg_types: Vec<Type> =
        mapvec(&op_args, |arg| definition_ref.type_of_value(arg, &mir.externals, &mir.definitions));

    let wrapper_type = match &cap_type {
        Type::Tuple(fields) => fields
            .get(op_index as usize)
            .cloned()
            .unwrap_or_else(|| panic!("effect_lowering: cap tuple has no slot {op_index} (cap_type = {cap_type:?})")),

        // Fall back to inferring from the Perform if the cap isn't a known tuple type yet.
        _ => Type::Function(Arc::new(FunctionType {
            parameters: op_arg_types,
            environment: Type::POINTER,
            return_type: return_type.clone(),
        })),
    };

    let definition = mir.definitions.get_mut(&definition_id).expect("definition disappeared mid-rewrite");

    let mut pending = Vec::new();
    {
        let mut emitter = Emitter::pending(definition, &mut pending);
        let wrapper =
            emitter.push_instruction(Instruction::IndexTuple { tuple: cap_value, index: op_index }, wrapper_type);
        emitter.reuse_instruction(
            original_id,
            Instruction::CallClosure { closure: wrapper, arguments: op_args },
            return_type,
        );
    }
    definition.blocks[block].instructions.splice(index..=index, pending);
}

struct HandleSite {
    block: BlockId,
    index: usize,
    id: InstructionId,
    body: Value,
    cases: Vec<HandlerCase>,
    result_type: Type,
}

fn collect_handle_sites(definition: &Definition) -> Vec<HandleSite> {
    collect_sites(definition, |block, index, id| {
        if let Instruction::Handle { body, cases } = &definition.instructions[id] {
            Some(HandleSite {
                block,
                index,
                id,
                body: *body,
                cases: cases.clone(),
                result_type: definition.instruction_result_types[id].clone(),
            })
        } else {
            None
        }
    })
}

/// Op-shape info extracted from a handler's MIR type.
/// Handlers are typed `fn op_args.., resume -> result`, where resume is
/// `fn r1 [Pointer] -> result`. We extract `op_args` and `r1`.
pub(super) struct CaseShape {
    pub(super) op_arg_types: Vec<Type>,
    /// The op's return type (= resume's parameter type).
    pub(super) op_return_type: Type,
    pub(super) handler_is_closure: bool,
}

pub(super) fn case_shape_from_handler_type(handler_type: &Type) -> Option<CaseShape> {
    let Type::Function(ft) = handler_type else { return None };
    let (resume_param, op_args) = ft.parameters.split_last()?;
    let Type::Function(resume_ft) = resume_param else { return None };
    let op_return_type = resume_ft.parameters.first().cloned().unwrap_or(Type::UNIT);
    Some(CaseShape { op_arg_types: op_args.to_vec(), op_return_type, handler_is_closure: ft.is_closure() })
}

/// Resolves a `handle` body Value to its `DefinitionId`, env value, and generic bindings, if any.
pub(super) fn resolve_body_function(
    mut body: Value, definition: &Definition,
) -> (DefinitionId, Option<Value>, Option<Arc<GenericBindings>>) {
    let mut env = None;

    if let Value::InstructionResult(instruction) = body
        && let Instruction::PackClosure { function, environment } = &definition.instructions[instruction]
    {
        body = *function;
        env = Some(*environment);
    }
    let (id, bindings) = resolve_id(body, definition);
    (id, env, bindings)
}

fn resolve_id(value: Value, definition: &Definition) -> (DefinitionId, Option<Arc<GenericBindings>>) {
    match value {
        Value::Definition(id) => (id, None),
        Value::InstructionResult(iid) => match &definition.instructions[iid] {
            Instruction::Id(inner) => resolve_id(*inner, definition),
            Instruction::Instantiate(id, bindings) => (*id, Some(bindings.clone())),
            other => panic!("handle body function did not resolve to a definition: {other:?}"),
        },
        other => panic!("handle body function has unexpected Value shape: {other:?}"),
    }
}

fn rewrite_single_handle(mir: &mut Mir, definition_id: DefinitionId, site: HandleSite, context: Context) {
    let HandleSite { block, index, id: original_id, body, cases, result_type } = site;

    let caller_generic_count = mir.definitions[&definition_id].generic_count;
    let caller_bindings = crate::mir::identity_bindings(caller_generic_count);

    let definition = mir.definitions.get(&definition_id).expect("definition vanished mid-rewrite");
    let (body_fn_id, body_env, body_bindings) = resolve_body_function(body, definition);

    let env_type =
        body_env.map(|env| definition.type_of_value(&env, &mir.externals, &mir.definitions)).unwrap_or(Type::UNIT);

    let handler_types =
        mapvec(&cases, |case| definition.type_of_value(&case.handler, &mir.externals, &mir.definitions));

    let case_shapes = mapvec(&handler_types, |t| {
        case_shape_from_handler_type(t).expect("handler type must be `fn op_args.., resume -> r`")
    });

    // Replaces `Capability` placeholders in the body with the user_data fetch chain.
    expand_handler_caps_in_body(mir, body_fn_id, &env_type, context);

    let wrapper_ids = mapvec(case_shapes.iter().enumerate(), |(i, shape)| {
        generate_capability_wrapper(mir, i as u32, shape, context, caller_generic_count)
    });
    let wrapper_closure_types = mapvec(&wrapper_ids, |id| mir.definitions[id].typ.clone());
    let cap_tuple_type = Type::Tuple(Arc::new(wrapper_closure_types.clone()));

    let body_wrapper_id = generate_body_wrapper(
        mir,
        body_fn_id,
        body_bindings.clone(),
        &env_type,
        &cap_tuple_type,
        &result_type,
        context,
        caller_generic_count,
    );
    let drive_id =
        generate_drive_function(mir, &cases, &handler_types, &case_shapes, &result_type, context, caller_generic_count);

    // Read generated def types up-front; the mutable borrow below precludes accessing them later.
    let body_wrapper_typ = mir.definitions[&body_wrapper_id].typ.clone();
    let drive_typ = mir.definitions[&drive_id].typ.clone();

    let definition = mir.definitions.get_mut(&definition_id).expect("definition disappeared mid-rewrite");
    let mut pending = Vec::new();

    let mut emitter = Emitter::pending(definition, &mut pending);

    // cap_state starts with a placeholder coro pointer so wrapper closures can reference it by
    // address before the real coro exists; patched once mco_coro_init returns.
    let cap_state_type = Type::Tuple(Arc::new(vec![Type::POINTER]));
    let null_ptr = emitter.push_instruction(Instruction::Transmute(Value::Integer(IntConstant::Usz(0))), Type::POINTER);
    let initial_cap_state = emitter.push_instruction(Instruction::MakeTuple(vec![null_ptr]), cap_state_type.clone());
    let cap_state_ptr = emitter.push_instruction(Instruction::StackAlloc(initial_cap_state), Type::POINTER);

    let cap_closures = mapvec(wrapper_ids.iter().zip(wrapper_closure_types.iter()), |(wrap_id, wrap_type)| {
        let pack = Instruction::PackClosure { function: Value::Definition(*wrap_id), environment: cap_state_ptr };
        emitter.push_instruction(pack, wrap_type.clone())
    });

    let cap_tuple = emitter.push_instruction(Instruction::MakeTuple(cap_closures), cap_tuple_type.clone());

    // user_data layout is `cap, env` - `env` is `()` when the body has no captures
    let env_value = body_env.unwrap_or(Value::Unit);
    let cap_and_env_type = Type::Tuple(Arc::new(vec![cap_tuple_type.clone(), env_type.clone()]));
    let cap_and_env =
        emitter.push_instruction(Instruction::MakeTuple(vec![cap_tuple, env_value]), cap_and_env_type.clone());
    let cap_and_env_ptr = emitter.push_instruction(Instruction::StackAlloc(cap_and_env), Type::POINTER);

    let body_wrapper_value = emitter.emit_definition_value(body_wrapper_id, body_wrapper_typ, caller_bindings.clone());
    let wrapper_ptr = emitter.push_instruction(Instruction::Transmute(body_wrapper_value), Type::POINTER);
    let coro = emitter.call_extern(&context.mco.init, vec![wrapper_ptr, cap_and_env_ptr]);

    // Patch cap_state to hold the real `coro` now that we have it
    let real_cap_state = emitter.push_instruction(Instruction::MakeTuple(vec![coro]), cap_state_type);
    emitter.push_instruction(Instruction::Store { pointer: cap_state_ptr, value: real_cap_state }, Type::UNIT);

    let mut drive_arguments = Vec::with_capacity(1 + cases.len());
    drive_arguments.push(coro);
    for case in &cases {
        drive_arguments.push(case.handler);
    }
    let drive_value = emitter.emit_definition_value(drive_id, drive_typ, caller_bindings.clone());
    emitter.reuse_instruction(
        original_id,
        Instruction::Call { function: drive_value, arguments: drive_arguments },
        result_type.clone(),
    );

    emitter.call_extern(&context.mco.free, vec![coro]);
    definition.blocks[block].instructions.splice(index..=index, pending);
}

/// Replace each [Instruction::Capability] within `body_fn_id`'s blocks with the chain that
/// recovers the capability from the running coroutine's user_data:
///
/// ```text
/// coro        = mco_coro_running()
/// user_data   = mco_coro_get_user_data(coro)
/// cap_and_env = Deref(user_data)              // (cap, env)
/// cap         = IndexTuple(cap_and_env, 0)    // reuses the Capability's instruction id
/// ```
///
/// The Capability's existing instruction id is reused for the final `IndexTuple` so any
/// downstream consumer of its result Value continues to work without rewriting.
fn expand_handler_caps_in_body(mir: &mut Mir, body_fn_id: DefinitionId, env_type: &Type, context: Context) {
    let Some(body) = mir.definitions.get_mut(&body_fn_id) else { return };

    struct CapSite {
        block: BlockId,
        index: usize,
        id: InstructionId,
        cap_type: Type,
    }
    let sites = collect_sites(body, |block, index, id| {
        matches!(body.instructions[id], Instruction::Capability).then(|| {
            let cap_type = body.instruction_result_types[id].clone();
            CapSite { block, index, id, cap_type }
        })
    });

    for site in sites {
        let cap_and_env_type = Type::Tuple(Arc::new(vec![site.cap_type.clone(), env_type.clone()]));
        let mut pending = Vec::new();
        let mut emitter = Emitter::pending(body, &mut pending);
        let coro = emitter.call_extern(&context.mco.running, vec![]);
        let user_data = emitter.call_extern(&context.mco.get_user_data, vec![coro]);
        let cap_and_env = emitter.push_instruction(Instruction::Deref(user_data), cap_and_env_type);
        emitter.reuse_instruction(site.id, Instruction::IndexTuple { tuple: cap_and_env, index: 0 }, site.cap_type);
        body.blocks[site.block].instructions.splice(site.index..=site.index, pending);
    }
}

/// Generate `wrap_i op_args.. (env: Pointer) -> ret_i`. The closure env carries the target coro.
/// The wrapper pushes (op_args, case_index, target) onto whichever coro is currently running and
/// suspends that coro. Each intermediate drive then either dispatches (if its own coro == target)
/// or forwards the message up through `mco_coro_transfer` until it reaches the target's drive.
/// After being resumed, the result has been pushed back onto the originally-running coro.
fn generate_capability_wrapper(
    mir: &mut Mir, case_index: u32, shape: &CaseShape, context: Context, caller_generic_count: u32,
) -> DefinitionId {
    let wrap_id = next_definition_id();
    let mut params: Vec<Type> = shape.op_arg_types.clone();
    params.push(Type::POINTER); // env (last entry-block parameter)

    let wrap_type = Type::Function(Arc::new(FunctionType {
        parameters: shape.op_arg_types.clone(),
        environment: Type::POINTER,
        return_type: shape.op_return_type.clone(),
    }));

    let mut definition =
        Definition::new(Arc::new(format!("handle_cap_wrap_{case_index}")), wrap_id, caller_generic_count, wrap_type);
    let entry = BlockId::ENTRY_BLOCK;
    for parameter_type in &params {
        definition.blocks[entry].parameter_types.push(parameter_type.clone());
    }
    let op_arg_count = shape.op_arg_types.len();
    let op_arg_values: Vec<Value> = (0..op_arg_count).map(|i| Value::Parameter(entry, i as u32)).collect();
    let env_value = Value::Parameter(entry, op_arg_count as u32);

    let mut emitter = Emitter::in_block(&mut definition, entry);

    // env: &(target_coro,)
    let state_type = Type::Tuple(Arc::new(vec![Type::POINTER]));
    let state = emitter.push_instruction(Instruction::Deref(env_value), state_type);
    let target = emitter.push_instruction(Instruction::IndexTuple { tuple: state, index: 0 }, Type::POINTER);

    let current = emitter.call_extern(&context.mco.running, vec![]);

    for (arg, arg_type) in op_arg_values.iter().zip(shape.op_arg_types.iter()) {
        emitter.push_bytes(context.mco, current, *arg, arg_type);
    }
    emitter.push_bytes(
        context.mco,
        current,
        Value::Integer(IntConstant::U32(case_index)),
        &Type::int(IntegerKind::U32),
    );
    emitter.push_bytes(context.mco, current, target, &Type::POINTER);
    emitter.call_extern(&context.mco.suspend, vec![current]);
    let result = emitter.pop_bytes(context.mco, current, &shape.op_return_type);

    definition.blocks[entry].terminator = Some(TerminatorInstruction::Return(result));
    mir.definitions.insert(wrap_id, definition);
    wrap_id
}

/// Generates `fn (coro: Pointer) -> Unit`, which calls the body and pushes its result for `drive` to pop.
fn generate_body_wrapper(
    mir: &mut Mir, body_fn_id: DefinitionId, body_bindings: Option<Arc<GenericBindings>>, env_type: &Type,
    cap_type: &Type, result_type: &Type, context: Context, caller_generic_count: u32,
) -> DefinitionId {
    let wrapper_id = next_definition_id();
    let wrapper_type = ptr_fn(vec![Type::POINTER], Type::UNIT);
    let body_type = mir.definitions[&body_fn_id].typ.clone();

    let mut definition =
        Definition::new(Arc::new("handle_body_wrapper".to_string()), wrapper_id, caller_generic_count, wrapper_type);
    let entry = BlockId::ENTRY_BLOCK;
    definition.blocks[entry].parameter_types.push(Type::POINTER);
    let coro = Value::Parameter(entry, 0);

    let mut emitter = Emitter::in_block(&mut definition, entry);

    let body_is_closure = matches!(&body_type, Type::Function(ft) if ft.is_closure());

    let result = if body_is_closure {
        // The body's prelude reads cap from user_data, only env needs reconstructing here.
        let user_data = emitter.call_extern(&context.mco.get_user_data, vec![coro]);
        let cap_and_env_type = Type::Tuple(Arc::new(vec![cap_type.clone(), env_type.clone()]));
        let cap_and_env = emitter.push_instruction(Instruction::Deref(user_data), cap_and_env_type);
        let env_value =
            emitter.push_instruction(Instruction::IndexTuple { tuple: cap_and_env, index: 1 }, env_type.clone());
        let body_fn_value = emitter.emit_definition_value(body_fn_id, body_type.clone(), body_bindings);
        let closure = emitter
            .push_instruction(Instruction::PackClosure { function: body_fn_value, environment: env_value }, body_type);
        emitter
            .push_instruction(Instruction::CallClosure { closure, arguments: vec![Value::Unit] }, result_type.clone())
    } else {
        let body_fn_value = emitter.emit_definition_value(body_fn_id, body_type, body_bindings);
        emitter.push_instruction(
            Instruction::Call { function: body_fn_value, arguments: vec![Value::Unit] },
            result_type.clone(),
        )
    };
    emitter.push_bytes(context.mco, coro, result, result_type);

    definition.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::Unit));
    mir.definitions.insert(wrapper_id, definition);
    wrapper_id
}

/// Per-Handle drive. The body coroutine is resumed in a loop. Each yield carries
/// (args, case_idx, target_coro) on the running coro's storage. The drive checks
/// `target == my_coro`, if so, it dispatches to the matching handler by `case_idx`,
/// otherwise it forwards the entire message bytewise to the parent coro, suspends the
/// parent, then transfers the result back and loops.
fn generate_drive_function(
    mir: &mut Mir, cases: &[HandlerCase], handler_types: &[Type], case_shapes: &[CaseShape], result_type: &Type,
    context: Context, caller_generic_count: u32,
) -> DefinitionId {
    let drive_id = next_definition_id();
    let mut drive_parameters = vec![Type::POINTER];
    drive_parameters.extend(handler_types.iter().cloned());
    let drive_type = ptr_fn(drive_parameters.clone(), result_type.clone());

    // One resume helper per case, capturing (coro, handlers..) to re-invoke drive on body resume.
    let resume_functions = mapvec(case_shapes, |shape| {
        generate_resume_function(
            mir,
            shape.op_return_type.clone(),
            drive_id,
            drive_type.clone(),
            handler_types,
            result_type,
            context,
            caller_generic_count,
        )
    });

    let mut definition =
        Definition::new(Arc::new("handle_drive".to_string()), drive_id, caller_generic_count, drive_type);
    let entry = BlockId::ENTRY_BLOCK;
    for parameter_type in &drive_parameters {
        definition.blocks[entry].parameter_types.push(parameter_type.clone());
    }
    let coro = Value::Parameter(entry, 0);
    let handler_parameters = mapvec(0..handler_types.len(), |i| Value::Parameter(entry, (i + 1) as u32));

    let loop_header = definition.blocks.push(Block::new(Vec::new()));
    let dispatch_block = definition.blocks.push(Block::new(Vec::new()));
    let complete_block = definition.blocks.push(Block::new(Vec::new()));
    let my_dispatch_block = definition.blocks.push(Block::new(Vec::new()));
    let forward_block = definition.blocks.push(Block::new(Vec::new()));
    let final_block = definition.blocks.push(Block::new(vec![result_type.clone()]));

    definition.blocks[entry].terminator = Some(TerminatorInstruction::jmp_no_args(loop_header));

    // loop_header: resume my_coro; if suspended -> dispatch else -> complete
    let suspended = {
        let mut emitter = Emitter::in_block(&mut definition, loop_header);
        emitter.call_extern(&context.mco.resume, vec![coro]);
        emitter.call_extern(&context.mco.is_suspended, vec![coro])
    };
    definition.blocks[loop_header].terminator = Some(TerminatorInstruction::If {
        condition: suspended,
        then: (dispatch_block, None),
        else_: (complete_block, None),
        end: complete_block,
    });

    // complete_block: pop R, jmp final_block(R)
    emit_pop_and_jmp(&mut definition, complete_block, coro, result_type, final_block, context);

    // dispatch_block: pop target, pop case_idx, branch on (target == my_coro)
    let usz_t = Type::int(IntegerKind::Usz);
    let (target, case_idx) = {
        let mut emitter = Emitter::in_block(&mut definition, dispatch_block);
        let target = emitter.pop_bytes(context.mco, coro, &Type::POINTER);
        let case_idx = emitter.pop_bytes(context.mco, coro, &Type::int(IntegerKind::U32));
        let target_int = emitter.push_instruction(Instruction::Transmute(target), usz_t.clone());
        let my_int = emitter.push_instruction(Instruction::Transmute(coro), usz_t.clone());
        let eq = emitter.push_instruction(Instruction::EqInt(target_int, my_int), Type::BOOL);

        // end = else_ since the branches don't converge, this is used elsewhere to skip the merge point.
        definition.blocks[dispatch_block].terminator = Some(TerminatorInstruction::If {
            condition: eq,
            then: (my_dispatch_block, None),
            else_: (forward_block, None),
            end: forward_block,
        });
        (target, case_idx)
    };

    // my_dispatch_block: switch case_idx over our local case indices.
    let mut switch_cases = Vec::with_capacity(cases.len());
    for (case_index, _case) in cases.iter().enumerate() {
        let case_block = definition.blocks.push(Block::new(Vec::new()));
        switch_cases.push((case_index as u32, (case_block, None)));

        emit_handler_case(
            &mut definition,
            case_block,
            coro,
            resume_functions[case_index],
            handler_parameters[case_index],
            &handler_parameters,
            &case_shapes[case_index],
            result_type,
            final_block,
            mir,
            context,
            caller_generic_count,
        );
    }

    let unreachable_else = definition.blocks.push(Block::new(Vec::new()));
    definition.blocks[unreachable_else].terminator = Some(TerminatorInstruction::Unreachable);

    definition.blocks[my_dispatch_block].terminator = Some(TerminatorInstruction::Switch {
        int_value: case_idx,
        cases: switch_cases,
        else_: (unreachable_else, None),
        end: final_block,
    });

    // forward_block: relay to parent, then transfer the result back and loop.
    let mut emitter = Emitter::in_block(&mut definition, forward_block);
    let parent = emitter.call_extern(&context.mco.running, vec![]);
    let n_args = emitter.call_extern(&context.mco.bytes_stored, vec![coro]);
    emitter.call_extern(&context.mco.transfer, vec![coro, parent, n_args]);
    emitter.push_bytes(context.mco, parent, case_idx, &Type::int(IntegerKind::U32));
    emitter.push_bytes(context.mco, parent, target, &Type::POINTER);
    emitter.call_extern(&context.mco.suspend, vec![parent]);
    let n_result = emitter.call_extern(&context.mco.bytes_stored, vec![parent]);
    emitter.call_extern(&context.mco.transfer, vec![parent, coro, n_result]);

    definition.blocks[forward_block].terminator = Some(TerminatorInstruction::jmp_no_args(loop_header));
    definition.blocks[final_block].terminator = Some(TerminatorInstruction::Return(Value::Parameter(final_block, 0)));
    mir.definitions.insert(drive_id, definition);
    drive_id
}

fn emit_pop_and_jmp(
    definition: &mut Definition, block: BlockId, coro: Value, value_type: &Type, jmp_target: BlockId, context: Context,
) {
    let mut emitter = Emitter::in_block(definition, block);
    let value = emitter.pop_bytes(context.mco, coro, value_type);
    definition.blocks[block].terminator = Some(TerminatorInstruction::Jmp((jmp_target, Some(value))));
}

#[allow(clippy::too_many_arguments)]
fn emit_handler_case(
    definition: &mut Definition, case_block: BlockId, coro: Value, resume_function_id: DefinitionId,
    handler_parameter: Value, all_handler_parameters: &[Value], shape: &CaseShape, result_type: &Type,
    final_block: BlockId, mir: &Mir, context: Context, caller_generic_count: u32,
) {
    let resume_function_type = mir.definitions.get(&resume_function_id).map(|d| d.typ.clone()).unwrap_or(Type::ERROR);

    let param_types =
        all_handler_parameters.iter().map(|value| definition.type_of_value(value, &mir.externals, &mir.definitions));

    let state_field_types = std::iter::once(Type::POINTER).chain(param_types).collect::<Vec<_>>();

    let mut emitter = Emitter::in_block(definition, case_block);
    let popped_arguments = pop_operation_arguments(&mut emitter, context, coro, &shape.op_arg_types);

    // resume's MIR-declared env type is Pointer, so its state is packed into a stack-allocated tuple.
    let mut state_elements = Vec::with_capacity(1 + all_handler_parameters.len());
    state_elements.push(coro);
    state_elements.extend(all_handler_parameters.iter().copied());
    let state =
        emitter.push_instruction(Instruction::MakeTuple(state_elements), Type::Tuple(Arc::new(state_field_types)));
    let environment = emitter.push_instruction(Instruction::StackAlloc(state), Type::POINTER);

    let resume_function_value = emitter.emit_definition_value(
        resume_function_id,
        resume_function_type.clone(),
        crate::mir::identity_bindings(caller_generic_count),
    );
    let resume_closure = emitter.push_instruction(
        Instruction::PackClosure { function: resume_function_value, environment },
        resume_function_type,
    );

    let mut handler_arguments = popped_arguments;
    handler_arguments.push(resume_closure);
    let call_instruction = if shape.handler_is_closure {
        Instruction::CallClosure { closure: handler_parameter, arguments: handler_arguments }
    } else {
        Instruction::Call { function: handler_parameter, arguments: handler_arguments }
    };
    let handler_result = emitter.push_instruction(call_instruction, result_type.clone());

    definition.blocks[case_block].terminator = Some(TerminatorInstruction::Jmp((final_block, Some(handler_result))));
}

/// `resume: fn r1 [Pointer] -> r2`; env is `&(coro, handler_0, .., handler_{N-1})`.
#[allow(clippy::too_many_arguments)]
fn generate_resume_function(
    mir: &mut Mir, r1_type: Type, drive_function_id: DefinitionId, drive_function_type: Type, handler_types: &[Type],
    result_type: &Type, context: Context, caller_generic_count: u32,
) -> DefinitionId {
    let mut state_field_types = Vec::with_capacity(1 + handler_types.len());
    state_field_types.push(Type::POINTER);
    state_field_types.extend(handler_types.iter().cloned());
    let state_tuple_type = Type::Tuple(Arc::new(state_field_types));

    let function_type = Type::Function(Arc::new(FunctionType {
        parameters: vec![r1_type.clone()],
        environment: Type::POINTER,
        return_type: result_type.clone(),
    }));

    let resume_id = next_definition_id();
    let mut definition =
        Definition::new(Arc::new("handle_resume".to_string()), resume_id, caller_generic_count, function_type);
    let entry = BlockId::ENTRY_BLOCK;
    definition.blocks[entry].parameter_types.push(r1_type.clone());
    definition.blocks[entry].parameter_types.push(Type::POINTER);
    let v_value = Value::Parameter(entry, 0);
    let environment_pointer = Value::Parameter(entry, 1);

    let mut emitter = Emitter::in_block(&mut definition, entry);

    let state = emitter.push_instruction(Instruction::Deref(environment_pointer), state_tuple_type);
    let coro = emitter.push_instruction(Instruction::IndexTuple { tuple: state, index: 0 }, Type::POINTER);
    let handler_values = mapvec(handler_types.iter().enumerate(), |(i, handler_type)| {
        emitter.push_instruction(Instruction::IndexTuple { tuple: state, index: (i + 1) as u32 }, handler_type.clone())
    });

    emitter.push_bytes(context.mco, coro, v_value, &r1_type);

    let mut drive_arguments = Vec::with_capacity(1 + handler_values.len());
    drive_arguments.push(coro);
    drive_arguments.extend(handler_values);
    let drive_value = emitter.emit_definition_value(
        drive_function_id,
        drive_function_type,
        crate::mir::identity_bindings(caller_generic_count),
    );
    let drive_result = emitter
        .push_instruction(Instruction::Call { function: drive_value, arguments: drive_arguments }, result_type.clone());

    definition.blocks[entry].terminator = Some(TerminatorInstruction::Return(drive_result));

    mir.definitions.insert(resume_id, definition);
    resume_id
}
