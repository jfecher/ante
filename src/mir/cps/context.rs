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
use std::{collections::{HashMap, VecDeque}, rc::Rc};

use crate::util::fmap;
use crate::mir::ir::{ self as mir, Ast, Atom, Variable, Type, DefinitionId, FunctionType };

use super::effect_stack::{EffectStack, Effect};

pub struct Context {
    global_definitions: Definitions,
    pub(super) local_definitions: HashMap<DefinitionId, Atom>,

    pub(super) definition_queue: VecDeque<(DefinitionId, DefinitionId, EffectStack)>,

    /// Default name to give to fresh variables
    pub(super) default_name: Rc<String>,

    /// The next free DefinitionId to create
    pub(super) next_id: usize,
}

pub(super) type Definitions = HashMap<DefinitionId, HashMap<EffectStack, Variable>>;

impl Context {
    pub fn new(next_id: usize) -> Self {
        Context {
            global_definitions: HashMap::new(),
            local_definitions: HashMap::new(),
            definition_queue: VecDeque::new(),
            default_name: Rc::new("_".into()),
            next_id,
        }
    }

    pub fn get_definition(&self, id: DefinitionId, effects: &EffectStack) -> Option<Atom> {
        self.local_definitions.get(&id).cloned().or_else(|| {
            let map = self.global_definitions.get(&id)?;
            let definition = map.get(effects)?.clone();
            Some(Atom::Variable(definition))
        })
    }

    pub fn insert_global_definition(&mut self, id: DefinitionId, value: Variable, effects: EffectStack) {
        self.global_definitions.entry(id).or_default().insert(effects, value);
    }

    pub fn insert_local_definition(&mut self, id: DefinitionId, atom: Atom) {
        self.local_definitions.insert(id, atom);
    }

