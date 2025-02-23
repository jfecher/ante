use std::rc::Rc;

use crate::util::{Id, VecMap};

use super::{
    instruction::{Instruction, InstructionId},
    Block, BlockId, Globals,
};

pub struct Function {
    name: String,
    entry_block: BlockId,

    globals: Rc<Globals>,
    blocks: VecMap<BlockId, Block>,
    instructions: VecMap<InstructionId, Instruction>,
}

pub type FunctionId = Id<Function>;
