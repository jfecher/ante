use crate::util::Id;

use super::{BlockId, FunctionId, InstructionId};

pub enum Value {
    Result(InstructionId, /*result index*/ u32),
    Constant(ConstantId),
    Parameter(BlockId, /*parameter index*/ u32),
    Function(FunctionId),
}

pub struct Constant {}

pub type ConstantId = Id<Constant>;
