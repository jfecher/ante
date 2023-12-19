use std::{collections::{HashMap, VecDeque}, rc::Rc};

use crate::{hir::{self, Literal, PrimitiveType}, util::fmap, mir::ir::{ParameterId, EffectIndices}};

use super::ir::{Mir, Atom, FunctionId, self, Type, Function, ExternId, EffectId, HandlerId};


pub struct Context {
    pub(super) mir: Mir,

    pure_definitions: Definitions,
    specialized_definitions: HashMap<HandlerId, SpecializedDefinitions>,

    pub(super) effects: HashMap<hir::DefinitionId, EffectId>,

    pub(super) definition_queue: VecDeque<(FunctionId, Option<HandlerId>, Rc<hir::Ast>)>,

    /// The function currently being translated. It is expected that the
    /// `body_continuation` and `body_args` fields of this function are filler
    /// and will be replaced once the function finishes translation.
    pub(super) current_function_id: FunctionId,

    next_function_id: u32,
    next_handler_id: u32,

    pub(super) current_handler: Option<HandlerId>,

    /// Maps an effect id to the parameter that corresponds to an effect to use.
    /// This is the parameter the handle branch will be passed in through.
    /// See comment on `EffectIndices` for the implicit parameters effects add to a function.
    pub(super) handlers: HashMap<EffectId, Atom>,

    /// Maps an effect id to the parameter that corresponds to a continuation to a handle branch for an
    /// effect. See comment on `EffectIndices` for the implicit parameters effects add to a function.
    pub(super) handler_ks: HashMap<EffectId, Atom>,

    /// If this is set, this tells the Context to use this ID as the next
    /// function id when creating a new function. This is set when a global
    /// function is queued and is expected to be created with an existing ID
    /// so that it can be referenced before it is actually translated.
    pub(super) expected_function_id: Option<FunctionId>,

    pub(super) continuation: Option<Atom>,

    /// If this is present, this variable gets defined as `self.continuation`
    /// when the next Lambda is converted. This is used to define `resume` to
    /// be the automatically-generated continuation parameter of a Lambda.
    pub(super) handler_continuation: Option<(EffectId, hir::DefinitionInfo)>,

    /// The name of any lambda when we need to make one up.
    /// This is stored here so that we can increment a Rc instead of allocating a new
    /// string for each variable named this way.
    pub(super) lambda_name: Rc<String>,
}

pub(super) type Definitions = HashMap<hir::DefinitionId, Atom>;

pub(super) struct SpecializedDefinitions {
    pub(super) effect_id: EffectId,
    pub(super) handler_type: Type,
    pub(super) definitions: Definitions,

    /// Points to the parent handler in the handler stack if there is more than one handler.
    pub(super) parent_handler: Option<HandlerId>,
}

/// Convenience struct for keeping track of handlers returned by register_handlers
pub struct Handlers {
    pub(super) handlers: HashMap<EffectId, Atom>,
    pub(super) handler_ks: HashMap<EffectId, Atom>,
}

impl Context {
    pub fn new() -> Self {
        let mut mir = Mir::default();

        let main_id = Mir::main_id();
        let main = ir::Function {
            id: main_id.clone(),
            body_continuation: Atom::Literal(Literal::Unit),
            body_args: Vec::new(),
            argument_types: vec![Type::Function(vec![Type::Primitive(PrimitiveType::Unit)], vec![])],
        };

        mir.functions.insert(main_id.clone(), main);

        Context {
            mir,
            pure_definitions: HashMap::new(),
            specialized_definitions: HashMap::new(),
            definition_queue: VecDeque::new(),
            effects: HashMap::new(),
            handlers: HashMap::new(),
            handler_ks: HashMap::new(),
            current_handler: None,
            current_function_id: main_id,
            next_function_id: 1, // Since 0 is taken for main
            next_handler_id: 0,
            continuation: None,
            handler_continuation: None,
            expected_function_id: None,
            lambda_name: Rc::new("lambda".into()),
        }
    }

