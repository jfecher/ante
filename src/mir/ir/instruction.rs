use crate::util::Id;

use super::{BlockId, Type, Value};

pub type InstructionId = Id<Instruction>;

pub enum Instruction {
    Builtin(Builtin),
    Call { function: Value, args: Vec<Value> },

    Allocate(Type),
    Load(Value),
    Store { address: Value, value: Value },
}

pub enum TerminatorInstruction {
    Jump { dest: BlockId, args: Vec<Value> },
    JumpIf { cond: Value, then: BlockId, else_: BlockId },
    Switch { int_value: Value, cases: Vec<BlockId>, else_: Option<BlockId> },
    Return { args: Vec<Value> },
}

pub enum Builtin {
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
    EqChar(Value, Value),
    EqBool(Value, Value),

    SignExtend(Value, Type),
    ZeroExtend(Value, Type),

    SignedToFloat(Value, Type),
    UnsignedToFloat(Value, Type),
    FloatToSigned(Value, Type),
    FloatToUnsigned(Value, Type),
    FloatPromote(Value),
    FloatDemote(Value),

    BitwiseAnd(Value, Value),
    BitwiseOr(Value, Value),
    BitwiseXor(Value, Value),
    BitwiseNot(Value),

    Truncate(Value, Type),
    Deref(Value, Type),
    Offset(Value, Value, Type),
    Transmute(Value, Type),

    /// Allocate space for the given value on the stack, and store it there. Return the stack address
    StackAlloc(Value),
}
