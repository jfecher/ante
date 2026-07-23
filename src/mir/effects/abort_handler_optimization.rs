//! Lower [super::Instruction::Handle]s whose handler cases never call resume into
//! setjmp/longjmp-based code. `Fail`, `Throw`, and `EarlyReturn` rely on this for performance
//! for now until we inline all handlers into their use sites.
//!
//! NOTE: Its possible we may keep this abort pass since it would be faster to compile for
//! debug mode than relying on all handlers to be specialized. It is broken though in that
//! an abort implementation skips any drops on the call stack.
//!
//! At each Handle site we stack-allocate a `jmp_buf` and a result slot, build
//! wrappers that store their handler-body value into the slot and `longjmp` back, then split
//! the containing block to emit `_setjmp(buf)` and branch into either a body call+store path
//! or a no-op path that just falls through to the merge block where the slot is loaded.
//!
//! Performs are left unchanged by this pass.
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::iterator_extensions::{mapvec, opt_mapvec};
use crate::lexer::token::IntegerKind;
use crate::mir::{
    Block, BlockId, Definition, DefinitionId, FunctionType, GenericBindings, HandlerCase, Instruction, InstructionId,
    IntConstant, Mir, TerminatorInstruction, Type, Value, next_definition_id,
};

use super::effect_lowering::{
    CaseShape, Emitter, case_shape_from_handler_type, resolve_body_function, run_handle_optimization_worklist,
};
use super::tail_resume_optimization::{
    OriginalEnvLayout, clone_body_with_extended_env, neutralize_handler_caps_in_dead_body, resolve_handler,
    rewrite_handler_caps_in_body, substitute_value,
};

impl Mir {
    pub(crate) fn optimize_abort_handlers(mut self) -> Self {
        run_handle_optimization_worklist(&mut self, optimize_in_definition);
        self
    }
}

/// Optimize every abort-only Handle in `definition_id`, returning the ids of any new definitions
/// created so the caller can enqueue them for processing.
fn optimize_in_definition(
    mir: &mut Mir, definition_id: DefinitionId, dead_bodies: &mut FxHashSet<DefinitionId>,
) -> Vec<DefinitionId> {
    collect_handle_sites(mir, definition_id)
        .into_iter()
        .flat_map(|site| try_optimize_handle(mir, definition_id, site, dead_bodies))
        .collect()
}

#[derive(Clone)]
struct HandleSite {
    block: BlockId,
    index: usize,
    id: InstructionId,
    body: Value,
    cases: Vec<HandlerCase>,
    result_type: Type,
}

fn collect_handle_sites(mir: &Mir, definition_id: DefinitionId) -> Vec<HandleSite> {
    let Some(def) = mir.definitions.get(&definition_id) else { return Vec::new() };
    let mut sites = Vec::new();
    for (block, block_data) in def.blocks.iter() {
        let start = sites.len();
        for (index, &id) in block_data.instructions.iter().enumerate() {
            if let Instruction::Handle { body, cases } = &def.instructions[id] {
                sites.push(HandleSite {
                    block,
                    index,
                    id,
                    body: *body,
                    cases: cases.clone(),
                    result_type: def.instruction_result_types[id].clone(),
                });
            }
        }
        sites[start..].reverse();
    }
    sites
}

struct CaseDecision {
    handler_def_id: DefinitionId,
    handler_env: Option<Value>,
    handler_bindings: Option<Arc<GenericBindings>>,
    handler_env_type: Type,
    handler_is_closure: bool,
    shape: CaseShape,
}

fn analyze_handle(mir: &Mir, definition_id: DefinitionId, site: &HandleSite) -> Option<Vec<CaseDecision>> {
    let definition = mir.definitions.get(&definition_id)?;
    opt_mapvec(&site.cases, |case| {
        let handler_type = definition.type_of_value(&case.handler, &mir.externals, &mir.definitions);
        let shape = case_shape_from_handler_type(&handler_type)?;
        let (handler_def_id, handler_env, handler_bindings) = resolve_handler(case.handler, definition)?;
        let handler_def = mir.definitions.get(&handler_def_id)?;
        if !case_is_abort_only(handler_def, &shape) {
            return None;
        }
        if !matches!(&handler_def.typ, Type::Function(_)) {
            return None;
        }
        let handler_env_type = match &handler_env {
            Some(env_value) => definition.type_of_value(env_value, &mir.externals, &mir.definitions),
            None => Type::UNIT,
        };
        let handler_is_closure = handler_env.is_some();
        Some(CaseDecision {
            handler_def_id,
            handler_env,
            handler_bindings,
            handler_env_type,
            handler_is_closure,
            shape,
        })
    })
}

