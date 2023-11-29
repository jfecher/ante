use std::{collections::{HashMap, VecDeque}, rc::Rc};

use crate::{hir::{self, Variable, Literal, PrimitiveType}, util::fmap};

use super::ir::{Mir, Atom, FunctionId, self, Type, Function};


pub struct Context {
    pub(super) mir: Mir,
    pub(super) definitions: HashMap<hir::DefinitionId, Atom>,

    pub(super) definition_queue: VecDeque<(FunctionId, Variable)>,

    /// The function currently being translated. It is expected that the
    /// `body_continuation` and `body_args` fields of this function are filler
    /// and will be replaced once the function finishes translation.
    pub(super) current_function_id: FunctionId,

    next_function_id: u32,

    /// If this is set, this tells the Context to use this ID as the next
    /// function id when creating a new function. This is set when a global
    /// function is queued and is expected to be created with an existing ID
    /// so that it can be referenced before it is actually translated.
    pub(super) expected_function_id: Option<FunctionId>,

    pub(super) continuation: Option<Atom>,

    /// The name of any intermediate result variable when we need to make one up.
    /// This is stored here so that we can increment a Rc instead of allocating a new
    /// string for each variable named this way.
    pub(super) intermediate_result_name: Rc<String>,

    /// Similar to the above, this holds the name for any continuations we have to create
    pub(super) continuation_name: Rc<String>,
}

