//! This file implements the Context object used to create the Mir from the Hir.
//!
//! Several Context functions (usually those named `convert_*`) use algorithms
//! from https://se.cs.uni-tuebingen.de/publications/schuster19zero.pdf
//! These will be marked in comments above the function where appropriate.
//! Additionally, doc comments cannot use color to distinguish terms, as is done
//! in the original source to distinguish compile-time and runtime types/values,
//! a different notation is used:
//!
//! For function types, a runtime function type is denoted by `a -> b` where a
//! compile-time function type uses `a => b`.
//!
//! For lambda values, `fn x -> e` is runtime, where `fn x => e` is a compile-time abstraction.
//!
//! For the `C` function (`convert_capability_type` in this file), an extra boolean parameter
//! is added. This parameter is `true` if `C` refers to the compile-time `C` rather than
//! the runtime version. This parameter is in addition to the change of making the subscript
//! effect stack a parameter to `C` as well. So a call to (red) `C[t]_ts` will translate to
//! `C(t, ts, false)`, and a call to (blue) `C[t]_ts` will translate to `C(t, ts, true)`
//!
//! Unless the term falls into one of the above cases, it is considered to be a runtime term.
use std::{collections::{HashMap, VecDeque}, rc::Rc, cell::RefCell};

use crate::{hir::{self, Ast, Variable, Type, DefinitionId, FunctionType}, util::fmap};

pub struct Context {
    global_definitions: Definitions,
    pub(super) local_definitions: HashMap<hir::DefinitionId, Variable>,

    pub(super) definition_queue: VecDeque<(Rc<RefCell<Option<Ast>>>, Rc<RefCell<Option<Ast>>>, EffectStack)>,

    /// The name of any lambda when we need to make one up.
    /// This is stored here so that we can increment a Rc instead of allocating a new
    /// string for each variable named this way.
    pub(super) lambda_name: Rc<String>,

    /// The next free ID unused by the existing Hir.
    /// This is currently carried on from the monomorphization pass in case their are conflicts,
    /// but since this pass will create an entirely new Hir, it could start from zero as well.
    next_id: usize,
}

pub(super) type Definitions = HashMap<hir::DefinitionId, HashMap<EffectStack, Variable>>;

pub(super) type Effect = (hir::DefinitionId, Type);
pub(super) type EffectStack = Vec<Effect>;

impl Context {
    pub fn new(next_id: usize) -> Self {
        Context {
            global_definitions: HashMap::new(),
            local_definitions: HashMap::new(),
            definition_queue: VecDeque::new(),
            lambda_name: Rc::new("lambda".into()),
            next_id,
        }
    }

    pub fn get_definition(&self, id: hir::DefinitionId, effects: &EffectStack) -> Option<Variable> {
        self.local_definitions.get(&id).or_else(|| {
            self.global_definitions.get(&id).and_then(|map| map.get(effects))
        }).cloned()
    }

    pub fn insert_global_definition(&mut self, id: hir::DefinitionId, value: Variable, effects: EffectStack) {
        self.global_definitions.entry(id).or_default().insert(effects, value);
    }

    pub fn insert_local_definition(&mut self, id: hir::DefinitionId, value: Variable) {
        self.local_definitions.insert(id, value);
    }

    fn next_id(&mut self) -> DefinitionId {
        let id = self.next_id;
        self.next_id += 1;
        DefinitionId(id)
    }

    pub fn placeholder_function_type() -> FunctionType {
        FunctionType {
            parameters: Vec::new(),
            return_type: Box::new(Type::unit()),
            effects: Vec::new(),
            is_varargs: false,
        }
    }

    pub fn lambda(args: Vec<Variable>, typ: FunctionType, body: Ast) -> Ast {
        Ast::Lambda(hir::Lambda { args, body: Box::new(body), typ })
    }

    /// Convenience function for getting the name of a definition which may not have one
    pub fn name_of(name: &Option<String>) -> String {
        name.clone().unwrap_or_else(|| "_".into())
    }

    /// A lambda to be evaluated at compile time.
    /// Currently these all have placeholder types.
    pub fn ct_lambda(args: Vec<Variable>, body: Ast) -> Ast {
        Self::lambda(args, Self::placeholder_function_type(), body)
    }

    /// Create a new variable but do not introduce it into `self.local_definitions`
    pub fn anonymous_variable(&mut self, name: impl Into<String>, typ: Type) -> Variable {
        let definition_id = self.next_id();
        let typ = Rc::new(typ);
        let name = Some(name.into());
        Variable { definition: None, definition_id, typ, name }
    }

    /// Create a new local and introduce it into `self.local_definitions`
    pub fn new_local(&mut self, id: DefinitionId, name: impl Into<String>, typ: Type) -> Variable {
        let local = self.anonymous_variable(name, typ);
        self.insert_local_definition(id, local.clone());
        local
    }

    /// Create a new local a new local from an existing one and introduce it into `self.local_definitions`
    pub fn new_local_from_existing(&mut self, variable: &Variable) -> Variable {
        let definition_id = self.next_id();
        let typ = variable.typ.clone();
        let name = variable.name.clone();
        let local = Variable { definition: None, definition_id, typ, name };

        self.insert_local_definition(variable.definition_id, local.clone());
        local
    }

