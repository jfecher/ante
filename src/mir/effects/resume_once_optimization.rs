//! Compile [Instruction::Handle]s without coroutines if their handler cases each:
//! - Call `resume` exactly once
//! - Do not store `resume`
//! - Do not resume in a loop
//! - Do not allocate a dynamically sized stack slot before resuming
//!
//! Each case `pre; y = resume v; post y` is split at its `resume` call:
//! - The part before `resume` becomes a function the capability calls directly. It pushes a
//!   frame, saves into it every value the part after `resume` uses, and returns `v` to the
//!   performer.
//! - The part after `resume` becomes a function taking that frame and `y`.
//!
//! Once the handled body returns `r`, the frames are popped and each do `r = post(frame, r)`.
//!
//! Frames live in chunks which are never moved or reallocated. The first chunk is a buffer of
//! [INLINE_FRAMES] frames on the stack of the function containing the Handle. Each later chunk is
//! heap allocated with twice the frames of the one before it.
//!
//! This pass is like half of a state-machine transformation. Maybe it should be a full conversion
//! to a state machine in the future.
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    iterator_extensions::{mapvec, opt_mapvec},
    lexer::token::IntegerKind,
    mir::{
        Block, BlockId, Definition, DefinitionId, GenericBindings, Instruction, InstructionId, IntConstant,
        MAX_ALIGNMENT, Mir, TerminatorInstruction, Type, Value, identity_bindings, next_definition_id,
    },
};

use super::{
    abort_handler_optimization::{clone_handler_for_wrapper, emit_body_closure, prepare_body_fn, preserve_op_indices},
    effect_lowering::{
        CaseShape, Emitter, WorklistState, case_shape_from_handler_type, resolve_body_function,
        run_handle_optimization_worklist,
    },
    tail_resume_optimization::{HandleSite, collect_handle_sites},
};

/// The number of frames in the chunk on the stack of the function containing the Handle
const INLINE_FRAMES: u32 = 4;

/// Bytes before the first frame of a heap chunk, holding its [chunk_header_type]. A multiple of
/// [MAX_ALIGNMENT] so frames are aligned.
const CHUNK_HEADER_SIZE: usize = 32;
const _: () = assert!(CHUNK_HEADER_SIZE % MAX_ALIGNMENT as usize == 0);

/// Fields of [state_type]
mod state_field {
    /// Where the next frame will be pushed
    pub(super) const TOP: u32 = 0;
    /// The end of the current chunk's frames
    pub(super) const END: u32 = 1;
    /// The first frame of the current chunk
    pub(super) const BASE: u32 = 2;
    /// The current chunk if it's heap allocated, otherwise null
    pub(super) const CHUNK: u32 = 3;
    /// The number of frames the current chunk holds
    pub(super) const CAPACITY: u32 = 4;
    /// The size of each frame
    pub(super) const STRIDE: u32 = 5;
    /// The offset of the tag within each frame
    pub(super) const TAG_OFFSET: u32 = 6;
    pub(super) const COUNT: u32 = 7;
}

/// The frame stack of a Handle. Lives on the stack of the function containing the Handle.
fn state_type() -> Type {
    Type::tuple(mapvec(0..state_field::COUNT, state_field_type))
}

/// The pointer fields of [state_type] come before its integer fields
fn state_field_type(field: u32) -> Type {
    if field < state_field::CAPACITY { Type::POINTER } else { usz_type() }
}

/// The start of each heap chunk: the state's `BASE`, `END`, `CHUNK`, and `CAPACITY` fields before
/// the chunk was pushed, which are restored once all of its frames are popped
fn chunk_header_type() -> Type {
    Type::tuple(vec![Type::POINTER, Type::POINTER, Type::POINTER, usz_type()])
}

fn usz_type() -> Type {
    Type::int(IntegerKind::Usz)
}

fn usz(value: usize) -> Value {
    Value::Integer(IntConstant::Usz(value))
}

fn null() -> Instruction {
    Instruction::Transmute(usz(0))
}

/// `pointer + offset`
fn offset_pointer(e: &mut Emitter, pointer: Value, offset: Value) -> Value {
    e.push_instruction(Instruction::PointerOffset { pointer, offset }, Type::POINTER)
}

/// `pointer` as a `Usz`
fn address(e: &mut Emitter, pointer: Value) -> Value {
    e.push_instruction(Instruction::Transmute(pointer), usz_type())
}

fn pointers_equal(e: &mut Emitter, a: Value, b: Value) -> Value {
    let a = address(e, a);
    let b = address(e, b);
    e.push_instruction(Instruction::EqInt(a, b), Type::BOOL)
}