/// True iff the handler's resume parameter is referenced nowhere in its body.
fn case_is_abort_only(handler_def: &Definition, shape: &CaseShape) -> bool {
    let resume_param = Value::Parameter(BlockId::ENTRY_BLOCK, shape.op_arg_types.len() as u32);
    let mut uses_resume = false;
    let mut check = |v: &Value| uses_resume |= *v == resume_param;
    for instr in handler_def.instructions.values() {
        instr.for_each_value(&mut check);
    }
    for (_, block) in handler_def.blocks.iter() {
        if let Some(t) = &block.terminator {
            t.for_each_value(&mut check);
        }
    }
    !uses_resume
}

fn try_optimize_handle(
    mir: &mut Mir, definition_id: DefinitionId, site: HandleSite, dead_bodies: &mut FxHashSet<DefinitionId>,
) -> Vec<DefinitionId> {
    let Some(decisions) = analyze_handle(mir, definition_id, &site) else { return Vec::new() };
    preserve_op_indices(&mut mir.preserved_op_indices, &site.cases);

    let wrapper_ids = mapvec(decisions.iter().enumerate(), |(i, d)| materialize_abort_wrapper(mir, d, i as u32));
    let cap_tuple_type = Type::Tuple(Arc::new(mapvec(&wrapper_ids, |id| mir.definitions[id].typ.clone())));

    let (prepared, created_ids) = prepare_body_fn(mir, definition_id, site.body, &cap_tuple_type, dead_bodies);
    splice_in_handle_replacement(mir, definition_id, site, &wrapper_ids, decisions, prepared, &cap_tuple_type);
    created_ids
}

fn preserve_op_indices(map: &mut FxHashMap<DefinitionId, u32>, cases: &[HandlerCase]) {
    for (i, case) in cases.iter().enumerate() {
        let i = u32::try_from(i).expect("effect with more than u32::MAX ops");
        map.insert(case.effect_op, i);
    }
}

struct PreparedBody {
    fn_id: DefinitionId,
    env_value: Option<Value>,
    bindings: Option<Arc<GenericBindings>>,
    layout: OriginalEnvLayout,
}

/// Returns the prepared body plus the ids of the body clones created here to be handled later
fn prepare_body_fn(
    mir: &mut Mir, definition_id: DefinitionId, body: Value, cap_tuple_type: &Type,
    dead_bodies: &mut FxHashSet<DefinitionId>,
) -> (PreparedBody, Vec<DefinitionId>) {
    let (orig_id, env_value, bindings) = resolve_body_function(body, &mir.definitions[&definition_id]);
    let (fn_id, layout, created_ids) = clone_body_with_extended_env(mir, orig_id, cap_tuple_type);
    neutralize_handler_caps_in_dead_body(mir, orig_id);
    rewrite_handler_caps_in_body(mir, fn_id, &layout, cap_tuple_type);

    // The original body fn is now dead (only the outer definition's dead PackClosure references it).
    // Don't re-process its Handles, the clone carries the live copies.
    dead_bodies.insert(orig_id);

    (PreparedBody { fn_id, env_value, bindings, layout }, created_ids)
}

/// Wrapper signature: `fn op_args.. [Pointer] -> op_return_type`. The Pointer points to a
/// stack-allocated `(buf_ptr, result_slot_ptr, handler_env)`. Each `Return v` in the
/// inlined handler body becomes `Store(slot, v); longjmp(buf, 1); unreachable`.
fn materialize_abort_wrapper(mir: &mut Mir, decision: &CaseDecision, case_index: u32) -> DefinitionId {
    let new_id = next_definition_id();
    let mut def =
        clone_handler_for_wrapper(&mir.definitions[&decision.handler_def_id], new_id, &decision.shape, case_index);
    let n = decision.shape.op_arg_types.len();
    let env_pointer = Value::Parameter(BlockId::ENTRY_BLOCK, n as u32);
    let old_env_param = Value::Parameter(BlockId::ENTRY_BLOCK, (n + 1) as u32);
    let (buf_ptr, result_slot, handler_env) =
        emit_wrapper_prologue(&mut def, env_pointer, &decision.handler_env_type, decision.handler_is_closure);
    if let Some(replacement) = handler_env {
        substitute_value(&mut def, old_env_param, replacement);
    }
    rewrite_returns_as_longjmp(&mut def, buf_ptr, result_slot);
    mir.definitions.insert(new_id, def);
    new_id
}

