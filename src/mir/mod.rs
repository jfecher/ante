//! This file implements the pass to convert Hir into Mir.
//! This is done following the "Translation of Statements" and "Translation of Expressions"
//! algorithms in https://se.cs.uni-tuebingen.de/publications/schuster19zero.pdf.
//!
//! Since Ante does not distinguish between expressions and statements however, both
//! the `E` and `S` functions in the paper correspond to the `cps_ast` function in this
//! file. Additionally, since expressions in Ante may themselves contain expressions that
//! the paper considers to be statements, almost all functions take an `EffectStack` parameter
//! instead of just functions operating on statement nodes.
//!
//! In addition to implementations for `E` and `S`, this file also implements `H` to
//! convert effect handlers. In this file, it is named `convert_effect`. Implementations
//! for `T` and `C` for converting types can be found in `src/mir/context.rs`.
//!
//! Where possible, functions in this file will document their corresponding case from the
//! paper, although since Ante is a larger language, many do not have corresponding cases.
//! Additionally, there are some notation changes from the linked paper as well:
//!
//! - Subscript arguments are converted into normal function arguments. E.g. `S(e)_ts` -> `S(e, ts)`.
//! - Since color cannot be used in doc comments, a different notation is used to distinguish
//!   compile-time terms from runtime terms:
//!
//!   - For function types, a runtime function type is denoted by `a -> b` where a
//!     compile-time function type uses `a => b`.
//!
//!   - For lambda values, `fn x -> e` is runtime, and `fn x => e` is a compile-time abstraction.
//!
//!   - For function calls, `f @ x` is runtime, and  `f @@ x` is a compile-time call.
//!
//!   - For the `C` function, an extra boolean parameter is added. This parameter is `true` if 
//!     `C` refers to the compile-time `C` rather than the runtime version. This parameter is in
//!     addition to the change of making the subscript effect stack a parameter to `C` as well.
//!     So a call to (red) `C[t]_ts` will translate to `C(t, ts, false)`, and a call to (blue)
//!     `C[t]_ts` will translate to `C(t, ts, true)`
//!
//!   Unless the term falls into one of the above cases, it is considered to be a runtime term.
use std::rc::Rc;

use self::ir::{Mir, Expr, FunctionId, Type};
use self::context::{Context, EffectStack};
use crate::hir::DecisionTree;
use crate::{hir, util::fmap};

pub mod ir;
mod context;
mod printer;
mod interpreter;
mod convert_to_hir;
mod evaluate;

pub fn convert_to_mir(hir: hir::Ast) -> Mir {
    let mut context = Context::new();
    context.definition_queue.push_front((Mir::main_id(), Rc::new(hir), Vec::new()));

    while let Some((id, ast, effects)) = context.definition_queue.pop_front() {
        context.expected_function_id = Some(id.clone());
        context.cps_ast(&ast, &effects);
    }

    context.mir
}

impl Context {
    fn cps_ast(&mut self, statement: &hir::Ast, effects: &EffectStack) -> Expr {
        match statement {
            hir::Ast::Literal(literal) => Self::cps_literal(literal),
            hir::Ast::Variable(variable) => self.cps_variable(variable, effects),
            hir::Ast::Lambda(lambda) => self.cps_lambda(lambda, effects, None),
            hir::Ast::FunctionCall(call) => self.cps_call(call, effects),
            hir::Ast::Definition(definition) => self.cps_definition(definition, effects),
            hir::Ast::If(if_expr) => self.cps_if(if_expr, effects),
            hir::Ast::Match(match_expr) => self.cps_match(match_expr, effects),
            hir::Ast::Return(return_expr) => self.cps_return(return_expr, effects),
            hir::Ast::Sequence(sequence) => self.cps_sequence(sequence, effects),
            hir::Ast::Extern(extern_reference) => self.cps_extern(extern_reference),
            hir::Ast::Assignment(assign) => self.cps_assign(assign, effects),
            hir::Ast::MemberAccess(access) => self.cps_member_access(access, effects),
            hir::Ast::Tuple(tuple) => self.cps_tuple(tuple, effects),
            hir::Ast::ReinterpretCast(reinterpret_cast) => self.cps_reinterpret_cast(reinterpret_cast, effects),
            hir::Ast::Builtin(builtin) => self.cps_builtin(builtin, effects),
            hir::Ast::Effect(effect) => self.cps_effect(effect, effects),
            hir::Ast::Handle(handle) => self.cps_handle(handle, effects),
        }
    }