    fn function(&self, id: &FunctionId) -> &Function {
        &self.mir.functions[&id]
    }

    fn current_function(&self) -> &Function {
        self.function(&self.current_function_id)
    }

    pub fn current_function_mut(&mut self) -> &mut Function {
        self.mir.functions.get_mut(&self.current_function_id).unwrap()
    }

    pub fn get_definition(&self, id: hir::DefinitionId, typ: &hir::Type) -> Option<Atom> {
        if Self::type_is_pure(typ) {
            self.pure_definitions.get(&id).cloned()
        } else {
            let handler = self.current_handler.unwrap();
            self.specialized_definitions[&handler].definitions.get(&id).cloned()
        }
    }

    pub fn insert_definition(&mut self, id: hir::DefinitionId, typ: &hir::Type, atom: Atom) {
        if Self::type_is_pure(typ) {
            self.pure_definitions.insert(id, atom);
        } else {
            let handler = self.current_handler.unwrap();
            self.specialized_definitions.get_mut(&handler).unwrap().definitions.insert(id, atom);
        }
    }

    fn type_is_pure(typ: &hir::Type) -> bool {
        match typ {
            hir::Type::Function(function) => function.effects.is_empty(),
            _ => true,
        }
    }

    /// Returns the next available function id but does not set the current id
    fn next_function_id(&mut self, name: Rc<String>) -> FunctionId {
        let id = self.next_function_id;
        self.next_function_id += 1;
        FunctionId { id, name }
    }

