//! Lower [crate::mir::Instruction::Handle] and [crate::mir::Instruction::Perform]
//! into aminicoro primitives, using the *capability* design introduced on the
//! `capabilities` branch.
//!
//! Each effect-handler `handler h for op_0 args -> b0 | ... | op_{N-1} args -> bN-1 in body`
//! lowers as follows:
//!
//! 1. The body closure is heap-anchored and a coroutine `coro` is created via
//!    `mco_coro_init(body_entry, &body)`.
//! 2. For each case `i`, a free function `wrap_i(op_args.., env: Pointer) -> ret_i`
//!    is emitted. The wrapper's environment carries the coroutine pointer
//!    directly (returned by `mco_coro_init`), so the wrapper does **not** call
//!    `mco_coro_running()`. Body of wrap_i:
//!        state = deref env                       (env is &(coro, ..))
//!        coro  = state.0
//!        push coro op_args..
//!        push coro case_tag = i                  (local tag, 0..N-1, NOT global)
//!        mco_coro_suspend coro
//!        pop coro result
//!        return result
//! 3. Each `wrap_i` is `PackClosure`'d with the state-pointer environment to
//!    form the slot-`i` capability closure. The capability value passed to
//!    `body` is `MakeTuple(closure_0, .., closure_{N-1})`.
//! 4. A per-Handle `drive` function:
//!        drive(coro, handler_0, .., handler_{N-1}) -> r:
//!          mco_coro_resume coro
//!          if !is_suspended: pop r and return
//!          pop tag (u32)
//!          switch tag in 0..N:
//!            case i: pop op_i_args; resume_closure = PackClosure(resume_i, state)
//!                    r = handler_i(op_i_args.., resume_closure); jmp final(r)
//!          final r: return r
//!    drives the coroutine to completion. After drive returns, the caller
//!    `mco_coro_free`s `coro`. resume_i pushes the resumed value onto coro
//!    and recurses into drive. No global op-table, no forwarded cases —
//!    nested handlers compose through normal function calls because each
//!    capability wrapper carries its own coro pointer.
//!
//! Each `Perform { effect_op, arguments }` is rewritten as:
//!    cap = arguments.last()                       (the implicit capability)
//!    wrapper = IndexTuple(cap, op_index)
//!    result  = CallClosure(wrapper, op_args..)
//! where `op_index` is the operation's position within its parent effect.
//! The position is determined by scanning `Handle.cases` in the MIR — every
//! `Handle` of an effect agrees on case order (post-typecheck invariant).
//!
//! Because wrappers are ordinary closures, a future tail-resume optimization
//! can substitute a non-coroutine wrapper closure transparently — the
//! capability shape (`fn op_args.. [env] -> ret`) is identical.
use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::{
    iterator_extensions::mapvec,
    lexer::token::IntegerKind,
    mir::{
        Block, BlockId, Definition, DefinitionId, FunctionType, HandlerCase, Instruction, InstructionId, IntConstant,
        Mir, PrimitiveType, TerminatorInstruction, Type, Value, next_definition_id,
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
    }
}

impl Mir {
    pub(crate) fn lower_effects(mut self, ptr_size: u32) -> Self {
        if !contains_effects(&self) {
            return self;
        }
        let fns = aminicoro_fns();
        let op_index = build_op_index(&self);
        let context = Context { mco: &fns, op_index: &op_index, ptr_size };

        let definition_ids: Vec<DefinitionId> = self.definitions.keys().copied().collect();

        // First pass: rewrite every Handle. This produces wrapper free functions and a
        // per-Handle drive function. The Handle slot itself becomes a Call to drive.
        for id in &definition_ids {
            rewrite_sites_in_definition(&mut self, *id, context, collect_handle_sites, rewrite_single_handle);
        }

        // Second pass: rewrite every Perform into IndexTuple + CallClosure on the
        // capability. The wrappers generated in pass 1 also contain the suspend/pop
        // sequence already; they have no Performs in them, so this pass touches only
        // user code (and the effect-op stub definitions emitted by the MIR builder).
        for id in self.definitions.keys().copied().collect::<Vec<_>>() {
            rewrite_sites_in_definition(&mut self, id, context, collect_perform_sites, rewrite_single_perform);
        }

        self
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
/// Built by scanning every Handle's case list — case order in the MIR is the
/// canonical position order, established by the parser/type-checker.
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
    index
}

#[derive(Clone, Copy)]
struct Context<'local> {
    mco: &'local AminicoroFns,
    op_index: &'local OpIndex,
    ptr_size: u32,
}