    fn cps_literal(literal: &hir::Literal) -> Expr {
        Expr::Literal(literal.clone())
    }

    fn cps_variable(&mut self, variable: &hir::Variable, effects: &EffectStack) -> Expr {
        self.get_definition(variable.definition_id, effects).unwrap_or_else(|| {
            self.add_global_to_queue(variable.clone(), effects.clone())
        })
    }

    /// E((fn x -> s) : t -> t' can eff) = fn x -> Reify(eff, S(s, eff))
    ///
    /// Handler abstraction:
    /// E([c : [F]τ] ⇒ e) = fn c => E(e)
    ///
    /// Note that effect arguments (handler abstractions) are on the outside of any letrecs of lambdas.
    fn cps_lambda(&mut self, lambda: &hir::Lambda, effects: &EffectStack, id: Option<hir::DefinitionId>) -> Expr {
        let effect_types = fmap(&lambda.typ.effects, |effect| self.convert_type(&effect.typ));
        let effect_ids = fmap(&lambda.typ.effects, |effect| effect.id);
        let name = Rc::new("lambda".to_string());

        // Reorder the effects if needed to match the lambda's effect type ordering
        let new_effects = fmap(&lambda.typ.effects, |effect| {
            let handler_type = effects.iter().find(|e| e.0 == effect.id).unwrap().1.clone();
            (effect.id, handler_type)
        });

        let lambda_body = |this: &mut Self| {
            let parameter_types = fmap(&lambda.args, |arg| this.convert_type(&arg.typ));
            let parameter_ids = lambda.args.iter().map(|arg| arg.definition_id);

            this.recursive_function(name.clone(), parameter_ids, parameter_types, new_effects.clone(), id, |this| {
                let body = this.cps_ast(&lambda.body, &new_effects);
                this.reify(&new_effects, body)
            })
        };

        if effect_types.is_empty() {
            lambda_body(self)
        } else {
            self.new_function(name.clone(), effect_ids.into_iter(), effect_types, new_effects.clone(), true, |this| {
                lambda_body(this)
            })
        }
    }

    /// S(e(e'), ts) = Reflect(ts, E(e) @ E(e'))
    ///
    /// E(e[h] : ts) = E(e) @@ H(h, ts)
    fn cps_call(&mut self, call: &hir::FunctionCall, effects: &EffectStack) -> Expr {
        let mut result = self.cps_ast(&call.function, effects);

        // E(e[h] : ts) = E(e) @@ H(h, ts)
        for effect in &call.function_type.effects {
            let effect = EffectAst::Variable(effect.id);
            let handler = self.convert_effect(effect, effects);

            // TODO: Remove this hack
            if result != handler {
                result = Expr::ct_call(result, handler)
            }
        }

        // S(e(e'), ts) = Reflect(ts, E(e) @ E(e'))
        for arg in &call.args {
            let arg = self.cps_ast(arg, effects);
            result = Expr::rt_call(result, arg);
            result = self.reflect(effects, result);
        }

        result
    }

    /// S(val x <- s; s', []) = let x = S(s, []) in S(s', [])
    /// S(val x <- s; s', [ts, t]) =
    ///     fn k -> S(s, [ts, t]) @ (fn x -> S(s', [ts, t]) @ k)
    fn cps_definition(&mut self, definition: &hir::Definition, effects: &EffectStack) -> Expr {
        let rhs = match definition.expr.as_ref() {
            hir::Ast::Lambda(lambda) => self.cps_lambda(lambda, effects, Some(definition.variable)),
            hir::Ast::Effect(effect) => {
                // Monomorphization wraps effects in an extra function, which itself is effectful.
                // So we need to return an `id` lambda since this pass will see the effect and
                // automatically try to thread the handler to itself.
                // TODO: Is this ever needed?
                let typ = self.convert_type(&effect.typ);
                let ret = self.intermediate_function("effect", typ, true, |_, arg| arg);
                eprintln!("!! Inserting id for effect {}", effect.id);
                ret
            },
            other => self.cps_ast(other, effects),
        };

        self.insert_definition(definition.variable, rhs, effects.clone());
        Expr::unit()
    }

