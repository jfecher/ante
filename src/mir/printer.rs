use std::{fmt::Display, rc::Rc, collections::HashMap};

use petgraph::prelude::DiGraph;

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

impl Mir {
    /// Output a printable GraphViz-format graph of the Mir program for debugging.
    /// This graph will show both control flow and data flow.
    #[allow(unused)]
    pub fn debug_print_graph(&self) {
        let mut builder = GraphBuilder::default();
        builder.create_nodes(self);
        builder.create_edges(self);

        let dot = petgraph::dot::Dot::with_config(&builder.graph, &[petgraph::dot::Config::EdgeNoLabel]);
        println!("{dot:?}");
    }

    /// Output a printable GraphViz-format graph of the Mir program for debugging.
    /// This graph will only have edges between data shared between different functions.
    /// Functions themselves are considered global and are not included as edges when referenced.
    #[allow(unused)]
    pub fn debug_print_data_flow_graph(&self) {
        let mut builder = GraphBuilder::default();
        builder.exclude_functions = true;
        builder.create_nodes(self);
        builder.create_edges(self);

        let dot = petgraph::dot::Dot::with_config(&builder.graph, &[petgraph::dot::Config::EdgeNoLabel]);
        println!("{dot:?}");
    }

    /// Output a printable GraphViz-format graph of the Mir program for debugging.
    /// This graph will only have edges between different functions to trace program execution.
    /// Function parameters are not included as edges - the original function reference edge from
    /// the caller function will need to be used to trace control flow instead.
    #[allow(unused)]
    pub fn debug_print_control_flow_graph(&self) {
        let mut builder = GraphBuilder::default();
        builder.exclude_parameters = true;
        builder.create_nodes(self);
        builder.create_edges(self);

        let dot = petgraph::dot::Dot::with_config(&builder.graph, &[petgraph::dot::Config::EdgeNoLabel]);
        println!("{dot:?}");
    }
}

#[derive(Default)]
struct GraphBuilder {
    graph: DiGraph<String, ()>,
    functions: HashMap<FunctionId, petgraph::graph::NodeIndex>,
    parameters: HashMap<ParameterId, petgraph::graph::NodeIndex>,

    exclude_functions: bool,
    exclude_parameters: bool,
}

impl GraphBuilder {
    fn create_nodes(&mut self, mir: &Mir) {
        for (function_id, function) in &mir.functions {
            let function_index = self.graph.add_node(function.to_string());
            self.functions.insert(function_id.clone(), function_index);

            if !self.exclude_parameters {
                for i in 0 .. function.argument_types.len() {
                    let parameter = ParameterId { function: function_id.clone(), parameter_index: i as u16, name: Rc::new(String::new()) };
                    let parameter_index = self.graph.add_node(parameter.to_string());
                    self.parameters.insert(parameter, parameter_index);

                    // This is easiest to add now instead of later in create_edges
                    self.graph.update_edge(function_index, parameter_index, ());
                }
            }
        }
    }

    fn create_edges(&mut self, mir: &Mir) {
        for (function_id, function) in &mir.functions {
            self.add_edges(function_id, &function.body_continuation);

            for arg in &function.body_args {
                self.add_edges(function_id, arg);
            }
        }
    }

    fn add_edges(&mut self, current_function: &FunctionId, atom: &Atom) {
        match atom {
            Atom::Primop => (),
            Atom::Branch => (),
            Atom::Literal(_) => (),
            Atom::Parameter(parameter_id) => {
                if !self.exclude_parameters {
                    let source_index = self.functions[current_function];
                    let destination_index = self.parameters[parameter_id];
                    self.graph.update_edge(source_index, destination_index, ());
                }
            },
            Atom::Function(function_id) => {
                if !self.exclude_functions {
                    let source_index = self.functions[current_function];
                    let destination_index = self.functions[function_id];
                    self.graph.update_edge(source_index, destination_index, ());
                }
            },
            Atom::Tuple(fields) => {
                for field in fields {
                    self.add_edges(current_function, field);
                }
            },
        }
    }
}