fn clone_handler_for_wrapper(
    handler_def: &Definition, new_id: DefinitionId, shape: &CaseShape, case_index: u32,
) -> Definition {
    let mut def = handler_def.clone_with_id(new_id);
    def.typ = Type::Function(Arc::new(FunctionType {
        parameters: shape.op_arg_types.clone(),
        environment: Type::POINTER,
        return_type: shape.op_return_type.clone(),
    }));
    def.name = Arc::new(format!("{}_abort_wrapper_{case_index}", handler_def.name));
    def.blocks[BlockId::ENTRY_BLOCK].parameter_types =
        shape.op_arg_types.iter().chain([&Type::POINTER]).cloned().collect();
    def
}

/// Emit `env = *env_pointer; (buf, slot, handler_env) = (env.0, env.1, env.2)` at the start
/// of the wrapper's entry block, returning the three Values.
fn emit_wrapper_prologue(
    def: &mut Definition, env_pointer: Value, handler_env_type: &Type, handler_is_closure: bool,
) -> (Value, Value, Option<Value>) {
    let env_struct_type = Type::Tuple(Arc::new(vec![Type::POINTER, Type::POINTER, handler_env_type.clone()]));
    let mut prologue = Vec::new();
    let mut e = Emitter::pending(def, &mut prologue);
    let env_struct = e.push_instruction(Instruction::Deref(env_pointer), env_struct_type);
    let buf_ptr = e.push_instruction(Instruction::IndexTuple { tuple: env_struct, index: 0 }, Type::POINTER);
    let result_slot = e.push_instruction(Instruction::IndexTuple { tuple: env_struct, index: 1 }, Type::POINTER);
    let handler_env = handler_is_closure
        .then(|| e.push_instruction(Instruction::IndexTuple { tuple: env_struct, index: 2 }, handler_env_type.clone()));
    def.blocks[BlockId::ENTRY_BLOCK].instructions.splice(0..0, prologue);
    (buf_ptr, result_slot, handler_env)
}

/// Replace each `Return v` terminator with `Store(slot, v); longjmp(buf, 1); unreachable`.
fn rewrite_returns_as_longjmp(def: &mut Definition, buf_ptr: Value, result_slot: Value) {
    let return_blocks: Vec<(BlockId, Value)> = def
        .blocks
        .iter()
        .filter_map(|(id, b)| match &b.terminator {
            Some(TerminatorInstruction::Return(v)) => Some((id, *v)),
            _ => None,
        })
        .collect();
    if return_blocks.is_empty() {
        return;
    }
    let longjmp_type = Type::Function(Arc::new(FunctionType {
        parameters: vec![Type::POINTER, Type::int(IntegerKind::I32)],
        environment: Type::NO_CLOSURE_ENV,
        return_type: Type::UNIT,
    }));
    let longjmp = Emitter::in_block(def, BlockId::ENTRY_BLOCK)
        .push_instruction(Instruction::Extern("mco_abort_longjmp".to_string()), longjmp_type);
    for (block, ret_val) in return_blocks {
        let mut e = Emitter::in_block(def, block);
        e.push_instruction(Instruction::Store { pointer: result_slot, value: ret_val }, Type::UNIT);
        e.push_instruction(
            Instruction::Call { function: longjmp, arguments: vec![buf_ptr, Value::Integer(IntConstant::I32(1))] },
            Type::UNIT,
        );
        def.blocks[block].terminator = Some(TerminatorInstruction::Unreachable);
    }
}

