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

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    iterator_extensions::mapvec,
    lexer::token::{F64, FloatKind, IntegerKind},
    parser::cst::Name,
    vecmap::VecMap,
};

pub(crate) mod builder;
mod display;
mod effects;
mod lower_closures;
pub(crate) mod monomorphization;
mod remove_unreachable;
mod validation;

#[derive(Default)]
pub struct Mir {
    pub(crate) definitions: Definitions,

    /// Any extern symbols used but not defined in this [Mir]
    pub(crate) externals: FxHashMap<DefinitionId, Extern>,

    /// Effect op → position-within-effect mappings recovered from Handles removed by
    /// [crate::mir::effects::tail_resume_optimization], so [crate::mir::effects::effect_lowering]
    /// can still resolve `Perform`s that lived inside an optimized Handle's body.
    pub(crate) preserved_op_indices: FxHashMap<DefinitionId, u32>,
}

#[derive(Debug)]
pub struct Extern {
    pub(crate) name: Name,
    pub(crate) typ: Type,
}

pub(crate) type Definitions = FxHashMap<DefinitionId, Definition>;

impl Mir {
    pub fn extend(mut self, other: Mir) -> Mir {
        self.definitions.extend(other.definitions);
        self.externals.extend(other.externals);
        self.preserved_op_indices.extend(other.preserved_op_indices);
        self
    }

    fn get(&self, id: DefinitionId) -> Option<&Definition> {
        self.definitions.get(&id)
    }

    pub fn get_name(&self, id: DefinitionId) -> Option<&Name> {
        self.get(id).map(|def| &def.name).or_else(|| self.externals.get(&id).map(|ext| &ext.name))
    }

    /// Returns the type of the given value given that the value originates in the given [Definition].
    pub fn type_of_value(&self, value: &Value, definition: &Definition) -> Type {
        definition
            .try_type_of_value(value, &self.externals, &self.definitions)
            .unwrap_or_else(|| panic!("\n{self}\n\nNo type for {value} in definition {}", definition.id))
    }

    /// Remove any entries in `self.externals` which are also present in `self.definitions`
    pub fn remove_internal_externs(mut self) -> Self {
        self.externals.retain(|k, _| !self.definitions.contains_key(k));
        self
    }
}

impl std::ops::Index<DefinitionId> for Mir {
    type Output = Definition;

    fn index(&self, index: DefinitionId) -> &Self::Output {
        &self.definitions[&index]
    }
}

/// A Definition may be a function or global. Globals are represented
/// as single blocks with no parameters to account for them needing to
/// construct tuples which are instructions in this IR. Additionally, the
/// terminator of a global's block is always `Result`.
#[derive(Clone)]
pub struct Definition {
    pub(crate) name: Name,

    /// The unique DefinitionId identifying this function
    pub(crate) id: DefinitionId,

    /// The number of generic type arguments of this definition
    pub(crate) generic_count: u32,

    /// The type of this Definition.
    pub(crate) typ: Type,

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
}

impl Definition {
    fn new(name: Name, id: DefinitionId, generic_count: u32, typ: Type) -> Definition {
        let mut blocks = VecMap::default();
        let entry = blocks.push(Block::new(Vec::new()));
        assert_eq!(entry, BlockId::ENTRY_BLOCK);

        Definition {
            name,
            id,
            blocks,
            typ,
            generic_count,
            instructions: VecMap::default(),
            instruction_result_types: VecMap::default(),
        }
    }

    /// True if this [Definition] is a global value rather than a function.
    /// Note that this includes globals whose types are functions.
    pub fn is_global(&self) -> bool {
        self.blocks.len() == 1 && matches!(&self.entry_block().terminator, Some(TerminatorInstruction::Result(_)))
    }

    /// Clone this definition but with a new id
    fn clone_with_id(&self, new_id: DefinitionId) -> Self {
        let mut clone = self.clone();
        clone.id = new_id;
        clone
    }

