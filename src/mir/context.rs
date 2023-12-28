use std::{collections::{HashMap, VecDeque}, rc::Rc};

use crate::{hir::{self, PrimitiveType}, util::fmap, mir::ir::ParameterId};

use super::ir::{Mir, Expr, FunctionId, self, Type, Function, ExternId};


pub struct Context {
    pub(super) mir: Mir,

    definitions: Definitions,

    pub(super) definition_queue: VecDeque<(FunctionId, Rc<hir::Ast>, EffectStack)>,

    /// The function currently being translated. It is expected that the
    /// `body_continuation` and `body_args` fields of this function are filler
    /// and will be replaced once the function finishes translation.
    pub(super) current_function_id: FunctionId,

    /// If this is set, the next function created will be given this id.
    /// This is used to ensure globals have the same ID as the one assigned to them ahead of time.
    pub(super) expected_function_id: Option<FunctionId>,

    /// The name of any lambda when we need to make one up.
    /// This is stored here so that we can increment a Rc instead of allocating a new
    /// string for each variable named this way.
    pub(super) lambda_name: Rc<String>,

    pub(super) extern_symbols: HashMap<String, ExternId>,
}

pub(super) type Definitions = HashMap<hir::DefinitionId, HashMap<EffectStack, Expr>>;

pub(super) type EffectStack = Vec<(hir::DefinitionId, Type)>;

impl Context {
    pub fn new() -> Self {
        let mut mir = Mir::default();
        mir.next_function_id = 1; // Since 0 is taken for main

        let main_id = Mir::main_id();
        let main = ir::Function {
            id: main_id.clone(),
            body: Expr::unit(),
            argument_type: Type::Primitive(PrimitiveType::Unit),
        };

        mir.functions.insert(main_id.clone(), main);

        Context {
            mir,
            definitions: HashMap::new(),
            definition_queue: VecDeque::new(),
            extern_symbols: HashMap::new(),
            current_function_id: main_id,
            expected_function_id: None,
            lambda_name: Rc::new("lambda".into()),
        }
    }

    pub fn function_mut(&mut self, id: &FunctionId) -> &mut Function {
        self.mir.functions.get_mut(&id).unwrap()
    }

    pub fn get_definition(&self, id: hir::DefinitionId, effects: &EffectStack) -> Option<Expr> {
        self.definitions.get(&id).and_then(|map| map.get(effects).cloned())
    }

    pub fn insert_definition(&mut self, id: hir::DefinitionId, atom: Expr, effects: EffectStack) {
        self.definitions.entry(id).or_default().insert(effects, atom);
    }

    /// Returns the next available function id but does not set the current id
    fn next_function_id(&mut self, name: Rc<String>) -> FunctionId {
        self.expected_function_id.take().unwrap_or_else(|| self.mir.next_function_id(name))
    }

    /// Create a new function with the given body and return a reference to it
    ///
    /// The `body` parameter will be run only after each parameter of the function
    /// is inserted into the context.
    ///
    /// This will automatically curry functions with multiple parameters
    pub fn new_function(
        &mut self,
        name: Rc<String>,
        parameters: impl ExactSizeIterator<Item = hir::DefinitionId>,
        argument_types: Vec<Type>,
        effects: EffectStack,
        body: impl FnOnce(&mut Self) -> Expr,
    ) -> Expr {
        let argument_count = argument_types.len();
        assert_eq!(parameters.len(), argument_count);

        let function_ids = fmap(parameters.zip(argument_types), |(parameter_id, typ)| {
            let id = self.next_function_id(name.clone());
            let new_function = ir::Function::empty(id.clone(), typ);
            self.mir.functions.insert(id.clone(), new_function);

            let parameter = ParameterId { function: id.clone(), parameter_index: 0 };
            self.insert_definition(parameter_id, Expr::Parameter(parameter), effects.clone());

            id
        });

        let first_id = function_ids[0].clone();

        // Now that the parameters are in scope we can get the function body
        let mut next = self.in_function(function_ids.last().unwrap().clone(), |this| {
            body(this)
        });

        for id in function_ids.into_iter().rev() {
            self.function_mut(&id).body = next;
            next = Expr::Function(id);
        }

        Expr::Function(first_id)
    }

    /// Set `self.current_function_id` to the given function and execute `f`.
    /// Resets `self.current_function_id` to the previous value before returning.
    fn in_function<T>(&mut self, function: FunctionId, f: impl FnOnce(&mut Self) -> T) -> T {
        let old_id = std::mem::replace(&mut self.current_function_id, function);
        let result = f(self);
        self.current_function_id = old_id;
        result
    }