/// Splice the abort-handler replacement at the original Handle's site:
///
/// ```text
/// original_block:                       merge_block:
///   ...prologue..                         final = Deref(slot)  ; reuses Handle's id
///   idx = _setjmp(buf)                    ...post-Handle...
///   if idx == 0 { then } else { else }    <original terminator>
/// then_block:                           else_block:
///   r = body(env)                         jmp merge_block
///   Store(slot, r)
///   jmp merge_block
/// ```
fn splice_in_handle_replacement(
    mir: &mut Mir, definition_id: DefinitionId, site: HandleSite, wrapper_ids: &[DefinitionId],
    decisions: Vec<CaseDecision>, body: PreparedBody, cap_tuple_type: &Type,
) {
    let wrapper_closure_types = mapvec(wrapper_ids, |id| mir.definitions[id].typ.clone());
    let body_fn_type = mir.definitions[&body.fn_id].typ.clone();
    let definition = mir.definitions.get_mut(&definition_id).expect("definition vanished");

    let mut prologue: Vec<InstructionId> = Vec::new();
    let SetjmpPrologue { result_slot_ptr, body_closure, is_zero } = build_setjmp_prologue(
        definition,
        &mut prologue,
        &site.result_type,
        decisions,
        wrapper_ids,
        wrapper_closure_types,
        body,
        body_fn_type,
        cap_tuple_type,
    );

    let split = split_block_at_handle(definition, &site, prologue);
    let (then_block, else_block, merge_block) = create_branch_blocks(definition);
    definition.blocks[split.original_block].terminator = Some(TerminatorInstruction::If {
        condition: is_zero,
        then: (then_block, None),
        else_: (else_block, None),
        end: merge_block,
    });
    fill_then_block(definition, then_block, body_closure, result_slot_ptr, &site.result_type, merge_block);
    definition.blocks[else_block].terminator = Some(TerminatorInstruction::jmp_no_args(merge_block));
    finalize_merge_block(definition, merge_block, site.id, result_slot_ptr, site.result_type, split);
}

struct SetjmpPrologue {
    result_slot_ptr: Value,
    body_closure: Value,
    is_zero: Value,
}

#[allow(clippy::too_many_arguments)]
fn build_setjmp_prologue(
    definition: &mut Definition, prologue: &mut Vec<InstructionId>, result_type: &Type, decisions: Vec<CaseDecision>,
    wrapper_ids: &[DefinitionId], wrapper_closure_types: Vec<Type>, body: PreparedBody, body_fn_type: Type,
    cap_tuple_type: &Type,
) -> SetjmpPrologue {
    let mut e = Emitter::pending(definition, prologue);
    let buf_ptr = e.push_instruction(Instruction::StackAllocUninit(jmp_buf_type()), Type::POINTER);
    let result_slot_ptr = e.push_instruction(Instruction::StackAllocUninit(result_type.clone()), Type::POINTER);
    let cap_value =
        emit_cap_tuple(&mut e, decisions, wrapper_ids, wrapper_closure_types, buf_ptr, result_slot_ptr, cap_tuple_type);
    let body_closure = emit_body_closure(&mut e, body, body_fn_type, cap_value);
    let is_zero = emit_setjmp_test(&mut e, buf_ptr);
    SetjmpPrologue { result_slot_ptr, body_closure, is_zero }
}

/// 256 bytes should hopefully covers all common platforms. Windows seems to be the largest at 256B.
///
/// TODO: Tighten this or replace it, test on more platforms, etc.
fn jmp_buf_type() -> Type {
    Type::Tuple(Arc::new(vec![Type::int(IntegerKind::U64); 32]))
}

/// For each case, build `(buf, slot, handler_env)`, stack-alloc it, and pack the wrapper
/// closure with that pointer. Tuple all wrapper closures into the cap value.
fn emit_cap_tuple(
    e: &mut Emitter, decisions: Vec<CaseDecision>, wrapper_ids: &[DefinitionId], wrapper_closure_types: Vec<Type>,
    buf_ptr: Value, result_slot_ptr: Value, cap_tuple_type: &Type,
) -> Value {
    let cap_closures = mapvec(decisions.into_iter().zip(wrapper_ids).zip(wrapper_closure_types), |((d, &wid), wty)| {
        let cap_state_type = Type::Tuple(Arc::new(vec![Type::POINTER, Type::POINTER, d.handler_env_type]));
        let cap_state = e.push_instruction(
            Instruction::MakeTuple(vec![buf_ptr, result_slot_ptr, d.handler_env.unwrap_or(Value::Unit)]),
            cap_state_type,
        );
        let cap_state_ptr = e.push_instruction(Instruction::StackAlloc(cap_state), Type::POINTER);
        let wrapper_value = e.emit_definition_value(wid, wty.clone(), d.handler_bindings);
        e.push_instruction(Instruction::PackClosure { function: wrapper_value, environment: cap_state_ptr }, wty)
    });
    e.push_instruction(Instruction::MakeTuple(cap_closures), cap_tuple_type.clone())
}