fn is_null(e: &mut Emitter, pointer: Value) -> Value {
    let address = address(e, pointer);
    e.push_instruction(Instruction::EqInt(address, usz(0)), Type::BOOL)
}

/// A new definition of type `fn parameters.. -> return_type` with only an entry block
fn new_function(name: &str, generic_count: u32, parameters: Vec<Type>, return_type: Type) -> Definition {
    let typ = Type::function(parameters.clone(), return_type);
    let mut definition = Definition::new(Arc::new(name.to_string()), next_definition_id(), generic_count, typ);
    definition.blocks[BlockId::ENTRY_BLOCK].parameter_types = parameters;
    definition
}

fn insert_definition(mir: &mut Mir, definition: Definition) -> DefinitionId {
    let id = definition.id;
    mir.definitions.insert(id, definition);
    id
}

impl Mir {
    pub(crate) fn optimize_resume_once(mut self) -> Self {
        let mut helpers = None;
        run_handle_optimization_worklist(&mut self, |mir, definition_id, state| {
            let mut new_ids = Vec::new();
            for site in collect_handle_sites(mir, definition_id) {
                new_ids.extend(try_optimize_handle(mir, definition_id, site, state, &mut helpers));
            }
            new_ids
        });
        self
    }
}

/// A case of a Handle which resumes exactly once, along with where to split it
struct CaseSplit {
    handler_id: DefinitionId,
    handler_env: Option<Value>,
    handler_bindings: Option<Arc<GenericBindings>>,
    shape: CaseShape,
    resume_block: BlockId,
    /// The index of the `resume` call within `resume_block`
    resume_index: usize,
    resume_id: InstructionId,
    resume_argument: Value,
    /// Each stack slot allocated before `resume`, in the order of its field in `frame_type`
    slots: Vec<InstructionId>,
    /// Each value defined before `resume` and used after it, in the order of its field in
    /// `frame_type` after the slots
    live: Vec<Value>,
    /// `(slots.., live..)` in terms of the handler's generics
    frame_type: Type,
}