    fn cps_if(&mut self, if_expr: &hir::If, effects: &EffectStack) -> Expr {
        let cond = self.cps_ast(&if_expr.condition, effects);
        let then = self.cps_ast(&if_expr.then, effects);
        let otherwise = self.cps_ast(&if_expr.otherwise, effects);
        Expr::If(Box::new(cond), Box::new(then), Box::new(otherwise))
    }

    fn cps_match(&mut self, match_expr: &hir::Match, effects: &EffectStack) -> Expr {
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
        //     self.set_function_body(Expr::Function(end.clone()), vec![result]);
        // }

        // self.current_function_id = end.clone();
        // Expr::Parameter(ParameterId {
        //     function: end,
        //     parameter_index: 0,
        // })
    }

    fn cps_decision_tree(&mut self, tree: &DecisionTree, leaves: &[FunctionId]) {
        todo!("cps_tree")
        // match tree {
        //     DecisionTree::Leaf(leaf_index) => {
        //         let function = Expr::Function(leaves[*leaf_index].clone());
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

        //         let switch = Expr::Switch(case_functions, else_function);

        //         self.current_function_id = original_function;
        //         self.set_function_body(switch, vec![tag]);
        //     },
        // }
    }

    /// S(return e, []) = E(e)
    /// S(return e, [ts, t]) = fn k -> k @ E(e)
    fn cps_return(&mut self, return_expr: &hir::Return, effects: &EffectStack) -> Expr {
        let result_type = self.convert_type(&return_expr.typ);
        let expr = self.cps_ast(&return_expr.expression, effects);
        self.cps_return_helper(expr, effects, result_type)
    }

    /// S(return e, []) = E(e)
    /// S(return e, [ts, t]) = fn k => k @@ E(e)
    fn cps_return_helper(&mut self, expr: Expr, effects: &EffectStack, result_type: Type) -> Expr {
        if effects.is_empty() {
            expr
        } else {
            let k_type = Type::Function(Box::new(result_type), None, true);
            self.intermediate_function("return_k", k_type, true, |this, k| Expr::ct_call(k, expr))
        }
    }

    fn cps_sequence(&mut self, sequence: &hir::Sequence, effects: &EffectStack) -> Expr {
        if effects.is_empty() {
            self.cps_statements_pure(&sequence.statements)
        } else {
            // TODO: Use real type here
            let result_type = Type::unit();
            self.cps_statements_effectful(&sequence.statements, effects, result_type)
        }
    }

    /// The rules for sequencing are somewhat mixed with the rules for let bindings
    ///
    /// S(val x <- s; s', [])
    ///     = (fn x -> S(s', [])) @ S(s, [])
    ///
    /// If there is only 1 statement it is interpreted as a return
    ///
    /// S(return e, []) = E(e)
    fn cps_statements_pure(&mut self, statements: &[hir::Ast]) -> Expr {
        if statements.is_empty() {
            Expr::unit()
        } else if statements.len() == 1 {
            self.cps_ast(&statements[0], &Vec::new())
        } else {
            let first = &statements[0];
            let rest = &statements[1..];

            if let hir::Ast::Definition(definition) = first {
                let definition_rhs = self.cps_ast(&definition.expr, &Vec::new());

                let argument_types = vec![self.convert_type(&definition.typ)];
                let parameters = std::iter::once(definition.variable);

                let name = Rc::new("let_statement".to_string());

                let lambda = self.new_function(name, parameters, argument_types, Vec::new(), false, |this| {
                    this.cps_statements_pure(rest)
                });

                Expr::rt_call(lambda, definition_rhs)
            } else {
                let first = self.cps_ast(&first, &Vec::new());

                let lambda = self.intermediate_function("statement", Type::unit(), false, |this, _| {
                    this.cps_statements_pure(rest)
                });
                Expr::rt_call(lambda, first)
            }
        }
    }