fn size_of_const(typ: &Type, ptr_size: u32) -> Value {
    Value::Integer(IntConstant::Usz(typ.size_in_bytes(ptr_size) as usize))
}

fn is_zero_sized(typ: &Type) -> bool {
    match typ {
        Type::Primitive(PrimitiveType::Unit | PrimitiveType::NoClosureEnv) => true,
        Type::Tuple(fields) => fields.iter().all(is_zero_sized),
        _ => false,
    }
}

/// Placeholder value for stack slots that are immediately overwritten
/// (e.g. the destination of `mco_coro_pop`). Avoids [Value::Error] since
/// LLVM codegen rejects it.
fn dummy_value(typ: &Type) -> Value {
    match typ {
        Type::Primitive(primitive) => match primitive {
            PrimitiveType::Bool => Value::Bool(false),
            PrimitiveType::Char => Value::Char('\0'),
            PrimitiveType::Int(kind) => Value::Integer(match kind {
                IntegerKind::I8 => IntConstant::I8(0),
                IntegerKind::I16 => IntConstant::I16(0),
                IntegerKind::I32 => IntConstant::I32(0),
                IntegerKind::I64 => IntConstant::I64(0),
                IntegerKind::Isz => IntConstant::Isz(0),
                IntegerKind::U8 => IntConstant::U8(0),
                IntegerKind::U16 => IntConstant::U16(0),
                IntegerKind::U32 => IntConstant::U32(0),
                IntegerKind::U64 => IntConstant::U64(0),
                IntegerKind::Usz => IntConstant::Usz(0),
            }),
            _ => Value::Unit,
        },
        _ => Value::Unit,
    }
}

enum EmitTarget<'local> {
    Block(BlockId),
    Pending(&'local mut Vec<InstructionId>),
}

struct Emitter<'local> {
    definition: &'local mut Definition,
    target: EmitTarget<'local>,
}

impl<'local> Emitter<'local> {
    fn in_block(definition: &'local mut Definition, block: BlockId) -> Self {
        Self { definition, target: EmitTarget::Block(block) }
    }

    fn pending(definition: &'local mut Definition, pending: &'local mut Vec<InstructionId>) -> Self {
        Self { definition, target: EmitTarget::Pending(pending) }
    }