/// Build the body's extended env (orig env fields + cap), then PackClosure body_fn with it.
fn emit_body_closure(e: &mut Emitter, body: PreparedBody, body_fn_type: Type, cap_value: Value) -> Value {
    let PreparedBody { fn_id, env_value, bindings, layout } = body;
    let new_env_type = match &body_fn_type {
        Type::Function(ft) => ft.environment.clone(),
        _ => panic!("body_fn type is not a function"),
    };
    let new_env_value = if layout.was_closure {
        let original_env = env_value.expect("body was a closure but no env value found");
        let mut fields = mapvec(layout.original_field_types.into_iter().enumerate(), |(i, ty)| {
            e.push_instruction(Instruction::IndexTuple { tuple: original_env, index: i as u32 }, ty)
        });
        fields.push(cap_value);
        e.push_instruction(Instruction::MakeTuple(fields), new_env_type)
    } else {
        e.push_instruction(Instruction::MakeTuple(vec![cap_value]), new_env_type)
    };
    let body_fn_value = e.emit_definition_value(fn_id, body_fn_type.clone(), bindings);
    e.push_instruction(Instruction::PackClosure { function: body_fn_value, environment: new_env_value }, body_fn_type)
}

/// `idx = _setjmp(buf); is_zero = idx == 0`. LLVM's TargetLibraryInfo recognizes `_setjmp`
/// by name and applies returns_twice automatically.
fn emit_setjmp_test(e: &mut Emitter, buf_ptr: Value) -> Value {
    let i32_t = Type::int(IntegerKind::I32);
    let setjmp_type = Type::Function(Arc::new(FunctionType {
        parameters: vec![Type::POINTER],
        environment: Type::NO_CLOSURE_ENV,
        return_type: i32_t.clone(),
    }));
    let setjmp_extern = e.push_instruction(Instruction::Extern("_setjmp".to_string()), setjmp_type);
    let setjmp_idx = e.push_instruction(Instruction::Call { function: setjmp_extern, arguments: vec![buf_ptr] }, i32_t);
    e.push_instruction(Instruction::EqInt(setjmp_idx, Value::Integer(IntConstant::I32(0))), Type::BOOL)
}

struct BlockSplit {
    original_block: BlockId,
    after_handle: Vec<InstructionId>,
    original_terminator: Option<TerminatorInstruction>,
}

/// Take instructions `[..handle_index]` + the prologue we built; everything after the Handle
/// (and the original terminator) gets stashed for the merge block.
fn split_block_at_handle(definition: &mut Definition, site: &HandleSite, prologue: Vec<InstructionId>) -> BlockSplit {
    let block = site.block;
    let original_terminator = definition.blocks[block].terminator.take();
    let after_handle: Vec<InstructionId> = definition.blocks[block].instructions.drain(site.index + 1..).collect();
    let removed = definition.blocks[block].instructions.pop();
    debug_assert_eq!(removed, Some(site.id), "handle_id mismatch when splitting block");
    definition.blocks[block].instructions.extend(prologue);
    BlockSplit { original_block: block, after_handle, original_terminator }
}

fn create_branch_blocks(definition: &mut Definition) -> (BlockId, BlockId, BlockId) {
    let then_block = definition.blocks.push(Block::new(Vec::new()));
    let else_block = definition.blocks.push(Block::new(Vec::new()));
    let merge_block = definition.blocks.push(Block::new(Vec::new()));
    (then_block, else_block, merge_block)
}

/// then_block: `r = body(()); Store(slot, r); jmp merge_block`.
fn fill_then_block(
    definition: &mut Definition, then_block: BlockId, body_closure: Value, result_slot_ptr: Value, result_type: &Type,
    merge_block: BlockId,
) {
    let mut e = Emitter::in_block(definition, then_block);
    let body_call = e.push_instruction(
        Instruction::CallClosure { closure: body_closure, arguments: vec![Value::Unit] },
        result_type.clone(),
    );
    e.push_instruction(Instruction::Store { pointer: result_slot_ptr, value: body_call }, Type::UNIT);
    drop(e);
    definition.blocks[then_block].terminator = Some(TerminatorInstruction::jmp_no_args(merge_block));
}

/// merge_block: reuse the original Handle's instruction id for `Deref(slot)`, then append the
/// instructions that lived after the Handle and the original block's terminator.
fn finalize_merge_block(
    definition: &mut Definition, merge_block: BlockId, handle_id: InstructionId, result_slot_ptr: Value,
    result_type: Type, split: BlockSplit,
) {
    definition.instructions[handle_id] = Instruction::Deref(result_slot_ptr);
    definition.instruction_result_types[handle_id] = result_type;
    definition.blocks[merge_block].instructions.push(handle_id);
    definition.blocks[merge_block].instructions.extend(split.after_handle);
    definition.blocks[merge_block].terminator = split.original_terminator;
}
