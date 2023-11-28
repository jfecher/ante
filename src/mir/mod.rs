use std::{rc::Rc, collections::{HashMap, VecDeque}};

use self::ir::{Mir, Atom, FunctionId, Function, ParameterId, Type};
use crate::{hir::{self, Literal, Variable, PrimitiveType}, util::fmap};

pub mod ir;
mod printer;

pub fn convert_to_mir(hir: hir::Ast) -> Mir {
    let mut context = Context::new();
    let ret = hir.to_atom(&mut context);

    if let Some(continuation) = context.continuation.take() {
        context.terminate_function_with_call(continuation, vec![ret]);
    }

    while let Some((_, variable)) = context.definition_queue.pop_front() {
        match &variable.definition {
            Some(definition) => {
                println!("Working on {definition}");
                let result = definition.to_mir(&mut context);
                assert!(matches!(result, AtomOrCall::Atom(Atom::Literal(Literal::Unit))));
            },
            None => unreachable!("No definition for {}", variable),
        }
    }

    context.mir
}

struct Context {
    mir: Mir,
    definitions: HashMap<hir::DefinitionId, Atom>,

    definition_queue: VecDeque<(FunctionId, Variable)>,

    /// The function currently being translated. It is expected that the
    /// `body_continuation` and `body_args` fields of this function are filler
    /// and will be replaced once the function finishes translation.
    current_function_id: FunctionId,

    next_function_id: u32,

    /// If this is set, this tells the Context to use this ID as the next
    /// function id when creating a new function. This is set when a global
    /// function is queued and is expected to be created with an existing ID
    /// so that it can be referenced before it is actually translated.
    expected_function_id: Option<FunctionId>,

    continuation: Option<Atom>,

    /// The name of any intermediate result variable when we need to make one up.
    /// This is stored here so that we can increment a Rc instead of allocating a new
    /// string for each variable named this way.
    intermediate_result_name: Rc<String>,

    /// Similar to the above, this holds the name for any continuations we have to create
    continuation_name: Rc<String>,
}

impl Context {
    fn new() -> Self {
        let mut mir = Mir::default();

        let main_id = FunctionId { id: 0, name: Rc::new("main".into()) };
        let main = ir::Function {
            id: main_id.clone(),
            body_continuation: Atom::Literal(Literal::Unit),
            body_args: Vec::new(),
            argument_types: vec![Type::Function(vec![Type::Primitive(PrimitiveType::Unit)])],
        };

        mir.functions.insert(main_id.clone(), main);

        Context {
            mir,
            definitions: HashMap::new(),
            definition_queue: VecDeque::new(),
            current_function_id: main_id,
            next_function_id: 1, // Since 0 is taken for main
            continuation: None,
            expected_function_id: None,
            intermediate_result_name: Rc::new("v".into()),
            continuation_name: Rc::new("k".into()),
        }
    }

    fn function(&self, id: &FunctionId) -> &Function {
        &self.mir.functions[&id]
    }

    fn function_mut(&mut self, id: &FunctionId) -> &mut Function {
        self.mir.functions.get_mut(&id).unwrap()
    }

    fn current_function(&self) -> &Function {
        self.function(&self.current_function_id)
    }

    fn current_function_mut(&mut self) -> &mut Function {
        self.mir.functions.get_mut(&self.current_function_id).unwrap()
    }

    /// Returns the next available function id but does not set the current id
    fn next_function_id(&mut self, name: Rc<String>) -> FunctionId {
        let id = self.next_function_id;
        self.next_function_id += 1;
        FunctionId { id, name }
    }

    /// Move on to a fresh function
    fn next_fresh_function(&mut self) -> FunctionId {
        let name = self.current_function().id.name.clone();
        self.next_fresh_function_with_name(name)
    }

    /// Move on to a fresh function
    fn next_fresh_function_with_name(&mut self, name: Rc<String>) -> FunctionId {
        let id = self.expected_function_id.take().unwrap_or_else(|| {
            let next_id = self.next_function_id(name);
            let new_function = ir::Function::empty(next_id.clone());
            self.mir.functions.insert(next_id.clone(), new_function);
            next_id
        });

        self.current_function_id = id.clone();
        id
    }

    /// Terminates the current function by setting its body to a function call
    fn terminate_function_with_call(&mut self, f: Atom, args: Vec<Atom>) {
        let function = self.current_function_mut();
        function.body_continuation = f;
        function.body_args = args;
    }

    fn add_global_to_queue(&mut self, variable: hir::Variable) -> Atom {
        let name = match &variable.name {
            Some(name) => Rc::new(name.to_owned()),
            None => self.intermediate_result_name.clone(),
        };

        let argument_types = match Self::convert_type(&variable.typ) {
            Type::Function(argument_types) => argument_types,
            other => unreachable!("add_global_to_queue: Expected function type for global, found {}", other),
        };

        let next_id = self.next_function_id(name);
        let atom = Atom::Function(next_id.clone());
        self.definitions.insert(variable.definition_id, atom.clone());
        self.definition_queue.push_back((next_id.clone(), variable));

        let mut function = Function::empty(next_id.clone());
        function.argument_types = argument_types;

        self.mir.functions.insert(next_id, function);
        atom
    }