    /// The rules for sequencing are somewhat mixed with the rules for let bindings
    ///
    /// S(val x <- s; s', [ts, t])
    ///     = fn k => S(s, [ts, t]) @@ (fn x => S(s', [ts, t]) @@ k)
    ///
    /// If there is only 1 statement it is interpreted as a return
    ///
    /// S(return e, [ts, t]) = fn k => k @@ E(e)
    fn cps_statements_effectful(&mut self, statements: &[hir::Ast], effects: &EffectStack, result_type: Type) -> Expr {
        if statements.is_empty() {
            Expr::unit()
        } else if statements.len() == 1 {
            let k_type = Type::continuation(result_type, true);
            let body = self.cps_ast(&statements[0], effects);

            self.intermediate_function("eff_statement_return_k", k_type, true, |_, k| Expr::ct_call(k, body))
        } else {
            let first = &statements[0];
            let rest = &statements[1..];

            if let hir::Ast::Definition(definition) = first {
                let definition_rhs = self.cps_ast(&definition.expr, effects);

                // TODO: What is the type of 'k' here?
                let k_type = Type::continuation(Type::unit(), true);
                let x_type = self.convert_type(&definition.typ);

                self.intermediate_function("eff_let_statement_k", k_type, true, |this, k| {
                    let inner_lambda = this.intermediate_function("eff_let_statement", x_type, true, |this, _x| {
                        let rest = this.cps_statements_effectful(rest, effects, result_type);
                        Expr::ct_call(rest, k)
                    });

                    Expr::ct_call(definition_rhs, inner_lambda)
                })
            } else {
                let first = self.cps_ast(&first, effects);
                let rest = self.cps_statements_effectful(rest, effects, result_type);

                // TODO: What is the type of 'k' here?
                let k_type = Type::continuation(Type::unit(), true);
                let x_type = Type::unit();

                self.intermediate_function("eff_statement_k", k_type, true, |this, k| {
                    let inner_lambda = this.intermediate_function("eff_statement", x_type, true, |_, _x| {
                        Expr::ct_call(rest, k)
                    });

                    Expr::ct_call(first, inner_lambda)
                })
            }
        }
    }

    fn cps_extern(&mut self, extern_reference: &hir::Extern) -> Expr {
        let id = self.import_extern(&extern_reference.name, &extern_reference.typ);
        Expr::Extern(id)
    }

    fn cps_assign(&mut self, assign: &hir::Assignment, effects: &EffectStack) -> Expr {
        todo!("cps_assign")
        // let lhs = assign.lhs.to_expr(self);
        // let rhs = assign.rhs.to_expr(self);

        // let unit = hir::Type::Primitive(hir::PrimitiveType::Unit);
        // self.with_next_function(&unit, &[], |this, k| {
        //     this.set_function_body(Expr::Assign, vec![lhs, rhs, k]);
        //     Expr::Literal(Literal::Unit)
        // })
    }

    fn cps_member_access(&mut self, access: &hir::MemberAccess, effects: &EffectStack) -> Expr {
        let lhs = Box::new(self.cps_ast(&access.lhs, effects));
        let typ = self.convert_type(&access.typ);
        Expr::MemberAccess(lhs, access.member_index, typ)
    }

    fn cps_tuple(&mut self, tuple: &hir::Tuple, effects: &EffectStack) -> Expr {
        Expr::Tuple(fmap(&tuple.fields, |field| self.cps_ast(field, effects)))
    }

    fn cps_reinterpret_cast(&mut self, reinterpret_cast: &hir::ReinterpretCast, effects: &EffectStack) -> Expr {
        let value = Box::new(self.cps_ast(&reinterpret_cast.lhs, effects));
        let typ = self.convert_type(&reinterpret_cast.target_type);
        Expr::Transmute(value, typ)
    }

