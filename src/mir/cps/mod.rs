use std::collections::BTreeMap;

use crate::{hir::{self, Type}, util::fmap};

use self::effect_stack::EffectStack;

use super::ir::{ self as mir, Ast, Mir, Atom };
use context::Context;

mod context;
mod effect_stack;

impl Mir {
    pub fn convert_to_cps(self) -> Mir {
        let mut context = Context::new(1);
        let mut new_mir = Mir {
            main: hir::DefinitionId(0),
            functions: BTreeMap::new(),
            next_id: 1,
        };

        let main = &self.functions[&self.main];
        let new_main = context.cps_statement(main, &EffectStack::default());
        new_mir.functions.insert(new_mir.main, new_main);

        while let Some((source, destination, effects)) = context.definition_queue.pop_front() {
            println!("CPS'ing {} -> {} with {} effects", source, destination, effects.len());
            context.local_definitions.clear();

            let function = &self.functions[&source];
            let new_function = context.cps_statement(function, &effects);

            new_mir.functions.insert(destination, new_function);
        }

        new_mir.next_id = context.next_id;
        new_mir
    }
}

impl Context {
    /// S(e(e'), ts) = Reflect(ts, E(e) @ E(e'))
    ///
    /// S(val x <- s; s', [])
    ///     = (fn x -> S(s', [])) @ S(s, [])
    ///
    /// S(val x <- s; s', [ts, t])
    ///     = fn k => S(s, [ts, t]) @@ (fn x => S(s', [ts, t]) @@ k)
    ///
    /// S(return e, []) = E(e)
    /// S(return e, [ts, t]) = fn k => k @@ E(e)
    ///
    /// S(handle c = h in s : t, ts)
    ///   = (fn c => S(s, [ts, t]) @@ (fn x => S(return x, ts))) @@ H(h, [ts, t])
    fn cps_statement(&mut self, statement: &Ast, effects: &EffectStack) -> Ast {
        match statement {
            Ast::FunctionCall(call) => self.cps_call(&call, effects),
            Ast::Let(let_) => self.cps_let(let_, effects),
            Ast::Return(return_expr) => self.cps_return(&return_expr.expression, &return_expr.typ, effects),
            Ast::Handle(handle) => self.cps_handle(handle, effects),

            // Each case from now on is an extension of the original rules
            Ast::Atom(atom) => Ast::Atom(self.cps_atom(atom, effects)),
            Ast::If(if_expr) => self.cps_if(if_expr, effects),
            Ast::Match(match_expr) => self.cps_match(match_expr, effects),
            Ast::Assignment(assign) => self.cps_assign(assign, effects),
            Ast::MemberAccess(access) => self.cps_member_access(access, effects),
            Ast::Tuple(tuple) => self.cps_tuple(tuple, effects),
            Ast::Builtin(builtin) => self.cps_builtin(builtin, effects),
        }
    }

    fn cps_atom(&mut self, atom: &Atom, effects: &EffectStack) -> Atom {
        match atom {
            Atom::Literal(literal) => Self::cps_literal(literal),
            Atom::Variable(variable) => self.cps_variable(variable, effects),
            Atom::Lambda(lambda) => self.cps_lambda(lambda, effects, None),
            Atom::Extern(extern_reference) => Self::cps_extern(extern_reference),
            Atom::Effect(effect) => self.cps_effect(effect),
        }
    }

    fn cps_literal(literal: &hir::Literal) -> Atom {
        Atom::Literal(literal.clone())
    }

    fn cps_variable(&mut self, variable: &mir::Variable, effects: &EffectStack) -> Atom {
        self.get_definition(variable.definition_id, effects).unwrap_or_else(|| {
            Atom::Variable(self.add_global_to_queue(variable.clone(), effects.clone()))
        })
    }