    fn push_instruction(&mut self, instruction: Instruction, result_type: Type) -> Value {
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

    fn push_bytes(&mut self, mco: &AminicoroFns, coro: Value, value: Value, value_type: &Type, ptr_size: u32) {
        if is_zero_sized(value_type) {
            return;
        }
        if let Type::Tuple(fields) = value_type {
            for (index, field_type) in fields.iter().enumerate() {
                let field = self.push_instruction(
                    Instruction::IndexTuple { tuple: value, index: index as u32 },
                    field_type.clone(),
                );
                self.push_bytes(mco, coro, field, field_type, ptr_size);
            }
            return;
        }
        let slot = self.push_instruction(Instruction::StackAlloc(value), Type::POINTER);
        let size = size_of_const(value_type, ptr_size);
        self.call_extern(&mco.push, vec![coro, slot, size]);
    }

    fn pop_bytes(&mut self, mco: &AminicoroFns, coro: Value, value_type: &Type, ptr_size: u32) -> Value {
        if is_zero_sized(value_type) {
            return Value::Unit;
        }
        if let Type::Tuple(fields) = value_type {
            let mut popped = mapvec(fields.iter().rev(), |field_type| self.pop_bytes(mco, coro, field_type, ptr_size));
            popped.reverse();
            return self.push_instruction(Instruction::MakeTuple(popped), value_type.clone());
        }
        let slot = self.push_instruction(Instruction::StackAlloc(dummy_value(value_type)), Type::POINTER);
        let size = size_of_const(value_type, ptr_size);
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

/// Pop a typed sequence of arguments off `coro`, restoring declared order.
/// aminicoro's channel is LIFO so args pushed last come off first.
fn pop_operation_arguments(emitter: &mut Emitter, context: Context, coro: Value, arg_types: &[Type]) -> Vec<Value> {
    let mut popped = mapvec(arg_types.iter().rev(), |typ| emitter.pop_bytes(context.mco, coro, typ, context.ptr_size));
    popped.reverse();
    popped
}

// ---------------------------------------------------------------------------
// Perform lowering
// ---------------------------------------------------------------------------

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

    let op_index = *context.op_index.get(&op).unwrap_or_else(|| {
        panic!("effect_lowering: effect op {op:?} has no Handle in the program — cannot resolve op-position")
    });

    // Capability is the implicit trailing argument, appended by implicit-arg resolution.
    let (cap_value, op_args) = arguments
        .split_last()
        .unwrap_or_else(|| panic!("effect_lowering: Perform {op:?} has no arguments — capability missing"));
    let cap_value = *cap_value;
    let op_args: Vec<Value> = op_args.to_vec();

    let definition = mir.definitions.get_mut(&definition_id).expect("definition disappeared mid-rewrite");

    // The wrapper closure's type is fn (op_args..) [Pointer] -> ret. It lives
    // at slot `op_index` of the capability tuple.
    let cap_type = definition.type_of_value(&cap_value, &FxHashMap::default(), &FxHashMap::default());
    let wrapper_type = match &cap_type {
        Type::Tuple(fields) => fields
            .get(op_index as usize)
            .cloned()
            .unwrap_or_else(|| panic!("effect_lowering: cap tuple has no slot {op_index} (cap_type = {cap_type:?})")),
        // If the capability isn't a known tuple type yet (upstream not finished wiring it),
        // fall back to inferring the wrapper type from the Perform itself.
        _ => Type::Function(Arc::new(FunctionType {
            parameters: mapvec(&op_args, |arg| {
                definition.type_of_value(arg, &FxHashMap::default(), &FxHashMap::default())
            }),
            environment: Type::POINTER,
            return_type: return_type.clone(),
        })),
    };

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

// ---------------------------------------------------------------------------
// Handle lowering
// ---------------------------------------------------------------------------

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
struct CaseShape {
    op_arg_types: Vec<Type>,
    /// The op's return type (= resume's parameter type).
    op_return_type: Type,
    handler_is_closure: bool,
}

fn case_shape_from_handler_type(handler_type: &Type) -> Option<CaseShape> {
    let Type::Function(ft) = handler_type else { return None };
    let (resume_param, op_args) = ft.parameters.split_last()?;
    let Type::Function(resume_ft) = resume_param else { return None };
    let op_return_type = resume_ft.parameters.first().cloned().unwrap_or(Type::UNIT);
    Some(CaseShape { op_arg_types: op_args.to_vec(), op_return_type, handler_is_closure: ft.is_closure() })
}

fn rewrite_single_handle(mir: &mut Mir, definition_id: DefinitionId, site: HandleSite, context: Context) {
    let HandleSite { block, index, id: original_id, body, cases, result_type } = site;

    let definition = mir.definitions.get(&definition_id).expect("definition vanished mid-rewrite");
    let body_type = definition.type_of_value(&body, &mir.externals, &mir.definitions);
    let handler_types =
        mapvec(&cases, |case| definition.type_of_value(&case.handler, &mir.externals, &mir.definitions));
    let case_shapes: Vec<CaseShape> = handler_types
        .iter()
        .map(|t| case_shape_from_handler_type(t).expect("handler type must be `fn op_args.., resume -> r`"))
        .collect();

    // Per-case capability-wrapper free function definitions. wrap_i(op_args.., env: Pointer)
    // suspends `coro` (held in env) with op_args + case-tag i. After the body resumes, it
    // pops the returned value and returns it.
    let wrapper_ids: Vec<DefinitionId> = case_shapes
        .iter()
        .enumerate()
        .map(|(i, shape)| generate_capability_wrapper(mir, i as u32, shape, context))
        .collect();
    let wrapper_closure_types: Vec<Type> =
        wrapper_ids.iter().map(|id| mir.definitions[id].typ.clone()).map(closurize_wrapper_type).collect();
    let cap_tuple_type = Type::Tuple(Arc::new(wrapper_closure_types.clone()));

    let body_wrapper_id = generate_body_wrapper(mir, body_type.clone(), &cap_tuple_type, &result_type, context);
    let drive_id = generate_drive_function(mir, &cases, &handler_types, &case_shapes, &result_type, context);

    let definition = mir.definitions.get_mut(&definition_id).expect("definition disappeared mid-rewrite");
    let mut pending = Vec::new();

    {
        let mut emitter = Emitter::pending(definition, &mut pending);

        let body_slot = emitter.push_instruction(Instruction::StackAlloc(body), Type::POINTER);
        let wrapper_ptr =
            emitter.push_instruction(Instruction::Transmute(Value::Definition(body_wrapper_id)), Type::POINTER);
        let coro = emitter.call_extern(&context.mco.init, vec![wrapper_ptr, body_slot]);

        // Build the capability tuple: each wrap_i is closure-packed with an env holding `coro`.
        // The same `coro` is passed to drive below; consistency between body's wrappers and
        // drive's pop sequence is what makes this Handle site self-consistent.
        let cap_state_type = Type::Tuple(Arc::new(vec![Type::POINTER]));
        let cap_state = emitter.push_instruction(Instruction::MakeTuple(vec![coro]), cap_state_type.clone());
        let cap_state_ptr = emitter.push_instruction(Instruction::StackAlloc(cap_state), Type::POINTER);

        let cap_closures: Vec<Value> = wrapper_ids
            .iter()
            .zip(wrapper_closure_types.iter())
            .map(|(wrap_id, wrap_type)| {
                emitter.push_instruction(
                    Instruction::PackClosure { function: Value::Definition(*wrap_id), environment: cap_state_ptr },
                    wrap_type.clone(),
                )
            })
            .collect();
        let cap_tuple = emitter.push_instruction(Instruction::MakeTuple(cap_closures), cap_tuple_type.clone());

        // Stash the capability tuple in the coroutine's user_data slot alongside body so the
        // body wrapper can pull it out and pass it to body. This avoids changing body's call
        // signature in the MIR builder. The user_data points to a (body, cap) pair.
        // TODO: When upstream MIR construction binds `h` to a parameter on `body` directly,
        // this indirection collapses to passing `cap` as a body argument.
        let body_and_cap_type = Type::Tuple(Arc::new(vec![body_type.clone(), cap_tuple_type.clone()]));
        let body_and_cap =
            emitter.push_instruction(Instruction::MakeTuple(vec![body, cap_tuple]), body_and_cap_type.clone());
        let body_and_cap_ptr = emitter.push_instruction(Instruction::StackAlloc(body_and_cap), Type::POINTER);

        // We initialized the coroutine before having the cap, so re-init now that we have a
        // proper user_data pointer. (Equivalently: we could split cap construction before init,
        // but cap_state needs `coro` itself, so the order is fixed: init→build cap→re-init.)
        // mco_coro_init is idempotent for our purposes — replacing user_data is the only effect.
        let _ = body_and_cap_ptr;
        // Instead, overwrite the original body_slot's content with the (body, cap) pair so the
        // body wrapper sees both when it derefs user_data.
        emitter.push_instruction(Instruction::Store { pointer: body_slot, value: body_and_cap }, Type::UNIT);

        // drive(coro, handler_0, .., handler_{N-1}) returns the Handle's result type.
        let mut drive_arguments = Vec::with_capacity(1 + cases.len());
        drive_arguments.push(coro);
        for case in &cases {
            drive_arguments.push(case.handler);
        }
        emitter.reuse_instruction(
            original_id,
            Instruction::Call { function: Value::Definition(drive_id), arguments: drive_arguments },
            result_type.clone(),
        );

        emitter.call_extern(&context.mco.free, vec![coro]);
    }
    definition.blocks[block].instructions.splice(index..=index, pending);
}

/// Convert the bare wrapper function type into its closure (capability slot) type.
/// Wrappers are emitted as `fn (op_args.., env: Pointer) -> ret` (a free function whose
/// last parameter is the env). The capability slot type is the closure-shaped equivalent
/// `fn (op_args..) [Pointer] -> ret`.
fn closurize_wrapper_type(wrapper_type: Type) -> Type {
    let Type::Function(ft) = wrapper_type else { return Type::ERROR };
    let (env, op_args) = match ft.parameters.split_last() {
        Some((env, rest)) => (env.clone(), rest.to_vec()),
        None => (Type::POINTER, Vec::new()),
    };
    Type::Function(Arc::new(FunctionType {
        parameters: op_args,
        environment: env,
        return_type: ft.return_type.clone(),
    }))
}

/// Generate `wrap_i(op_args.., env: Pointer) -> ret_i`. Pulls `coro` out of env, pushes
/// op_args.. + tag=i onto coro, suspends, pops the resumed result, returns it.
fn generate_capability_wrapper(mir: &mut Mir, case_index: u32, shape: &CaseShape, context: Context) -> DefinitionId {
    let wrap_id = next_definition_id();
    let mut params: Vec<Type> = shape.op_arg_types.clone();
    params.push(Type::POINTER); // env
    let wrap_type = ptr_fn(params.clone(), shape.op_return_type.clone());

    let mut definition = Definition::new(Arc::new(format!("handle_cap_wrap_{case_index}")), wrap_id, 0, wrap_type);
    let entry = BlockId::ENTRY_BLOCK;
    for parameter_type in &params {
        definition.blocks[entry].parameter_types.push(parameter_type.clone());
    }
    let op_arg_count = shape.op_arg_types.len();
    let op_arg_values: Vec<Value> = (0..op_arg_count).map(|i| Value::Parameter(entry, i as u32)).collect();
    let env_value = Value::Parameter(entry, op_arg_count as u32);

    let mut emitter = Emitter::in_block(&mut definition, entry);

    // env: &(coro,)
    let state_type = Type::Tuple(Arc::new(vec![Type::POINTER]));
    let state = emitter.push_instruction(Instruction::Deref(env_value), state_type);
    let coro = emitter.push_instruction(Instruction::IndexTuple { tuple: state, index: 0 }, Type::POINTER);

    // push op_args.. then tag, suspend, pop result.
    for (arg, arg_type) in op_arg_values.iter().zip(shape.op_arg_types.iter()) {
        emitter.push_bytes(context.mco, coro, *arg, arg_type, context.ptr_size);
    }
    emitter.push_bytes(
        context.mco,
        coro,
        Value::Integer(IntConstant::U32(case_index)),
        &Type::int(IntegerKind::U32),
        context.ptr_size,
    );
    emitter.call_extern(&context.mco.suspend, vec![coro]);
    let result = emitter.pop_bytes(context.mco, coro, &shape.op_return_type, context.ptr_size);

    definition.blocks[entry].terminator = Some(TerminatorInstruction::Return(result));
    mir.definitions.insert(wrap_id, definition);
    wrap_id
}

/// Generate `fn (coro: Pointer) -> Unit`. Reads the (body, cap) pair from the coroutine's
/// user_data, invokes body with cap as its argument, and pushes the result onto the
/// coroutine's channel for `drive` to pop.
fn generate_body_wrapper(
    mir: &mut Mir, body_type: Type, cap_type: &Type, result_type: &Type, context: Context,
) -> DefinitionId {
    let wrapper_id = next_definition_id();
    let wrapper_type = ptr_fn(vec![Type::POINTER], Type::UNIT);

    let mut definition = Definition::new(Arc::new("handle_body_wrapper".to_string()), wrapper_id, 0, wrapper_type);
    let entry = BlockId::ENTRY_BLOCK;
    definition.blocks[entry].parameter_types.push(Type::POINTER);
    let coro = Value::Parameter(entry, 0);

    let mut emitter = Emitter::in_block(&mut definition, entry);

    let user_data = emitter.call_extern(&context.mco.get_user_data, vec![coro]);
    let body_and_cap_type = Type::Tuple(Arc::new(vec![body_type.clone(), cap_type.clone()]));
    let body_and_cap = emitter.push_instruction(Instruction::Deref(user_data), body_and_cap_type);
    let body_value =
        emitter.push_instruction(Instruction::IndexTuple { tuple: body_and_cap, index: 0 }, body_type.clone());
    let cap_value =
        emitter.push_instruction(Instruction::IndexTuple { tuple: body_and_cap, index: 1 }, cap_type.clone());

    let call = match &body_type {
        Type::Function(function_type) if function_type.is_closure() => {
            Instruction::CallClosure { closure: body_value, arguments: vec![cap_value] }
        },
        Type::Tuple(_) => Instruction::CallClosure { closure: body_value, arguments: vec![cap_value] },
        _ => Instruction::Call { function: body_value, arguments: vec![cap_value] },
    };
    let result = emitter.push_instruction(call, result_type.clone());
    emitter.push_bytes(context.mco, coro, result, result_type, context.ptr_size);

    definition.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::Unit));
    mir.definitions.insert(wrapper_id, definition);
    wrapper_id
}

/// Per-Handle drive: switches over only this Handle's cases (0..N-1, by case
/// index — *not* a global op-tag). Forwarding is handled implicitly by
/// capability wrappers carrying their own coro pointer, so there's no
/// "forward to outer coro" branch.
fn generate_drive_function(
    mir: &mut Mir, cases: &[HandlerCase], handler_types: &[Type], case_shapes: &[CaseShape], result_type: &Type,
    context: Context,
) -> DefinitionId {
    let drive_id = next_definition_id();
    let mut drive_parameters = vec![Type::POINTER];
    drive_parameters.extend(handler_types.iter().cloned());
    let drive_type = ptr_fn(drive_parameters.clone(), result_type.clone());

    // One resume helper per case. Each captures (coro, handlers..) so it can
    // re-invoke drive with the same handler set when the body resumes.
    let resume_functions: Vec<DefinitionId> = case_shapes
        .iter()
        .map(|shape| {
            generate_resume_function(mir, shape.op_return_type.clone(), drive_id, handler_types, result_type, context)
        })
        .collect();

    let mut definition = Definition::new(Arc::new("handle_drive".to_string()), drive_id, 0, drive_type);
    let entry = BlockId::ENTRY_BLOCK;
    for parameter_type in &drive_parameters {
        definition.blocks[entry].parameter_types.push(parameter_type.clone());
    }
    let coro = Value::Parameter(entry, 0);
    let handler_parameters = mapvec(0..handler_types.len(), |i| Value::Parameter(entry, (i + 1) as u32));

    let dispatch_block = definition.blocks.push(Block::new(Vec::new()));
    let complete_block = definition.blocks.push(Block::new(Vec::new()));
    let final_block = definition.blocks.push(Block::new(vec![result_type.clone()]));

    let mut emitter = Emitter::in_block(&mut definition, entry);
    emitter.call_extern(&context.mco.resume, vec![coro]);
    let suspended = emitter.call_extern(&context.mco.is_suspended, vec![coro]);

    definition.blocks[entry].terminator = Some(TerminatorInstruction::If {
        condition: suspended,
        then: (dispatch_block, None),
        else_: (complete_block, None),
        end: complete_block,
    });

    // complete_block: pop R, jmp final_block(R)
    emit_pop_and_jmp(&mut definition, complete_block, coro, result_type, final_block, context);

    // dispatch_block: pop u32 case-tag, switch over local case indices 0..N-1.
    let mut emitter = Emitter::in_block(&mut definition, dispatch_block);
    let tag = emitter.pop_bytes(context.mco, coro, &Type::int(IntegerKind::U32), context.ptr_size);

    let mut case_blocks = Vec::with_capacity(cases.len());
    for (case_index, _case) in cases.iter().enumerate() {
        let case_block = definition.blocks.push(Block::new(Vec::new()));
        case_blocks.push((case_index as u32, (case_block, None)));

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
        );
    }

