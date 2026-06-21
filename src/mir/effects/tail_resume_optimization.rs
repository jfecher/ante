//! Inline [super::Instruction::Handle]s whose handler cases all use `resume` in tail position
//! into plain MIR control flow using closures. The transformation resembles function inlining:
//! the body closure is pasted in, each [super::Instruction::Perform] is replaced with the inlined
//! handler, and `resume v` in the handler becomes a jump to the continuation block with `v` as its
//! argument.
//!
//! If any case uses `resume` outside of a tail-position, the Handle is left for
//! [crate::mir::effects::effect_lowering] to lower into coroutine primitives.

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::mir::{
    BlockId, Definition, DefinitionId, FunctionType, GenericBindings, HandlerCase, Instruction, InstructionId, Mir,
    TerminatorInstruction, Type, Value, next_definition_id,
};

use super::effect_lowering::{
    CaseShape, case_shape_from_handler_type, resolve_body_function, run_handle_optimization_worklist,
};

impl Mir {
    pub(crate) fn optimize_tail_resume(mut self) -> Self {
        run_handle_optimization_worklist(&mut self, optimize_in_definition);
        self
    }
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

/// Optimize every tail-resumptive Handle in `definition_id`, returning the ids of any new
/// definitions created (body clones) so the caller can enqueue them for processing.
fn optimize_in_definition(
    mir: &mut Mir, definition_id: DefinitionId, dead_bodies: &mut FxHashSet<DefinitionId>,
) -> Vec<DefinitionId> {
    collect_handle_sites(mir, definition_id)
        .into_iter()
        .flat_map(|site| try_optimize_handle(mir, definition_id, site, dead_bodies))
        .collect()
}

fn collect_handle_sites(mir: &Mir, definition_id: DefinitionId) -> Vec<HandleSite> {
    let Some(def) = mir.definitions.get(&definition_id) else { return Vec::new() };
    let mut sites = Vec::new();
    for (block_id, block) in def.blocks.iter() {
        let start = sites.len();
        for (index, instruction_id) in block.instructions.iter().enumerate() {
            if let Instruction::Handle { body, cases } = &def.instructions[*instruction_id] {
                sites.push(HandleSite {
                    block: block_id,
                    index,
                    id: *instruction_id,
                    body: *body,
                    cases: cases.clone(),
                    result_type: def.instruction_result_types[*instruction_id].clone(),
                });
            }
        }
        sites[start..].reverse();
    }
    sites
}

/// Per-case data needed to materialize a direct wrapper.
struct CaseDecision {
    handler_def_id: DefinitionId,
    handler_env: Option<Value>,
    /// Generic bindings from the original `Instantiate` of `case.handler`,
    handler_bindings: Option<Arc<GenericBindings>>,
    shape: CaseShape,
}

fn analyze_handle(mir: &Mir, definition_id: DefinitionId, site: &HandleSite) -> Option<Vec<CaseDecision>> {
    let definition = mir.definitions.get(&definition_id)?;

    let mut decisions = Vec::with_capacity(site.cases.len());
    for case in &site.cases {
        let handler_type = definition.type_of_value(&case.handler, &mir.externals, &mir.definitions);
        let shape = case_shape_from_handler_type(&handler_type)?;
        let (handler_def_id, handler_env, handler_bindings) = resolve_handler(case.handler, definition)?;
        let handler_def = mir.definitions.get(&handler_def_id)?;
        if !case_is_tail_resumptive(handler_def, &shape) {
            return None;
        }
        decisions.push(CaseDecision { handler_def_id, handler_env, handler_bindings, shape });
    }
    Some(decisions)
}

/// Resolve the handler value to its underlying [DefinitionId], optional closure env,
/// and optional generic bindings recovered from any `Instantiate` along the chain. Mirrors
/// [resolve_body_function] but tolerates handler shapes specifically.
pub(super) fn resolve_handler(
    handler: Value, definition: &Definition,
) -> Option<(DefinitionId, Option<Value>, Option<Arc<GenericBindings>>)> {
    fn resolve_id(value: Value, definition: &Definition) -> Option<(DefinitionId, Option<Arc<GenericBindings>>)> {
        match value {
            Value::Definition(id) => Some((id, None)),
            Value::InstructionResult(iid) => match &definition.instructions[iid] {
                Instruction::Id(inner) => resolve_id(*inner, definition),
                Instruction::Instantiate(id, bindings) => Some((*id, Some(bindings.clone()))),
                _ => None,
            },
            _ => None,
        }
    }

    match handler {
        Value::InstructionResult(iid) => match &definition.instructions[iid] {
            Instruction::PackClosure { function, environment } => {
                let (id, bindings) = resolve_id(*function, definition)?;
                Some((id, Some(*environment), bindings))
            },
            _ => {
                let (id, bindings) = resolve_id(handler, definition)?;
                Some((id, None, bindings))
            },
        },
        _ => {
            let (id, bindings) = resolve_id(handler, definition)?;
            Some((id, None, bindings))
        },
    }
}

/// True if every Return in `handler_def` is a tail-call to `resume_param`, the resume parameter
/// is used nowhere else, and at least one tail-resume call exists.
fn case_is_tail_resumptive(handler_def: &Definition, shape: &CaseShape) -> bool {
    let resume_param = Value::Parameter(BlockId::ENTRY_BLOCK, shape.op_arg_types.len() as u32);

    // 1. Find all tail CallClosure(resume_param, [arg]) ids: these are the CallClosure
    //    instructions that are the last instruction in their block, where the block's terminator
    //    is `Return(InstructionResult(call_id))`.
    let mut tail_call_ids: FxHashSet<InstructionId> = FxHashSet::default();
    let mut tail_call_blocks: FxHashSet<BlockId> = FxHashSet::default();
    for (block_id, block) in handler_def.blocks.iter() {
        let Some(TerminatorInstruction::Return(Value::InstructionResult(call_id))) = &block.terminator else {
            continue;
        };
        if block.instructions.last() != Some(call_id) {
            continue;
        }
        let Instruction::CallClosure { closure, arguments } = &handler_def.instructions[*call_id] else {
            continue;
        };
        if *closure == resume_param && arguments.len() == 1 {
            tail_call_ids.insert(*call_id);
            tail_call_blocks.insert(block_id);
        }
    }

    if tail_call_ids.is_empty() {
        return false;
    }

    // 2. Every Return must be one of our recognized tail-resume returns.
    for (block_id, block) in handler_def.blocks.iter() {
        if let Some(TerminatorInstruction::Return(_)) = &block.terminator
            && !tail_call_blocks.contains(&block_id)
        {
            return false;
        }
    }

    // 3. resume_param appears nowhere outside a recognized tail-CallClosure's `closure` slot.
    for (id, instruction) in handler_def.instructions.iter() {
        if tail_call_ids.contains(&id) {
            // The tail CallClosure is allowed to use resume_param as its closure target,
            // but its argument must not also reference resume_param (e.g. `resume(resume)`).
            if let Instruction::CallClosure { arguments, .. } = instruction
                && arguments.contains(&resume_param)
            {
                return false;
            }
        } else {
            let mut found = false;
            instruction.for_each_value(|v| {
                if *v == resume_param {
                    found = true;
                }
            });
            if found {
                return false;
            }
        }
    }

    // 4. resume_param doesn't appear in non-tail terminators (Jmp/If/Switch/Result args).
    for (block_id, block) in handler_def.blocks.iter() {
        if tail_call_blocks.contains(&block_id) {
            continue;
        }
        if let Some(t) = &block.terminator {
            let mut found = false;
            t.for_each_value(|v| {
                if *v == resume_param {
                    found = true;
                }
            });
            if found {
                return false;
            }
        }
    }

    true
}

fn try_optimize_handle(
    mir: &mut Mir, definition_id: DefinitionId, site: HandleSite, dead_bodies: &mut FxHashSet<DefinitionId>,
) -> Vec<DefinitionId> {
    let Some(decisions) = analyze_handle(mir, definition_id, &site) else { return Vec::new() };

    // Preserve the op→index mapping so [crate::mir::effects::effect_lowering] can still lower
    // `Perform`s targeting these ops after we remove the Handle. The position is the case's
    // index in the Handle's cases vector, matching what [build_op_index] would have recorded.
    for (i, case) in site.cases.iter().enumerate() {
        let i = u32::try_from(i).expect("effect with more than u32::MAX ops");
        mir.preserved_op_indices.insert(case.effect_op, i);
    }

    // Materialize a direct wrapper definition per case.
    let wrapper_ids: Vec<DefinitionId> =
        decisions.iter().map(|d| materialize_direct_wrapper(mir, d.handler_def_id, &d.shape)).collect();

    let wrapper_closure_types: Vec<Type> = wrapper_ids.iter().map(|id| mir.definitions[id].typ.clone()).collect();
    let cap_tuple_type = Type::Tuple(Arc::new(wrapper_closure_types.clone()));

    // Resolve the body once more (we re-fetched definition because `mir` may have changed).
    let (orig_body_fn_id, body_env_value, body_bindings) = {
        let definition = &mir.definitions[&definition_id];
        resolve_body_function(site.body, definition)
    };

    // Clone body_fn into a fresh definition specialized for tail-resume. We MUST clone rather
    // than mutate in-place because the outer definition still contains the original
    // PackClosure / Id instructions for body_fn (now dead but still type-checked by validation),
    // and changing body_fn's type would leave those instructions with stale `result_types`.
    let (body_fn_id, original_env_layout, created_ids) =
        clone_body_with_extended_env(mir, orig_body_fn_id, &cap_tuple_type);

    // The original body fn is now dead (only the outer definition's dead PackClosure references
    // it). Don't process its Handles again; the clone carries the live copies.
    dead_bodies.insert(orig_body_fn_id);

    // The original body_fn is now dead code: the outer definition's now-dead PackClosure still
    // references it, but its result is unused. It still gets visited by validation and codegen,
    // so we have to make any `Capability` it contains lowering-safe. The simplest neutralization
    // is `Transmute(Value::Unit)`: it type-checks (the Transmute validation arm has no
    // constraints) and codegens to an undef of the expected cap tuple type. We do this *after*
    // cloning so the clone retains its Capability instructions (which `rewrite_handler_caps_in_body`
    // is about to expand into the env-IndexTuple form).
    neutralize_handler_caps_in_dead_body(mir, orig_body_fn_id);

    // Replace each Capability in the cloned body with an IndexTuple that fetches cap from the
    // (extended) env parameter.
    rewrite_handler_caps_in_body(mir, body_fn_id, &original_env_layout, &cap_tuple_type);

    // Rewrite the Handle site: build wrappers' closures, the cap tuple, the extended body env,
    // PackClosure(body_fn, extended_env), and reuse the original Handle's id for the final
    // CallClosure so downstream consumers of the Handle's result Value continue to work.
    splice_in_handle_replacement(
        mir,
        definition_id,
        site,
        &wrapper_ids,
        &decisions,
        body_fn_id,
        body_env_value,
        body_bindings,
        &cap_tuple_type,
        &original_env_layout,
    );

    created_ids
}

/// Records what the body's env looked like *before* we appended cap. Used both to compute the
/// Capability's IndexTuple index (= original count) and to splice the original env values into
/// the new env at the Handle site.
pub(super) struct OriginalEnvLayout {
    /// True iff body_fn was originally a closure (had an env parameter). When false, the body
    /// had no captures and we are upgrading it into a closure that carries only `cap`.
    pub(super) was_closure: bool,
    /// Element types of the original env tuple (empty when `was_closure` is false).
    pub(super) original_field_types: Vec<Type>,
}

/// Clone body_fn into a fresh [DefinitionId] whose closure environment is
/// `Tuple([orig.., cap_type])`. If body_fn had no env originally, the new clone has one with a
/// single field (cap). Returns the new id and a record describing the original env layout so
/// callers can reconstruct the new env value at the Handle site and know which IndexTuple
/// index to use for `cap` inside the body.
///
/// We clone instead of mutating so the outer definition's now-dead instructions that still
/// reference the original body_fn (e.g. the original PackClosure produced by the MIR builder
/// before the Handle was rewritten) keep type-checking against the original signature.
pub(super) fn clone_body_with_extended_env(
    mir: &mut Mir, orig_body_fn_id: DefinitionId, cap_type: &Type,
) -> (DefinitionId, OriginalEnvLayout, Vec<DefinitionId>) {
    // Deep-clone the body together with the nested sub-definitions it privately owns. This keeps
    // the live clone's nested-handle subtree separate from the now-dead original, so neutralizing
    // the original's capabilities can never corrupt the clone.
    let (new_id, id_map) = deep_clone_body_subtree(mir, orig_body_fn_id);
    let created_ids: Vec<DefinitionId> = id_map.values().copied().collect();

    let original_typ = mir.definitions[&orig_body_fn_id].typ.clone();
    let original_name = mir.definitions[&orig_body_fn_id].name.clone();
    let Type::Function(body_ft) = &original_typ else {
        panic!("body_fn type is not a function");
    };
    let was_closure = body_ft.is_closure();
    let (original_field_types, new_env_type) = if was_closure {
        let Type::Tuple(fields) = &body_ft.environment else {
            // Defensive: the MIR builder always emits a tuple env. If something else ended up
            // here we can't safely extend; return the deep clone unmutated.
            return (new_id, OriginalEnvLayout { was_closure, original_field_types: Vec::new() }, created_ids);
        };
        let mut new_fields: Vec<Type> = (**fields).clone();
        let original_fields = new_fields.clone();
        new_fields.push(cap_type.clone());
        (original_fields, Type::Tuple(Arc::new(new_fields)))
    } else {
        (Vec::new(), Type::Tuple(Arc::new(vec![cap_type.clone()])))
    };

    let new_def = mir.definitions.get_mut(&new_id).expect("deep clone root missing");
    new_def.typ = Type::Function(Arc::new(FunctionType {
        parameters: body_ft.parameters.clone(),
        environment: new_env_type.clone(),
        return_type: body_ft.return_type.clone(),
    }));
    new_def.name = Arc::new(format!("{original_name}_tail_resume_body"));

    let entry = BlockId::ENTRY_BLOCK;
    if was_closure {
        // Replace the existing env parameter type at the last slot. The body lambda has 0
        // explicit params, so env is at the last slot of the entry block's parameter list.
        let n = new_def.blocks[entry].parameter_types.len();
        new_def.blocks[entry].parameter_types[n - 1] = new_env_type;
    } else {
        new_def.blocks[entry].parameter_types.push(new_env_type);
    }

    (new_id, OriginalEnvLayout { was_closure, original_field_types }, created_ids)
}

/// The set of definitions privately owned by `root`: `root` itself plus every definition reachable
/// from it whose every referrer is also owned. Definitions referenced from outside this set (named
/// top-level functions, shared helpers) and externals stay shared and are not cloned.
fn owned_subtree(mir: &Mir, root: DefinitionId) -> FxHashSet<DefinitionId> {
    let mut referrers: FxHashMap<DefinitionId, FxHashSet<DefinitionId>> = FxHashMap::default();
    for (id, def) in mir.definitions.iter() {
        def.for_each_referenced_definition(|child| {
            referrers.entry(child).or_default().insert(*id);
        });
    }

    let mut owned = FxHashSet::default();
    owned.insert(root);
    loop {
        let mut candidates: Vec<DefinitionId> = Vec::new();
        for def in owned.iter().filter_map(|id| mir.definitions.get(id)) {
            def.for_each_referenced_definition(|c| {
                if !owned.contains(&c) && mir.definitions.contains_key(&c) {
                    candidates.push(c);
                }
            });
        }
        let mut changed = false;
        for c in candidates {
            let all_referrers_owned = referrers.get(&c).is_none_or(|rs| rs.iter().all(|r| owned.contains(r)));
            if all_referrers_owned && owned.insert(c) {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    owned
}

/// Deep-clone `root` and its [`owned_subtree`] into fresh definitions, remapping all internal
/// definition-id references to the clones. Returns the cloned root id and the full old→new id map.
fn deep_clone_body_subtree(mir: &mut Mir, root: DefinitionId) -> (DefinitionId, FxHashMap<DefinitionId, DefinitionId>) {
    let owned = owned_subtree(mir, root);
    let id_map: FxHashMap<DefinitionId, DefinitionId> = owned.iter().map(|&old| (old, next_definition_id())).collect();

    for (&old, &new) in id_map.iter() {
        let mut clone = mir.definitions[&old].clone_with_id(new);
        remap_definition_ids(&mut clone, &id_map);
        mir.definitions.insert(new, clone);
    }
    (id_map[&root], id_map)
}

/// Rewrite every `Value::Definition` and `Instruction::Instantiate` target in `def` through `id_map`.
fn remap_definition_ids(def: &mut Definition, id_map: &FxHashMap<DefinitionId, DefinitionId>) {
    for instr in def.instructions.values_mut() {
        if let Instruction::Instantiate(id, _) = instr
            && let Some(&new) = id_map.get(id)
        {
            *id = new;
        }
    }
    for (&old, &new) in id_map.iter() {
        if old != new {
            substitute_value(def, Value::Definition(old), Value::Definition(new));
        }
    }
}

/// Replace each [Instruction::Capability] in body_fn with `IndexTuple(env_param, cap_index)`.
pub(super) fn rewrite_handler_caps_in_body(
    mir: &mut Mir, body_fn_id: DefinitionId, layout: &OriginalEnvLayout, cap_tuple_type: &Type,
) {
    let body_fn = mir.definitions.get_mut(&body_fn_id).expect("body_fn missing");
    let entry = BlockId::ENTRY_BLOCK;
    // After extend_body_env, env is the last (and only) parameter of the entry block, since the
    // body lambda always has zero lambda params.
    let env_param_index = (body_fn.blocks[entry].parameter_types.len() - 1) as u32;
    let env_param = Value::Parameter(entry, env_param_index);
    let cap_index = layout.original_field_types.len() as u32;

    // The abort-resume optimization can insert transmute instructions from a closure's environment
    // which may be Unit but actually contains the capability.
    let cap_instr_ids: Vec<InstructionId> = body_fn
        .instructions
        .iter()
        .filter_map(|(id, instr)| {
            let is_cap = match instr {
                Instruction::Capability => true,
                Instruction::Transmute(Value::Unit) => &body_fn.instruction_result_types[id] == cap_tuple_type,
                _ => false,
            };
            is_cap.then_some(id)
        })
        .collect();

    for id in cap_instr_ids {
        body_fn.instructions[id] = Instruction::IndexTuple { tuple: env_param, index: cap_index };
        body_fn.instruction_result_types[id] = cap_tuple_type.clone();
    }
}

/// Build a fresh wrapper [DefinitionId] for a tail-resumptive case. The wrapper's signature
/// matches what `Perform` expects from the cap tuple (`fn op_args.. [Pointer] -> op_return_type`),
/// so the call site stays untouched. The handler's actual closure env (if any) is recovered by
/// dereferencing the Pointer at the wrapper's prologue, and references to it inside the inlined
/// handler body are remapped to the loaded value.
fn materialize_direct_wrapper(mir: &mut Mir, handler_def_id: DefinitionId, shape: &CaseShape) -> DefinitionId {
    let handler_def = mir.definitions[&handler_def_id].clone();
    let new_id = next_definition_id();
    let mut new_def = handler_def.clone_with_id(new_id);

    let Type::Function(handler_ft) = &handler_def.typ else {
        panic!("handler def is not a function");
    };
    let env_present = handler_ft.is_closure();
    let handler_env_type = handler_ft.environment.clone();

    new_def.typ = Type::Function(Arc::new(FunctionType {
        parameters: shape.op_arg_types.clone(),
        environment: Type::POINTER,
        return_type: shape.op_return_type.clone(),
    }));
    new_def.name = Arc::new(format!("{}_tail_resume_wrapper", handler_def.name));

    let entry = BlockId::ENTRY_BLOCK;
    let n = shape.op_arg_types.len();

    // Replace the entry block's parameter list with `op_args.., Pointer`. The Pointer occupies
    // the slot that resume used to occupy (Value::Parameter(entry, n)). If the handler also had
    // an env tuple parameter (Value::Parameter(entry, n+1)), its uses in the cloned body are
    // remapped to the dereferenced env value below.
    let mut new_param_types: Vec<Type> = shape.op_arg_types.clone();
    new_param_types.push(Type::POINTER);
    new_def.blocks[entry].parameter_types = new_param_types;

    let env_pointer = Value::Parameter(entry, n as u32);
    let old_env_param = Value::Parameter(entry, (n + 1) as u32);

    let env_substitute: Option<Value> = if env_present {
        let deref_id = new_def.instructions.push(Instruction::Deref(env_pointer));
        new_def.instruction_result_types.push_existing(deref_id, handler_env_type);
        new_def.blocks[entry].instructions.insert(0, deref_id);
        Some(Value::InstructionResult(deref_id))
    } else {
        None
    };

    // Rewrite each tail `CallClosure(resume_param, [arg])` into `Id(arg)`, in-place. The
    // instruction id stays valid (so the matching `Return(InstructionResult(id))` terminator
    // still resolves to `arg` via the Id passthrough), block instruction lists don't change,
    // and validation still type-checks every entry in `definition.instructions`. Removing the
    // instruction from the VecMap would have left validation iterating an entry whose closure
    // operand (the old resume parameter slot, now reused for the env Pointer) is not a function.
    let resume_param = Value::Parameter(entry, n as u32);
    let mut rewrites: Vec<(InstructionId, Value)> = Vec::new();
    for (_block_id, block) in new_def.blocks.iter() {
        let Some(TerminatorInstruction::Return(Value::InstructionResult(call_id))) = &block.terminator else {
            continue;
        };
        if block.instructions.last() != Some(call_id) {
            continue;
        }
        let Instruction::CallClosure { closure, arguments } = &new_def.instructions[*call_id] else { continue };
        if *closure != resume_param || arguments.len() != 1 {
            continue;
        }
        rewrites.push((*call_id, arguments[0]));
    }
    for (id, arg) in rewrites {
        new_def.instructions[id] = Instruction::Id(arg);
        // The original CallClosure's result type was the handler's `r2` (= handler return
        // type); now that we forward `arg` (= what resume was called with), the result type
        // becomes `r1` (= op_return_type). The only consumer of this instruction was the
        // handler's tail Return and its block's terminator still names this instruction id,
        // so the wrapper's overall return type also becomes op_return_type, matching the
        // wrapper's declared signature.
        new_def.instruction_result_types[id] = shape.op_return_type.clone();
    }

    // Substitute Value references throughout the body:
    //  - `old_env_param` (Value::Parameter(entry, n+1)) → `env_substitute` (the Deref result),
    //    needed because the parameter at index n+1 no longer exists.
    if let Some(replacement) = env_substitute {
        substitute_value(&mut new_def, old_env_param, replacement);
    }

    mir.definitions.insert(new_id, new_def);
    new_id
}

/// Replace every `Instruction::Capability` in `body_fn_id` with `Transmute(Value::Unit)`, keeping
/// the recorded result type unchanged. Used on the orig body_fn after we've cloned a new one for
/// the tail-resume path: the orig is unreachable at runtime but still gets validated and codegened
/// because the outer definition's now-dead PackClosure still references it. Transmute neutralizes
/// the Capability without affecting types or other consumers.
pub(super) fn neutralize_handler_caps_in_dead_body(mir: &mut Mir, body_fn_id: DefinitionId) {
    let Some(body_fn) = mir.definitions.get_mut(&body_fn_id) else { return };
    let cap_ids: Vec<InstructionId> = body_fn
        .instructions
        .iter()
        .filter_map(|(id, instr)| if matches!(instr, Instruction::Capability) { Some(id) } else { None })
        .collect();
    for id in cap_ids {
        body_fn.instructions[id] = Instruction::Transmute(Value::Unit);
        // result_type stays the cap tuple type; Transmute's validation has no constraints.
    }
}

/// Replace every occurrence of `find` with `replace` across a Definition's instructions and
/// block terminators. This is a structural Value-level substitution; it does not modify
/// instruction result types or block parameter types.
pub(super) fn substitute_value(definition: &mut Definition, find: Value, replace: Value) {
    let sub = |v: &mut Value| {
        if *v == find {
            *v = replace;
        }
    };
    for instruction in definition.instructions.values_mut() {
        match instruction {
            Instruction::Call { function, arguments } => {
                sub(function);
                for a in arguments.iter_mut() {
                    sub(a);
                }
            },
            Instruction::CallClosure { closure, arguments } => {
                sub(closure);
                for a in arguments.iter_mut() {
                    sub(a);
                }
            },
            Instruction::Perform { effect_op: _, arguments } => {
                for a in arguments.iter_mut() {
                    sub(a);
                }
            },
            Instruction::Handle { body, cases } => {
                sub(body);
                for case in cases.iter_mut() {
                    sub(&mut case.handler);
                }
            },
            Instruction::PackClosure { function, environment } => {
                sub(function);
                sub(environment);
            },
            Instruction::IndexTuple { tuple, .. } => sub(tuple),
            Instruction::MakeTuple(values) | Instruction::MakeArray(values) => {
                for v in values.iter_mut() {
                    sub(v);
                }
            },
            Instruction::StackAlloc(v)
            | Instruction::AllocShared(v)
            | Instruction::Transmute(v)
            | Instruction::Id(v) => sub(v),
            Instruction::Store { pointer, value } => {
                sub(pointer);
                sub(value);
            },
            Instruction::AddInt(a, b)
            | Instruction::AddFloat(a, b)
            | Instruction::SubInt(a, b)
            | Instruction::SubFloat(a, b)
            | Instruction::MulInt(a, b)
            | Instruction::MulFloat(a, b)
            | Instruction::DivSigned(a, b)
            | Instruction::DivUnsigned(a, b)
            | Instruction::DivFloat(a, b)
            | Instruction::ModSigned(a, b)
            | Instruction::ModUnsigned(a, b)
            | Instruction::ModFloat(a, b)
            | Instruction::LessSigned(a, b)
            | Instruction::LessUnsigned(a, b)
            | Instruction::LessFloat(a, b)
            | Instruction::EqInt(a, b)
            | Instruction::EqFloat(a, b)
            | Instruction::BitwiseAnd(a, b)
            | Instruction::BitwiseOr(a, b)
            | Instruction::BitwiseXor(a, b) => {
                sub(a);
                sub(b);
            },
            Instruction::BitwiseNot(v)
            | Instruction::SignExtend(v)
            | Instruction::ZeroExtend(v)
            | Instruction::SignedToFloat(v)
            | Instruction::UnsignedToFloat(v)
            | Instruction::FloatToSigned(v)
            | Instruction::FloatToUnsigned(v)
            | Instruction::FloatPromote(v)
            | Instruction::FloatDemote(v)
            | Instruction::Truncate(v)
            | Instruction::Deref(v) => sub(v),
            Instruction::GetFieldPtr { struct_ptr, .. } => sub(struct_ptr),
            Instruction::MakeBytes(_)
            | Instruction::Instantiate(_, _)
            | Instruction::Extern(_)
            | Instruction::SizeOf(_)
            | Instruction::ArrayLen(_)
            | Instruction::StackAllocUninit(_)
            | Instruction::Capability => (),
        }
    }
    for block in definition.blocks.values_mut() {
        let Some(t) = &mut block.terminator else { continue };
        match t {
            TerminatorInstruction::Jmp((_, arg)) => {
                if let Some(a) = arg {
                    sub(a);
                }
            },
            TerminatorInstruction::If { condition, then, else_, end: _ } => {
                sub(condition);
                if let Some(a) = &mut then.1 {
                    sub(a);
                }
                if let Some(a) = &mut else_.1 {
                    sub(a);
                }
            },
            TerminatorInstruction::Switch { int_value, cases, else_, end: _ } => {
                sub(int_value);
                for (_, jt) in cases.iter_mut() {
                    if let Some(a) = &mut jt.1 {
                        sub(a);
                    }
                }
                if let Some(a) = &mut else_.1 {
                    sub(a);
                }
            },
            TerminatorInstruction::Unreachable => (),
            TerminatorInstruction::Return(v) => sub(v),
            TerminatorInstruction::Result(v) => sub(v),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn splice_in_handle_replacement(
    mir: &mut Mir, definition_id: DefinitionId, site: HandleSite, wrapper_ids: &[DefinitionId],
    decisions: &[CaseDecision], body_fn_id: DefinitionId, body_env_value: Option<Value>,
    body_bindings: Option<Arc<GenericBindings>>, cap_tuple_type: &Type, layout: &OriginalEnvLayout,
) {
    // We need the wrapper closure types, the new body env type, and the body fn type *after*
    // mutation. Read them all up front (immutable borrows) before getting the &mut to the def.
    let wrapper_closure_types: Vec<Type> = wrapper_ids.iter().map(|id| mir.definitions[id].typ.clone()).collect();
    let body_fn_type = mir.definitions[&body_fn_id].typ.clone();
    let new_env_type = match &body_fn_type {
        Type::Function(ft) => ft.environment.clone(),
        _ => panic!("body_fn type is not a function"),
    };

    let definition = mir.definitions.get_mut(&definition_id).expect("definition vanished");

    let mut pending: Vec<InstructionId> = Vec::new();

    let mut push = |def: &mut Definition, instr: Instruction, ty: Type| -> Value {
        let id = def.instructions.push(instr);
        def.instruction_result_types.push_existing(id, ty);
        pending.push(id);
        Value::InstructionResult(id)
    };

    // Helper: emit either Value::Definition(target) or an Instantiate instruction
    // depending on whether bindings were recovered from the original use site.
    let make_def_value = |def: &mut Definition,
                          push_fn: &mut dyn FnMut(&mut Definition, Instruction, Type) -> Value,
                          target,
                          target_typ,
                          bindings|
     -> Value {
        if let Some(bindings) = bindings {
            push_fn(def, Instruction::Instantiate(target, bindings), target_typ)
        } else {
            Value::Definition(target)
        }
    };

    // 1. PackClosure each wrapper with a Pointer env: a StackAlloc of the handler's env tuple,
    //    or a null pointer when the handler had no env.
    let mut cap_closures: Vec<Value> = Vec::with_capacity(wrapper_ids.len());
    for (i, wrapper_id) in wrapper_ids.iter().enumerate() {
        let env_pointer_value = match decisions[i].handler_env {
            Some(env) => push(definition, Instruction::StackAlloc(env), Type::POINTER),
            None => {
                push(definition, Instruction::Transmute(Value::Integer(crate::mir::IntConstant::Usz(0))), Type::POINTER)
            },
        };
        let closure_type = wrapper_closure_types[i].clone();
        let wrapper_value = make_def_value(
            definition,
            &mut push,
            *wrapper_id,
            closure_type.clone(),
            decisions[i].handler_bindings.clone(),
        );
        let value = push(
            definition,
            Instruction::PackClosure { function: wrapper_value, environment: env_pointer_value },
            closure_type,
        );
        cap_closures.push(value);
    }

    // 2. Tuple the wrappers into the cap value.
    let cap = push(definition, Instruction::MakeTuple(cap_closures), cap_tuple_type.clone());

    // 3. Build the new env value (existing fields + cap) and PackClosure body_fn with it.
    let new_env_value = if layout.was_closure {
        let mut fields: Vec<Value> = Vec::with_capacity(layout.original_field_types.len() + 1);
        let original_env = body_env_value.expect("body was a closure but no env value found");
        for (i, ty) in layout.original_field_types.iter().enumerate() {
            let v = push(definition, Instruction::IndexTuple { tuple: original_env, index: i as u32 }, ty.clone());
            fields.push(v);
        }
        fields.push(cap);
        push(definition, Instruction::MakeTuple(fields), new_env_type.clone())
    } else {
        push(definition, Instruction::MakeTuple(vec![cap]), new_env_type.clone())
    };

    let body_fn_value = make_def_value(definition, &mut push, body_fn_id, body_fn_type.clone(), body_bindings);
    let body_closure = push(
        definition,
        Instruction::PackClosure { function: body_fn_value, environment: new_env_value },
        body_fn_type,
    );

    // 4. Reuse the original Handle's instruction id for the CallClosure so consumers of the
    //    Handle's result Value keep working. Body lambda has a single Unit parameter.
    let HandleSite { block, index, id: original_id, result_type, .. } = site;
    definition.instructions[original_id] =
        Instruction::CallClosure { closure: body_closure, arguments: vec![Value::Unit] };
    definition.instruction_result_types[original_id] = result_type;
    pending.push(original_id);

    definition.blocks[block].instructions.splice(index..=index, pending);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::lexer::token::IntegerKind;
    use crate::mir::{
        BlockId, Definition, FunctionType, HandlerCase, Instruction, Mir, TerminatorInstruction, Type, Value,
        next_definition_id,
    };

    /// Build a handler function `fn(x: U32, resume: fn U32 [Pointer] -> U32) -> U32`. If
    /// `tail_resume` is true, the body is `Return(CallClosure(resume, [x]))` (tail-resumptive);
    /// otherwise the body is `let r = resume(x); Return(MakeTuple([])) /* drop r, return unit */`
    /// non-tail calls introduce an AddInt instruction that uses the resume call's result.
    fn make_simple_handler(tail_resume: bool) -> (Definition, Type) {
        let u32_t = Type::int(IntegerKind::U32);
        let resume_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![u32_t.clone()],
            environment: Type::POINTER,
            return_type: u32_t.clone(),
        }));
        let handler_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![u32_t.clone(), resume_type.clone()],
            environment: Type::NO_CLOSURE_ENV,
            return_type: u32_t.clone(),
        }));

        let id = next_definition_id();
        let mut def = Definition::new(Arc::new("handler".to_string()), id, 0, handler_type.clone());
        let entry = BlockId::ENTRY_BLOCK;
        def.blocks[entry].parameter_types = vec![u32_t.clone(), resume_type];
        let x = Value::Parameter(entry, 0);
        let resume = Value::Parameter(entry, 1);

        let call_id = def.instructions.push(Instruction::CallClosure { closure: resume, arguments: vec![x] });
        def.instruction_result_types.push_existing(call_id, u32_t.clone());
        def.blocks[entry].instructions.push(call_id);

        if tail_resume {
            def.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::InstructionResult(call_id)));
        } else {
            // `let r = resume(x); return r + 1`
            let one_id = def.instructions.push(Instruction::Id(Value::Integer(crate::mir::IntConstant::U32(1))));
            def.instruction_result_types.push_existing(one_id, u32_t.clone());
            def.blocks[entry].instructions.push(one_id);

            let add_id = def
                .instructions
                .push(Instruction::AddInt(Value::InstructionResult(call_id), Value::InstructionResult(one_id)));
            def.instruction_result_types.push_existing(add_id, u32_t.clone());
            def.blocks[entry].instructions.push(add_id);
            def.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::InstructionResult(add_id)));
        }
        (def, handler_type)
    }

    /// Build a body lambda with no captures: `fn () -> Unit { perform op (); () }`. Returns the
    /// body's [DefinitionId] and the effect-op's id (which we treat as opaque since the test
    /// only inspects pre/post Mir state).
    fn make_body_with_perform(op_id: DefinitionId, op_arg_type: Type, op_return_type: Type) -> Definition {
        let body_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![],
            environment: Type::NO_CLOSURE_ENV,
            return_type: Type::UNIT,
        }));

        let id = next_definition_id();
        let mut def = Definition::new(Arc::new("body".to_string()), id, 0, body_type);

        let entry = BlockId::ENTRY_BLOCK;
        let _ = op_arg_type;
        let _ = op_return_type;
        let _ = op_id;
        // Empty body. It's enough for the optimization tests not to need a real Perform.
        def.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::Unit));
        def
    }

    /// Wrap a Handle around `body` with a single case using `handler_def`. Returns the outer
    /// definition (containing the Handle) and the Handle's instruction id.
    fn make_outer_with_handle(
        body_def_id: DefinitionId, handler_def_id: DefinitionId, op_id: DefinitionId, result_type: Type,
    ) -> (Definition, InstructionId) {
        let outer_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![],
            environment: Type::NO_CLOSURE_ENV,
            return_type: result_type.clone(),
        }));
        let id = next_definition_id();
        let mut def = Definition::new(Arc::new("outer".to_string()), id, 0, outer_type);

        let body_value = Value::Definition(body_def_id);
        let handler_value = Value::Definition(handler_def_id);
        let cases = vec![HandlerCase { effect_op: op_id, handler: handler_value }];

        let handle_id = def.instructions.push(Instruction::Handle { body: body_value, cases });
        def.instruction_result_types.push_existing(handle_id, result_type.clone());
        def.blocks[BlockId::ENTRY_BLOCK].instructions.push(handle_id);
        def.blocks[BlockId::ENTRY_BLOCK].terminator = Some(TerminatorInstruction::Return(Value::Unit));
        (def, handle_id)
    }

    fn count_handles(mir: &Mir, def_id: DefinitionId) -> usize {
        mir.definitions[&def_id].instructions.values().filter(|i| matches!(i, Instruction::Handle { .. })).count()
    }

    #[test]
    fn tail_resume_eliminates_simple_handle() {
        let (handler_def, _) = make_simple_handler(true);
        let handler_id = handler_def.id;

        let op_id = next_definition_id();
        let body_def = make_body_with_perform(op_id, Type::int(IntegerKind::U32), Type::int(IntegerKind::U32));
        let body_id = body_def.id;

        let (outer_def, _handle_id) = make_outer_with_handle(body_id, handler_id, op_id, Type::UNIT);
        let outer_id = outer_def.id;

        let mut mir = Mir::default();
        mir.definitions.insert(handler_id, handler_def);
        mir.definitions.insert(body_id, body_def);
        mir.definitions.insert(outer_id, outer_def);

        let mir = mir.optimize_tail_resume();
        assert_eq!(count_handles(&mir, outer_id), 0, "Handle should be eliminated");

        // A new wrapper definition should exist.
        let wrapper_count = mir.definitions.values().filter(|d| d.name.contains("tail_resume_wrapper")).count();
        assert_eq!(wrapper_count, 1, "expected one materialized wrapper");
    }

    #[test]
    fn tail_resume_skips_non_tail() {
        let (handler_def, _) = make_simple_handler(false);
        let handler_id = handler_def.id;

        let op_id = next_definition_id();
        let body_def = make_body_with_perform(op_id, Type::int(IntegerKind::U32), Type::int(IntegerKind::U32));
        let body_id = body_def.id;

        let (outer_def, _) = make_outer_with_handle(body_id, handler_id, op_id, Type::UNIT);
        let outer_id = outer_def.id;

        let mut mir = Mir::default();
        mir.definitions.insert(handler_id, handler_def);
        mir.definitions.insert(body_id, body_def);
        mir.definitions.insert(outer_id, outer_def);

        let mir = mir.optimize_tail_resume();
        assert_eq!(count_handles(&mir, outer_id), 1, "non-tail Handle should be left alone");
        let wrapper_count = mir.definitions.values().filter(|d| d.name.contains("tail_resume_wrapper")).count();
        assert_eq!(wrapper_count, 0, "expected no wrappers materialized");
    }

    #[test]
    fn tail_resume_skips_resume_escape() {
        // Build a handler whose body returns resume itself, packed in a tuple. resume_param
        // appears in MakeTuple's args, which is a non-CallClosure use.
        let u32_t = Type::int(IntegerKind::U32);
        let resume_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![u32_t.clone()],
            environment: Type::POINTER,
            return_type: u32_t.clone(),
        }));
        let result_type = Type::Tuple(Arc::new(vec![resume_type.clone()]));
        let handler_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![u32_t.clone(), resume_type.clone()],
            environment: Type::NO_CLOSURE_ENV,
            return_type: result_type.clone(),
        }));

        let handler_id = next_definition_id();
        let mut handler_def = Definition::new(Arc::new("handler".to_string()), handler_id, 0, handler_type);
        let entry = BlockId::ENTRY_BLOCK;
        handler_def.blocks[entry].parameter_types = vec![u32_t.clone(), resume_type.clone()];
        let resume = Value::Parameter(entry, 1);

        let mk_id = handler_def.instructions.push(Instruction::MakeTuple(vec![resume]));
        handler_def.instruction_result_types.push_existing(mk_id, result_type.clone());
        handler_def.blocks[entry].instructions.push(mk_id);
        handler_def.blocks[entry].terminator = Some(TerminatorInstruction::Return(Value::InstructionResult(mk_id)));

        let op_id = next_definition_id();
        let body_def = make_body_with_perform(op_id, u32_t.clone(), u32_t.clone());
        let body_id = body_def.id;

        let (outer_def, _) = make_outer_with_handle(body_id, handler_id, op_id, result_type);
        let outer_id = outer_def.id;

        let mut mir = Mir::default();
        mir.definitions.insert(handler_id, handler_def);
        mir.definitions.insert(body_id, body_def);
        mir.definitions.insert(outer_id, outer_def);

        let mir = mir.optimize_tail_resume();
        assert_eq!(count_handles(&mir, outer_id), 1, "escaping resume should disqualify the Handle");
    }

    #[test]
    fn tail_resume_all_or_nothing() {
        // Two cases: one tail-resumptive, one not. Whole Handle must be left alone.
        let (tr_handler, _) = make_simple_handler(true);
        let tr_id = tr_handler.id;
        let (non_tr_handler, _) = make_simple_handler(false);
        let non_tr_id = non_tr_handler.id;

        let op1 = next_definition_id();
        let op2 = next_definition_id();
        let body_def = make_body_with_perform(op1, Type::int(IntegerKind::U32), Type::int(IntegerKind::U32));
        let body_id = body_def.id;

        // Build outer with two cases.
        let outer_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![],
            environment: Type::NO_CLOSURE_ENV,
            return_type: Type::UNIT,
        }));
        let outer_id = next_definition_id();
        let mut outer_def = Definition::new(Arc::new("outer".to_string()), outer_id, 0, outer_type);
        let cases = vec![
            HandlerCase { effect_op: op1, handler: Value::Definition(tr_id) },
            HandlerCase { effect_op: op2, handler: Value::Definition(non_tr_id) },
        ];
        let handle_id = outer_def.instructions.push(Instruction::Handle { body: Value::Definition(body_id), cases });
        outer_def.instruction_result_types.push_existing(handle_id, Type::UNIT);
        outer_def.blocks[BlockId::ENTRY_BLOCK].instructions.push(handle_id);
        outer_def.blocks[BlockId::ENTRY_BLOCK].terminator = Some(TerminatorInstruction::Return(Value::Unit));

        let mut mir = Mir::default();
        mir.definitions.insert(tr_id, tr_handler);
        mir.definitions.insert(non_tr_id, non_tr_handler);
        mir.definitions.insert(body_id, body_def);
        mir.definitions.insert(outer_id, outer_def);

        let mir = mir.optimize_tail_resume();
        assert_eq!(count_handles(&mir, outer_id), 1, "mixed Handle should be left alone");
        let wrapper_count = mir.definitions.values().filter(|d| d.name.contains("tail_resume_wrapper")).count();
        assert_eq!(wrapper_count, 0);
    }

    /// Nested tail-resumptive handlers: optimizing the outer Handle clones its body (which itself
    /// contains the inner Handle). The worklist must re-process that clone so the inner Handle is
    /// also eliminated, and the deep clone must give it a private copy of the inner body so the
    /// dead original cannot corrupt it.
    #[test]
    fn tail_resume_eliminates_nested_handle_in_clone() {
        let u32_t = Type::int(IntegerKind::U32);

        let (inner_handler, _) = make_simple_handler(true);
        let inner_handler_id = inner_handler.id;
        let inner_op = next_definition_id();
        let inner_body = make_body_with_perform(inner_op, u32_t.clone(), u32_t.clone());
        let inner_body_id = inner_body.id;

        // The outer Handle's body is itself a definition containing the inner Handle.
        let (outer_body, _) = make_outer_with_handle(inner_body_id, inner_handler_id, inner_op, Type::UNIT);
        let outer_body_id = outer_body.id;

        let (outer_handler, _) = make_simple_handler(true);
        let outer_handler_id = outer_handler.id;
        let outer_op = next_definition_id();
        let (outermost, _) = make_outer_with_handle(outer_body_id, outer_handler_id, outer_op, Type::UNIT);
        let outermost_id = outermost.id;

        let mut mir = Mir::default();
        for def in [inner_handler, inner_body, outer_body, outer_handler, outermost] {
            mir.definitions.insert(def.id, def);
        }

        let mir = mir.optimize_tail_resume();

        // The outer Handle is eliminated...
        assert_eq!(count_handles(&mir, outermost_id), 0, "outer Handle should be eliminated");
        // ...and the nested Handle inside every live body clone is eliminated too (the worklist
        // re-processes clones rather than leaving their Handles for the coroutine fallback).
        let clone_handles: usize = mir
            .definitions
            .values()
            .filter(|d| d.name.contains("tail_resume_body"))
            .map(|d| d.instructions.values().filter(|i| matches!(i, Instruction::Handle { .. })).count())
            .sum();
        assert_eq!(clone_handles, 0, "nested Handle in the body clone should be eliminated");
    }
}