    fn cps_builtin(&mut self, builtin: &hir::Builtin, effects: &EffectStack) -> Expr {
        let binary_fn = |f: fn(_, _) -> _, context: &mut Context, lhs: &hir::Ast, rhs: &hir::Ast| {
            let lhs = Box::new(context.cps_ast(lhs, effects));
            let rhs = Box::new(context.cps_ast(rhs, effects));
            f(lhs, rhs)
        };

        let unary_fn = |f: fn(_) -> _, context: &mut Self, lhs| {
            f(Box::new(context.cps_ast(lhs, effects)))
        };

        let unary_fn_with_type = |f: fn(_, _) -> _, context: &mut Self, lhs, rhs| {
            let lhs = Box::new(context.cps_ast(lhs, effects));
            let rhs = context.convert_type(rhs);
            f(lhs, rhs)
        };

        match builtin {
            hir::Builtin::AddInt(lhs, rhs) => binary_fn(Expr::AddInt, self, lhs, rhs),
            hir::Builtin::AddFloat(lhs, rhs) => binary_fn(Expr::AddFloat, self, lhs, rhs),
            hir::Builtin::SubInt(lhs, rhs) => binary_fn(Expr::SubInt, self, lhs, rhs),
            hir::Builtin::SubFloat(lhs, rhs) => binary_fn(Expr::SubFloat, self, lhs, rhs),
            hir::Builtin::MulInt(lhs, rhs) => binary_fn(Expr::MulInt, self, lhs, rhs),
            hir::Builtin::MulFloat(lhs, rhs) => binary_fn(Expr::MulFloat, self, lhs, rhs),
            hir::Builtin::DivSigned(lhs, rhs) => binary_fn(Expr::DivSigned, self, lhs, rhs),
            hir::Builtin::DivUnsigned(lhs, rhs) => binary_fn(Expr::DivUnsigned, self, lhs, rhs),
            hir::Builtin::DivFloat(lhs, rhs) => binary_fn(Expr::DivFloat, self, lhs, rhs),
            hir::Builtin::ModSigned(lhs, rhs) => binary_fn(Expr::ModSigned, self, lhs, rhs),
            hir::Builtin::ModUnsigned(lhs, rhs) => binary_fn(Expr::ModUnsigned, self, lhs, rhs),
            hir::Builtin::ModFloat(lhs, rhs) => binary_fn(Expr::ModFloat, self, lhs, rhs),
            hir::Builtin::LessSigned(lhs, rhs) => binary_fn(Expr::LessSigned, self, lhs, rhs),
            hir::Builtin::LessUnsigned(lhs, rhs) => binary_fn(Expr::LessUnsigned, self, lhs, rhs),
            hir::Builtin::LessFloat(lhs, rhs) => binary_fn(Expr::LessFloat, self, lhs, rhs),
            hir::Builtin::EqInt(lhs, rhs) => binary_fn(Expr::EqInt, self, lhs, rhs),
            hir::Builtin::EqFloat(lhs, rhs) => binary_fn(Expr::EqFloat, self, lhs, rhs),
            hir::Builtin::EqChar(lhs, rhs) => binary_fn(Expr::EqChar, self, lhs, rhs),
            hir::Builtin::EqBool(lhs, rhs) => binary_fn(Expr::EqBool, self, lhs, rhs),
            hir::Builtin::SignExtend(lhs, typ) => unary_fn_with_type(Expr::SignExtend, self, lhs, typ),
            hir::Builtin::ZeroExtend(lhs, typ) => unary_fn_with_type(Expr::ZeroExtend, self, lhs, typ),
            hir::Builtin::SignedToFloat(lhs, typ) => unary_fn_with_type(Expr::SignedToFloat, self, lhs, typ),
            hir::Builtin::UnsignedToFloat(lhs, typ) => unary_fn_with_type(Expr::UnsignedToFloat, self, lhs, typ),
            hir::Builtin::FloatToSigned(lhs, typ) => unary_fn_with_type(Expr::FloatToSigned, self, lhs, typ),
            hir::Builtin::FloatToUnsigned(lhs, typ) => unary_fn_with_type(Expr::FloatToUnsigned, self, lhs, typ),
            hir::Builtin::FloatPromote(value, typ) => unary_fn_with_type(Expr::FloatPromote, self, value, typ),
            hir::Builtin::FloatDemote(value, typ) => unary_fn_with_type(Expr::FloatDemote, self, value, typ),
            hir::Builtin::BitwiseAnd(lhs, rhs) => binary_fn(Expr::BitwiseAnd, self, lhs, rhs),
            hir::Builtin::BitwiseOr(lhs, rhs) => binary_fn(Expr::BitwiseOr, self, lhs, rhs),
            hir::Builtin::BitwiseXor(lhs, rhs) => binary_fn(Expr::BitwiseXor, self, lhs, rhs),
            hir::Builtin::BitwiseNot(value) => unary_fn(Expr::BitwiseNot, self, value),
            hir::Builtin::Truncate(lhs, typ) => unary_fn_with_type(Expr::Truncate, self, lhs, typ),
            hir::Builtin::Deref(lhs, typ) => unary_fn_with_type(Expr::Deref, self, lhs, typ),
            hir::Builtin::Transmute(lhs, typ) => unary_fn_with_type(Expr::Transmute, self, lhs, typ),
            hir::Builtin::StackAlloc(value) => unary_fn(Expr::StackAlloc, self, value),
            hir::Builtin::Offset(lhs, rhs, typ) => {
                let lhs = Box::new(self.cps_ast(lhs, effects));
                let rhs = Box::new(self.cps_ast(rhs, effects));
                let typ = self.convert_type(typ);
                Expr::Offset(lhs, rhs, typ)
            },
        }
    }

