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

use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::{
    lexer::token::{FloatKind, IntegerKind}, parser::{cst::Name, ids::{TopLevelId, TopLevelName}}, type_inference::generics::Generic, vecmap::VecMap
};
pub(crate) mod builder;
mod display;
pub(crate) mod monomorphization;

pub(crate) struct Function {
    name: Name,

    /// The unique FunctionId identifying this function
    id: FunctionId,

    /// A function's blocks are always non-empty, consisting of at least an entry
    /// block with `BlockId(0)`
    blocks: VecMap<BlockId, Block>,

    /// Each instruction in the function, in no particular order.
    /// `Function::blocks` contains the logical order of each instruction. This
    /// field is for storing instruction data itself so instructions may be assigned
    /// unique IDs within a function.
    instructions: VecMap<InstructionId, Instruction>,

    /// The result type of each instruction in this function
    instruction_result_types: VecMap<InstructionId, Type>,

    global_types: FxHashMap<GlobalId, Type>,
    function_types: FxHashMap<FunctionId, Type>,
}

impl Function {
    fn new(name: Name, id: FunctionId) -> Function {
        let mut blocks = VecMap::default();
        let entry = blocks.push(Block::new(Vec::new()));
        assert_eq!(entry, BlockId::ENTRY_BLOCK);
        Function {
            name,
            id,
            blocks,
            instructions: VecMap::default(),
            instruction_result_types: VecMap::default(),
            global_types: Default::default(),
            function_types: Default::default(),
        }
    }

    fn type_of_value(&self, value: Value) -> Type {
        match value {
            Value::Error => Type::ERROR,
            Value::Unit => Type::UNIT,
            Value::Bool(_) => Type::BOOL,
            Value::Char(_) => Type::CHAR,
            Value::Integer(constant) => Type::int(constant.kind()),
            Value::Float(constant) => Type::float(constant.kind()),
            Value::InstructionResult(instruction_id) => self.instruction_result_types[instruction_id].clone(),
            Value::Parameter(block_id, parameter_index) => {
                self.blocks[block_id].parameter_types[parameter_index as usize].clone()
            },
            Value::Function(function_id) => self.function_types[&function_id].clone(),
            Value::Global(global_id) => self.global_types[&global_id].clone(),
        }
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

/// A function or lambda originally located within [Self::item], identified
/// by its index in the CST traversal order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FunctionId {
    item: TopLevelId,
    index: u32,
}

type GlobalId = TopLevelName;

struct Block {
    parameter_types: Vec<Type>,
    instructions: Vec<InstructionId>,
    terminator: Option<TerminatorInstruction>,
}

impl Block {
    fn new(parameter_types: Vec<Type>) -> Block {
        Block { parameter_types, instructions: Default::default(), terminator: None }
    }
}

enum Instruction {
    Call {
        function: Value,
        arguments: Vec<Value>,
    },
    IndexTuple {
        tuple: Value,
        index: u32,
    },
    MakeString(String),
    MakeTuple(Vec<Value>),
    StackAlloc(Value),

    /// Reinterpret one value as another type.
    /// The destination type is given by the type of the resulting value.
    /// Requires the destination type's size to be less than or equal to the original type's size.
    Transmute(Value),
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

impl IntConstant {
    fn kind(self) -> IntegerKind {
        match self {
            IntConstant::U8(_) => IntegerKind::U8,
            IntConstant::U16(_) => IntegerKind::U16,
            IntConstant::U32(_) => IntegerKind::U32,
            IntConstant::U64(_) => IntegerKind::U64,
            IntConstant::Usz(_) => IntegerKind::Usz,
            IntConstant::I8(_) => IntegerKind::I8,
            IntConstant::I16(_) => IntegerKind::I16,
            IntConstant::I32(_) => IntegerKind::I32,
            IntConstant::I64(_) => IntegerKind::I64,
            IntConstant::Isz(_) => IntegerKind::Isz,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum FloatConstant {
    F32(f32),
    F64(f64),
}

impl FloatConstant {
    fn kind(self) -> FloatKind {
        match self {
            FloatConstant::F32(_) => FloatKind::F32,
            FloatConstant::F64(_) => FloatKind::F64,
        }
    }
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

/// TODO: This is very similar to [TopLevelType] - do we really need both?
#[derive(Debug, Clone, PartialEq, Eq)]
enum Type {
    Primitive(PrimitiveType),
    Tuple(Arc<Vec<Type>>),
    Function(Arc<FunctionType>),

    /// TODO: These should probably be in a simpler form.
    /// E.g. numbered from the function they were declared in.
    Generic(Generic),
}

impl Type {
    const ERROR: Type = Type::Primitive(PrimitiveType::Error);
    const UNIT: Type = Type::Primitive(PrimitiveType::Unit);
    const BOOL: Type = Type::Primitive(PrimitiveType::Bool);
    const POINTER: Type = Type::Primitive(PrimitiveType::Pointer);
    const CHAR: Type = Type::Primitive(PrimitiveType::Char);

    fn int(kind: IntegerKind) -> Type {
        Type::Primitive(PrimitiveType::Int(kind))
    }

    fn float(kind: FloatKind) -> Type {
        Type::Primitive(PrimitiveType::Float(kind))
    }

    fn string() -> Type {
        Type::Tuple(Arc::new(vec![Type::POINTER, Type::int(IntegerKind::U32)]))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PrimitiveType {
    Error,
    Unit,
    Bool,
    /// An opaque pointer type
    Pointer,
    Char,
    Int(IntegerKind),
    Float(FloatKind),
}

#[derive(Debug, PartialEq, Eq)]
struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
}
