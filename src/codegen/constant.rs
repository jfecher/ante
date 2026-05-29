//! Backend-neutral constant evaluation of MIR globals.
//!
//! A [mir::Definition] that [is a global](mir::Definition::is_global) is a single block whose
//! instructions are all constant-foldable and whose `Result` terminator names the value the
//! global holds. This module folds such a global into a [ConstantValue] tree without committing
//! to any particular backend representation, so each backend can render the result however it
//! likes (the C backend emits a file-scope initializer; the LLVM backend can later replace its
//! inline `codegen_constant_instruction` with a call here plus a `BasicValueEnum` renderer).

use rustc_hash::FxHashMap;

use crate::mir::{self, BlockId, DefinitionId, InstructionId, TerminatorInstruction, Type, Value};

/// The constant value a global evaluates to. Variants cover exactly the instructions that are
/// constant-foldable in a global initializer.
#[derive(Debug, Clone)]
pub(crate) enum ConstantValue {
    Unit,
    Bool(bool),
    Char(char),
    Int(mir::IntConstant),
    Float(mir::FloatConstant),
    Tuple(Vec<ConstantValue>),
    /// `element_type` lets a backend spell the array type even when `elements` is empty (the C
    /// backend reads it via the variable's declarator instead, so it ignores this field).
    Array {
        elements: Vec<ConstantValue>,
        element_type: Type,
    },
    /// An immutable byte blob (a string literal). Backed by static storage; rendered as a pointer.
    Bytes(Vec<u8>),
    /// A reference to another global or function, rendered by name.
    Definition(DefinitionId),
    /// An external symbol, rendered by name (its `typ` lets a backend declare it).
    Extern {
        name: String,
        typ: Type,
    },
    /// A heap-shared value. A backend with no heap at init time backs it with static storage of
    /// `typ` and takes its address.
    Shared {
        value: Box<ConstantValue>,
        typ: Type,
    },
    /// A transmute from a zero-sized source: yields an indeterminate value of `typ`.
    Transmute {
        typ: Type,
    },
}

/// Fold a global definition into a [ConstantValue]. Panics if the definition contains an
/// instruction that is not constant-foldable in a global initializer.
pub(crate) fn evaluate_global(mir: &mir::Mir, global: &mir::Definition) -> ConstantValue {
    let mut values = FxHashMap::default();
    for id in global.entry_block().instructions.iter().copied() {
        let value = evaluate_instruction(mir, global, id, &values);
        values.insert(Value::InstructionResult(id), value);
    }

    let TerminatorInstruction::Result(result) = global.entry_block().terminator.as_ref().unwrap() else {
        panic!("Global definition missing Result terminator");
    };
    constant_value(*result, &values)
}

/// Resolve a [Value] to a [ConstantValue], reading instruction/parameter results from `values`.
fn constant_value(value: Value, values: &FxHashMap<Value, ConstantValue>) -> ConstantValue {
    match value {
        Value::Unit => ConstantValue::Unit,
        Value::Bool(b) => ConstantValue::Bool(b),
        Value::Char(c) => ConstantValue::Char(c),
        Value::Integer(constant) => ConstantValue::Int(constant),
        Value::Float(constant) => ConstantValue::Float(constant),
        Value::InstructionResult(_) | Value::Parameter(..) => {
            values.get(&value).cloned().unwrap_or_else(|| panic!("constant value not cached: {value}"))
        },
        Value::Definition(id) => ConstantValue::Definition(id),
        Value::Error => unreachable!("Error value in global initializer"),
    }
}

