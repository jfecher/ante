mod block;
mod function;
mod instruction;
mod value;

pub use crate::hir::Type;
use crate::util::VecMap;
pub use block::{Block, BlockId};
pub use function::{Function, FunctionId};
pub use instruction::{Builtin, Instruction, InstructionId, TerminatorInstruction};
pub use value::Value;

pub struct Mir {
    functions: VecMap<FunctionId, Function>,
    main: FunctionId,
}

pub struct Globals {
    function_types: VecMap<FunctionId, Type>,
}