    /// Returns the next available function id but does not set the current id
    fn next_handler_id(&mut self) -> HandlerId {
        let id = self.next_handler_id;
        self.next_handler_id += 1;
        HandlerId(id)
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

    /// Terminates the current function by setting its body to a function call.
    /// This function also automatically inserts any required effect handler parameters.
    pub fn terminate_function_with_call_and_effects(&mut self, f: Atom, mut args: Vec<Atom>, k: Atom, f_effects: &[EffectIndices]) {
        for effect in f_effects {
            let id = effect.effect_id;

            if let Some(handler) = self.handlers.get(&id) {
                args.insert(effect.effect_index as usize, handler.clone());
            }

            // each handler_k can only be used once
            if let Some(handler_k) = self.handler_ks.remove(&id) {
                args.insert(effect.effect_k_index as usize, handler_k.clone());
            }
        }

        args.push(k);

        let function = self.current_function_mut();
        function.body_continuation = f;
        function.body_args = args;
    }

    pub fn add_global_to_queue(&mut self, variable: hir::Variable) -> Atom {
        let name = match &variable.name {
            Some(name) => Rc::new(name.to_owned()),
            None => self.lambda_name.clone(),
        };

        let (argument_types, is_pure) = match self.convert_type(&variable.typ) {
            Type::Function(argument_types, effects) => (argument_types, effects.is_empty()),
            other => unreachable!("add_global_to_queue: Expected function type for global, found {}: {}", variable, other),
        };

        let next_id = self.next_function_id(name);
        let atom = Atom::Function(next_id.clone());

        if is_pure {
            self.pure_definitions.insert(variable.definition_id, atom.clone());
        } else {
            let handler = self.current_handler.unwrap();
            self.specialized_definitions.get_mut(&handler).unwrap()
                .definitions.insert(variable.definition_id, atom.clone());
        }

        let definition = variable.definition.clone().unwrap_or_else(|| {
            panic!("No definition for global '{}'", variable)
        });
        self.definition_queue.push_back((next_id.clone(), self.current_handler.clone(), definition));

        let mut function = Function::empty(next_id.clone());
        function.argument_types = argument_types;

        self.mir.functions.insert(next_id, function);
        atom
    }

    /// Set the current handlers back to an old set of handlers returned by register_handlers
    pub fn set_handlers(&mut self, handlers: Handlers) {
        self.handlers = handlers.handlers;
        self.handler_ks = handlers.handler_ks;
    }

    pub fn register_handlers(&mut self, effects: &[EffectIndices], function_id: &FunctionId) -> Handlers {
        let old_handlers = std::mem::take(&mut self.handlers);
        let old_handler_ks = std::mem::take(&mut self.handler_ks);

        for effect_indices in effects {
            let handler = Atom::Parameter(ParameterId {
                function: function_id.clone(),
                parameter_index: effect_indices.effect_index,
            });

            let handler_k = Atom::Parameter(ParameterId {
                function: function_id.clone(),
                parameter_index: effect_indices.effect_k_index,
            });

            self.handlers.insert(effect_indices.effect_id, handler);
            self.handler_ks.insert(effect_indices.effect_id, handler_k);
        }

        Handlers {
            handlers: old_handlers,
            handler_ks: old_handler_ks,
        }
    }

    /// Converts a Hir Type to a Mir Type
    pub fn convert_type(&mut self, typ: &hir::Type) -> Type {
        match typ {
            hir::Type::Primitive(primitive) => Type::Primitive(primitive.clone()),
            hir::Type::Function(function_type) => {
                let (args, effects) = self.convert_function_type(&function_type);
                Type::Function(args, effects)
            },
            hir::Type::Tuple(fields) => {
                Type::Tuple(fmap(fields, |field| self.convert_type(field)))
            },
        }
    }

    pub fn convert_function_type(&mut self, typ: &hir::FunctionType) -> (Vec<Type>, Vec<EffectIndices>) {
        let mut args = fmap(&typ.parameters, |param| self.convert_type(param));

        let return_type = self.convert_type(&typ.return_type);

        let effects = fmap(&typ.effects, |effect| {
            let start_index = args.len() as u16;
            let effect_id = self.lookup_or_create_effect(effect.id);
            let mut effect_type = self.convert_type(&effect.typ);
            let handler_type = self.lookup_handler_type(effect_id);

            // effect
            match &mut effect_type {
                Type::Function(arg_types, _) => {
                    match arg_types.last_mut().unwrap() {
                        Type::Function(inner_arg_types, _) => {
                            inner_arg_types.push(handler_type.clone());
                        },
                        other => unreachable!("Expected function type while CPS'ing effect, got {}", other),
                    }

                    arg_types.insert(arg_types.len() - 1, Type::Function(vec![handler_type.clone()], vec![]));
                }
                other => unreachable!("Expected function type while CPS'ing effect, got {}", other),
            }
            args.push(effect_type);

            // effect_k
            args.push(Type::Function(vec![handler_type.clone()], vec![]));

            // k
            args.push(Type::function(vec![return_type.clone()], handler_type));

            EffectIndices {
                effect_id,
                effect_index: start_index,
                effect_k_index: start_index + 1,
                k_index: start_index + 2,
            }
        });

        if effects.is_empty() {
            args.push(Type::Function(vec![return_type], vec![]));
        }

        (args, effects)
    }

    fn lookup_handler_type(&self, effect_id: EffectId) -> Type {
        fn lookup_handler_type_rec(this: &Context, effect_id: EffectId, handler: HandlerId) -> Type {
            let definitions = &this.specialized_definitions[&handler];

            if definitions.effect_id == effect_id {
                definitions.handler_type.clone()
            } else {
                let parent = definitions.parent_handler.unwrap();
                lookup_handler_type_rec(this, effect_id, parent)
            }
        }

        lookup_handler_type_rec(self, effect_id, self.current_handler.unwrap())
    }

    pub fn import_extern(&mut self, extern_name: &str, extern_type: &hir::Type) -> ExternId {
        if let Some((_, id)) = self.mir.extern_symbols.get(extern_name) {
            return *id;
        }

        let typ = self.convert_type(extern_type);
        let id = ExternId(self.mir.extern_symbols.len() as u32);
        self.mir.extern_symbols.insert(extern_name.to_owned(), (typ, id));
        id
    }

    pub fn add_parameter(&mut self, parameter_type: &hir::Type) {
        let typ = self.convert_type(parameter_type);
        self.current_function_mut().argument_types.push(typ);
    }

    pub fn current_parameters(&self) -> Vec<Atom> {
        let parameter_count = self.current_function().argument_types.len();
        fmap(0 .. parameter_count, |i| Atom::Parameter(ir::ParameterId {
            function: self.current_function_id.clone(),
            parameter_index: i as u16,
        }))
    }

    pub fn lookup_or_create_effect(&mut self, id: hir::DefinitionId) -> EffectId {
        if let Some(effect_id) = self.effects.get(&id) {
            return *effect_id;
        }

        let effect_id = EffectId(self.effects.len() as u32);
        self.effects.insert(id, effect_id);
        effect_id
    }

    /// Create a fresh function with the given argument type and call `f` with it as an argument.
    /// After `f` is called, the current function is switched to the new function
    pub fn with_next_function<T>(&mut self, result_type: &hir::Type, effects: &[hir::Effect], f: impl FnOnce(&mut Self, Atom) -> T) -> T {
        let old_function = self.current_function_id.clone();
        let next_function_id = self.next_fresh_function();
        self.add_parameter(result_type);

        let effect_handlers_start = self.current_function().argument_types.len();

        for effect in effects {
            let effect_id = self.lookup_or_create_effect(effect.id);
            let handler_type = self.lookup_handler_type(effect_id);
            let effect_type = Type::Function(vec![handler_type], vec![]);
            self.current_function_mut().argument_types.push(effect_type);
        }

        self.current_function_id = old_function;
        let result = f(self, Atom::Function(next_function_id.clone()));

        // Insert the new handler continuations into scope
        for (i, effect) in effects.iter().enumerate() {
            let parameter_index = (effect_handlers_start + i) as u16;
            let effect_id = self.lookup_or_create_effect(effect.id);
            let function = next_function_id.clone();
            let handler_k = Atom::Parameter(ParameterId { function, parameter_index });
            self.handler_ks.insert(effect_id, handler_k);
        }

        self.current_function_id = next_function_id;
        result
    }

    pub fn lookup_handler(&self, effect_id: EffectId) -> Atom {
        self.handlers.get(&effect_id).unwrap().clone()
    }

    pub fn enter_handler(&mut self, effect_id: EffectId, handler_type: Type) -> Option<HandlerId> {
        let new_id = self.next_handler_id();
        let old_id = self.current_handler.take();
        self.current_handler = Some(new_id);

        self.specialized_definitions.insert(new_id, SpecializedDefinitions {
            effect_id,
            handler_type,
            parent_handler: old_id,
            definitions: HashMap::new(),
        });

        old_id
    }

    pub fn enter_handler_expression(&mut self, effect_id: EffectId, handler: Atom, handler_k: Atom) -> (Option<Atom>, Option<Atom>) {
        let old_handler = self.handlers.insert(effect_id, handler);
        let old_handler_k = self.handler_ks.insert(effect_id, handler_k);
        (old_handler, old_handler_k)
    }

    pub fn exit_handler_and_expression(&mut self, effect_id: EffectId, parent_handler: Option<HandlerId>, old_handler: Option<Atom>, old_handler_k: Option<Atom>) {
        self.current_handler = parent_handler;

        // We must remember to remove this handler from the self when finished
        match old_handler {
            Some(old_handler) => self.handlers.insert(effect_id, old_handler),
            None => self.handlers.remove(&effect_id),
        };

        match old_handler_k {
            Some(old_handler_k) => self.handler_ks.insert(effect_id, old_handler_k),
            None => self.handler_ks.remove(&effect_id),
        };
    }
}
