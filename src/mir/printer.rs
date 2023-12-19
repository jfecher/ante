use std::{fmt::Display, collections::HashMap};

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
        let parameters = fmap(&self.argument_types, ToString::to_string).join(", ");

        writeln!(f, "{}({}):\n  {}({})", self.id, parameters, self.body_continuation, body_args)
    }
}

impl Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Branch => write!(f, "branch"),
            Atom::Switch(branches, else_branch) => {
                let branches = fmap(branches, |(value, branch)| format!("{} -> {}", value, branch)).join(", ");
                let else_branch = match else_branch {
                    Some(branch) => format!(", {}", branch),
                    None => String::new(),
                };
                write!(f, "switch[{}{}]", branches, else_branch)
            },
            Atom::Literal(literal) => write!(f, "{literal}"),
            Atom::Parameter(parameter) => write!(f, "{parameter}"),
            Atom::Function(lambda) => write!(f, "{lambda}"),
            Atom::Tuple(fields) => {
                let fields = fmap(fields, ToString::to_string).join(", ");
                write!(f, "({fields})")
            },
            Atom::MemberAccess(lhs, index, typ) => {
                write!(f, "({lhs} . {index} : {typ})")
            }
            Atom::Assign => write!(f, ":="),
            Atom::Extern(extern_id) => write!(f, "extern_{}", extern_id.0),
            Atom::Handle(id, branch) => write!(f, "handler_{}[{}]", id.0, branch),
            Atom::Effect(id, _typ) => write!(f, "effect_{}", id.0),
            Atom::AddInt(lhs, rhs) => write!(f, "({lhs} + {rhs})"),
            Atom::AddFloat(lhs, rhs) => write!(f, "({lhs} + {rhs})"),
            Atom::SubInt(lhs, rhs) => write!(f, "({lhs} - {rhs})"),
            Atom::SubFloat(lhs, rhs) => write!(f, "({lhs} - {rhs})"),
            Atom::MulInt(lhs, rhs) => write!(f, "({lhs} * {rhs})"),
            Atom::MulFloat(lhs, rhs) => write!(f, "({lhs} * {rhs})"),
            Atom::DivSigned(lhs, rhs) => write!(f, "({lhs} / {rhs})"),
            Atom::DivUnsigned(lhs, rhs) => write!(f, "({lhs} / {rhs})"),
            Atom::DivFloat(lhs, rhs) => write!(f, "({lhs} / {rhs})"),
            Atom::ModSigned(lhs, rhs) => write!(f, "({lhs} % {rhs})"),
            Atom::ModUnsigned(lhs, rhs) => write!(f, "({lhs} % {rhs})"),
            Atom::ModFloat(lhs, rhs) => write!(f, "({lhs} % {rhs})"),
            Atom::LessSigned(lhs, rhs) => write!(f, "({lhs} < {rhs})"),
            Atom::LessUnsigned(lhs, rhs) => write!(f, "({lhs} < {rhs})"),
            Atom::LessFloat(lhs, rhs) => write!(f, "({lhs} < {rhs})"),
            Atom::EqInt(lhs, rhs) => write!(f, "({lhs} == {rhs})"),
            Atom::EqFloat(lhs, rhs) => write!(f, "({lhs} == {rhs})"),
            Atom::EqChar(lhs, rhs) => write!(f, "({lhs} == {rhs})"),
            Atom::EqBool(lhs, rhs) => write!(f, "({lhs} == {rhs})"),
            Atom::SignExtend(lhs, rhs) => write!(f, "(sign_extend {lhs} {rhs})"),
            Atom::ZeroExtend(lhs, rhs) => write!(f, "(zero_extend {lhs} {rhs})"),
            Atom::SignedToFloat(lhs, rhs) => write!(f, "(signed_to_float {lhs} {rhs})"),
            Atom::UnsignedToFloat(lhs, rhs) => write!(f, "(unsigned_to_float {lhs} {rhs})"),
            Atom::FloatToSigned(lhs, rhs) => write!(f, "(float_to_signed {lhs} {rhs})"),
            Atom::FloatToUnsigned(lhs, rhs) => write!(f, "(float_to_unsigned {lhs} {rhs})"),
            Atom::FloatPromote(lhs, rhs) => write!(f, "(float_promote {lhs} {rhs})"),
            Atom::FloatDemote(lhs, rhs) => write!(f, "(float_demote {lhs} {rhs})"),
            Atom::BitwiseAnd(lhs, rhs) => write!(f, "({lhs} & {rhs})"),
            Atom::BitwiseOr(lhs, rhs) => write!(f, "({lhs} | {rhs})"),
            Atom::BitwiseXor(lhs, rhs) => write!(f, "({lhs} ^ {rhs})"),
            Atom::BitwiseNot(lhs) => write!(f, "(bitwise_not {lhs})"),
            Atom::Truncate(lhs, rhs) => write!(f, "(truncate {lhs} {rhs})"),
            Atom::Deref(lhs, rhs) => write!(f, "(deref {lhs} {rhs})"),
            Atom::Offset(lhs, rhs, typ) => write!(f, "(offset {lhs} {rhs} {typ})"),
            Atom::Transmute(lhs, rhs) => write!(f, "(transmute {lhs} {rhs})"),
            Atom::StackAlloc(lhs) => write!(f, "(stack_alloc {lhs})"),
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
        write!(f, "{}_{}", self.function, self.parameter_index)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Primitive(primitive) => write!(f, "{primitive}"),
            Type::Function(args, _) => {
                let args = fmap(args, ToString::to_string).join(", ");
                write!(f, "fn({})", args)
            },
            Type::Tuple(fields) => {
                for (i, arg) in fields.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                Ok(())
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
                    let parameter = ParameterId { function: function_id.clone(), parameter_index: i as u16 };
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
        let on_function = |this: &mut Self, function_id: &FunctionId| {
            if !this.exclude_functions {
                let source_index = this.functions[current_function];
                let destination_index = this.functions[function_id];
                this.graph.update_edge(source_index, destination_index, ());
            }
        };

        let on_parameter = |this: &mut Self, parameter_id: &ParameterId| {
            if !this.exclude_parameters {
                let source_index = this.functions[current_function];
                let destination_index = this.parameters[parameter_id];
                this.graph.update_edge(source_index, destination_index, ());
            }
        };

        atom.for_each_id(self, on_function, on_parameter);
    }
}