fn try_optimize_handle(
    mir: &mut Mir, definition_id: DefinitionId, site: HandleSite, state: &mut WorklistState,
    helpers: &mut Option<FrameStackHelpers>,
) -> Vec<DefinitionId> {
    let Some(splits) = analyze_handle(mir, definition_id, &site) else { return Vec::new() };
    let helpers = *helpers.get_or_insert_with(|| FrameStackHelpers::new(mir));

    // Preserve the op→index mapping so [crate::mir::effects::effect_lowering] can still lower
    // `Perform`s targeting these ops after we remove the Handle. See `optimize_tail_resume`.
    preserve_op_indices(&mut mir.preserved_op_indices, &site.cases);

    let pre_ids = mapvec(splits.iter().enumerate(), |(i, split)| build_pre_function(mir, split, helpers, i as u32));
    let post_ids = mapvec(&splits, |split| build_post_function(mir, split));

    // Types as seen from the Handle's definition
    let substitute = |typ: &Type, split: &CaseSplit| typ.substitute_optional(split.handler_bindings.as_deref());
    let pre_types = mapvec(pre_ids.iter().zip(&splits), |(id, split)| substitute(&mir.definitions[id].typ, split));
    let frame_union = Type::Union(Arc::new(mapvec(&splits, |split| substitute(&split.frame_type, split))));
    let frame_type = Type::tuple(vec![frame_union, Type::int(IntegerKind::U32)]);
    let cap_tuple_type = Type::tuple(pre_types.clone());

    let posts = mapvec(post_ids.iter().zip(&splits), |(id, split)| PostFunction {
        id: *id,
        typ: substitute(&mir.definitions[id].typ, split),
        bindings: split.handler_bindings.clone(),
    });
    let caller_generic_count = mir.definitions[&definition_id].generic_count;
    let unwind_id = build_unwind_function(mir, &site.result_type, &frame_type, posts, helpers, caller_generic_count);

    // Clone the body to extend its environment with the capability, see `optimize_tail_resume`
    let (body, mut new_ids) = prepare_body_fn(mir, definition_id, site.body, &cap_tuple_type, state);

    let body_type = mir.definitions[&body.fn_id].typ.clone();
    let unwind_type = mir.definitions[&unwind_id].typ.clone();
    let site_definition = &mir.definitions[&definition_id];
    let handler_env_types = mapvec(&splits, |split| {
        split.handler_env.map(|env| site_definition.type_of_value(&env, &mir.externals, &mir.definitions))
    });

    let definition = mir.definitions.get_mut(&definition_id).expect("definition vanished mid-rewrite");
    let mut pending = Vec::new();
    let mut emitter = Emitter::pending(definition, &mut pending);

    // The frame stack, starting with a chunk on this function's stack
    let inline_chunk_type =
        Type::Array { length: Arc::new(Type::U32(INLINE_FRAMES)), element: Arc::new(frame_type.clone()) };
    let inline_chunk = emitter.push_instruction(Instruction::StackAllocUninit(inline_chunk_type), Type::POINTER);
    let stride = emitter.emit_size_of(&frame_type);
    let inline_size = emitter.push_instruction(Instruction::MulInt(stride, usz(INLINE_FRAMES as usize)), usz_type());
    let inline_end = offset_pointer(&mut emitter, inline_chunk, inline_size);
    let tag = Instruction::GetFieldPtr { struct_ptr: inline_chunk, struct_type: frame_type, index: 1 };
    let tag = emitter.push_instruction(tag, Type::POINTER);
    let tag_address = address(&mut emitter, tag);
    let frame_address = address(&mut emitter, inline_chunk);
    let tag_offset = emitter.push_instruction(Instruction::SubInt(tag_address, frame_address), usz_type());
    let null_chunk = emitter.push_instruction(null(), Type::POINTER);
    let fields =
        vec![inline_chunk, inline_end, inline_chunk, null_chunk, usz(INLINE_FRAMES as usize), stride, tag_offset];
    let state = emitter.push_instruction(Instruction::MakeTuple(fields), state_type());
    let state = emitter.push_instruction(Instruction::StackAlloc(state), Type::POINTER);

    // Each capability closure's environment is `(state, handler_env..)`
    let cases = splits.iter().zip(handler_env_types).zip(pre_ids.iter().zip(&pre_types));
    let capabilities = mapvec(cases, |((split, handler_env_type), (pre_id, pre_type))| {
        let fields = std::iter::once(state).chain(split.handler_env).collect();
        let field_types = std::iter::once(Type::POINTER).chain(handler_env_type).collect();
        let environment = emitter.push_instruction(Instruction::MakeTuple(fields), Type::tuple(field_types));
        let environment = emitter.push_instruction(Instruction::StackAlloc(environment), Type::POINTER);
        let pre = emitter.emit_definition_value(*pre_id, pre_type.clone(), split.handler_bindings.clone());
        emitter.push_instruction(Instruction::PackClosure { function: pre, environment }, pre_type.clone())
    });
    let capability = emitter.push_instruction(Instruction::MakeTuple(capabilities), cap_tuple_type);

    let body = emit_body_closure(&mut emitter, body, body_type, capability);
    let call = Instruction::CallClosure { closure: body, arguments: vec![Value::Unit] };
    let result = emitter.push_instruction(call, site.result_type.clone());

    // Run each frame's part after `resume`, reusing the Handle's id for the final result
    let unwind = emitter.emit_definition_value(unwind_id, unwind_type, identity_bindings(caller_generic_count));
    let unwind = Instruction::Call { function: unwind, arguments: vec![state, result] };
    emitter.reuse_instruction(site.id, unwind, site.result_type);
    definition.blocks[site.block].instructions.splice(site.index..=site.index, pending);

    // Handles within the new definitions can be optimized as well
    new_ids.extend(pre_ids);
    new_ids.extend(post_ids);
    new_ids
}

fn analyze_handle(mir: &Mir, definition_id: DefinitionId, site: &HandleSite) -> Option<Vec<CaseSplit>> {
    let definition = mir.definitions.get(&definition_id)?;
    opt_mapvec(&site.cases, |case| {
        let handler_type = definition.type_of_value(&case.handler, &mir.externals, &mir.definitions);
        let shape = case_shape_from_handler_type(&handler_type)?;
        let (handler_id, handler_env, handler_bindings) = resolve_body_function(case.handler, definition)?;
        let handler = mir.definitions.get(&handler_id)?;
        analyze_case(mir, handler, handler_id, handler_env, handler_bindings, shape)
    })
}