    fn convert_type(typ: &hir::Type) -> Type {
        match typ {
            hir::Type::Primitive(primitive) => Type::Primitive(primitive.clone()),
            hir::Type::Function(function_type) => {
                let mut args = fmap(&function_type.parameters, Self::convert_type);
                // The return type becomes a return continuation
                args.push(Type::Function(vec![Self::convert_type(&function_type.return_type)]));
                Type::Function(args)
            },
            hir::Type::Tuple(fields) => {
                Type::Tuple(fmap(fields, Self::convert_type))
            },
        }
    }

    fn add_parameter(&mut self, parameter_type: &hir::Type) {
        let typ = Self::convert_type(parameter_type);
        self.current_function_mut().argument_types.push(typ);
    }

    fn add_continuation_parameter(&mut self, parameter_type: &hir::Type) {
        let typ = Type::Function(vec![Self::convert_type(parameter_type)]);
        self.current_function_mut().argument_types.push(typ);
    }

    fn continuation_types_of(&self, f: &Atom, args: &[Atom]) -> Vec<Type> {
        match f {
            Atom::Primop => todo!("return_type_of primop"),
            Atom::Branch => vec![self.type_of(&args[0])],
            Atom::Parameter(parameter_id) => {
                let function = self.function(&parameter_id.function);
                match &function.argument_types[parameter_id.parameter_index as usize] {
                    Type::Function(arguments) => arguments.clone(),
                    other => unreachable!("Expected function type, found {}", other),
                }
            },
            Atom::Function(function_id) => {
                let function = self.function(function_id);
                let continuation_type = function.argument_types.last().unwrap_or_else(|| panic!("Expected at least 1 argument from {}", function_id));

                match continuation_type {
                    Type::Function(arguments) => arguments.clone(),
                    other => unreachable!("Expected function type, found {}", other),
                }
            },
            Atom::Literal(_) => unreachable!("Cannot call a literal {}", f),
            Atom::Tuple(_) => unreachable!("Cannot call a tuple"),
        }
    }

    fn type_of(&self, atom: &Atom) -> Type {
        match atom {
            Atom::Primop => todo!("type_of Primop"),
            Atom::Branch => unreachable!("Atom::Branch has no type"),
            Atom::Literal(literal) => {
                match literal {
                    Literal::Integer(_, kind) => Type::Primitive(PrimitiveType::Integer(*kind)),
                    Literal::Float(_, kind) => Type::Primitive(PrimitiveType::Float(*kind)),
                    Literal::CString(_) => Type::Primitive(PrimitiveType::Pointer),
                    Literal::Char(_) => Type::Primitive(PrimitiveType::Char),
                    Literal::Bool(_) => Type::Primitive(PrimitiveType::Boolean),
                    Literal::Unit => Type::Primitive(PrimitiveType::Unit),
                }
            },
            Atom::Parameter(parameter_id) => {
                let function = self.function(&parameter_id.function);
                function.argument_types[parameter_id.parameter_index as usize].clone()
            },
            Atom::Function(function_id) => {
                let function = self.function(function_id);
                Type::Function(function.argument_types.clone())
            },
            Atom::Tuple(fields) => {
                let field_types = fmap(fields, |field| self.type_of(field));
                Type::Tuple(field_types)
            },
        }
    }
}

enum AtomOrCall {
    Atom(Atom),
    Call(Atom, Vec<Atom>),
}

impl AtomOrCall {
    fn into_atom(self, context: &mut Context) -> Atom {
        match self {
            AtomOrCall::Atom(atom) => atom,
            AtomOrCall::Call(f, args) => {
                // The argument types of the continuation for the new function we're creating
                let k_types = context.continuation_types_of(&f, &args);

                let current_function_id = context.current_function_id.clone();
                let function = context.current_function_mut();

                function.body_continuation = f;
                function.body_args = args;

                // Create a new function `|rv| ...` as the continuation
                // for the call. Then resume inserting into this new function.
                // The value of the Atom is the new `rv` parameter holding the result value.
                let k = context.next_fresh_function();
                let function = context.current_function_mut();
                function.argument_types = k_types;

                // Make sure to go back to add the continuation argument
                let prev_function = context.function_mut(&current_function_id);
                prev_function.body_args.push(Atom::Function(k.clone()));

                Atom::Parameter(ParameterId {
                    function: k,
                    parameter_index: 0,
                    name: Rc::new("rv".into()),
                })
            },
        }
    }

    fn unit() -> Self {
        AtomOrCall::Atom(Atom::Literal(Literal::Unit))
    }
}

trait ToMir {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall;

    fn to_atom(&self, context: &mut Context) -> Atom {
        self.to_mir(context).into_atom(context)
    }
}

