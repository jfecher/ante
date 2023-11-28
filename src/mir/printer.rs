use std::{fmt::Display, rc::Rc};

use crate::util::fmap;

use super::ir::{Mir, Function, FunctionId, Atom, ParameterId, Type};


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
        let parameters = fmap(self.argument_types.iter().enumerate(), |(i, typ)| {
            let id = ParameterId { function: self.id.clone(), parameter_index: i as u16, name: Rc::new(String::new()) };
            format!("{}: {}", id, typ)
        }).join(", ");

        writeln!(f, "{}({}):\n  {}({})", self.id, parameters, self.body_continuation, body_args)
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
            Atom::Tuple(fields) => {
                let fields = fmap(fields, ToString::to_string).join(", ");
                write!(f, "({fields})")
            },
        }
    }
}

impl Display for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.name, self.id)
    }
}

impl Display for ParameterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}_{}", self.name, self.function, self.parameter_index)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Primitive(primitive) => write!(f, "{primitive}"),
            Type::Function(arguments) => {
                let args = fmap(arguments, ToString::to_string).join(", ");
                write!(f, "fn({args})")
            },
            Type::Tuple(fields) => {
                let fields = fmap(fields, ToString::to_string).join(", ");
                write!(f, "({fields})")
            }
        }
    }
}