fn evaluate_instruction(
    mir: &mir::Mir, definition: &mir::Definition, id: InstructionId, values: &FxHashMap<Value, ConstantValue>,
) -> ConstantValue {
    match &definition.instructions[id] {
        mir::Instruction::MakeTuple(fields) => {
            ConstantValue::Tuple(fields.iter().map(|f| constant_value(*f, values)).collect())
        },
        mir::Instruction::MakeArray(elements) => {
            let element_type = match definition.instruction_result_type(id) {
                Type::Array { element, .. } => (**element).clone(),
                other => panic!("MakeArray result type is not an array: {other}"),
            };
            ConstantValue::Array {
                elements: elements.iter().map(|e| constant_value(*e, values)).collect(),
                element_type,
            }
        },
        mir::Instruction::MakeBytes(bytes) => ConstantValue::Bytes(bytes.clone()),
        mir::Instruction::Id(value) => constant_value(*value, values),
        mir::Instruction::Transmute(_) => {
            // Const context can only transmute zero-sized sources, so the result is just an
            // indeterminate value of the destination type (mirrors the LLVM backend's `undef`).
            ConstantValue::Transmute { typ: definition.instruction_result_type(id).clone() }
        },
        mir::Instruction::Extern(name) => {
            ConstantValue::Extern { name: name.clone(), typ: definition.instruction_result_type(id).clone() }
        },
        mir::Instruction::AllocShared(value) => {
            let typ = mir.type_of_value(value, definition);
            ConstantValue::Shared { value: Box::new(constant_value(*value, values)), typ }
        },
        mir::Instruction::Call { function, arguments } => {
            // Constructor-style calls appear in `implicit` globals after monomorphization. The
            // callee is a single-block, constant-foldable function, so inline it: bind its entry
            // parameters to the argument values and fold its body.
            let callee_id = resolve_constant_call_target(definition, *function)
                .unwrap_or_else(|| panic!("Call in global initializer to non-resolvable function value: {function}"));
            let callee = mir
                .definitions
                .get(&callee_id)
                .unwrap_or_else(|| panic!("Call in global initializer: target definition {callee_id} not found"));
            assert!(
                callee.blocks.len() == 1,
                "Call in global initializer to non-constant-evaluable function `{}`: callee has multiple blocks",
                callee.name
            );
            let callee_result = match callee.entry_block().terminator.as_ref().expect("missing callee terminator") {
                TerminatorInstruction::Return(v) | TerminatorInstruction::Result(v) => *v,
                _ => panic!(
                    "Call in global initializer to non-constant-evaluable function `{}`: terminator is not Result/Return",
                    callee.name
                ),
            };

            let mut callee_values = FxHashMap::default();
            for (i, argument) in arguments.iter().enumerate() {
                let value = constant_value(*argument, values);
                callee_values.insert(Value::Parameter(BlockId::ENTRY_BLOCK, i as u32), value);
            }
            for instr_id in callee.entry_block().instructions.iter().copied() {
                let value = evaluate_instruction(mir, callee, instr_id, &callee_values);
                callee_values.insert(Value::InstructionResult(instr_id), value);
            }
            constant_value(callee_result, &callee_values)
        },
        other => panic!("Unsupported instruction in global initializer: {other:?}"),
    }
}

/// Whether `value` can be emitted as a C file-scope (static) initializer, which must be a
/// constant expression. Reading another global *variable's* stored value is not constant, so a
/// [ConstantValue::Definition] referring to a global is rejected; a reference to a function decays
/// to its (constant) address and is fine. A [ConstantValue::Shared] backs itself with a `static`
/// whose own initializer must likewise be constant, so it inherits its inner value's constness.
pub(crate) fn is_c_constant(value: &ConstantValue, mir: &mir::Mir) -> bool {
    match value {
        ConstantValue::Unit
        | ConstantValue::Bool(_)
        | ConstantValue::Char(_)
        | ConstantValue::Int(_)
        | ConstantValue::Float(_)
        | ConstantValue::Bytes(_)
        | ConstantValue::Extern { .. }
        | ConstantValue::Transmute { .. } => true,
        ConstantValue::Definition(id) => !mir.definitions.get(id).is_some_and(|d| d.is_global()),
        ConstantValue::Tuple(values) => values.iter().all(|v| is_c_constant(v, mir)),
        ConstantValue::Array { elements, .. } => elements.iter().all(|v| is_c_constant(v, mir)),
        ConstantValue::Shared { value, .. } => is_c_constant(value, mir),
    }
}

/// Collect into `out` every other global *variable* this value reads by value (a
/// [ConstantValue::Definition] naming a global). These are the globals whose runtime
/// initialization must precede this one's. Functions are skipped (their address is constant).
pub(crate) fn referenced_globals(value: &ConstantValue, mir: &mir::Mir, out: &mut Vec<DefinitionId>) {
    match value {
        ConstantValue::Definition(id) => {
            if mir.definitions.get(id).is_some_and(|d| d.is_global()) {
                out.push(*id);
            }
        },
        ConstantValue::Tuple(values) => values.iter().for_each(|v| referenced_globals(v, mir, out)),
        ConstantValue::Array { elements, .. } => elements.iter().for_each(|v| referenced_globals(v, mir, out)),
        ConstantValue::Shared { value, .. } => referenced_globals(value, mir, out),
        ConstantValue::Unit
        | ConstantValue::Bool(_)
        | ConstantValue::Char(_)
        | ConstantValue::Int(_)
        | ConstantValue::Float(_)
        | ConstantValue::Bytes(_)
        | ConstantValue::Extern { .. }
        | ConstantValue::Transmute { .. } => {},
    }
}

/// Trace a Call's function-position value back to a [DefinitionId]. Follows `Id`-chains since
/// `lower_closures` leaves a free function reference as `Id(Value::Definition(_))`.
fn resolve_constant_call_target(definition: &mir::Definition, value: Value) -> Option<DefinitionId> {
    match value {
        Value::Definition(id) => Some(id),
        Value::InstructionResult(iid) => match &definition.instructions[iid] {
            mir::Instruction::Id(inner) => resolve_constant_call_target(definition, *inner),
            mir::Instruction::Instantiate(id, _) => Some(*id),
            _ => None,
        },
        _ => None,
    }
}