    pub fn add_global_to_queue(&mut self, variable: hir::Variable, effects: EffectStack) -> Variable {
        let definition_id = self.next_id();
        let typ = variable.typ.clone();
        let name = variable.name.clone();

        let new_definition = Rc::new(RefCell::new(None));
        let new_variable = Variable { definition: Some(new_definition.clone()), definition_id, typ, name };

        self.insert_global_definition(variable.definition_id, new_variable.clone(), effects.clone());

        let old_variable = variable.definition.clone().unwrap_or_else(|| {
            panic!("No definition for global '{}'", variable)
        });

        self.definition_queue.push_back((old_variable, new_definition, effects));
        new_variable
    }

    /// Converts a Hir Type to a Mir Type
    ///
    /// From "Translation of Types" https://se.cs.uni-tuebingen.de/publications/schuster19zero.pdf
    ///
    /// T(Int) = Int
    /// T(t -> t' can t'') = T(t) -> C(t', t'', false)
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

    /// T(t -> t' can t'') = T(t) -> C(t', t'', false)
    ///
    /// TODO: Need to differentiate handler types from non-handler types
    pub fn convert_function_type(&mut self, typ: &hir::FunctionType) -> Type {
        let parameters = fmap(&typ.parameters, |param| self.convert_type(param));
        let return_type = self.convert_capability_type(&typ.return_type, &typ.effects, false);

        Type::Function(hir::FunctionType {
            parameters,
            return_type: Box::new(return_type),
            effects: Vec::new(),
            is_varargs: typ.is_varargs,
        })
    }

    /// From "Translation of Types" https://se.cs.uni-tuebingen.de/publications/schuster19zero.pdf
    /// See note at the top of this file for the notation changes here.
    ///
    /// C(t, [], _) = T(t)
    /// C(t, [t'.., t''], false) = (T(t) -> C(t'', t', false)) -> C(t'', t', false)
    /// C(t, [t'.., t''], true) = (T(t) => C(t'', t', true)) => C(t'', t', true)
    fn convert_capability_type(&mut self, typ: &hir::Type, effects: &[hir::Effect], compile_time: bool) -> Type {
        if effects.is_empty() {
            self.convert_type(typ)
        } else {
            let (last, rest) = effects.split_last().unwrap();
            let head = vec![self.convert_type(typ)];
            let return_type = Box::new(self.convert_capability_type(&last.typ, rest, compile_time));

            let inner_function = Type::Function(hir::FunctionType {
                parameters: head,
                return_type: return_type.clone(),
                effects: Vec::new(),
                is_varargs: false,
            });

            Type::Function(hir::FunctionType {
                parameters: vec![inner_function],
                return_type,
                effects: Vec::new(),
                is_varargs: false,
            })
        }
    }

    /// reify converts a compile-time (static) term to a runtime (residual) term.
    ///
    /// Reify(ts) : C(t, ts, true) -> C(t, ts, false) 
    /// Reify([], s) = s
    /// Reify([ts.., t], s) = fn k -> Reify(ts, s @@ (fn x => Reflect(ts, k @ x)))
    pub fn reify(&mut self, effects: &[Effect], s: Ast) -> Ast {
        match effects.split_last() {
            None => s,
            Some(((_, _t), ts)) => {
                // What is the type of `k` here?
                let k_type = Context::placeholder_function_type();
                let k = self.anonymous_variable("reify_k", Type::Function(k_type.clone()));

                let lambda_type = Context::placeholder_function_type();

                Context::lambda(vec![k.clone()], lambda_type, {
                    // What is the type of 'x' here?
                    let x = self.anonymous_variable("reify_x", Type::unit());

                    let reify_inner = Context::ct_lambda(vec![x.clone()], {
                        let inner_call = Ast::rt_call1(Ast::Variable(k), Ast::Variable(x), k_type);
                        self.reflect(ts, inner_call)
                    });

                    let call = Ast::ct_call1(s, reify_inner);
                    self.reify(ts, call)
                })
            },
        }
    }

    /// reflect converts a runtime (residual) term to a compile-time (static) term.
    ///
    /// Reflect(ts) : C(t, ts, false) -> C(t, ts, true) 
    /// Reflect([], s) = s
    /// Reflect([ts.., t], s) = fn k => Reflect(ts, s @ (fn x -> Reify(ts, k @@ x)))
    pub fn reflect(&mut self, effects: &[Effect], s: Ast) -> Ast {
        match effects.split_last() {
            None => s,
            Some(((_, _t), ts)) => {
                // What is the type of `k` here?
                let k_type = Context::placeholder_function_type();
                let k = self.anonymous_variable("reflect_k", Type::Function(k_type.clone()));

                Context::ct_lambda(vec![k.clone()], {
                    let x = self.anonymous_variable("reflect_x", Type::unit());
                    let lambda_type = Context::placeholder_function_type();

                    let reflect_inner = Context::lambda(vec![x.clone()], lambda_type.clone(), {
                        let inner_call = Ast::ct_call1(Ast::Variable(k), Ast::Variable(x));
                        self.reify(ts, inner_call)
                    });

                    let call = Ast::rt_call1(s, reflect_inner, lambda_type);
                    self.reflect(ts, call)
                })
            },
        }
    }
}
