use std::fmt::Display;

use crate::util::fmap;

use super::ir::{Mir, Function, FunctionId, Atom, ParameterId};


impl Display for Mir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (_, function) in &self.functions {
            writeln!(f, "{function}")?;
        }

        Ok(())
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body_args = fmap(&self.body_args, ToString::to_string).join(", ");
        writeln!(f, "{}():\n  {}({})", self.id, self.body_continuation, body_args)
    }
}

impl Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Primop => write!(f, "primop"),
            Atom::Branch => write!(f, "branch"),
            Atom::Literal(literal) => write!(f, "{literal}"),
            Atom::Parameter(parameter) => write!(f, "{parameter}"),
            Atom::Function(lambda) => write!(f, "{lambda}"),
        }
    }
}

impl Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}${}", self.name, self.id)
    }
}

impl Display for ParameterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}${}${}", self.name, self.function, self.parameter_index)
    }
}