/// Check `handler` calls `resume` exactly once on every path through it and find what must be
/// saved in its frame
fn analyze_case(
    mir: &Mir, handler: &Definition, handler_id: DefinitionId, handler_env: Option<Value>,
    handler_bindings: Option<Arc<GenericBindings>>, shape: CaseShape,
) -> Option<CaseSplit> {
    let Type::Function(_) = &handler.typ else { return None };
    let resume = Value::Parameter(BlockId::ENTRY_BLOCK, shape.op_arg_types.len() as u32);
    let (resume_block, resume_index, resume_id, resume_argument) = find_only_resume_call(handler, resume)?;

    // Every block after `resume`. `resume` must not be in a loop.
    let after = handler.reachable_from(resume_block);
    if after.contains(&resume_block) {
        return None;
    }

    // In block order so the frame layout is deterministic
    let mut after_blocks = Vec::new();
    let mut slots = Vec::new();
    let mut field_types = Vec::new();
    for (block_id, block) in handler.blocks.iter() {
        if after.contains(&block_id) {
            after_blocks.push(block_id);
            continue;
        }

        // Stack slots allocated before `resume` move into the frame
        let end = if block_id == resume_block { resume_index } else { block.instructions.len() };
        for id in &block.instructions[..end] {
            let typ = match &handler.instructions[*id] {
                Instruction::StackAlloc(value) => handler.type_of_value(value, &mir.externals, &mir.definitions),
                Instruction::StackAllocUninit(typ) => typ.clone(),
                Instruction::StackAllocBytes(_) => return None,
                _ => continue,
            };
            slots.push(*id);
            field_types.push(typ);
        }

        // Blocks before `resume` may only reach blocks after it through `resume`, and can't return
        // without resuming. They may still end in `Unreachable`, e.g. after a panic.
        if block_id == resume_block {
            continue;
        }
        let Some(terminator) = &block.terminator else { continue };
        if matches!(terminator, TerminatorInstruction::Return(_) | TerminatorInstruction::Result(_)) {
            return None;
        }
        let mut enters_after = false;
        terminator.for_each_successor(|target| enters_after |= after.contains(&target));
        if enters_after {
            return None;
        }
    }

    let tail = &handler.blocks[resume_block].instructions[resume_index + 1..];
    let after_order: Vec<InstructionId> = tail
        .iter()
        .copied()
        .chain(after_blocks.iter().flat_map(|block| handler.blocks[*block].instructions.iter().copied()))
        .collect();
    let after_instructions: FxHashSet<InstructionId> = after_order.iter().copied().collect();

    // Values defined before `resume` and used after it
    let is_before = |value: &Value| match value {
        Value::InstructionResult(id) => *id != resume_id && !after_instructions.contains(id),
        Value::Parameter(block, _) => !after.contains(block),
        _ => false,
    };
    let mut live = Vec::new();
    let mut seen = FxHashSet::default();
    let mut add_live = |value: &Value| {
        if is_before(value) && seen.insert(*value) {
            live.push(*value);
        }
    };
    for id in &after_order {
        handler.instructions[*id].for_each_value(&mut add_live);
    }
    for block in std::iter::once(resume_block).chain(after_blocks.iter().copied()) {
        if let Some(terminator) = &handler.blocks[block].terminator {
            terminator.for_each_value(&mut add_live);
        }
    }
    for value in &live {
        field_types.push(handler.local_value_type(value)?.clone());
    }

    Some(CaseSplit {
        handler_id,
        handler_env,
        handler_bindings,
        shape,
        resume_block,
        resume_index,
        resume_id,
        resume_argument,
        slots,
        live,
        frame_type: Type::tuple(field_types),
    })
}

/// The block, index within it, id, and argument of the only use of `resume`: a call to it
fn find_only_resume_call(handler: &Definition, resume: Value) -> Option<(BlockId, usize, InstructionId, Value)> {
    let mut uses = 0;
    handler.for_each_used_value(|value| uses += usize::from(*value == resume));
    if uses != 1 {
        return None;
    }
    for (block_id, block) in handler.blocks.iter() {
        for (index, id) in block.instructions.iter().enumerate() {
            if let Instruction::CallClosure { closure, arguments } = &handler.instructions[*id]
                && *closure == resume
            {
                return (arguments.len() == 1).then(|| (block_id, index, *id, arguments[0]));
            }
        }
    }
    None
}