    pub fn next_id(&mut self) -> DefinitionId {
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

    pub fn lambda(args: Vec<Variable>, typ: FunctionType, body: Ast) -> Atom {
        Atom::Lambda(mir::Lambda { args, body: Box::new(body), typ, compile_time: false })
    }

    /// A lambda to be evaluated at compile time.
    /// Currently these all have placeholder types.
    pub fn ct_lambda(args: Vec<Variable>, body: Ast) -> Atom {
        let typ = Self::placeholder_function_type();
        Atom::Lambda(mir::Lambda { args, body: Box::new(body), typ, compile_time: true })
    }

    /// Create a new variable but do not introduce it into `self.local_definitions`
    pub fn anonymous_variable(&mut self, name: impl Into<String>, typ: Type) -> Variable {
        let typ = Rc::new(typ);
        let name = Rc::new(name.into());
        self.fresh_existing_variable(name, typ)
    }

    /// Create a fresh variable with the same name and type as an existing variable,
    /// and do not introduce it into `self.local_definitions`
    pub fn fresh_existing_variable(&mut self, name: Rc<String>, typ: Rc<Type>) -> Variable {
        Variable { definition_id: self.next_id(), typ, name }
    }

    /// Create a new local and introduce it into `self.local_definitions`
    pub fn new_local(&mut self, id: DefinitionId, name: impl Into<String>, typ: Type) -> Variable {
        let local = self.anonymous_variable(name, typ);
        self.insert_local_definition(id, Atom::Variable(local.clone()));
        local
    }

    /// Create a new local a new local from an existing one and introduce it into `self.local_definitions`
    pub fn new_local_from_existing(&mut self, variable: &Variable) -> Variable {
        let definition_id = self.next_id();
        let typ = variable.typ.clone();
        let name = variable.name.clone();
        let local = Variable { definition_id, typ, name };

        self.insert_local_definition(variable.definition_id, Atom::Variable(local.clone()));
        local
    }

    pub fn add_global_to_queue(&mut self, variable: Variable, effects: EffectStack) -> Variable {
        let definition_id = self.next_id();
        let typ = variable.typ.clone();
        let name = variable.name.clone();

        let new_variable = Variable { definition_id, typ, name };

        self.insert_global_definition(variable.definition_id, new_variable.clone(), effects.clone());

        self.definition_queue.push_back((variable.definition_id, definition_id, effects));
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
    pub fn convert_type(&mut self, typ: &Type) -> Type {
        match typ {
            Type::Primitive(primitive) => Type::Primitive(primitive.clone()),
            Type::Function(function_type) => self.convert_function_type(function_type),
            Type::Tuple(fields) => {
                Type::Tuple(fmap(fields, |field| self.convert_type(field)))
            },
        }
    }

    /// T(t -> t' can t'') = T(t) -> C(t', t'', false)
    ///
    /// TODO: Need to differentiate handler types from non-handler types
    pub fn convert_function_type(&mut self, typ: &FunctionType) -> Type {
        let parameters = fmap(&typ.parameters, |param| self.convert_type(param));
        let return_type = self.convert_capability_type(&typ.return_type, &typ.effects, false);

        Type::Function(FunctionType {
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
    fn convert_capability_type(&mut self, typ: &Type, effects: &[mir::Effect], compile_time: bool) -> Type {
        if effects.is_empty() {
            self.convert_type(typ)
        } else {
            let (last, rest) = effects.split_last().unwrap();
            let head = vec![self.convert_type(typ)];
            let return_type = Box::new(self.convert_capability_type(&last.typ, rest, compile_time));

            let inner_function = Type::Function(FunctionType {
                parameters: head,
                return_type: return_type.clone(),
                effects: Vec::new(),
                is_varargs: false,
            });

            Type::Function(FunctionType {
                parameters: vec![inner_function],
                return_type,
                effects: Vec::new(),
                is_varargs: false,
            })
        }
    }

    pub fn let_binding(&mut self, typ: Type, ast: Ast, f: impl FnOnce(&mut Self, Atom) -> Ast) -> Ast {
        match ast {
            Ast::Atom(atom) => f(self, atom),
            Ast::Let(mut let_) => {
                *let_.body = self.let_binding(typ, *let_.body, f);
                Ast::Let(let_)
            }
            ast => {
                let fresh_id = self.next_id();
                let typ = Rc::new(typ);

                let variable = Atom::Variable(Variable {
                    definition_id: fresh_id,
                    typ: typ.clone(),
                    name: self.default_name.clone(),
                });

                Ast::Let(mir::Let {
                    variable: fresh_id,
                    name: self.default_name.clone(),
                    expr: Box::new(ast),
                    body: Box::new(f(self, variable)),
                    typ,
                })
            }
        }
    }

    pub fn let_binding_atom(&mut self, typ: Type, ast: Ast, f: impl FnOnce(&mut Self, Atom) -> Atom) -> Ast {
        self.let_binding(typ, ast, move |this, atom| Ast::Atom(f(this, atom)))
    }

    pub fn new_effect(&mut self, id: DefinitionId, handler: Atom, handler_type: Type) -> Effect {
        // This could use its own counter instead of sharing next_id, but this does no harm.
        let time_stamp = self.next_id;
        self.next_id += 1;
        Effect { id, handler, handler_type, time_stamp }
    }

    /// reify converts a compile-time (static) term to a runtime (residual) term.
    ///
    /// Reify(ts) : C(t, ts, true) -> C(t, ts, false) 
    /// Reify([], s) = s
    /// Reify([ts.., t], s) = fn k -> Reify(ts, s @@ (fn x => Reflect(ts, k @ x)))
    pub fn reify(&mut self, effects: &[Effect], s: Atom) -> Atom {
        match effects.split_last() {
            None => s,
            Some((_t, ts)) => {
                // What is the type of `k` here?
                let k_type = Context::placeholder_function_type();
                let k = self.anonymous_variable("reify_k", Type::Function(k_type.clone()));

                let lambda_type = Context::placeholder_function_type();

                Context::lambda(vec![k.clone()], lambda_type, {
                    // What is the type of 'x' here?
                    let x = self.anonymous_variable("reify_x", Type::unit());

                    let reify_inner = Context::ct_lambda(vec![x.clone()], {
                        let inner_call = Ast::rt_call1(Atom::Variable(k), Atom::Variable(x), k_type);
                        // What type should `inner_call` have?
                        self.let_binding_atom(Type::unit(), inner_call, |this, inner_call| {
                            this.reflect(ts, inner_call)
                        })
                    });

                    let call = Ast::ct_call1(s, reify_inner);
                    self.let_binding_atom(Type::unit(), call, |this, call| {
                        this.reify(ts, call)
                    })
                })
            },
        }
    }

    /// reflect converts a runtime (residual) term to a compile-time (static) term.
    ///
    /// Reflect(ts) : C(t, ts, false) -> C(t, ts, true) 
    /// Reflect([], s) = s
    /// Reflect([ts.., t], s) = fn k => Reflect(ts, s @ (fn x -> Reify(ts, k @@ x)))
    pub fn reflect(&mut self, effects: &[Effect], s: Atom) -> Atom {
        match effects.split_last() {
            None => s,
            Some((_t, ts)) => {
                // What is the type of `k` here?
                let k_type = Context::placeholder_function_type();
                let k = self.anonymous_variable("reflect_k", Type::Function(k_type.clone()));

                Context::ct_lambda(vec![k.clone()], {
                    let x = self.anonymous_variable("reflect_x", Type::unit());
                    let lambda_type = Context::placeholder_function_type();

                    let reflect_inner = Context::lambda(vec![x.clone()], lambda_type.clone(), {
                        let inner_call = Ast::ct_call1(Atom::Variable(k), Atom::Variable(x));

                        // What type should `inner_call` have?
                        self.let_binding_atom(Type::unit(), inner_call, |this, inner_call| {
                            this.reify(ts, inner_call)
                        })
                    });

                    let call = Ast::rt_call1(s, reflect_inner, lambda_type);

                    self.let_binding_atom(Type::unit(), call, |this, call| {
                        this.reflect(ts, call)
                    })
                })
            },
        }
    }
}
