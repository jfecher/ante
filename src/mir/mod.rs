//! The Medium-level Intermediate Representation (MIR) is Ante's representation for code after
//! monomorphization/boxing occurs. After type-checking, ante will either implicitly box generics
//! (when running in debug mode) or monomorphize them away (when in release mode). Both of these
//! passes will output MIR, although the form will slightly differ. In general, MIR:
//! - Contains no generics. Generics are monomorphized away or replaced with opaque pointers
//! - Has no mutable variables (they are translated into values of a mutable reference type)
//! - Has explicit drops
//! - Makes all arguments explicit
//! - Replaces higher-level control-flow constructs with basic blocks and jumps.
//!
//! This file contains the various types which comprise the IR. See the submodules for the
//! passes that translate the cst to the IR.

use crate::{
    parser::ids::{TopLevelId, TopLevelName},
    vecmap::VecMap,
};
pub(crate) mod builder;
mod display;
pub(crate) mod monomorphization;

pub(crate) struct Function {
    /// The unique FunctionId identifying this function
    id: FunctionId,

    /// A function's blocks are always non-empty, consisting of at least an entry
    /// block with `BlockId(0)`
    blocks: VecMap<BlockId, Block>,
}

impl Function {
    fn new(id: FunctionId) -> Function {
        let mut blocks = VecMap::default();
        let entry = blocks.push(Block::new(0));
        assert_eq!(entry, BlockId::ENTRY_BLOCK);
        Function { id, blocks }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BlockId(u32);

impl BlockId {
    const ENTRY_BLOCK: BlockId = BlockId(0);
}

impl From<BlockId> for usize {
    fn from(value: BlockId) -> usize {
        value.0 as usize
    }
}

impl From<usize> for BlockId {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InstructionId(u32);

impl From<InstructionId> for usize {
    fn from(value: InstructionId) -> usize {
        value.0 as usize
    }
}

impl From<usize> for InstructionId {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Value {
    Error,
    Unit,
    Bool(bool),
    Char(char),
    Integer(IntConstant),
    Float(FloatConstant),

    /// Each Instruction defines exactly 1 Value
    InstructionResult(InstructionId),

    /// The Nth parameter of the given block (starting from 0)
    /// If the block is the entry block, these are the function parameters
    Parameter(BlockId, u32),

    /// A function or lambda originally local to the current definition, identified
    /// by its index in the CST traversal order.
    Function(FunctionId),

    /// A global belonging to another top-level item.
    Global(GlobalId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FunctionId {
    item: TopLevelId,
    index: u32,
}

type GlobalId = TopLevelName;

struct Block {
    parameter_count: u32,
    instructions: VecMap<InstructionId, Instruction>,
    terminator: Option<TerminatorInstruction>,
}

impl Block {
    fn new(parameter_count: u32) -> Block {
        Block { parameter_count, instructions: Default::default(), terminator: None }
    }
}

enum Instruction {
    Call { function: Value, arguments: Vec<Value> },
    IndexTuple { tuple: Value, index: u32 },
    MakeString(String),
    MakeTuple(Vec<Value>),
    StackAlloc(Value),
}

enum TerminatorInstruction {
    Jmp(BlockId, /* block arguments */ Vec<Value>),
    If {
        condition: Value,
        then: BlockId,
        else_: BlockId,
    },
    Switch {
        int_value: Value,
        cases: Vec<BlockId>,
        else_: Option<BlockId>,
    },
    #[allow(unused)]
    Unreachable,
    Return(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntConstant {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    /// TODO: This should depend on the target architecture
    Usz(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    /// TODO: This should depend on the target architecture
    Isz(isize),
}

#[derive(Debug, Clone, Copy)]
enum FloatConstant {
    F32(f32),
    F64(f64),
}

impl PartialEq for FloatConstant {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::F32(l0), Self::F32(r0)) => l0.to_bits() == r0.to_bits(),
            (Self::F64(l0), Self::F64(r0)) => l0.to_bits() == r0.to_bits(),
            _ => false,
        }
    }
}

impl Eq for FloatConstant {}
