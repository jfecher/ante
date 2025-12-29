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

use std::{collections::VecDeque, sync::Arc};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    lexer::token::{FloatKind, IntegerKind},
    parser::{
        cst::Name,
        ids::{TopLevelId, TopLevelName},
    },
    type_inference::generics::Generic,
    vecmap::VecMap,
};
pub(crate) mod builder;
mod display;
pub(crate) mod monomorphization;

pub(crate) struct Function {
    pub(crate) name: Name,

    /// The unique FunctionId identifying this function
    id: FunctionId,

    /// A function's blocks are always non-empty, consisting of at least an entry
    /// block with `BlockId(0)`
    pub(crate) blocks: VecMap<BlockId, Block>,

    /// Each instruction in the function, in no particular order.
    /// `Function::blocks` contains the logical order of each instruction. This
    /// field is for storing instruction data itself so instructions may be assigned
    /// unique IDs within a function.
    pub(crate) instructions: VecMap<InstructionId, Instruction>,

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

    pub fn type_of_value(&self, value: Value) -> Type {
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

    /// Returns a topological sort of the blocks in this function.
    /// This ordering is beneficial to consumers as it will ensure
    /// all values are defined before they are used when iterating
    /// blocks in this order.
    ///
    /// Note that in the presense of loops a strict topological ordering is typically undefined.
    /// In this case, loop blocks will be ordered before blocks after the loop.
    pub fn topological_sort(&self) -> Vec<BlockId> {
        let mut stack = vec![BlockId::ENTRY_BLOCK];

        let mut order = Vec::new();
        let mut visited = FxHashSet::<BlockId>::default();
        let mut merge_points = Vec::<(BlockId, Vec<BlockId>)>::new();

        while let Some(block) = stack.pop() {
            // Place `else` branches before branch ends and keep loop bodies before their ends.
            if merge_points.last().map_or(false, |(merge, _)| *merge == block) {
                let remaining_branch_blocks = &merge_points.last().unwrap().1;
                if !remaining_branch_blocks.iter().all(|remaining| visited.contains(remaining)) {
                    continue;
                }
                merge_points.pop();
            }

            if !visited.insert(block) {
                continue;
            }

            order.push(block);

            match &self.blocks[block].terminator {
                Some(TerminatorInstruction::Jmp((target, _))) => {
                    stack.push(*target);
                },
                Some(TerminatorInstruction::If { condition: _, then, else_, end }) => {
                    stack.push(else_.0);
                    stack.push(then.0);
                    if else_.0 != *end {
                        merge_points.push((*end, vec![else_.0]));
                    }
                },
                Some(TerminatorInstruction::Switch { int_value: _, cases, else_, end }) => {
                    let mut blocks = Vec::with_capacity(cases.len() + 1);
                    if let Some(else_) = else_ {
                        stack.push(else_.0);
                        blocks.push(else_.0);
                    }
                    for case in cases.iter().rev() {
                        stack.push(case.0);
                        blocks.push(case.0);
                    }
                    merge_points.push((*end, blocks));
                },
                Some(TerminatorInstruction::Unreachable) => (),
                Some(TerminatorInstruction::Return(_)) => (),
                None => unreachable!("Function::topological_sort: block {block} has no terminator"),
            }
        }

        assert_eq!(merge_points, Vec::new());
        assert_eq!(visited.len(), self.blocks.len());
        order
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(u32);

impl BlockId {
    pub const ENTRY_BLOCK: BlockId = BlockId(0);
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
pub struct InstructionId(u32);

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
pub enum Value {
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

pub struct Block {
    pub parameter_types: Vec<Type>,
    pub instructions: Vec<InstructionId>,
    pub terminator: Option<TerminatorInstruction>,
}

impl Block {
    fn new(parameter_types: Vec<Type>) -> Block {
        Block { parameter_types, instructions: Default::default(), terminator: None }
    }
}

pub enum Instruction {
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

/// A block's arguments is MIR's equivalent of PHI values in other SSA-based IRs.
type BlockArguments = Vec<Value>;

/// A [JmpTarget] is a block to jump to with arguments for that block.
/// A block's arguments is MIR's equivalent of PHI values in other SSA-based IRs.
type JmpTarget = (BlockId, BlockArguments);

pub enum TerminatorInstruction {
    Jmp(JmpTarget),
    If {
        condition: Value,
        then: JmpTarget,
        else_: JmpTarget,
        /// The block expected to merge the then and else paths after the if
        end: BlockId,
    },
    Switch {
        int_value: Value,
        cases: Vec<JmpTarget>,
        else_: Option<JmpTarget>,
        end: BlockId,
    },
    #[allow(unused)]
    Unreachable,
    Return(Value),
}

impl TerminatorInstruction {
    fn jmp(target: BlockId, args: Vec<Value>) -> Self {
        TerminatorInstruction::Jmp((target, args))
    }

    fn jmp_no_args(target: BlockId) -> Self {
        TerminatorInstruction::Jmp((target, Vec::new()))
    }

    fn if_(condition: Value, then: BlockId, else_: BlockId, end: BlockId) -> Self {
        TerminatorInstruction::If { condition, then: (then, Vec::new()), else_: (else_, Vec::new()), end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntConstant {
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
    pub fn kind(self) -> IntegerKind {
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

    /// Bitcast this value to a u64
    pub(crate) fn as_u64(&self) -> u64 {
        match self {
            IntConstant::U8(x) => *x as u64,
            IntConstant::U16(x) => *x as u64,
            IntConstant::U32(x) => *x as u64,
            IntConstant::U64(x) => *x as u64,
            IntConstant::Usz(x) => *x as u64,
            IntConstant::I8(x) => *x as u64,
            IntConstant::I16(x) => *x as u64,
            IntConstant::I32(x) => *x as u64,
            IntConstant::I64(x) => *x as u64,
            IntConstant::Isz(x) => *x as u64,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FloatConstant {
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
pub enum Type {
    Primitive(PrimitiveType),
    Tuple(Arc<Vec<Type>>),
    Function(Arc<FunctionType>),

    /// A C-style union of the given types. Sum types are encoded as this + a tag.
    Union(Arc<Vec<Type>>),

    /// TODO: These should probably be in a simpler form.
    /// E.g. numbered from the function they were declared in.
    Generic(Generic),
}

impl Type {
    pub const ERROR: Type = Type::Primitive(PrimitiveType::Error);
    pub const UNIT: Type = Type::Primitive(PrimitiveType::Unit);
    pub const BOOL: Type = Type::Primitive(PrimitiveType::Bool);
    pub const POINTER: Type = Type::Primitive(PrimitiveType::Pointer);
    pub const CHAR: Type = Type::Primitive(PrimitiveType::Char);

    fn int(kind: IntegerKind) -> Type {
        Type::Primitive(PrimitiveType::Int(kind))
    }

    fn float(kind: FloatKind) -> Type {
        Type::Primitive(PrimitiveType::Float(kind))
    }

    fn string() -> Type {
        Type::Tuple(Arc::new(vec![Type::POINTER, Type::int(IntegerKind::U32)]))
    }

    /// The type of a tagged-union's tag
    fn tag_type() -> Type {
        Type::int(IntegerKind::U8)
    }

    fn tuple(fields: Vec<Type>) -> Type {
        if fields.is_empty() { Type::UNIT } else { Type::Tuple(Arc::new(fields)) }
    }

    fn union(mut variants: Vec<Type>) -> Type {
        if variants.is_empty() {
            Type::UNIT
        } else if variants.len() == 1 {
            variants.pop().unwrap()
        } else {
            Type::Union(Arc::new(variants))
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrimitiveType {
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
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Type,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::mir::{Block, BlockId, Function, FunctionId, TerminatorInstruction, Value};

    /// Create an empty function for testing
    fn make_function() -> Function {
        // Safety: `FunctionId` is POD and this should never be read by `topological_sort` anyway
        let id = unsafe { std::mem::zeroed::<FunctionId>() };
        Function::new(Arc::new(String::new()), id)
    }

    #[test]
    fn topological_sort_if_else() {
        // b0(v0):
        //   if false then b1 else b2 end b3
        // b1():
        //   jmp b3
        // b2():
        //   jmp b3()
        // b3():
        //   return
        let mut function = make_function();

        let b0 = BlockId::ENTRY_BLOCK;
        let b1 = function.blocks.push(Block::new(Vec::new()));
        let b2 = function.blocks.push(Block::new(Vec::new()));
        let b3 = function.blocks.push(Block::new(Vec::new()));

        function.blocks[b0].terminator = Some(TerminatorInstruction::If {
            condition: Value::Bool(false),
            then: (b1, Vec::new()),
            else_: (b2, Vec::new()),
            end: b3,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::jmp(b3, Vec::new()));
        function.blocks[b2].terminator = Some(TerminatorInstruction::jmp(b3, Vec::new()));
        function.blocks[b3].terminator = Some(TerminatorInstruction::Return(Value::Unit));

        let order = function.topological_sort();
        assert_eq!(order, vec![b0, b1, b2, b3]);
    }

    #[test]
    fn topological_sort_if() {
        // b0(v0):
        //   if false then b1 else b2 end b2
        // b1():
        //   jmp b2
        // b2():
        //   return
        let mut function = make_function();

        let b0 = BlockId::ENTRY_BLOCK;
        let b1 = function.blocks.push(Block::new(Vec::new()));
        let b2 = function.blocks.push(Block::new(Vec::new()));

        function.blocks[b0].terminator = Some(TerminatorInstruction::If {
            condition: Value::Bool(false),
            then: (b1, Vec::new()),
            else_: (b2, Vec::new()),
            end: b2,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::jmp(b2, Vec::new()));
        function.blocks[b2].terminator = Some(TerminatorInstruction::Return(Value::Unit));

        let order = function.topological_sort();
        assert_eq!(order, vec![b0, b1, b2]);
    }

    #[test]
    fn topological_sort_nested_if_else() {
        // b0(v0):
        //   if false then b1 else b2 end b3
        // b1():
        //   if false then b4 else b5 end b6
        // b2():
        //   if false then b7 else b8 end b9
        // b3():
        //   return
        // b4():
        //   jmp b6()
        // b5():
        //   jmp b6()
        // b6():
        //   jmp b3()
        // b7():
        //   jmp b9()
        // b8():
        //   jmp b9()
        // b9():
        //   jmp b3()
        let mut function = make_function();

        let b0 = BlockId::ENTRY_BLOCK;
        let b1 = function.blocks.push(Block::new(Vec::new()));
        let b2 = function.blocks.push(Block::new(Vec::new()));
        let b3 = function.blocks.push(Block::new(Vec::new()));
        let b4 = function.blocks.push(Block::new(Vec::new()));
        let b5 = function.blocks.push(Block::new(Vec::new()));
        let b6 = function.blocks.push(Block::new(Vec::new()));
        let b7 = function.blocks.push(Block::new(Vec::new()));
        let b8 = function.blocks.push(Block::new(Vec::new()));
        let b9 = function.blocks.push(Block::new(Vec::new()));

        function.blocks[b0].terminator = Some(TerminatorInstruction::If {
            condition: Value::Bool(false),
            then: (b1, Vec::new()),
            else_: (b2, Vec::new()),
            end: b3,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::If {
            condition: Value::Bool(false),
            then: (b4, Vec::new()),
            else_: (b5, Vec::new()),
            end: b6,
        });

        function.blocks[b2].terminator = Some(TerminatorInstruction::If {
            condition: Value::Bool(false),
            then: (b7, Vec::new()),
            else_: (b8, Vec::new()),
            end: b9,
        });

        function.blocks[b3].terminator = Some(TerminatorInstruction::Return(Value::Unit));
        function.blocks[b4].terminator = Some(TerminatorInstruction::jmp(b6, Vec::new()));
        function.blocks[b5].terminator = Some(TerminatorInstruction::jmp(b6, Vec::new()));
        function.blocks[b6].terminator = Some(TerminatorInstruction::jmp(b3, Vec::new()));
        function.blocks[b7].terminator = Some(TerminatorInstruction::jmp(b9, Vec::new()));
        function.blocks[b8].terminator = Some(TerminatorInstruction::jmp(b9, Vec::new()));
        function.blocks[b9].terminator = Some(TerminatorInstruction::jmp(b3, Vec::new()));

        let order = function.topological_sort();
        assert_eq!(order, vec![b0, b1, b4, b5, b6, b2, b7, b8, b9, b3]);
    }

    #[test]
    fn topological_sort_switch() {
        // b0(v0):
        //   switch ()
        //   | 0 -> b1
        //   | 1 -> b2
        //   | 2 -> b3
        //   | _ -> b4
        //   end b5
        // b1():
        //   jmp b5
        // b2():
        //   jmp b5
        // b3():
        //   jmp b6   // b3 has an extra block
        // b4():
        //   jmp b5
        // b5():
        //   return
        // b6():
        //   jmp b5
        let mut function = make_function();

        let b0 = BlockId::ENTRY_BLOCK;
        let b1 = function.blocks.push(Block::new(Vec::new()));
        let b2 = function.blocks.push(Block::new(Vec::new()));
        let b3 = function.blocks.push(Block::new(Vec::new()));
        let b4 = function.blocks.push(Block::new(Vec::new()));
        let b5 = function.blocks.push(Block::new(Vec::new()));
        let b6 = function.blocks.push(Block::new(Vec::new()));

        function.blocks[b0].terminator = Some(TerminatorInstruction::Switch {
            int_value: Value::Unit,
            cases: vec![(b1, Vec::new()), (b2, Vec::new()), (b3, Vec::new())],
            else_: Some((b4, Vec::new())),
            end: b5,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::jmp(b5, Vec::new()));
        function.blocks[b2].terminator = Some(TerminatorInstruction::jmp(b5, Vec::new()));
        function.blocks[b3].terminator = Some(TerminatorInstruction::jmp(b6, Vec::new()));
        function.blocks[b4].terminator = Some(TerminatorInstruction::jmp(b5, Vec::new()));
        function.blocks[b5].terminator = Some(TerminatorInstruction::Return(Value::Unit));
        function.blocks[b6].terminator = Some(TerminatorInstruction::jmp(b5, Vec::new()));

        let order = function.topological_sort();
        assert_eq!(order, vec![b0, b1, b2, b3, b6, b4, b5]);
    }
}