    let unreachable_else = definition.blocks.push(Block::new(Vec::new()));
    definition.blocks[unreachable_else].terminator = Some(TerminatorInstruction::Unreachable);

    definition.blocks[dispatch_block].terminator = Some(TerminatorInstruction::Switch {
        int_value: tag,
        cases: case_blocks,
        else_: Some((unreachable_else, None)),
        end: final_block,
    });

    definition.blocks[final_block].terminator = Some(TerminatorInstruction::Return(Value::Parameter(final_block, 0)));

    mir.definitions.insert(drive_id, definition);
    drive_id
}

fn emit_pop_and_jmp(
    definition: &mut Definition, block: BlockId, coro: Value, value_type: &Type, jmp_target: BlockId, context: Context,
) {
    let value = {
        let mut emitter = Emitter::in_block(definition, block);
        emitter.pop_bytes(context.mco, coro, value_type, context.ptr_size)
    };
    definition.blocks[block].terminator = Some(TerminatorInstruction::Jmp((jmp_target, Some(value))));
}

#[allow(clippy::too_many_arguments)]
fn emit_handler_case(
    definition: &mut Definition, case_block: BlockId, coro: Value, resume_function_id: DefinitionId,
    handler_parameter: Value, all_handler_parameters: &[Value], shape: &CaseShape, result_type: &Type,
    final_block: BlockId, mir: &Mir, context: Context,
) {
    let resume_function_type = mir.definitions.get(&resume_function_id).map(|d| d.typ.clone()).unwrap_or(Type::ERROR);

    let state_field_types = std::iter::once(Type::POINTER)
        .chain(
            all_handler_parameters
                .iter()
                .map(|value| definition.type_of_value(value, &mir.externals, &mir.definitions)),
        )
        .collect::<Vec<_>>();

    let mut emitter = Emitter::in_block(definition, case_block);
    let popped_arguments = pop_operation_arguments(&mut emitter, context, coro, &shape.op_arg_types);

    // Build the resume closure's environment: pack (coro, handlers..) into an
    // inline tuple and stack-allocate it. Same shape as the original lowering;
    // resume's MIR-declared env type is Pointer.
    let mut state_elements = Vec::with_capacity(1 + all_handler_parameters.len());
    state_elements.push(coro);
    state_elements.extend(all_handler_parameters.iter().copied());
    let state =
        emitter.push_instruction(Instruction::MakeTuple(state_elements), Type::Tuple(Arc::new(state_field_types)));
    let environment = emitter.push_instruction(Instruction::StackAlloc(state), Type::POINTER);

    let resume_closure = emitter.push_instruction(
        Instruction::PackClosure { function: Value::Definition(resume_function_id), environment },
        resume_function_type,
    );

    // Call the handler: handler(args.., resume_closure)
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

/// `resume: fn r1 [Pointer] -> r2`. env is `&(coro, handler_0, .., handler_{N-1})`,
/// set up by the drive function before each handler invocation.
fn generate_resume_function(
    mir: &mut Mir, r1_type: Type, drive_function_id: DefinitionId, handler_types: &[Type], result_type: &Type,
    context: Context,
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
    let mut definition = Definition::new(Arc::new("handle_resume".to_string()), resume_id, 0, function_type);
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

    emitter.push_bytes(context.mco, coro, v_value, &r1_type, context.ptr_size);

    let mut drive_arguments = Vec::with_capacity(1 + handler_values.len());
    drive_arguments.push(coro);
    drive_arguments.extend(handler_values);
    let drive_result = emitter.push_instruction(
        Instruction::Call { function: Value::Definition(drive_function_id), arguments: drive_arguments },
        result_type.clone(),
    );

    definition.blocks[entry].terminator = Some(TerminatorInstruction::Return(drive_result));

    mir.definitions.insert(resume_id, definition);
    resume_id
}
