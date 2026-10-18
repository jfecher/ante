//! This file implements a [Mir] pass to inline small functions under [MAX_INSTRUCTIONS] instructions.
//! This pass is very important for performance when lowering generics via existentialization.
//! Because this pass is meant for use with existentialization, we run this inlining even in -O0 so
//! it is less aggressive than full inlining would be in llvm under e.g. -O2.
use std::{borrow::Cow, sync::Arc};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    iterator_extensions::mapvec,
    mir::{
        BlockId, ConstantEnvironment, Definition, DefinitionId, GenericBindings, Instruction, InstructionId, Mir,
        TerminatorInstruction, Type, Value,
    },
};

/// Functions with at most this many instructions are inlined
const MAX_INSTRUCTIONS: usize = 8;

/// Maximum number of recursive calls to do since inlining can expose more calls to inline.
/// This is just meant to be a quick pass in -O0 for existentialization so we keep this small.
const ROUNDS: usize = 2;

impl Mir {
    pub(crate) fn inline_small_functions(mut self) -> Mir {
        // After the first round only definitions that changed can have new calls to inline
        let mut changed: Option<Vec<DefinitionId>> = None;
        for _ in 0..ROUNDS {
            let callees = Callees::new(&self);
            let inline = |id: &DefinitionId| Some((*id, inline_calls(&self.definitions[id], &callees)?));
            let inlined: Vec<_> = match &changed {
                None => self.definitions.par_iter().filter_map(|(id, _)| inline(id)).collect(),
                Some(ids) => ids.par_iter().filter_map(inline).collect(),
            };
            if inlined.is_empty() {
                break;
            }
            changed = Some(mapvec(&inlined, |(id, _)| *id));
            self.definitions.extend(inlined);
        }
        if changed.is_some() { self.remove_unreachable_functions() } else { self }
    }
}

struct Callees<'a> {
    mir: &'a Mir,
    inlinable: FxHashSet<DefinitionId>,
}

impl<'a> Callees<'a> {
    fn new(mir: &'a Mir) -> Self {
        let inlinable = mir
            .definitions
            .par_iter()
            .filter(|(_, definition)| is_small(definition) && !refers_to_itself_or_generics(definition, mir))
            .map(|(id, _)| *id)
            .collect();
        Self { mir, inlinable }
    }
}

fn is_generic(mir: &Mir, id: &DefinitionId) -> bool {
    mir.definitions.get(id).is_some_and(|definition| definition.generic_count > 0)
}

fn is_small(definition: &Definition) -> bool {
    !definition.is_global()
        && definition.blocks.len() == 1
        && definition.instructions.len() <= MAX_INSTRUCTIONS
        && matches!(definition.entry_block().terminator, Some(TerminatorInstruction::Return(_)))
}

/// A callee to inline
struct Inlinee<'a> {
    callee: &'a Definition,
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

/// Returns the definition with its calls inlined or `None` if none were
fn inline_calls(original: &Definition, mir: &Callees) -> Option<Definition> {
    let mut definition = Cow::Borrowed(original);
    let blocks = mapvec(definition.blocks.iter(), |(id, _)| id);
    for block in blocks {
        let mut index = 0;
        while index < definition.blocks[block].instructions.len() {
            let id = definition.blocks[block].instructions[index];
            let (Instruction::Call { function, arguments } | Instruction::CallClosure { closure: function, arguments }) =
                &definition.instructions[id]
            else {
                index += 1;
                continue;
            };
            let Some(inlinee) = inlinee(&definition, function, mir) else {
                index += 1;
                continue;
            };
            let arguments = arguments.clone();
            let definition = definition.to_mut();
            let (inlined, result) = splice(definition, &inlinee, &arguments);
            definition.instructions[id] = Instruction::Id(result);
            let count = inlined.len();
            definition.blocks[block].instructions.splice(index..index, inlined);
            index += count + 1;
        }
    }
    match definition {
        Cow::Owned(definition) => Some(definition),
        Cow::Borrowed(_) => None,
    }
}