    /// Invoke `f` once for each [DefinitionId] this definition references
    pub fn for_each_referenced_definition(&self, mut f: impl FnMut(DefinitionId)) {
        for instruction in self.instructions.values() {
            instruction.for_each_value(|value| {
                if let Value::Definition(id) = value {
                    f(*id);
                }
            });
            match instruction {
                Instruction::Instantiate(id, _) => f(*id),
                Instruction::Perform { effect_op, .. } => f(*effect_op),
                Instruction::Handle { cases, .. } => {
                    for case in cases {
                        f(case.effect_op);
                    }
                },
                _ => (),
            }
        }
        for block in self.blocks.values() {
            if let Some(terminator) = &block.terminator {
                terminator.for_each_value(|value| {
                    if let Value::Definition(id) = value {
                        f(*id);
                    }
                });
            }
        }
    }

    pub fn entry_block(&self) -> &Block {
        &self.blocks[BlockId::ENTRY_BLOCK]
    }

    pub fn parameters(&self) -> impl Iterator<Item = (Value, &Type)> {
        let entry = self.entry_block();
        entry.parameter_types.iter().enumerate().map(|(i, typ)| (Value::Parameter(BlockId::ENTRY_BLOCK, i as u32), typ))
    }

    pub fn instruction_result_type(&self, id: InstructionId) -> &Type {
        &self.instruction_result_types[id]
    }

    pub fn type_of_value(
        &self, value: &Value, externals: &FxHashMap<DefinitionId, Extern>,
        definitions: &FxHashMap<DefinitionId, Definition>,
    ) -> Type {
        self.try_type_of_value(value, externals, definitions)
            .unwrap_or_else(|| panic!("\n{}\nexterns = {externals:?}\n\nNo type for {value}", self.display(None)))
    }

    fn try_type_of_value(
        &self, value: &Value, externals: &FxHashMap<DefinitionId, Extern>,
        definitions: &FxHashMap<DefinitionId, Definition>,
    ) -> Option<Type> {
        Some(match value {
            Value::Error => Type::ERROR,
            Value::Unit => Type::UNIT,
            Value::Bool(_) => Type::BOOL,
            Value::Char(_) => Type::CHAR,
            Value::Integer(constant) => Type::int(constant.kind()),
            Value::Float(constant) => Type::float(constant.kind()),
            Value::InstructionResult(instruction_id) => self.instruction_result_types[*instruction_id].clone(),
            Value::Parameter(block_id, parameter_index) => {
                // Return Error for out-of-bounds parameters. This can occur when closure
                // conversion has not yet been implemented and a lambda body references a captured
                // outer parameter that was not declared as a block parameter.
                self.blocks
                    .get(*block_id)
                    .and_then(|b| b.parameter_types.get(*parameter_index as usize))
                    .cloned()
                    .unwrap_or(Type::ERROR)
            },
            Value::Definition(definition_id) => {
                if let Some(definition) = definitions.get(definition_id) {
                    definition.typ.clone()
                } else if let Some(external) = externals.get(definition_id) {
                    external.typ.clone()
                } else if *definition_id == self.id {
                    self.typ.clone()
                } else {
                    return None;
                }
            },
        })
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
            if merge_points.last().is_some_and(|(merge, _)| *merge == block) {
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
                    stack.push(*end);
                    stack.push(else_.0);
                    stack.push(then.0);
                    if else_.0 != *end {
                        merge_points.push((*end, vec![else_.0]));
                    }
                },
                Some(TerminatorInstruction::Switch { int_value: _, cases, else_, end }) => {
                    let mut blocks = Vec::with_capacity(cases.len() + 1);
                    stack.push(*end);
                    stack.push(else_.0);
                    blocks.push(else_.0);
                    for (_, case) in cases.iter().rev() {
                        stack.push(case.0);
                        blocks.push(case.0);
                    }
                    merge_points.push((*end, blocks));
                },
                Some(TerminatorInstruction::Unreachable) => (),
                Some(TerminatorInstruction::Return(_)) => (),
                Some(TerminatorInstruction::Result(_)) => (),
                None => unreachable!("Function::topological_sort: block {block} has no terminator"),
            }
        }

        assert_eq!(merge_points, Vec::new());
        assert_eq!(visited.len(), self.blocks.len());
        order
    }

    /// True if this function is not generic over any type variables
    fn is_monomorphic(&self) -> bool {
        self.generic_count == 0
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    /// A global, often a function
    Definition(DefinitionId),
}