/// Build `fn op_args.. [Pointer] -> op_return_type`, the part of the handler before `resume`,
/// called directly by the performer. Its environment points to `(state, handler_env..)`.
fn build_pre_function(mir: &mut Mir, split: &CaseSplit, helpers: FrameStackHelpers, tag: u32) -> DefinitionId {
    let id = next_definition_id();
    let handler = &mir.definitions[&split.handler_id];
    let Type::Function(handler_type) = &handler.typ else { unreachable!("handler is a function") };
    let handler_env_type = handler_type.is_closure().then(|| handler_type.environment.clone());
    let mut definition = clone_handler_for_wrapper(handler, id, &split.shape, "resume_once_pre");

    // `resume`'s parameter becomes the environment pointer and the handler's environment parameter
    // is replaced by the environment's second field
    let entry = BlockId::ENTRY_BLOCK;
    let argument_count = split.shape.op_arg_types.len() as u32;
    let environment = Value::Parameter(entry, argument_count);
    let handler_env_parameter = Value::Parameter(entry, argument_count + 1);

    // The prologue is created now and placed at the start of the entry block last since the
    // entry block may be the one with `resume`, whose instruction indices are used below
    let environment_type = Type::tuple(std::iter::once(Type::POINTER).chain(handler_env_type.clone()).collect());
    let mut prologue = Vec::new();
    let mut e = Emitter::pending(&mut definition, &mut prologue);
    let environment = e.push_instruction(Instruction::Deref(environment), environment_type);
    let state = e.push_instruction(Instruction::IndexTuple { tuple: environment, index: 0 }, Type::POINTER);
    let handler_env =
        handler_env_type.map(|typ| e.push_instruction(Instruction::IndexTuple { tuple: environment, index: 1 }, typ));
    let push = Instruction::Call {
        function: Value::Definition(helpers.push),
        arguments: vec![state, Value::Integer(IntConstant::U32(tag))],
    };
    let frame = e.push_instruction(push, Type::POINTER);

    // Save each live value then return the value `resume` was called with
    let block = split.resume_block;
    definition.blocks[block].instructions.truncate(split.resume_index);
    for (i, value) in split.live.iter().enumerate() {
        let index = (split.slots.len() + i) as u32;
        let field = Instruction::GetFieldPtr { struct_ptr: frame, struct_type: split.frame_type.clone(), index };
        let field = definition.push_instruction_in(block, field, Type::POINTER);
        definition.push_instruction_in(block, Instruction::Store { pointer: field, value: *value }, Type::UNIT);
    }
    definition.blocks[block].terminator = Some(TerminatorInstruction::Return(split.resume_argument));

    // Allocate each stack slot in the frame instead
    let slot_indices: FxHashMap<InstructionId, u32> =
        split.slots.iter().enumerate().map(|(i, id)| (*id, i as u32)).collect();
    for block in definition.blocks.values_mut() {
        let mut i = 0;
        while i < block.instructions.len() {
            let id = block.instructions[i];
            i += 1;
            let Some(index) = slot_indices.get(&id) else { continue };
            let field =
                Instruction::GetFieldPtr { struct_ptr: frame, struct_type: split.frame_type.clone(), index: *index };
            if let Instruction::StackAlloc(value) = std::mem::replace(&mut definition.instructions[id], field) {
                let store = Instruction::Store { pointer: Value::InstructionResult(id), value };
                let store_id = definition.instructions.push(store);
                definition.instruction_result_types.push_existing(store_id, Type::UNIT);
                block.instructions.insert(i, store_id);
                i += 1;
            }
        }
    }

    if let Some(handler_env) = handler_env {
        definition.replace_value(handler_env_parameter, handler_env);
    }
    definition.blocks[entry].instructions.splice(0..0, prologue);
    definition.remove_unreachable_blocks();
    // Drop the `resume` call and the instructions after it, which would no longer type check
    definition.compact_instructions();
    insert_definition(mir, definition)
}