    /// E((fn x -> s) : t -> t' can eff) = fn x -> Reify(eff, S(s, eff))
    ///
    /// Handler abstraction:
    /// E([c : [F]τ] ⇒ e) = fn c => E(e)
    ///
    /// Note that effect arguments (handler abstractions) are on the outside of any letrecs of lambdas.
    fn cps_lambda(&mut self, lambda: &mir::Lambda, effects: &EffectStack, _id: Option<hir::DefinitionId>) -> Atom {
        // Reorder the effects if needed to match the lambda's effect type ordering
        let new_effects = EffectStack::new(fmap(&lambda.typ.effects, |effect| {
            let handler_type = effects.find(effect.id).handler_type.clone();
            let effect_type = self.convert_type(&effect.typ);
            let handler = self.anonymous_variable("effect", effect_type);
            self.new_effect(effect.id, Atom::Variable(handler), handler_type)
        }));

        // TODO: Restore old definitions of effect ids
        let effect_args = fmap(&new_effects, |effect| match &effect.handler {
            Atom::Variable(variable) => variable.clone(),
            _ => unreachable!("Effect arguments to a lambda are always variables"),
        });

        let parameters = fmap(&lambda.args, |arg| self.new_local_from_existing(arg));

        let body = Context::lambda(parameters, lambda.typ.clone(), {
            let body = self.cps_statement(&lambda.body, &new_effects);

            self.let_binding_atom(lambda.typ.return_type.as_ref().clone(), body, |this, body| {
                this.reify(new_effects.as_slice(), body)
            })
        });

        // TODO: set definition to body for recursive calls (not \effect_args -> body)

        if effect_args.is_empty() {
            body
        } else {
            Context::ct_lambda(effect_args, Ast::Atom(body))
        }
    }

    /// S(e(e'), ts) = Reflect(ts, E(e) @ E(e'))
    ///
    /// E(e[h] : ts) = E(e) @@ H(h, ts)
    fn cps_call(&mut self, call: &mir::FunctionCall, effects: &EffectStack) -> Ast {
        let mut result = Ast::Atom(self.cps_atom(&call.function, effects));

        // E(e[h] : ts) = E(e) @@ H(h, ts)
        for call_effect in &call.function_type.effects {
            let handler = effects.find(call_effect.id).handler.clone();

            // What type should be used here?
            result = self.let_binding(Type::unit(), result, |_, result| {
                Ast::ct_call1(result, handler)
            })
        }

        // S(e(e'), ts) = Reflect(ts, E(e) @ E(e'))
        let args = fmap(&call.args, |arg| self.cps_atom(arg, effects));

        let result = self.let_binding(Type::Function(call.function_type.clone()), result, |_, result| {
            Ast::rt_call(result, args, call.function_type.clone())
        });

        self.let_binding_atom(call.function_type.return_type.as_ref().clone(), result, |this, result| {
            this.reflect(effects.as_slice(), result)
        })
    }

    fn cps_if(&mut self, if_expr: &mir::If, effects: &EffectStack) -> Ast {
        let condition = self.cps_atom(&if_expr.condition, effects);
        let then = self.cps_statement(&if_expr.then, effects);
        let otherwise = self.cps_statement(&if_expr.otherwise, effects);

        Ast::If(mir::If {
            condition,
            then: Box::new(then),
            otherwise: Box::new(otherwise),
            result_type: if_expr.result_type.clone(),
        })
    }

    fn cps_match(&mut self, _match_expr: &mir::Match, _effects: &EffectStack) -> Ast {
        todo!("cps_match")
        // let original_function = self.current_function_id.clone();
        // let leaves = fmap(&match_expr.branches, |_| self.next_fresh_function());

        // // Codegen the switches first to eventually jump to each leaf
        // self.current_function_id = original_function;
        // self.cps_decision_tree(&match_expr.decision_tree, &leaves);

        // let end = self.next_fresh_function();
        // self.add_parameter(&match_expr.result_type);

        // // Now codegen each leaf, all jumping to the same end continuation afterward
        // for (leaf_hir, leaf_function) in match_expr.branches.iter().zip(leaves) {
        //     self.current_function_id = leaf_function;
        //     let result = self.cps_ast(leaf_hir, effects);
        //     self.set_function_body(Ast::Function(end.clone()), vec![result]);
        // }

        // self.current_function_id = end.clone();
        // Ast::Parameter(ParameterId {
        //     function: end,
        //     parameter_index: 0,
        // })
    }

    fn cps_decision_tree(&mut self, _tree: &mir::DecisionTree) {
        todo!("cps_tree")
        // match tree {
        //     DecisionTree::Leaf(leaf_index) => {
        //         let function = Ast::Function(leaves[*leaf_index].clone());
        //         self.set_function_body(function, vec![]);
        //     },
        //     DecisionTree::Definition(definition, rest) => {
        //         definition.to_expr(self);
        //         self.cps_decision_tree(&rest, leaves);
        //     },
        //     DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
        //         let tag = int_to_switch_on.to_expr(self);
        //         let original_function = self.current_function_id.clone();

        //         let case_functions = fmap(cases, |(tag_to_match, case_tree)| {
        //             let function = self.next_fresh_function();
        //             self.cps_decision_tree(case_tree, leaves);
        //             (*tag_to_match, function)
        //         });

        //         let else_function = else_case.as_ref().map(|else_tree| {
        //             let function = self.next_fresh_function();
        //             self.cps_decision_tree(else_tree, leaves);
        //             function
        //         });

        //         let switch = Ast::Switch(case_functions, else_function);

        //         self.current_function_id = original_function;
        //         self.set_function_body(switch, vec![tag]);
        //     },
        // }
    }