/// Returns a function to inline if found
fn inlinee<'a>(caller: &Definition, function: &Value, mir: &Callees<'a>) -> Option<Inlinee<'a>> {
    let (id, bindings, environment) = if let Some((id, bindings)) = caller.definition_of(*function) {
        (id, bindings, None)
    } else if let Value::InstructionResult(result) = caller.follow_ids(*function)
        && let Instruction::IndexTuple { tuple, index } = &caller.instructions[result]
    {
        dictionary_method(caller, tuple, *index, mir.mir)?
    } else {
        return None;
    };
    let inlinable = id != caller.id && mir.inlinable.contains(&id);
    inlinable.then(|| Inlinee { callee: &mir.mir.definitions[&id], bindings, environment })
}

/// The function of a closure stored in a constant global. This is usually a trait impl method
fn dictionary_method(
    caller: &Definition, tuple: &Value, index: u32, mir: &Mir,
) -> Option<(DefinitionId, Option<Arc<GenericBindings>>, Option<Environment>)> {
    let (global, outer) = caller.definition_of(*tuple)?;
    let outer = outer.unwrap_or_default();
    let global = mir.definitions.get(&global)?;
    if !global.is_global() || global.blocks.len() != 1 || outer.len() != global.generic_count as usize {
        return None;
    }
    let (function, environment) = global.constant_closure_field(index)?;
    let (function, bindings) = match global.definition_of(function)? {
        (id, None) if !is_generic(mir, &id) => (id, None),
        (id, Some(bindings)) => (id, Some(Arc::new(mapvec(bindings.iter(), |binding| binding.substitute(&outer))))),
        _ => return None,
    };
    let environment = match environment {
        ConstantEnvironment::Constant(constant) => Environment::Value(constant),
        ConstantEnvironment::Transmuted(constant, id) => {
            Environment::Transmuted(constant, global.instruction_result_type(id).substitute(&outer))
        },
        ConstantEnvironment::Runtime => return None,
    };
    Some((function, bindings, Some(environment)))
}

/// Referring to itself would inline forever, and a reference to a generic definition without an
/// `Instantiate` means the callee's own generics, which the caller does not share
fn refers_to_itself_or_generics(callee: &Definition, mir: &Mir) -> bool {
    let mut found = false;
    callee.for_each_used_value(|value| {
        if let Value::Definition(other) = value {
            found |= *other == callee.id || is_generic(mir, other);
        }
    });
    let instantiates_itself = |instruction| matches!(instruction, &Instruction::Instantiate(id, _) if id == callee.id);
    found || callee.instructions.values().any(instantiates_itself)
}

/// Copy the callee's instructions into `caller`, returning their ids and the callee's result
fn splice(caller: &mut Definition, inlinee: &Inlinee, arguments: &[Value]) -> (Vec<InstructionId>, Value) {
    let Inlinee { callee, bindings, environment } = inlinee;
    let mut values: FxHashMap<Value, Value> = FxHashMap::default();
    let mut ids = Vec::new();

    let parameters = &callee.entry_block().parameter_types;
    for (index, argument) in arguments.iter().enumerate() {
        values.insert(Value::Parameter(BlockId::ENTRY_BLOCK, index as u32), *argument);
    }
    if parameters.len() == arguments.len() + 1 {
        let environment = match environment {
            Some(Environment::Value(value)) => *value,
            Some(Environment::Transmuted(constant, typ)) => {
                let id = caller.instructions.push(Instruction::Transmute(*constant));
                caller.instruction_result_types.push_existing(id, typ.clone());
                ids.push(id);
                Value::InstructionResult(id)
            },
            None => panic!("inlining a closure without its environment"),
        };
        values.insert(Value::Parameter(BlockId::ENTRY_BLOCK, arguments.len() as u32), environment);
    }

    for old in &callee.entry_block().instructions {
        let mut instruction = callee.instructions[*old].clone();
        instruction.for_each_value_mut(|value| {
            if let Some(new) = values.get(value) {
                *value = *new;
            }
        });
        let mut typ = callee.instruction_result_type(*old).clone();
        if let Some(bindings) = bindings {
            instruction.for_each_type_mut(|typ| *typ = typ.substitute(bindings));
            typ = typ.substitute(bindings);
        }
        let id = caller.instructions.push(instruction);
        caller.instruction_result_types.push_existing(id, typ);
        values.insert(Value::InstructionResult(*old), Value::InstructionResult(id));
        ids.push(id);
    }

    let Some(TerminatorInstruction::Return(result)) = &callee.entry_block().terminator else { unreachable!() };
    (ids, values.get(result).copied().unwrap_or(*result))
}