/// Build `fn (frame: Pointer, result: r) -> r`, the part of the handler after `resume`, where
/// `result` is the result of the rest of the handled computation.
fn build_post_function(mir: &mut Mir, split: &CaseSplit) -> DefinitionId {
    let mut definition = mir.definitions[&split.handler_id].clone_with_id(next_definition_id());
    let Type::Function(handler_type) = &definition.typ else { unreachable!("handler is a function") };
    let return_type = handler_type.return_type.clone();
    definition.typ = Type::function(vec![Type::POINTER, return_type.clone()], return_type.clone());
    definition.name = Arc::new(format!("{}_resume_once_post", definition.name));

    let entry = BlockId::ENTRY_BLOCK;
    let frame = Value::Parameter(entry, 0);
    let result = Value::Parameter(entry, 1);

    // Load each live value from the frame. Each field pointer's frame operand is filled in after
    // substituting the live values, since the frame parameter may share a live value's id.
    let Type::Tuple(field_types) = &split.frame_type else { unreachable!("frames are tuples") };
    let mut field_pointers = Vec::with_capacity(split.live.len());
    let mut loads = Vec::with_capacity(split.live.len() * 2);
    let mut substitutions = FxHashMap::default();
    let live_types = &field_types[split.slots.len()..];
    for (i, (value, typ)) in split.live.iter().zip(live_types).enumerate() {
        let index = (split.slots.len() + i) as u32;
        let field = Instruction::GetFieldPtr { struct_ptr: Value::Unit, struct_type: split.frame_type.clone(), index };
        let field = definition.add_instruction(field, Type::POINTER);
        let load = definition.add_instruction(Instruction::Deref(Value::InstructionResult(field)), typ.clone());
        field_pointers.push(field);
        loads.extend([field, load]);
        substitutions.insert(*value, Value::InstructionResult(load));
    }
    substitutions.insert(Value::InstructionResult(split.resume_id), result);
    definition.for_each_used_value_mut(|value| {
        if let Some(new) = substitutions.get(value) {
            *value = *new;
        }
    });
    for id in field_pointers {
        if let Instruction::GetFieldPtr { struct_ptr, .. } = &mut definition.instructions[id] {
            *struct_ptr = frame;
        }
    }

    // The entry block becomes the rest of the block which called `resume`
    let block = split.resume_block;
    let rest: Vec<_> = definition.blocks[block].instructions.drain(split.resume_index + 1..).collect();
    let terminator = definition.blocks[block].terminator.replace(TerminatorInstruction::Unreachable);
    let entry_block = &mut definition.blocks[entry];
    entry_block.parameter_types = vec![Type::POINTER, return_type];
    entry_block.instructions = loads;
    entry_block.instructions.extend(rest);
    entry_block.terminator = terminator;

    definition.remove_unreachable_blocks();
    // Drop the `resume` call and the instructions before it, which would no longer type check
    definition.compact_instructions();
    insert_definition(mir, definition)
}

/// A case's part after `resume`, as seen from the Handle's definition
struct PostFunction {
    id: DefinitionId,
    typ: Type,
    bindings: Option<Arc<GenericBindings>>,
}

/// Build `fn (state: Pointer, result: r) -> r` for a Handle, which pops each frame, passing the
/// result through each frame's part after `resume`.
fn build_unwind_function(
    mir: &mut Mir, result_type: &Type, frame_type: &Type, posts: Vec<PostFunction>, helpers: FrameStackHelpers,
    generic_count: u32,
) -> DefinitionId {
    let parameters = vec![Type::POINTER, result_type.clone()];
    let mut definition = new_function("handle_unwind", generic_count, parameters, result_type.clone());

    let entry = BlockId::ENTRY_BLOCK;
    let state = Value::Parameter(entry, 0);

    let loop_header = definition.blocks.push(Block::new(vec![result_type.clone()]));
    let done = definition.blocks.push(Block::new(Vec::new()));
    let dispatch = definition.blocks.push(Block::new(Vec::new()));
    let next = definition.blocks.push(Block::new(vec![result_type.clone()]));
    let result = Value::Parameter(loop_header, 0);

    definition.blocks[entry].terminator =
        Some(TerminatorInstruction::Jmp((loop_header, Some(Value::Parameter(entry, 1)))));

    let mut e = Emitter::in_block(&mut definition, loop_header);
    let pop = Instruction::Call { function: Value::Definition(helpers.pop), arguments: vec![state] };
    let frame = e.push_instruction(pop, Type::POINTER);
    let is_empty = is_null(&mut e, frame);
    // `end` is `else_` since the branches don't converge
    definition.blocks[loop_header].terminator = Some(TerminatorInstruction::If {
        condition: is_empty,
        then: (done, None),
        else_: (dispatch, None),
        end: dispatch,
    });
    definition.blocks[done].terminator = Some(TerminatorInstruction::Return(result));
    definition.blocks[next].terminator =
        Some(TerminatorInstruction::Jmp((loop_header, Some(Value::Parameter(next, 0)))));

    let call_post = |definition: &mut Definition, block: BlockId, post: PostFunction| {
        let mut e = Emitter::in_block(definition, block);
        let function = e.emit_definition_value(post.id, post.typ, post.bindings);
        let call = Instruction::Call { function, arguments: vec![frame, result] };
        let new_result = e.push_instruction(call, result_type.clone());
        definition.blocks[block].terminator = Some(TerminatorInstruction::Jmp((next, Some(new_result))));
    };

    if posts.len() == 1 {
        let post = posts.into_iter().next().unwrap();
        call_post(&mut definition, dispatch, post);
    } else {
        let mut e = Emitter::in_block(&mut definition, dispatch);
        let tag = Instruction::GetFieldPtr { struct_ptr: frame, struct_type: frame_type.clone(), index: 1 };
        let tag = e.push_instruction(tag, Type::POINTER);
        let tag = e.push_instruction(Instruction::Deref(tag), Type::int(IntegerKind::U32));
        let cases = mapvec(posts.into_iter().enumerate(), |(case, post)| {
            let block = definition.blocks.push(Block::new(Vec::new()));
            call_post(&mut definition, block, post);
            (case as u64, (block, None))
        });
        let unreachable = definition.blocks.push(Block::new(Vec::new()));
        definition.blocks[unreachable].terminator = Some(TerminatorInstruction::Unreachable);
        definition.blocks[dispatch].terminator =
            Some(TerminatorInstruction::Switch { int_value: tag, cases, else_: (unreachable, None), end: next });
    }

    insert_definition(mir, definition)
}