    /// S(return e, []) = E(e)
    /// S(return e, [ts, t]) = fn k -> k @ E(e)
    fn cps_return(&mut self, expression: &Atom, result_type: &hir::Type, effects: &EffectStack) -> Ast {
        let result_type = self.convert_type(result_type);
        let expr = self.cps_atom(expression, effects);
        self.cps_return_helper(expr, effects, result_type)
    }

    /// S(return e, []) = E(e)
    /// S(return e, [ts, t]) = fn k => k @@ E(e)
    fn cps_return_helper(&mut self, expr: Atom, effects: &EffectStack, _result_type: Type) -> Ast {
        if effects.is_empty() {
            Ast::Atom(expr)
        } else {
            let k = self.anonymous_variable("k", Type::Function(Context::placeholder_function_type()));

            Ast::Atom(Context::ct_lambda(vec![k.clone()], {
                Ast::ct_call1(Atom::Variable(k), expr)
            }))
        }
    }

    /// S(val x <- s; s', [])
    ///     = (fn x -> S(s', [])) @ S(s, [])
    ///
    /// S(val x <- s; s', [ts, t])
    ///     = fn k => S(s, [ts, t]) @@ (fn x => S(s', [ts, t]) @@ k)
    fn cps_let(&mut self, let_binding: &mir::Let<Ast>, effects: &EffectStack) -> Ast {
        if effects.is_empty() {
            self.cps_let_binding_pure(let_binding)
        } else {
            self.cps_let_binding_impure(let_binding, effects)
        }
    }

    /// S(val x <- s; s', [])
    ///     = (fn x -> S(s', [])) @ S(s, [])
    ///
    /// The above is equivalent to a regular let binding in lambda calculus,
    /// so cps_let_binding_pure recurs on its arguments but otherwise returns
    /// the same structure.
    fn cps_let_binding_pure(&mut self, let_binding: &mir::Let<Ast>) -> Ast {
        let expr = self.cps_statement(&let_binding.expr, &EffectStack::default());

        self.let_binding(let_binding.typ.as_ref().clone(), expr, |this, atom| {
            this.insert_local_definition(let_binding.variable, atom);
            this.cps_statement(&let_binding.body, &EffectStack::default())
        })
    }

    /// S(val x <- s; s', [ts, t])
    ///     = fn k => S(s, [ts, t]) @@ (fn x => S(s', [ts, t]) @@ k)
    fn cps_let_binding_impure(&mut self, let_binding: &mir::Let<Ast>, effects: &EffectStack) -> Ast {
        let definition_rhs = self.cps_statement(&let_binding.expr, effects);

        // TODO: What is the type of 'k' here?
        let k_type = Type::Function(Context::placeholder_function_type());
        let x_type = let_binding.typ.clone();

        let k = self.anonymous_variable("k", k_type);
        let x = self.fresh_existing_variable(let_binding.name.clone(), x_type);
        let x_type = let_binding.typ.as_ref().clone();

        self.insert_local_definition(let_binding.variable, Atom::Variable(x.clone()));

        Ast::Atom(Context::ct_lambda(vec![k.clone()], {
            let inner_lambda = Context::ct_lambda(vec![x], {
                let rest = self.cps_statement(&let_binding.body, effects);

                // TODO: Fix the type
                self.let_binding(Type::unit(), rest, |_, rest| {
                    Ast::ct_call1(rest, Atom::Variable(k))
                })
            });

            self.let_binding(x_type, definition_rhs, |_, definition_rhs| {
                Ast::ct_call1(definition_rhs, inner_lambda)
            })
        }))
    }

    fn cps_extern(extern_reference: &hir::Extern) -> Atom {
        Atom::Extern(extern_reference.clone())
    }

    fn cps_assign(&mut self, assign: &mir::Assignment, effects: &EffectStack) -> Ast {
        let lhs = self.cps_atom(&assign.lhs, effects);
        let rhs = self.cps_atom(&assign.rhs, effects);

        Ast::Assignment(mir::Assignment { lhs, rhs })
    }

