use crate::util::Id;

use super::{
    instruction::{InstructionId, TerminatorInstruction},
    Type,
};

pub struct Block {
    parameters: Vec<Type>,
    instructions: Vec<InstructionId>,
    terminator: TerminatorInstruction,
}

pub type BlockId = Id<Block>;