    /// The rule for converting effect calls:
    ///
    /// S(do h(e), ts) = H(h, ts) @@ E(e)
    ///
    /// TODO: Need to ensure effects are ct_call
    ///
    /// Has been adapted here since this effect node excludes the arguments:
    ///
    /// S(h, ts) = H(h, ts)
    fn cps_effect(&mut self, effect: &hir::Effect, effects: &EffectStack) -> Expr {
        self.convert_effect(EffectAst::Variable(effect.id), effects)
    }

    /// S(handle c = h in s : t, ts)
    ///   = (fn c => S(s, [ts, t]) @@ (fn x => S(return x, ts))) @@ H(h, [ts, t])
    fn cps_handle(&mut self, handle: &hir::Handle, effects: &EffectStack) -> Expr {
        let mut new_effects = effects.to_vec();
        let result_type = self.convert_type(&handle.result_type);
        new_effects.push((handle.effect.id, result_type.clone()));

        let handler = self.convert_effect(EffectAst::Handle(handle), &new_effects);

        let id = std::iter::once(handle.effect.id);
        let c_type = self.convert_type(&handle.effect.typ);

        let name = Rc::new("handle_expression".to_string());

        let c_lambda = self.new_function(name, id, vec![c_type], new_effects.clone(), true, |this| {
            let expression = this.cps_ast(&handle.expression, &new_effects);

            let k = this.intermediate_function("handle_k", result_type.clone(), true, |this, x| {
                this.cps_return_helper(x, effects, result_type)
            });

            Expr::ct_call(expression, k)
        });

        Expr::ct_call(c_lambda, handler)
    }

    /// H(c, ts) = c
    /// H(F(x, k) -> s, [ts, t]) = fn x => fn k => S(s, ts)
    /// H(lift h, [t]) = fn x => fn k => k @@ (H(h, []) @@ x)
    /// H(lift h, [ts, t, t'])
    ///   = fn x => fn k => fn k' => H(h, [ts, t]) @@ x @@ (fn y => k @@ y @@ k')
    fn convert_effect(&mut self, effect: EffectAst, effects: &EffectStack) -> Expr {
        match effect {
            // H(c, ts) = c
            // TODO: implement lift cases
            EffectAst::Variable(id) => {
                self.get_definition(id, effects).unwrap_or_else(|| 
                    panic!("No handler for effect {}", id))
            },
            // H(F(x, k) -> s, [ts, t]) = fn x => fn k => S(s, ts)
            EffectAst::Handle(handle) => {
                let argument_types = fmap(&handle.branch_body.args, |arg| self.convert_type(&arg.typ));
                let argument_ids = handle.branch_body.args.iter().map(|arg| arg.definition_id);

                // TODO: assert effects.pop() == t
                let mut effects = effects.to_vec();
                effects.pop();
                let name = Rc::new("handle".to_string());

                self.new_function(name, argument_ids, argument_types, effects.clone(), true, |this| {
                    let k_id = std::iter::once(handle.resume.definition_id);
                    let k_type = vec![this.convert_type(&handle.resume.typ)];

                    let name = Rc::new("handle_k".to_string());

                    this.new_function(name, k_id, k_type, effects.clone(), true, |this| {
                        this.cps_ast(&handle.branch_body.body, &effects)
                    })
                })
            },
        }
    }
}

enum EffectAst<'ast> {
    Variable(hir::DefinitionId),
    Handle(&'ast hir::Handle),
}
