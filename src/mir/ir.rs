mod id;

use std::collections::HashMap;

use crate::hir::{Literal, PrimitiveType};
pub use id::*;

#[derive(Default)]
pub struct Mir {
    pub functions: HashMap<FunctionId, Function>,
}

pub struct Function {
    pub id: FunctionId,
    pub argument_types: Vec<Type>,

    // A function's body is always a function call
    pub body_continuation: Atom,
    pub body_args: Vec<Atom>,
}

impl Function {
    /// Return an empty function with the given id that is expected to have its body filled in later
    pub fn empty(id: FunctionId) -> Self {
        Self { id, body_continuation: Atom::Literal(Literal::Unit), body_args: Vec::new(), argument_types: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atom {
    Primop,
    Branch,
    Literal(Literal),
    Parameter(ParameterId),
    Function(FunctionId),
    Tuple(Vec<Atom>),
}

/// This type representation is largely the same as a HIR type
/// except functions have a continuation parameter instead of a return type.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Function(Vec<Type>),

    /// Tuples have a TypeId to allow for struct recursion
    Tuple(Vec<Type>),
}