/// The non-generic functions every Handle optimized by this pass shares to push and pop frames
#[derive(Clone, Copy)]
struct FrameStackHelpers {
    /// `fn (state: Pointer, tag: U32) -> Pointer`: push a frame with the given tag, returning it
    push: DefinitionId,
    /// `fn (state: Pointer) -> Pointer`: pop a frame, returning it, or null if there are none
    pop: DefinitionId,
}

impl FrameStackHelpers {
    fn new(mir: &mut Mir) -> Self {
        let push = build_push_function(mir);
        let pop = build_pop_function(mir);
        FrameStackHelpers { push, pop }
    }
}

/// Emits instructions reading and writing a frame stack's [state_type]
struct StateAccess {
    state: Value,
    fields: Value,
}

impl StateAccess {
    fn load(e: &mut Emitter, state: Value) -> Self {
        let fields = e.push_instruction(Instruction::Deref(state), state_type());
        StateAccess { state, fields }
    }

    fn get(&self, e: &mut Emitter, field: u32) -> Value {
        e.push_instruction(Instruction::IndexTuple { tuple: self.fields, index: field }, state_field_type(field))
    }

    fn set(&self, e: &mut Emitter, field: u32, value: Value) {
        let pointer = Instruction::GetFieldPtr { struct_ptr: self.state, struct_type: state_type(), index: field };
        let pointer = e.push_instruction(pointer, Type::POINTER);
        e.push_instruction(Instruction::Store { pointer, value }, Type::UNIT);
    }
}

fn build_push_function(mir: &mut Mir) -> DefinitionId {
    let parameters = vec![Type::POINTER, Type::int(IntegerKind::U32)];
    let mut definition = new_function("handle_frame_push", 0, parameters, Type::POINTER);

    let entry = BlockId::ENTRY_BLOCK;
    let tag = Value::Parameter(entry, 1);
    let grow = definition.blocks.push(Block::new(Vec::new()));
    let push = definition.blocks.push(Block::new(vec![Type::POINTER]));

    let mut e = Emitter::in_block(&mut definition, entry);
    let state = StateAccess::load(&mut e, Value::Parameter(entry, 0));
    let top = state.get(&mut e, state_field::TOP);
    let end = state.get(&mut e, state_field::END);
    let stride = state.get(&mut e, state_field::STRIDE);
    let tag_offset = state.get(&mut e, state_field::TAG_OFFSET);
    let is_full = pointers_equal(&mut e, top, end);
    definition.blocks[entry].terminator =
        Some(TerminatorInstruction::If { condition: is_full, then: (grow, None), else_: (push, Some(top)), end: push });

    // The chunk is full: allocate a new one with twice its frames, starting with the old state.
    // Its remaining space is only the tail of the last frame, so the old chunk's frames needn't
    // be revisited until every frame of the new one is popped.
    let mut e = Emitter::in_block(&mut definition, grow);
    let capacity = state.get(&mut e, state_field::CAPACITY);
    let new_capacity = e.push_instruction(Instruction::MulInt(capacity, usz(2)), usz_type());
    let frames_size = e.push_instruction(Instruction::MulInt(new_capacity, stride), usz_type());
    let size = e.push_instruction(Instruction::AddInt(frames_size, usz(CHUNK_HEADER_SIZE)), usz_type());
    let malloc_type = Type::function(vec![usz_type()], Type::POINTER);
    let malloc = e.push_instruction(Instruction::Extern("malloc".to_string()), malloc_type);
    let chunk = e.push_instruction(Instruction::Call { function: malloc, arguments: vec![size] }, Type::POINTER);
    let base = state.get(&mut e, state_field::BASE);
    let old_chunk = state.get(&mut e, state_field::CHUNK);
    let header = e.push_instruction(Instruction::MakeTuple(vec![base, end, old_chunk, capacity]), chunk_header_type());
    e.push_instruction(Instruction::Store { pointer: chunk, value: header }, Type::UNIT);
    let new_base = offset_pointer(&mut e, chunk, usz(CHUNK_HEADER_SIZE));
    let new_end = offset_pointer(&mut e, new_base, frames_size);
    state.set(&mut e, state_field::END, new_end);
    state.set(&mut e, state_field::BASE, new_base);
    state.set(&mut e, state_field::CHUNK, chunk);
    state.set(&mut e, state_field::CAPACITY, new_capacity);
    definition.blocks[grow].terminator = Some(TerminatorInstruction::Jmp((push, Some(new_base))));

    // push(frame): bump the top past `frame` and tag it
    let frame = Value::Parameter(push, 0);
    let mut e = Emitter::in_block(&mut definition, push);
    let new_top = offset_pointer(&mut e, frame, stride);
    state.set(&mut e, state_field::TOP, new_top);
    let tag_pointer = offset_pointer(&mut e, frame, tag_offset);
    e.push_instruction(Instruction::Store { pointer: tag_pointer, value: tag }, Type::UNIT);
    definition.blocks[push].terminator = Some(TerminatorInstruction::Return(frame));

    insert_definition(mir, definition)
}

