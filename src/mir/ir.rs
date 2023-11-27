mod id;

use std::collections::HashMap;

use crate::hir::Literal;
pub use id::*;

#[derive(Default)]
pub struct Mir {
    pub functions: HashMap<FunctionId, Function>,
}

pub struct Function {
    pub id: FunctionId,

    // A function's body is always a function call
    pub body_continuation: Atom,
    pub body_args: Vec<Atom>,
}

impl Function {
    /// Return an empty function with the given id that is expected to have its body filled in later
    pub fn empty(id: FunctionId) -> Self {
        Self { id, body_continuation: Atom::Literal(Literal::Unit), body_args: Vec::new() }
    }
}

#[derive(Clone)]
pub enum Atom {
    Primop,
    Branch,
    Literal(Literal),
    Parameter(ParameterId),
    Function(FunctionId),
}