    /// Create a new function with the given body and return a reference to it
    ///
    /// The `body` parameter will be run only after each parameter of the function
    /// is inserted into the context.
    ///
    /// This will automatically curry functions with multiple parameters
    pub fn recursive_function(
        &mut self,
        name: Rc<String>,
        parameters: impl ExactSizeIterator<Item = hir::DefinitionId>,
        argument_types: Vec<Type>,
        effects: EffectStack,
        id: Option<hir::DefinitionId>,
        body: impl FnOnce(&mut Self) -> Expr,
    ) -> Expr {
        let argument_count = argument_types.len();
        assert_eq!(parameters.len(), argument_count);

        let function_ids = fmap(parameters.zip(argument_types), |(parameter_id, typ)| {
            let id = self.next_function_id(name.clone());
            let new_function = ir::Function::empty(id.clone(), typ);
            self.mir.functions.insert(id.clone(), new_function);

            let parameter = ParameterId { function: id.clone(), parameter_index: 0 };
            self.insert_definition(parameter_id, Expr::Parameter(parameter), effects.clone());

            id
        });

        let first_id = function_ids[0].clone();

        if let Some(id) = id {
            self.insert_definition(id, Expr::Function(first_id.clone()), effects.clone());
        }

        // Now that the parameters are in scope we can get the function body
        let mut next = self.in_function(function_ids.last().unwrap().clone(), |this| {
            body(this)
        });

        for id in function_ids.into_iter().rev() {
            self.function_mut(&id).body = next;
            next = Expr::Function(id);
        }

        Expr::Function(first_id)
    }

    /// Similar to `new_function` except this does not introduce any DefinitionIds
    /// into scope. As such, this is meant for statements that do not return values
    /// or bind variables.
    pub fn intermediate_function(
        &mut self,
        name: Rc<String>,
        argument_type: Type,
        body: impl FnOnce(&mut Self, Expr) -> Expr,
    ) -> Expr {
        let id = self.next_function_id(name);
        let parameter = Expr::Parameter(ParameterId { function: id.clone(), parameter_index: 0 });

        let body = self.in_function(id.clone(), |this| body(this, parameter));

        let new_function = ir::Function { id: id.clone(), argument_type, body };
        self.mir.functions.insert(id.clone(), new_function);
        Expr::Function(id)
    }

    pub fn add_global_to_queue(&mut self, variable: hir::Variable, effects: EffectStack) -> Expr {
        let name = match &variable.name {
            Some(name) => Rc::new(name.to_owned()),
            None => self.lambda_name.clone(),
        };

        let argument_type = match self.convert_type(&variable.typ) {
            Type::Function(argument_types, _) => argument_types,
            other => unreachable!("add_global_to_queue: Expected function type for global, found {}: {}", variable, other),
        };

        let next_id = self.next_function_id(name);
        let atom = Expr::Function(next_id.clone());

        self.insert_definition(variable.definition_id, atom.clone(), effects.clone());

        let definition = variable.definition.clone().unwrap_or_else(|| {
            panic!("No definition for global '{}'", variable)
        });
        self.definition_queue.push_back((next_id.clone(), definition, effects));

        let function = Function::empty(next_id.clone(), *argument_type);
        self.mir.functions.insert(next_id, function);
        atom
    }

    /// Converts a Hir Type to a Mir Type
    ///
    /// From "Translation of Types" https://ps.informatik.uni-tuebingen.de/publications/schuster19zero.pdf
    ///
    /// T(Int) = Int
    /// T(t -> t' can t'') = T(t) -> C(t', t'')
    ///
    /// Ante currently doesn't separate capability types from other function types so there
    /// are no cases for these.
    pub fn convert_type(&mut self, typ: &hir::Type) -> Type {
        match typ {
            hir::Type::Primitive(primitive) => Type::Primitive(primitive.clone()),
            hir::Type::Function(function_type) => self.convert_function_type(function_type),
            hir::Type::Tuple(fields) => {
                Type::Tuple(fmap(fields, |field| self.convert_type(field)))
            },
        }
    }

    /// T(t -> t' can t'') = T(t) -> C(t', t'')
    pub fn convert_function_type(&mut self, typ: &hir::FunctionType) -> Type {
        let params = fmap(&typ.parameters, |param| self.convert_type(param));
        let result = self.convert_capability_type(&typ.return_type, &typ.effects);
        Type::function(params, result)
    }

    /// From "Translation of Types" https://ps.informatik.uni-tuebingen.de/publications/schuster19zero.pdf
    ///
    /// C(t, []) = T(t)
    /// C(t, [t'.., t'']) = (T(t) -> C(t'', t')) -> C(t'', t')
    fn convert_capability_type(&mut self, typ: &hir::Type, effects: &[hir::Effect]) -> Type {
        if effects.is_empty() {
            self.convert_type(typ)
        } else {
            let (last, rest) = effects.split_last().unwrap();
            let head = self.convert_type(typ);
            let result = self.convert_capability_type(&last.typ, rest);
            Type::function(vec![Type::function(vec![head], result.clone())], result)
        }
    }

    pub fn import_extern(&mut self, extern_name: &str, extern_type: &hir::Type) -> ExternId {
        if let Some(id) = self.extern_symbols.get(extern_name) {
            return *id;
        }

        let typ = self.convert_type(extern_type);
        let id = ExternId(self.extern_symbols.len() as u32);
        self.mir.extern_symbols.insert(id, (extern_name.to_owned(), typ));
        self.extern_symbols.insert(extern_name.to_owned(), id);
        id
    }
}