    fn cps_member_access(&mut self, access: &mir::MemberAccess, effects: &EffectStack) -> Ast {
        let lhs = self.cps_atom(&access.lhs, effects);
        let typ = self.convert_type(&access.typ);
        Ast::MemberAccess(mir::MemberAccess { lhs, typ, member_index: access.member_index })
    }

    fn cps_tuple(&mut self, tuple: &mir::Tuple, effects: &EffectStack) -> Ast {
        let fields = fmap(&tuple.fields, |field| self.cps_atom(field, effects));
        Ast::Tuple(mir::Tuple { fields })
    }

    fn cps_builtin(&mut self, builtin: &mir::Builtin, effects: &EffectStack) -> Ast {
        let binary_fn = |f: fn(_, _) -> _, context: &mut Context, lhs, rhs| {
            let lhs = context.cps_atom(lhs, effects);
            let rhs = context.cps_atom(rhs, effects);
            Ast::Builtin(f(lhs, rhs))
        };

        let unary_fn = |f: fn(_) -> _, context: &mut Self, lhs| {
            Ast::Builtin(f(context.cps_atom(lhs, effects)))
        };

        let unary_fn_with_type = |f: fn(_, _) -> _, context: &mut Self, lhs, rhs| {
            let lhs = context.cps_atom(lhs, effects);
            let rhs = context.convert_type(rhs);
            Ast::Builtin(f(lhs, rhs))
        };

        use mir::Builtin::*;
        match builtin {
            AddInt(lhs, rhs) => binary_fn(AddInt, self, lhs, rhs),
            AddFloat(lhs, rhs) => binary_fn(AddFloat, self, lhs, rhs),
            SubInt(lhs, rhs) => binary_fn(SubInt, self, lhs, rhs),
            SubFloat(lhs, rhs) => binary_fn(SubFloat, self, lhs, rhs),
            MulInt(lhs, rhs) => binary_fn(MulInt, self, lhs, rhs),
            MulFloat(lhs, rhs) => binary_fn(MulFloat, self, lhs, rhs),
            DivSigned(lhs, rhs) => binary_fn(DivSigned, self, lhs, rhs),
            DivUnsigned(lhs, rhs) => binary_fn(DivUnsigned, self, lhs, rhs),
            DivFloat(lhs, rhs) => binary_fn(DivFloat, self, lhs, rhs),
            ModSigned(lhs, rhs) => binary_fn(ModSigned, self, lhs, rhs),
            ModUnsigned(lhs, rhs) => binary_fn(ModUnsigned, self, lhs, rhs),
            ModFloat(lhs, rhs) => binary_fn(ModFloat, self, lhs, rhs),
            LessSigned(lhs, rhs) => binary_fn(LessSigned, self, lhs, rhs),
            LessUnsigned(lhs, rhs) => binary_fn(LessUnsigned, self, lhs, rhs),
            LessFloat(lhs, rhs) => binary_fn(LessFloat, self, lhs, rhs),
            EqInt(lhs, rhs) => binary_fn(EqInt, self, lhs, rhs),
            EqFloat(lhs, rhs) => binary_fn(EqFloat, self, lhs, rhs),
            EqChar(lhs, rhs) => binary_fn(EqChar, self, lhs, rhs),
            EqBool(lhs, rhs) => binary_fn(EqBool, self, lhs, rhs),
            SignExtend(lhs, typ) => unary_fn_with_type(SignExtend, self, lhs, typ),
            ZeroExtend(lhs, typ) => unary_fn_with_type(ZeroExtend, self, lhs, typ),
            SignedToFloat(lhs, typ) => unary_fn_with_type(SignedToFloat, self, lhs, typ),
            UnsignedToFloat(lhs, typ) => unary_fn_with_type(UnsignedToFloat, self, lhs, typ),
            FloatToSigned(lhs, typ) => unary_fn_with_type(FloatToSigned, self, lhs, typ),
            FloatToUnsigned(lhs, typ) => unary_fn_with_type(FloatToUnsigned, self, lhs, typ),
            FloatPromote(value, typ) => unary_fn_with_type(FloatPromote, self, value, typ),
            FloatDemote(value, typ) => unary_fn_with_type(FloatDemote, self, value, typ),
            BitwiseAnd(lhs, rhs) => binary_fn(BitwiseAnd, self, lhs, rhs),
            BitwiseOr(lhs, rhs) => binary_fn(BitwiseOr, self, lhs, rhs),
            BitwiseXor(lhs, rhs) => binary_fn(BitwiseXor, self, lhs, rhs),
            BitwiseNot(value) => unary_fn(BitwiseNot, self, value),
            Truncate(lhs, typ) => unary_fn_with_type(Truncate, self, lhs, typ),
            Deref(lhs, typ) => unary_fn_with_type(Deref, self, lhs, typ),
            Transmute(lhs, typ) => unary_fn_with_type(Transmute, self, lhs, typ),
            StackAlloc(value) => unary_fn(StackAlloc, self, value),
            Offset(lhs, rhs, typ) => {
                let lhs = self.cps_atom(lhs, effects);
                let rhs = self.cps_atom(rhs, effects);
                let typ = self.convert_type(typ);
                Ast::Builtin(Offset(lhs, rhs, typ))
            },
        }
    }