impl ToMir for hir::Ast {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        dispatch_on_hir!(self, ToMir::to_mir, context)
    }
}

impl ToMir for hir::Literal {
    fn to_mir(&self, _mir: &mut Context) -> AtomOrCall {
        AtomOrCall::Atom(Atom::Literal(self.clone()))
    }
}

impl ToMir for hir::Variable {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let atom = context.definitions.get(&self.definition_id).cloned().unwrap_or_else(|| {
            context.add_global_to_queue(self.clone())
        });
        AtomOrCall::Atom(atom)
    }
}

impl ToMir for hir::Lambda {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let original_function = context.current_function_id.clone();
        let original_continuation = context.continuation.take();

        // make sure to add k parameter
        let name = Rc::new("lambda".to_owned());
        let lambda_id = context.next_fresh_function_with_name(name);

        // Add args to scope
        for (i, arg) in self.args.iter().enumerate() {
            context.definitions.insert(arg.definition_id, Atom::Parameter(ParameterId {
                function: lambda_id.clone(),
                parameter_index: i as u16,
                name: arg.name.as_ref().map_or_else(|| Rc::new(format!("p{i}")), |name| Rc::new(name.to_string())),
            }));

            context.add_parameter(&arg.typ);
        }

        // If the argument types were not already set, set them now
        let function = context.current_function_mut();
        if function.argument_types.is_empty() {
            function.argument_types.reserve_exact(self.args.len() + 1);

            for arg in &self.args {
                context.add_parameter(&arg.typ);
            }
            context.add_continuation_parameter(&self.typ.return_type);
        }

        let k = Atom::Parameter(ParameterId {
            function: lambda_id.clone(),
            parameter_index: self.args.len() as u16,
            name: context.continuation_name.clone(),
        });

        context.continuation = Some(k.clone());

        let lambda_body = self.body.to_atom(context);
        context.terminate_function_with_call(k, vec![lambda_body]);

        context.current_function_id = original_function;
        context.continuation = original_continuation;

        AtomOrCall::Atom(Atom::Function(lambda_id))
    }
}

impl ToMir for hir::FunctionCall {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let f = self.function.to_atom(context);
        let args = fmap(&self.args, |arg| arg.to_atom(context));
        AtomOrCall::Call(f, args)
    }
}

impl ToMir for hir::Definition {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        if let Some(expected) = context.definitions.get(&self.variable).cloned() {
            let function = match &expected {
                Atom::Function(function_id) => function_id.clone(),
                other => unreachable!("Expected Atom::Function, found {:?}", other),
            };

            let old = context.expected_function_id.take();
            context.expected_function_id = Some(function);
            let rhs = self.expr.to_atom(context);
            assert_eq!(rhs, expected);
            context.expected_function_id = old;
        } else {
            let rhs = self.expr.to_atom(context);
            context.definitions.insert(self.variable, rhs);
        }

        AtomOrCall::unit()
    }
}

impl ToMir for hir::If {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let cond = self.condition.to_atom(context);
        let original_function = context.current_function_id.clone();

        // needs param
        let end_function_id = context.next_fresh_function();
        context.add_parameter(&self.result_type);
        let end_function = Atom::Function(end_function_id.clone());

        let then_fn = Atom::Function(context.next_fresh_function()) ;
        let then_value = self.then.to_atom(context);
        context.terminate_function_with_call(end_function.clone(), vec![then_value]);

        let else_fn = Atom::Function(context.next_fresh_function()) ;
        let else_value = self.otherwise.to_atom(context);
        context.terminate_function_with_call(end_function, vec![else_value]);

        context.current_function_id = original_function;
        context.terminate_function_with_call(Atom::Branch, vec![cond, then_fn, else_fn]);

        context.current_function_id = end_function_id.clone();
        AtomOrCall::Atom(Atom::Parameter(ParameterId { 
            function: end_function_id,
            parameter_index: 0,
            name: context.intermediate_result_name.clone(),
        }))
    }
}

impl ToMir for hir::Match {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::Return {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let continuation = context.continuation.clone().expect("No continuation for hir::Return!");
        let value = self.expression.to_atom(context);
        AtomOrCall::Call(continuation, vec![value])
    }
}

impl ToMir for hir::Sequence {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let count = self.statements.len();

        // The first statements must be converted to atoms to
        // ensure we create any intermediate continuations needed
        for statement in self.statements.iter().take(count.saturating_sub(1)) {
            statement.to_atom(context);
        }

        // The last statement is kept as an AtomOrCall since it is directly returned
        match self.statements.last() {
            Some(statement) => statement.to_mir(context),
            None => AtomOrCall::unit(),
        }
    }
}

impl ToMir for hir::Extern {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::Assignment {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::MemberAccess {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::Tuple {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let fields = fmap(&self.fields, |field| field.to_atom(context));
        AtomOrCall::Atom(Atom::Tuple(fields))
    }
}

impl ToMir for hir::ReinterpretCast {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::Builtin {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}