impl Context {
    pub fn new() -> Self {
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

    pub fn function_mut(&mut self, id: &FunctionId) -> &mut Function {
        self.mir.functions.get_mut(&id).unwrap()
    }

    fn current_function(&self) -> &Function {
        self.function(&self.current_function_id)
    }

    pub fn current_function_mut(&mut self) -> &mut Function {
        self.mir.functions.get_mut(&self.current_function_id).unwrap()
    }

    /// Returns the next available function id but does not set the current id
    fn next_function_id(&mut self, name: Rc<String>) -> FunctionId {
        let id = self.next_function_id;
        self.next_function_id += 1;
        FunctionId { id, name }
    }

    /// Move on to a fresh function
    pub fn next_fresh_function(&mut self) -> FunctionId {
        let name = self.current_function().id.name.clone();
        self.next_fresh_function_with_name(name)
    }

    /// Move on to a fresh function
    pub fn next_fresh_function_with_name(&mut self, name: Rc<String>) -> FunctionId {
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
    pub fn terminate_function_with_call(&mut self, f: Atom, args: Vec<Atom>) {
        let function = self.current_function_mut();
        function.body_continuation = f;
        function.body_args = args;
    }

    pub fn add_global_to_queue(&mut self, variable: hir::Variable) -> Atom {
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

    pub fn convert_type(typ: &hir::Type) -> Type {
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

    pub fn add_parameter(&mut self, parameter_type: &hir::Type) {
        let typ = Self::convert_type(parameter_type);
        self.current_function_mut().argument_types.push(typ);
    }

    pub fn add_continuation_parameter(&mut self, parameter_type: &hir::Type) {
        let typ = Type::Function(vec![Self::convert_type(parameter_type)]);
        self.current_function_mut().argument_types.push(typ);
    }

    pub fn continuation_types_of(&self, f: &Atom, args: &[Atom]) -> Vec<Type> {
        match f {
            Atom::Branch => vec![self.type_of(&args[0])],
            Atom::Parameter(parameter_id) => {
                let function = self.function(&parameter_id.function);
                function.argument_types[parameter_id.parameter_index as usize].get_continuation_types(parameter_id)
            },
            Atom::Function(function_id) => {
                let function = self.function(function_id);
                let continuation_type = function.argument_types.last().unwrap_or_else(|| panic!("Expected at least 1 argument from {}", function_id));

                match continuation_type {
                    Type::Function(arguments) => arguments.clone(),
                    other => unreachable!("Expected function type, found {}", other),
                }
            },
            Atom::Deref(_, typ)
            | Atom::Transmute(_, typ) => typ.get_continuation_types(f),

            Atom::Literal(_)
            | Atom::Tuple(_)
            | Atom::AddInt(_, _)
            | Atom::AddFloat(_, _)
            | Atom::SubInt(_, _)
            | Atom::SubFloat(_, _)
            | Atom::MulInt(_, _)
            | Atom::MulFloat(_, _)
            | Atom::DivSigned(_, _)
            | Atom::DivUnsigned(_, _)
            | Atom::DivFloat(_, _)
            | Atom::ModSigned(_, _)
            | Atom::ModUnsigned(_, _)
            | Atom::ModFloat(_, _)
            | Atom::LessSigned(_, _)
            | Atom::LessUnsigned(_, _)
            | Atom::LessFloat(_, _)
            | Atom::EqInt(_, _)
            | Atom::EqFloat(_, _)
            | Atom::EqChar(_, _)
            | Atom::EqBool(_, _)
            | Atom::SignExtend(_, _)
            | Atom::ZeroExtend(_, _)
            | Atom::SignedToFloat(_, _)
            | Atom::UnsignedToFloat(_, _)
            | Atom::FloatToSigned(_, _)
            | Atom::FloatToUnsigned(_, _)
            | Atom::FloatPromote(_, _)
            | Atom::FloatDemote(_, _)
            | Atom::BitwiseAnd(_, _)
            | Atom::BitwiseOr(_, _)
            | Atom::BitwiseXor(_, _)
            | Atom::BitwiseNot(_)
            | Atom::Truncate(_, _)
            | Atom::Offset(_, _, _)
            | Atom::StackAlloc(_) => unreachable!("Cannot call a {}", f),
        }
    }

    fn type_of(&self, atom: &Atom) -> Type {
        match atom {
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
            Atom::AddInt(lhs, _) => self.type_of(lhs),
            Atom::AddFloat(lhs, _) => self.type_of(lhs),
            Atom::SubInt(lhs, _) => self.type_of(lhs),
            Atom::SubFloat(lhs, _) => self.type_of(lhs),
            Atom::MulInt(lhs, _) => self.type_of(lhs),
            Atom::MulFloat(lhs, _) => self.type_of(lhs),
            Atom::DivSigned(lhs, _) => self.type_of(lhs),
            Atom::DivUnsigned(lhs, _) => self.type_of(lhs),
            Atom::DivFloat(lhs, _) => self.type_of(lhs),
            Atom::ModSigned(lhs, _) => self.type_of(lhs),
            Atom::ModUnsigned(lhs, _) => self.type_of(lhs),
            Atom::ModFloat(lhs, _) => self.type_of(lhs),
            Atom::LessSigned(_, _) => Type::Primitive(PrimitiveType::Boolean),
            Atom::LessUnsigned(_, _) => Type::Primitive(PrimitiveType::Boolean),
            Atom::LessFloat(_, _) => Type::Primitive(PrimitiveType::Boolean),
            Atom::EqInt(_, _) => Type::Primitive(PrimitiveType::Boolean),
            Atom::EqFloat(_, _) => Type::Primitive(PrimitiveType::Boolean),
            Atom::EqChar(_, _) => Type::Primitive(PrimitiveType::Boolean),
            Atom::EqBool(_, _) => Type::Primitive(PrimitiveType::Boolean),
            Atom::SignExtend(_, typ) => typ.clone(),
            Atom::ZeroExtend(_, typ) => typ.clone(),
            Atom::SignedToFloat(_, typ) => typ.clone(),
            Atom::UnsignedToFloat(_, typ) => typ.clone(),
            Atom::FloatToSigned(_, typ) => typ.clone(),
            Atom::FloatToUnsigned(_, typ) => typ.clone(),
            Atom::FloatPromote(_, typ) => typ.clone(),
            Atom::FloatDemote(_, typ) => typ.clone(),
            Atom::BitwiseAnd(lhs, _) => self.type_of(lhs),
            Atom::BitwiseOr(lhs, _) => self.type_of(lhs),
            Atom::BitwiseXor(lhs, _) => self.type_of(lhs),
            Atom::BitwiseNot(lhs) => self.type_of(lhs),
            Atom::Truncate(_, typ) => typ.clone(),
            Atom::Deref(_, typ) => typ.clone(),
            Atom::Offset(_, _, _) => Type::Primitive(PrimitiveType::Pointer),
            Atom::Transmute(_, typ) => typ.clone(),
            Atom::StackAlloc(_) => Type::Primitive(PrimitiveType::Pointer),
        }
    }
}