fn build_pop_function(mir: &mut Mir) -> DefinitionId {
    let mut definition = new_function("handle_frame_pop", 0, vec![Type::POINTER], Type::POINTER);

    let entry = BlockId::ENTRY_BLOCK;
    let chunk_empty = definition.blocks.push(Block::new(Vec::new()));
    let empty = definition.blocks.push(Block::new(Vec::new()));
    let free_chunk = definition.blocks.push(Block::new(Vec::new()));
    let pop = definition.blocks.push(Block::new(vec![Type::POINTER]));

    let mut e = Emitter::in_block(&mut definition, entry);
    let state = StateAccess::load(&mut e, Value::Parameter(entry, 0));
    let top = state.get(&mut e, state_field::TOP);
    let base = state.get(&mut e, state_field::BASE);
    let stride = state.get(&mut e, state_field::STRIDE);
    let at_base = pointers_equal(&mut e, top, base);
    definition.blocks[entry].terminator = Some(TerminatorInstruction::If {
        condition: at_base,
        then: (chunk_empty, None),
        else_: (pop, Some(top)),
        end: pop,
    });

    // The current chunk is empty: if it's the stack chunk there are no frames left
    let mut e = Emitter::in_block(&mut definition, chunk_empty);
    let chunk = state.get(&mut e, state_field::CHUNK);
    let is_inline = is_null(&mut e, chunk);
    definition.blocks[chunk_empty].terminator = Some(TerminatorInstruction::If {
        condition: is_inline,
        then: (empty, None),
        else_: (free_chunk, None),
        end: free_chunk,
    });
    let null_frame = definition.push_instruction_in(empty, null(), Type::POINTER);
    definition.blocks[empty].terminator = Some(TerminatorInstruction::Return(null_frame));

    // Otherwise free it and return to the previous chunk, which was full when this one was pushed
    let mut e = Emitter::in_block(&mut definition, free_chunk);
    let header = e.push_instruction(Instruction::Deref(chunk), chunk_header_type());
    let [old_base, old_end, old_chunk, old_capacity] = [0, 1, 2, 3].map(|index| {
        let typ = if index == 3 { usz_type() } else { Type::POINTER };
        e.push_instruction(Instruction::IndexTuple { tuple: header, index }, typ)
    });
    let free_type = Type::function(vec![Type::POINTER], Type::UNIT);
    let free = e.push_instruction(Instruction::Extern("free".to_string()), free_type);
    e.push_instruction(Instruction::Call { function: free, arguments: vec![chunk] }, Type::UNIT);
    state.set(&mut e, state_field::END, old_end);
    state.set(&mut e, state_field::BASE, old_base);
    state.set(&mut e, state_field::CHUNK, old_chunk);
    state.set(&mut e, state_field::CAPACITY, old_capacity);
    definition.blocks[free_chunk].terminator = Some(TerminatorInstruction::Jmp((pop, Some(old_end))));

    // pop(top): the frame below `top` becomes the new top
    let top = Value::Parameter(pop, 0);
    let mut e = Emitter::in_block(&mut definition, pop);
    let top_address = address(&mut e, top);
    let frame_address = e.push_instruction(Instruction::SubInt(top_address, stride), usz_type());
    let frame = e.push_instruction(Instruction::Transmute(frame_address), Type::POINTER);
    state.set(&mut e, state_field::TOP, frame);
    definition.blocks[pop].terminator = Some(TerminatorInstruction::Return(frame));

    insert_definition(mir, definition)
}
