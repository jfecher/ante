//! This module implements atomic operations on values whose size is only known at runtime. Each
//! calls a helper which switches on the size to the atomic instruction of that width.

use std::sync::Arc;

use crate::{
    lexer::token::IntegerKind,
    mir::{
        AtomicOrdering, AtomicRmwOp, Block, BlockId, Definition, DefinitionId, Instruction, TerminatorInstruction,
        Type, Value,
        existentialization::{
            Helper,
            builder::{FunctionBuilder, Op, usz_value},
            emit,
            types::usz,
        },
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum AtomicOperation {
    Load(AtomicOrdering),
    Store(AtomicOrdering),
    Rmw(AtomicRmwOp, AtomicOrdering),
    Cmpxchg(AtomicOrdering, AtomicOrdering),
}

/// How many operands an atomic operation takes besides its pointer, and whether it has a result
fn atomic_signature(operation: AtomicOperation) -> (usize, bool) {
    match operation {
        AtomicOperation::Load(_) => (0, true),
        AtomicOperation::Store(_) => (1, false),
        AtomicOperation::Rmw(..) => (1, true),
        AtomicOperation::Cmpxchg(..) => (2, true),
    }
}

impl FunctionBuilder<'_> {
    /// Perform `operation` on the `size` bytes at `pointer`, with `operands` given by address
    pub(crate) fn call_atomic(
        &mut self, operation: AtomicOperation, pointer: Value, size: Op, operands: Vec<Value>, result_type: &Type,
    ) -> Value {
        let (_, has_result) = atomic_signature(operation);
        let mut arguments = vec![pointer, usz_value(size)];
        arguments.extend(operands);
        let slot = has_result.then(|| self.slot(result_type));
        arguments.extend(slot);
        let function = Value::Definition(self.shared.helper(Helper::Atomic(operation)));
        let unit = self.emit(Instruction::Call { function, arguments }, Type::UNIT);
        slot.unwrap_or(unit)
    }
}

/// `fn (pointer: Pointer) (size: Usz) (operands: Pointer..) (result: Pointer) -> Unit`
pub(super) fn helper(operation: AtomicOperation, id: DefinitionId, ptr_size: u32) -> Definition {
    let (operand_count, has_result) = atomic_signature(operation);
    let mut parameters = vec![Type::POINTER, usz()];
    parameters.extend(std::iter::repeat_n(Type::POINTER, operand_count + has_result as usize));
    let typ = Type::function(parameters.clone(), Type::UNIT);
    let mut definition = Definition::new(Arc::new("atomic".to_string()), id, 0, typ);
    let entry = BlockId::ENTRY_BLOCK;
    let parameter = |index: usize| Value::Parameter(entry, index as u32);
    definition.blocks[entry].parameter_types = parameters;
    let exit = definition.blocks.push(Block::new(Vec::new()));
    let unsupported = definition.blocks.push(Block::new(Vec::new()));

    let mut cases = Vec::new();
    for kind in [IntegerKind::U8, IntegerKind::U16, IntegerKind::U32, IntegerKind::U64] {
        let block = definition.blocks.push(Block::new(Vec::new()));
        cases.push((kind.size_in_bytes(ptr_size), (block, None)));

        let int = Type::int(kind);
        let mut emit = |instruction, typ| emit(&mut definition, block, instruction, typ);
        let operands: Vec<Value> =
            (0..operand_count).map(|index| emit(Instruction::Deref(parameter(2 + index)), int.clone())).collect();
        let pointer = parameter(0);
        let result = match operation {
            AtomicOperation::Load(ordering) => emit(Instruction::AtomicLoad { pointer, ordering }, int.clone()),
            AtomicOperation::Store(ordering) => {
                emit(Instruction::AtomicStore { pointer, value: operands[0], ordering }, Type::UNIT)
            },
            AtomicOperation::Rmw(op, ordering) => {
                emit(Instruction::AtomicRmw { op, pointer, value: operands[0], ordering }, int.clone())
            },
            AtomicOperation::Cmpxchg(success, failure) => {
                let (expected, desired) = (operands[0], operands[1]);
                emit(Instruction::AtomicCmpxchg { pointer, expected, desired, success, failure }, int.clone())
            },
        };
        if has_result {
            emit(Instruction::Store { pointer: parameter(2 + operand_count), value: result }, Type::UNIT);
        }
        definition.blocks[block].terminator = Some(TerminatorInstruction::jmp_no_args(exit));
    }

    let size = parameter(1);
    let int_value = emit(&mut definition, entry, Instruction::Truncate(size), Type::int(IntegerKind::U32));
    definition.blocks[entry].terminator =
        Some(TerminatorInstruction::Switch { int_value, cases, else_: (unsupported, None), end: exit });
    definition.blocks[unsupported].terminator = Some(TerminatorInstruction::Unreachable);
    definition.blocks[exit].terminator = Some(TerminatorInstruction::Return(Value::Unit));
    definition
}