    /// The rule for converting effect calls:
    ///
    /// S(do h(e), ts) = H(h, ts) @@ E(e)
    ///
    /// Has been adapted here since Ante's type system includes `can Effect` even
    /// for the original effect function. From this, if we followed the original rule,
    /// we'd return the handler `h`, then a function call would apply `h` to its arguments.
    /// In doing this, it sees that it `can Effect` and will pass in the effect handler `h`
    /// automatically, leading to `h h`.
    ///
    /// To prevent this, instead of translating the effect itself via
    ///
    /// S(h, ts) = H(h, ts)
    ///
    /// We translate it to the identity function `fn x -> x`.
    fn cps_effect(&mut self, effect: &hir::Effect) -> Atom {
        let x = self.anonymous_variable("effect", effect.typ.clone());
        Context::ct_lambda(vec![x.clone()], Ast::Atom(Atom::Variable(x)))
    }

    /// S(handle c = h in s : t, ts)
    ///   = (fn c => S(s, [ts, t]) @@ (fn x => S(return x, ts))) @@ H(h, [ts, t])
    fn cps_handle(&mut self, handle: &mir::Handle, effects: &EffectStack) -> Ast {
        let mut new_effects = effects.clone();
        let result_type = self.convert_type(&handle.result_type);

        // Despite the rule above, the handler is converted with the old effect stack
        // since the rule for converting handlers pops the top effect anyway, and we
        // need the handler to create the newest effect in the stack.
        let handler = self.convert_handler(handle, &new_effects);

        let c_type = self.convert_type(&handle.effect.typ);
        let c = self.anonymous_variable("c", c_type);

        new_effects.push(self.new_effect(handle.effect.id, Atom::Variable(c.clone()), result_type.clone()));

        let c_lambda = Context::ct_lambda(vec![c], {
            let expression = self.cps_statement(&handle.expression, &new_effects);
            let x = self.anonymous_variable("x", result_type.clone());

            // x: result_type
            let k = Context::ct_lambda(vec![x.clone()], {
                self.cps_return_helper(Atom::Variable(x), effects, result_type.clone())
            });

            self.let_binding(result_type, expression, |_, expression| {
                Ast::ct_call1(expression, k)
            })
        });

        Ast::ct_call1(c_lambda, handler)
    }

    /// H(c, ts) = c
    /// H(F(x, k) -> s, [ts, t]) = fn x => fn k => S(s, ts)
    /// H(lift h, [t]) = fn x => fn k => k @@ (H(h, []) @@ x)
    /// H(lift h, [ts, t, t'])
    ///   = fn x => fn k => fn k' => H(h, [ts, t]) @@ x @@ (fn y => k @@ y @@ k')
    ///
    /// Although only:
    ///
    /// H(F(x, k) -> s, [ts, t]) = fn x => fn k => S(s, ts)
    ///
    /// Is used here.
    fn convert_handler(&mut self, handle: &mir::Handle, effects: &EffectStack) -> Atom {
        // These lines aren't needed due to the change in cps_handle where we convert
        // handlers before the new effect is pushed to the EffectStack
        // let mut effects = effects.to_vec();
        // effects.pop();
        let xs = fmap(&handle.branch_args, |arg| self.new_local_from_existing(arg));

        let k = self.new_local(handle.resume.definition_id, "k", handle.resume.typ.as_ref().clone());

        Context::ct_lambda(xs, {
            Ast::Atom(Context::ct_lambda(vec![k], {
                self.cps_statement(&handle.branch_body, &effects)
            }))
        })
    }
}