impl Value {
    /// Returns a value representing a union's tag.
    /// This should always be of type [Type::tag_type()]
    pub fn tag_value(value: u8) -> Value {
        Value::Integer(IntConstant::U8(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefinitionId(pub u32);

/// DefinitionIds are assigned in monotonically increasing order. These IDs are nondeterministic in
/// practice due to this counter being used concurrently. As a result, anything using these ids
/// should not be used as the input or result of an incremental computation.
static NEXT_DEFINITION_ID: AtomicU32 = AtomicU32::new(0);

fn next_definition_id() -> DefinitionId {
    // Relaxed ordering since we only care the resulting id is unique
    DefinitionId(NEXT_DEFINITION_ID.fetch_add(1, Ordering::Relaxed))
}

/// A basic block with linear control-flow until the terminator instruction which may branch.
#[derive(Clone)]
pub struct Block {
    pub parameter_types: Vec<Type>,
    pub instructions: Vec<InstructionId>,
    pub terminator: Option<TerminatorInstruction>,
}

impl Block {
    fn new(parameter_types: Vec<Type>) -> Block {
        Block { parameter_types, instructions: Default::default(), terminator: None }
    }

    pub fn parameters(&self, block_id: BlockId) -> impl ExactSizeIterator<Item = (Value, Type)> {
        self.parameter_types.iter().enumerate().map(move |(i, typ)| (Value::Parameter(block_id, i as u32), typ.clone()))
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Call {
        function: Value,
        arguments: Vec<Value>,
    },
    /// Similar to `Call` but expects a closure tuple of `(function, environment)`
    /// instead of a raw function value. The environment should not be added to the
    /// arguments but the function referred to should have an environment parameter.
    ///
    /// These instructions should be removed before codegen.
    CallClosure {
        closure: Value,
        arguments: Vec<Value>,
    },
    /// Transfers control to the handler for `effect_op` somewhere up the call stack.
    ///
    /// This instruction will later be removed by either the tail resume optimization pass
    /// (if its handler uses resume in a tail position) or the effect lowering pass otherwise.
    ///
    /// Currently unconstructed: ability-method calls are lowered directly to
    /// `IndexTuple cap op_index + CallClosure` at MIR build time, since the cap value
    /// (whether an ability impl or a handler-installed wrapper tuple) carries the operation
    /// closures. Kept here so the optimization passes' lowering for legacy Perform-shaped
    /// MIR continues to compile; can be removed when those passes are revisited.
    #[allow(dead_code)]
    Perform {
        effect_op: DefinitionId,
        arguments: Vec<Value>,
    },
    /// Run `body: fn () [env] -> r`. Any `Perform` instructions to an `effect_op` in this
    /// handle's cases will transfer control to that case.
    ///
    /// Each case contains a closure of type `fn (op_args...) (fn r -> r2) [env] -> r2` where the
    /// last function argument is the `resume` or continuation function.
    Handle {
        body: Value,
        cases: Vec<HandlerCase>,
    },
    /// Placeholder bound to the handler binding `h` inside a `handle` body lambda. Emitted by
    /// the MIR builder so the body can reference `h` without committing to a particular lowering
    /// strategy. The body's lowering pass is responsible for replacing each `Capability` with the
    /// appropriate value: [crate::mir::effect_lowering] expands it into a coroutine `user_data`
    /// fetch, and [crate::mir::tail_resume_optimization] rewrites it to `Id(cap)` where `cap` is
    /// a directly-built capability tuple. Must be removed before LLVM codegen.
    Capability,
    /// Returns a closure value after packing the function with the given environment.
    /// This is equivalent to a `MakeTuple` instruction but is distinguished because the
    /// compiler will optimize closure values & calls into free functions, removing the
    /// environment after monomorphization.
    PackClosure {
        function: Value,
        environment: Value,
    },
    IndexTuple {
        tuple: Value,
        index: u32,
    },
    /// Embed an immutable byte array into the binary and return a [`Type::POINTER`] to it.
    /// Used for string literals.
    MakeBytes(Vec<u8>),
    MakeTuple(Vec<Value>),
    MakeArray(Vec<Value>),

    // TODO: Should we remove this in favor of StackAllocUninit + Store?
    StackAlloc(Value),
    StackAllocUninit(Type),

    /// Heap-allocate and store a shared value, returning its pointer.
    AllocShared(Value),

    /// Store a value into a pointer location. Returns unit.
    Store {
        pointer: Value,
        value: Value,
    },

    /// Get a pointer to a field at `index` within the struct pointed to by `struct_ptr`.
    /// `struct_type` is the type of the struct being pointed to (needed for GEP codegen).
    /// Returns a `Pointer` to the field.
    GetFieldPtr {
        struct_ptr: Value,
        struct_type: Type,
        index: u32,
    },

    /// Reinterpret one value as another type.
    /// The destination type is given by the type of the resulting value.
    /// Requires the destination type's size to be less than or equal to the original type's size.
    Transmute(Value),

    /// Instantiate a polymorphic value with concrete types. The actual bindings are not given
    /// but can be found from the result type.
    Instantiate(DefinitionId, Arc<GenericBindings>),

    /// Returns the given value as-is. Used by monomorphization to replace `Instantiate` instructions.
    Id(Value),

    /// Returns the value of the given external symbol. Its type is given by the result type of
    /// this instruction. Note that "extern" here refers to an external definition such as the C
    /// function `puts` rather than a definition which is just excluded from the current Mir
    /// compilation unit.
    Extern(String),

    AddInt(Value, Value),
    AddFloat(Value, Value),
    SubInt(Value, Value),
    SubFloat(Value, Value),
    MulInt(Value, Value),
    MulFloat(Value, Value),
    DivSigned(Value, Value),
    DivUnsigned(Value, Value),
    DivFloat(Value, Value),
    ModSigned(Value, Value),
    ModUnsigned(Value, Value),
    ModFloat(Value, Value),
    LessSigned(Value, Value),
    LessUnsigned(Value, Value),
    LessFloat(Value, Value),
    EqInt(Value, Value),
    EqFloat(Value, Value),

    BitwiseAnd(Value, Value),
    BitwiseOr(Value, Value),
    BitwiseXor(Value, Value),
    BitwiseNot(Value),

    SignExtend(Value),
    ZeroExtend(Value),
    SignedToFloat(Value),
    UnsignedToFloat(Value),
    FloatToSigned(Value),
    FloatToUnsigned(Value),
    FloatPromote(Value),
    FloatDemote(Value),

    Truncate(Value),
    Deref(Value),
    /// Static byte size of a type. Resolved to a `Usz` constant during monomorphization.
    SizeOf(Type),
    /// Static length of an array. Resolved to a `Usz` constant during monomorphization.
    ArrayLen(Type),
}

/// A handler case attached to an [Instruction::Handle]. Maps an effect operation
/// (identified by its top-level [DefinitionId]) to the function that handles it.
#[derive(Debug, Clone)]
pub struct HandlerCase {
    pub effect_op: DefinitionId,
    pub handler: Value,
}

impl Instruction {
    pub fn for_each_value(&self, mut f: impl FnMut(&Value)) {
        let mut two = |a, b| {
            f(a);
            f(b);
        };
        match self {
            Instruction::Call { function, arguments } => {
                f(function);
                arguments.iter().for_each(f);
            },
            Instruction::CallClosure { closure: function, arguments } => {
                f(function);
                arguments.iter().for_each(f);
            },
            Instruction::Perform { effect_op: _, arguments } => arguments.iter().for_each(f),
            Instruction::Handle { body, cases } => {
                f(body);
                for case in cases {
                    f(&case.handler);
                }
            },
            Instruction::Capability => (),
            Instruction::PackClosure { function, environment } => two(function, environment),
            Instruction::IndexTuple { tuple, index: _ } => f(tuple),
            Instruction::MakeBytes(_) => (),
            Instruction::MakeTuple(elements) => elements.iter().for_each(f),
            Instruction::MakeArray(elements) => elements.iter().for_each(f),
            Instruction::StackAlloc(value) => f(value),
            Instruction::StackAllocUninit(_) => (),
            Instruction::AllocShared(value) => f(value),
            Instruction::Store { pointer, value } => two(pointer, value),
            Instruction::Transmute(value) => f(value),
            Instruction::Instantiate(_, _) => (),
            Instruction::Id(value) => f(value),
            Instruction::AddInt(a, b) => two(a, b),
            Instruction::AddFloat(a, b) => two(a, b),
            Instruction::SubInt(a, b) => two(a, b),
            Instruction::SubFloat(a, b) => two(a, b),
            Instruction::MulInt(a, b) => two(a, b),
            Instruction::MulFloat(a, b) => two(a, b),
            Instruction::DivSigned(a, b) => two(a, b),
            Instruction::DivUnsigned(a, b) => two(a, b),
            Instruction::DivFloat(a, b) => two(a, b),
            Instruction::ModSigned(a, b) => two(a, b),
            Instruction::ModUnsigned(a, b) => two(a, b),
            Instruction::ModFloat(a, b) => two(a, b),
            Instruction::LessSigned(a, b) => two(a, b),
            Instruction::LessUnsigned(a, b) => two(a, b),
            Instruction::LessFloat(a, b) => two(a, b),
            Instruction::EqInt(a, b) => two(a, b),
            Instruction::EqFloat(a, b) => two(a, b),
            Instruction::BitwiseAnd(a, b) => two(a, b),
            Instruction::BitwiseOr(a, b) => two(a, b),
            Instruction::BitwiseXor(a, b) => two(a, b),
            Instruction::BitwiseNot(value) => f(value),
            Instruction::SignExtend(value) => f(value),
            Instruction::ZeroExtend(value) => f(value),
            Instruction::SignedToFloat(value) => f(value),
            Instruction::UnsignedToFloat(value) => f(value),
            Instruction::FloatToSigned(value) => f(value),
            Instruction::FloatToUnsigned(value) => f(value),
            Instruction::FloatPromote(value) => f(value),
            Instruction::FloatDemote(value) => f(value),
            Instruction::Truncate(value) => f(value),
            Instruction::Deref(value) => f(value),
            Instruction::SizeOf(_typ) => (),
            Instruction::ArrayLen(_typ) => (),
            Instruction::GetFieldPtr { struct_ptr, .. } => f(struct_ptr),
            Instruction::Extern(_) => (),
        }
    }
}

/// A [JmpTarget] is a block to jump to with arguments for that block.
/// A block's arguments is MIR's equivalent of PHI values in other SSA-based IRs.
type JmpTarget = (BlockId, Option<Value>);

#[derive(Clone)]
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
        cases: Vec<(/*tag_to_match*/ u32, JmpTarget)>,
        else_: JmpTarget,
        end: BlockId,
    },
    #[allow(unused)]
    Unreachable,
    Return(Value),

    /// Similar to `Return` but for non-function globals. Such globals do not correspond to an
    /// actual return instruction. Instead, they result in a value that is put into storage.
    Result(Value),
}

impl TerminatorInstruction {
    fn jmp(target: BlockId, arg: Value) -> Self {
        TerminatorInstruction::Jmp((target, Some(arg)))
    }

    fn jmp_no_args(target: BlockId) -> Self {
        TerminatorInstruction::Jmp((target, None))
    }

    fn if_(condition: Value, then: BlockId, else_: BlockId, end: BlockId) -> Self {
        TerminatorInstruction::If { condition, then: (then, None), else_: (else_, None), end }
    }

    pub fn for_each_value(&self, mut f: impl FnMut(&Value)) {
        match self {
            TerminatorInstruction::Jmp((_, Some(value))) => f(value),
            TerminatorInstruction::Jmp((_, None)) => (),
            TerminatorInstruction::If { condition, then, else_, end: _ } => {
                f(condition);
                if let Some(then) = &then.1 {
                    f(then);
                }
                if let Some(else_) = &else_.1 {
                    f(else_);
                }
            },
            TerminatorInstruction::Switch { int_value, cases, else_, end: _ } => {
                f(int_value);
                for (_, (_, case_value)) in cases {
                    if let Some(value) = case_value {
                        f(value);
                    }
                }
                if let Some(else_value) = &else_.1 {
                    f(else_value);
                }
            },
            TerminatorInstruction::Unreachable => (),
            TerminatorInstruction::Return(value) => f(value),
            TerminatorInstruction::Result(value) => f(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntConstant {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    Usz(usize),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
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
    #[cfg(feature = "llvm")]
    pub(crate) fn as_u64(&self) -> u64 {
        match self {
            IntConstant::U8(x) => *x as u64,
            IntConstant::U16(x) => *x as u64,
            IntConstant::U32(x) => *x as u64,
            IntConstant::U64(x) => *x,
            IntConstant::Usz(x) => *x as u64,
            IntConstant::I8(x) => *x as u64,
            IntConstant::I16(x) => *x as u64,
            IntConstant::I32(x) => *x as u64,
            IntConstant::I64(x) => *x as u64,
            IntConstant::Isz(x) => *x as u64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatConstant {
    F32(F64),
    F64(F64),
}

impl FloatConstant {
    fn kind(self) -> FloatKind {
        match self {
            FloatConstant::F32(_) => FloatKind::F32,
            FloatConstant::F64(_) => FloatKind::F64,
        }
    }
}

/// TODO: This is very similar to [crate::type_inference::types::Type] - do we really need both?
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Tuple(Arc<Vec<Type>>),
    Function(Arc<FunctionType>),

    /// A C-style union of the given types. Sum types are encoded as this + a tag.
    Union(Arc<Vec<Type>>),

    Array {
        length: Arc<Type>,
        element: Arc<Type>,
    },

    /// A type-level U32
    U32(u32),

    Generic(Generic),
}

impl Type {
    pub const ERROR: Type = Type::Primitive(PrimitiveType::Error);
    pub const UNIT: Type = Type::Primitive(PrimitiveType::Unit);
    pub const BOOL: Type = Type::Primitive(PrimitiveType::Bool);
    pub const POINTER: Type = Type::Primitive(PrimitiveType::Pointer);
    pub const CHAR: Type = Type::Primitive(PrimitiveType::Char);
    pub const NO_CLOSURE_ENV: Type = Type::Primitive(PrimitiveType::NoClosureEnv);

    pub fn int(kind: IntegerKind) -> Type {
        Type::Primitive(PrimitiveType::Int(kind))
    }

    pub fn float(kind: FloatKind) -> Type {
        Type::Primitive(PrimitiveType::Float(kind))
    }

    /// The type of a string: this needs to be updated whenever the repr of a string is changed in
    /// the prelude.
    pub fn string() -> Type {
        Type::Tuple(Arc::new(vec![
            Type::POINTER,
            Type::POINTER,
            Type::int(IntegerKind::U32),
            Type::int(IntegerKind::U32),
        ]))
    }

    pub fn generic(index: u32) -> Type {
        Type::Generic(Generic(index))
    }

    /// The type of a tagged-union's tag
    pub fn tag_type() -> Type {
        Type::int(IntegerKind::U8)
    }

    pub fn tuple(fields: Vec<Type>) -> Type {
        Type::Tuple(Arc::new(fields))
    }

    pub fn union(variants: Vec<Type>) -> Type {
        Type::Union(Arc::new(variants))
    }

    pub fn array_with_length(length: Type, element: Type) -> Type {
        Type::Array { length: Arc::new(length), element: Arc::new(element) }
    }

    fn function_return_type(&self) -> Option<&Type> {
        match self {
            Type::Function(function) => Some(&function.return_type),
            _ => None,
        }
    }

    /// If this is a tagged-union type in the form `(tag, {union})`, return `{union}`
    /// otherwise return None.
    fn without_union_tag(&self) -> Option<Self> {
        match self {
            Type::Tuple(fields) if fields.len() == 2 => fields.get(1).cloned(),
            _ => None,
        }
    }

    /// Substitute in the given generic arguments. Replacing each `Generic(i)` with `generic_args[i]`.
    fn substitute(&self, generic_args: &Vec<Type>) -> Type {
        match self {
            Type::Primitive(primitive) => Type::Primitive(*primitive),
            Type::Generic(generic) if (generic.0 as usize) < generic_args.len() => {
                generic_args[generic.0 as usize].clone()
            },
            Type::Generic(generic) => Type::Generic(*generic),
            Type::U32(n) => Type::U32(*n),
            Type::Tuple(elements) => Type::Tuple(Arc::new(mapvec(elements.iter(), |typ| typ.substitute(generic_args)))),
            Type::Function(function_type) => {
                let parameters = mapvec(&function_type.parameters, |typ| typ.substitute(generic_args));
                let environment = function_type.environment.substitute(generic_args);
                let return_type = function_type.return_type.substitute(generic_args);
                Type::Function(Arc::new(FunctionType { parameters, environment, return_type }))
            },
            Type::Union(variants) => Type::Union(Arc::new(mapvec(variants.iter(), |typ| typ.substitute(generic_args)))),
            Type::Array { length, element } => Type::Array {
                length: Arc::new(length.substitute(generic_args)),
                element: Arc::new(element.substitute(generic_args)),
            },
        }
    }

    /// Retrieves the size in bytes of this type as it sits in memory, including any
    /// trailing alignment padding (the equivalent of LLVM's `getTypeAllocSize`).
    /// Result depends on the target machine.
    ///
    /// Panics if there is a generic within this type.
    fn size_in_bytes(&self, ptr_size: u32) -> u32 {
        match self {
            Type::Primitive(primitive) => primitive.size_in_bytes(ptr_size),
            Type::Tuple(fields) => {
                // The tuple's total size is rounded up to the tuple's own alignment
                let mut offset: u32 = 0;
                let mut max_align: u32 = 1;
                for field in fields.iter() {
                    let a = field.align_in_bytes(ptr_size).max(1);
                    let s = field.size_in_bytes(ptr_size);
                    offset = (offset + a - 1) & !(a - 1);
                    offset += s;
                    if a > max_align {
                        max_align = a;
                    }
                }
                (offset + max_align - 1) & !(max_align - 1)
            },
            Type::Function(_) => ptr_size,
            // This is a raw union so the tag isn't counted here
            Type::Union(variants) => variants.iter().map(|typ| typ.size_in_bytes(ptr_size)).max().unwrap_or(0),
            Type::Array { length, element } => {
                let elem_size = element.size_in_bytes(ptr_size);
                let elem_align = element.align_in_bytes(ptr_size).max(1);
                let stride = (elem_size + elem_align - 1) & !(elem_align - 1);
                let length = match length.as_ref() {
                    Type::U32(n) => *n,
                    other => panic!("size_in_bytes called on Array with non-constant length: {other}"),
                };
                stride * length
            },
            Type::U32(_) => 0,
            Type::Generic(_) => panic!("size_in_bytes called on Type::Generic"),
        }
    }

    /// Natural alignment of this type in bytes (LLVM's `getABITypeAlignment`).
    /// For tuples this is the maximum alignment of any field. Panics on generics.
    fn align_in_bytes(&self, ptr_size: u32) -> u32 {
        match self {
            Type::Primitive(primitive) => primitive.size_in_bytes(ptr_size).max(1),
            Type::Tuple(fields) => fields.iter().map(|f| f.align_in_bytes(ptr_size)).max().unwrap_or(1),
            Type::Function(_) => ptr_size,
            Type::Union(variants) => variants.iter().map(|v| v.align_in_bytes(ptr_size)).max().unwrap_or(1),
            Type::Array { length: _, element } => element.align_in_bytes(ptr_size),
            Type::U32(_) => 1,
            Type::Generic(_) => panic!("align_in_bytes called on Type::Generic"),
        }
    }

    /// True if the underlying representation of this type can be treated as an integer,
    /// often for casting intrinsics.
    fn can_be_used_as_integer(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Int(_) | PrimitiveType::Bool | PrimitiveType::Char))
    }

    fn is_int(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Int(_)))
    }

    fn is_float(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Float(_)))
    }

    fn is_unsigned_int(&self) -> bool {
        match self {
            Type::Primitive(PrimitiveType::Int(kind)) => !kind.is_signed(),
            _ => false,
        }
    }

    fn is_signed_int(&self) -> bool {
        match self {
            Type::Primitive(PrimitiveType::Int(kind)) => kind.is_signed(),
            _ => false,
        }
    }

    fn is_closure(&self) -> bool {
        match self {
            Type::Function(function) => function.is_closure(),
            _ => false,
        }
    }
}

/// Generics are represented as their index into their function's generic_count
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Generic(u32);

/// Each nth item in the bindings Vec corresponds to the nth generic of a definition.
pub(crate) type GenericBindings = Vec<Type>;

/// `[Generic(0), Generic(1), ..., Generic(N-1)]`. Used inside generated
/// definitions whose `generic_count == N` to forward identity bindings to
/// other generated defs that share the same generics.
pub(crate) fn identity_bindings(generic_count: u32) -> Option<Arc<GenericBindings>> {
    let bindings = mapvec(0..generic_count, Type::generic);
    (!bindings.is_empty()).then(|| Arc::new(bindings))
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PrimitiveType {
    Error,
    Unit,
    Bool,
    /// An opaque pointer type
    Pointer,
    Char,
    Int(IntegerKind),
    Float(FloatKind),
    NoClosureEnv,
}

impl PrimitiveType {
    fn size_in_bytes(self, ptr_size: u32) -> u32 {
        match self {
            PrimitiveType::Error | PrimitiveType::NoClosureEnv | PrimitiveType::Unit => 0,
            PrimitiveType::Bool => 1,
            PrimitiveType::Pointer => ptr_size,
            PrimitiveType::Char => 1,
            PrimitiveType::Int(kind) => kind.size_in_bytes(ptr_size),
            PrimitiveType::Float(kind) => kind.size_in_bytes(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub parameters: Vec<Type>,

    /// If this is anything other than Type::NO_CLOSURE_ENV, then this function is a closure
    pub environment: Type,
    pub return_type: Type,
}

impl FunctionType {
    pub fn is_closure(&self) -> bool {
        !matches!(self.environment, Type::NO_CLOSURE_ENV)
    }

    /// Returns `Some(env)` if the environment is not `Type::NO_CLOSURE_ENV`,
    /// otherwise returns `None`
    pub fn environment(&self) -> Option<&Type> {
        match &self.environment {
            Type::Primitive(PrimitiveType::NoClosureEnv) => None,
            other => Some(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::mir::{Block, BlockId, Definition, TerminatorInstruction, Type, Value, next_definition_id};

    /// Create an empty function for testing
    fn make_function() -> Definition {
        Definition::new(Arc::new(String::new()), next_definition_id(), 0, Type::ERROR)
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
            then: (b1, None),
            else_: (b2, None),
            end: b3,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::jmp_no_args(b3));
        function.blocks[b2].terminator = Some(TerminatorInstruction::jmp_no_args(b3));
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
            then: (b1, None),
            else_: (b2, None),
            end: b2,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::jmp_no_args(b2));
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
            then: (b1, None),
            else_: (b2, None),
            end: b3,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::If {
            condition: Value::Bool(false),
            then: (b4, None),
            else_: (b5, None),
            end: b6,
        });

        function.blocks[b2].terminator = Some(TerminatorInstruction::If {
            condition: Value::Bool(false),
            then: (b7, None),
            else_: (b8, None),
            end: b9,
        });

        function.blocks[b3].terminator = Some(TerminatorInstruction::Return(Value::Unit));
        function.blocks[b4].terminator = Some(TerminatorInstruction::jmp_no_args(b6));
        function.blocks[b5].terminator = Some(TerminatorInstruction::jmp_no_args(b6));
        function.blocks[b6].terminator = Some(TerminatorInstruction::jmp_no_args(b3));
        function.blocks[b7].terminator = Some(TerminatorInstruction::jmp_no_args(b9));
        function.blocks[b8].terminator = Some(TerminatorInstruction::jmp_no_args(b9));
        function.blocks[b9].terminator = Some(TerminatorInstruction::jmp_no_args(b3));

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
            cases: vec![(0, (b1, None)), (1, (b2, None)), (2, (b3, None))],
            else_: (b4, None),
            end: b5,
        });

        function.blocks[b1].terminator = Some(TerminatorInstruction::jmp_no_args(b5));
        function.blocks[b2].terminator = Some(TerminatorInstruction::jmp_no_args(b5));
        function.blocks[b3].terminator = Some(TerminatorInstruction::jmp_no_args(b6));
        function.blocks[b4].terminator = Some(TerminatorInstruction::jmp_no_args(b5));
        function.blocks[b5].terminator = Some(TerminatorInstruction::Return(Value::Unit));
        function.blocks[b6].terminator = Some(TerminatorInstruction::jmp_no_args(b5));

        let order = function.topological_sort();
        assert_eq!(order, vec![b0, b1, b2, b3, b6, b4, b5]);
    }
}
