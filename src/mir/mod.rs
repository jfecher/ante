//! This file implements the pass to convert Hir into the initial Mir in 
//! capability passing style (not to be confused with continuation passing style).
//!
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

use self::context::{Context, EffectStack};
use crate::hir::{DecisionTree, Type};
use crate::{hir::{self, Ast}, util::fmap};

pub mod ir;
mod context;
mod printer;
mod convert_to_hir;
mod evaluate;
mod optimizations;

pub fn convert_to_mir(hir: Ast, next_id: usize) -> Ast {
    let mut context = Context::new(next_id);
    let main = context.cps_ast(&hir, &Vec::new());

    while let Some((source, destination, effects)) = context.definition_queue.pop_front() {
        context.local_definitions.clear();
        let ast = source.borrow();
        let result = context.cps_ast(&ast, &effects);
        *destination.borrow_mut() = result;
    }

    main
}

/// To match more closely with the syntax in https://se.cs.uni-tuebingen.de/publications/schuster19zero.pdf,
/// effects and handlers are wrapped in this enum which corresponds to cases of `H` in the link.
/// The `lift` cases are determined automatically from the shape of the effect stack.
enum EffectAst<'ast> {
    Variable(hir::DefinitionId),
    Handle(&'ast hir::Handle),
}

/// To match more closely with the syntax in https://se.cs.uni-tuebingen.de/publications/schuster19zero.pdf,
/// statements within a `hir::Sequence` are wrapped in this enum which corresponds to cases of `S` in the link.
#[derive(Debug)]
enum Statement<'ast> {
    Application(&'ast hir::Ast, &'ast [hir::Ast], &'ast hir::FunctionType),
    LetBinding(LetBinding<'ast>),
    Return(&'ast hir::Ast, &'ast hir::Type),
    Handle(&'ast hir::Handle),
}

#[derive(Debug)]
struct LetBinding<'ast> {
    variable: Option<(hir::DefinitionId, &'ast hir::Type)>,
    name: String,
    rhs: Box<Statement<'ast>>,
    body: Box<Statement<'ast>>,
}

impl Context {
    fn cps_ast(&mut self, statement: &hir::Ast, effects: &EffectStack) -> Ast {
        match statement {
            hir::Ast::Literal(literal) => Self::cps_literal(literal),
            hir::Ast::Variable(variable) => self.cps_variable(variable, effects),
            hir::Ast::Lambda(lambda) => self.cps_lambda(lambda, effects, None),
            hir::Ast::FunctionCall(call) => self.cps_call(&call.function, &call.args, &call.function_type, effects),
            hir::Ast::Definition(definition) => self.cps_definition(definition, effects),
            hir::Ast::If(if_expr) => self.cps_if(if_expr, effects),
            hir::Ast::Match(match_expr) => self.cps_match(match_expr, effects),
            hir::Ast::Return(return_expr) => self.cps_return(&return_expr.expression, &return_expr.typ, effects),
            hir::Ast::Sequence(sequence) => self.cps_sequence(sequence, effects),
            hir::Ast::Extern(extern_reference) => Self::cps_extern(extern_reference),
            hir::Ast::Assignment(assign) => self.cps_assign(assign, effects),
            hir::Ast::MemberAccess(access) => self.cps_member_access(access, effects),
            hir::Ast::Tuple(tuple) => self.cps_tuple(tuple, effects),
            hir::Ast::ReinterpretCast(reinterpret_cast) => self.cps_reinterpret_cast(reinterpret_cast, effects),
            hir::Ast::Builtin(builtin) => self.cps_builtin(builtin, effects),
            hir::Ast::Effect(effect) => self.cps_effect(effect, effects),
            hir::Ast::Handle(handle) => self.cps_handle(handle, effects),
        }
    }

    fn cps_literal(literal: &hir::Literal) -> Ast {
        Ast::Literal(literal.clone())
    }

    fn cps_variable(&mut self, variable: &hir::Variable, effects: &EffectStack) -> Ast {
        let variable = self.get_definition(variable.definition_id, effects).unwrap_or_else(|| {
            self.add_global_to_queue(variable.clone(), effects.clone())
        });
        Ast::Variable(variable)
    }

    /// E((fn x -> s) : t -> t' can eff) = fn x -> Reify(eff, S(s, eff))
    ///
    /// Handler abstraction:
    /// E([c : [F]τ] ⇒ e) = fn c => E(e)
    ///
    /// Note that effect arguments (handler abstractions) are on the outside of any letrecs of lambdas.
    fn cps_lambda(&mut self, lambda: &hir::Lambda, effects: &EffectStack, id: Option<hir::DefinitionId>) -> Ast {
        let effect_types = fmap(&lambda.typ.effects, |effect| self.convert_type(&effect.typ));
        let effect_ids = fmap(&lambda.typ.effects, |effect| effect.id);
        let name = Rc::new("lambda".to_string());

        eprintln!("In lambda {}", lambda);

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
            self.new_function(name.clone(), effect_ids.into_iter(), effect_types, true, |this| {
                lambda_body(this)
            })
        }
    }

    /// S(e(e'), ts) = Reflect(ts, E(e) @ E(e'))
    ///
    /// E(e[h] : ts) = E(e) @@ H(h, ts)
    fn cps_call(&mut self, function: &hir::Ast, args: &[hir::Ast], function_type: &hir::FunctionType, effects: &EffectStack) -> Ast {
        let mut result = self.cps_ast(function, effects);

        // E(e[h] : ts) = E(e) @@ H(h, ts)
        for effect in &function_type.effects {
            let effect = EffectAst::Variable(effect.id);
            let handler = self.convert_effect(effect, effects);

            // TODO: Remove this hack
            // if result != handler {
                result = Ast::ct_call1(result, handler)
            // }
        }

        // S(e(e'), ts) = Reflect(ts, E(e) @ E(e'))
        let args = fmap(args, |arg| self.cps_ast(arg, effects));
        result = Ast::rt_call(result, args, function_type.clone());
        self.reflect(effects, result)
    }

    /// S(val x <- s; s', []) = let x = S(s, []) in S(s', [])
    /// S(val x <- s; s', [ts, t]) =
    ///     fn k -> S(s, [ts, t]) @ (fn x -> S(s', [ts, t]) @ k)
    fn cps_definition(&mut self, definition: &hir::Definition, effects: &EffectStack) -> Ast {
        let rhs = match definition.expr.as_ref() {
            hir::Ast::Lambda(lambda) => self.cps_lambda(lambda, effects, Some(definition.variable)),
            hir::Ast::Effect(effect) => {
                // Monomorphization wraps effects in an extra function, which itself is effectful.
                // So we need to return an `id` lambda since this pass will see the effect and
                // automatically try to thread the handler to itself.
                // TODO: Is this ever needed?
                let typ = self.convert_type(&effect.typ);
                self.intermediate_function("effect", typ, true, |_, arg| arg)
            },
            other => self.cps_ast(other, effects),
        };

        self.insert_global_definition(definition.variable, rhs, effects.clone());
        Ast::unit()
    }

    fn cps_if(&mut self, if_expr: &hir::If, effects: &EffectStack) -> Ast {
        let cond = self.cps_ast(&if_expr.condition, effects);
        let then = self.cps_ast(&if_expr.then, effects);
        let otherwise = self.cps_ast(&if_expr.otherwise, effects);

        Ast::If(hir::If {
            condition: Box::new(cond),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
            result_type: if_expr.result_type.clone(),
        })
    }

    fn cps_match(&mut self, match_expr: &hir::Match, effects: &EffectStack) -> Ast {
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

    fn cps_decision_tree(&mut self, tree: &DecisionTree) {
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
    fn cps_return(&mut self, expression: &hir::Ast, result_type: &hir::Type, effects: &EffectStack) -> Ast {
        let result_type = self.convert_type(result_type);
        let expr = self.cps_ast(expression, effects);
        self.cps_return_helper(expr, effects, result_type)
    }

    /// S(return e, []) = E(e)
    /// S(return e, [ts, t]) = fn k => k @@ E(e)
    fn cps_return_helper(&mut self, expr: Ast, effects: &EffectStack, result_type: Type) -> Ast {
        if effects.is_empty() {
            expr
        } else {
            let k = self.anonymous_local("k", Type::Function(Context::placeholder_function_type()));

            Context::ct_lambda(vec![k.clone()], {
                Ast::ct_call1(Ast::Variable(k), expr)
            })
        }
    }

    fn cps_sequence(&mut self, sequence: &hir::Sequence, effects: &EffectStack) -> Ast {
        // convert to a closer syntax to the original source paper first for a more direct translation
        let statements = Self::convert_statements(&sequence.statements);
        self.cps_statement(statements, effects)
    }

    /// Convert hir::Asts to something closer to the target statement syntax:
    ///
    /// s ::= e(e)                application
    ///     | val x ← s; s        sequence
    ///     | return e            return
    ///     | do h(e)             effect call (not included)
    ///     | handle c = h in s   effect handler
    fn convert_statements(statements: &[hir::Ast]) -> Statement {
        match statements {
            [first, _, ..] => {
                let rest = &statements[1..];
                let body = Box::new(Self::convert_statements(rest));

                let (rhs, name, variable) = if let hir::Ast::Definition(definition) = first {
                    let rhs = Box::new(Self::convert_statement(&definition.expr));
                    (rhs, definition.name.clone(), Some((definition.variable, &definition.typ)))
                } else {
                    // `val x <- s; s` is the only rule for sequencing statements, so we have
                    // to create a LetBinding where the argument is ignored in order to keep
                    // sequencing the remainder of the statements.
                    (Box::new(Self::convert_statement(first)), None, None)
                };

                let name = name.unwrap_or_else(|| "_".into());
                Statement::LetBinding(LetBinding { variable, name, rhs, body })
            },
            [last] => Self::convert_statement(last),

            // This case can only occur if the statements list is empty
            [] => Statement::Return(&hir::Ast::Literal(hir::Literal::Unit), &hir::Type::Primitive(hir::PrimitiveType::Unit)),
        }
    }

    /// s ::= e(e)                application
    ///     | val x ← s; s        sequence
    ///     | return e            return
    ///     | do h(e)             effect call (not included)
    ///     | handle c = h in s   effect handler
    fn convert_statement(statement: &hir::Ast) -> Statement {
        match statement {
            hir::Ast::FunctionCall(call) => Statement::Application(&call.function, &call.args, &call.function_type),
            hir::Ast::Return(expr) => Statement::Return(&expr.expression, &expr.typ),
            hir::Ast::Handle(handle) => Statement::Handle(handle),
            hir::Ast::Sequence(sequence) => Self::convert_statements(&sequence.statements),

            // There's no `rest` here so we translate to `val x <- s; ()`
            hir::Ast::Definition(definition) => {
                Statement::LetBinding(LetBinding {
                    variable: Some((definition.variable, &definition.typ)),
                    name: definition.name.clone().unwrap_or_else(|| "_".into()),
                    rhs: Box::new(Self::convert_statement(&definition.expr)),
                    body: Box::new(Statement::Return(&hir::Ast::Literal(hir::Literal::Unit), &hir::Type::Primitive(hir::PrimitiveType::Unit))),
                })
            },

            other => Statement::Return(other, &hir::Type::Primitive(hir::PrimitiveType::Unit)),
        }
    }

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
    fn cps_statement(&mut self, statement: Statement, effects: &EffectStack) -> Ast {
        match statement {
            Statement::Application(function, args, function_type) => self.cps_call(function, args, function_type, effects),
            Statement::LetBinding(let_binding) if effects.is_empty() => self.cps_let_binding_pure(let_binding),
            Statement::LetBinding(let_binding) => self.cps_let_binding_impure(let_binding, effects),
            Statement::Return(expression, typ) => self.cps_return(expression, typ, effects),
            Statement::Handle(handle) => self.cps_handle(handle, effects),
        }
    }

    /// S(val x <- s; s', [])
    ///     = (fn x -> S(s', [])) @ S(s, [])
    ///
    /// The above is equivalent to a regular let binding in lambda calculus,
    /// so cps_let_binding_pure recurs on its arguments but otherwise returns
    /// the same structure.
    fn cps_let_binding_pure(&mut self, let_binding: LetBinding) -> Ast {
        let expr = Box::new(self.cps_statement(*let_binding.rhs, &Vec::new()));
        let body = *let_binding.body;

        let (name, variable, typ) = match let_binding.variable {
            // In a pure context, the result type is the same as the source type
            Some((id, typ)) => (Some(let_binding.name), id, typ.clone()),
            None => todo!("Fresh variable"),
        };

        let definition = Ast::Definition(hir::Definition { variable, name, expr, typ });
        let body = self.cps_statement(*let_binding.body, &Vec::new());

        Ast::Sequence(hir::Sequence { statements: vec![definition, body]})
    }

    /// S(val x <- s; s', [ts, t])
    ///     = fn k => S(s, [ts, t]) @@ (fn x => S(s', [ts, t]) @@ k)
    fn cps_let_binding_impure(&mut self, let_binding: LetBinding, effects: &EffectStack) -> Ast {
        let definition_rhs = self.cps_statement(*let_binding.rhs, effects);
        let body = *let_binding.body;

        // TODO: What is the type of 'k' here?
        let k_type = Type::Function(Context::placeholder_function_type());

        let x_type = let_binding.variable.map(|(_, typ)| typ.clone()).unwrap_or_else(Type::unit);

        let k = self.anonymous_local("k", k_type);
        let x = self.anonymous_local(let_binding.name, x_type);

        if let Some((id, _)) = let_binding.variable {
            self.insert_local_definition(id, x.clone());
        }

        Context::ct_lambda(vec![k.clone()], {
            let inner_lambda = Context::ct_lambda(vec![x], {
                let rest = self.cps_statement(body, effects);
                Ast::ct_call1(rest, Ast::Variable(k))
            });

            Ast::ct_call1(definition_rhs, inner_lambda)
        })
    }

    fn cps_extern(extern_reference: &hir::Extern) -> Ast {
        Ast::Extern(extern_reference.clone())
    }

    fn cps_assign(&mut self, assign: &hir::Assignment, effects: &EffectStack) -> Ast {
        todo!("cps_assign")
        // let lhs = assign.lhs.to_expr(self);
        // let rhs = assign.rhs.to_expr(self);

        // let unit = hir::Type::Primitive(hir::PrimitiveType::Unit);
        // self.with_next_function(&unit, &[], |this, k| {
        //     this.set_function_body(Ast::Assign, vec![lhs, rhs, k]);
        //     Ast::Literal(Literal::Unit)
        // })
    }

    fn cps_member_access(&mut self, access: &hir::MemberAccess, effects: &EffectStack) -> Ast {
        let lhs = Box::new(self.cps_ast(&access.lhs, effects));
        let typ = self.convert_type(&access.typ);
        Ast::MemberAccess(hir::MemberAccess { lhs, typ, member_index: access.member_index })
    }

    fn cps_tuple(&mut self, tuple: &hir::Tuple, effects: &EffectStack) -> Ast {
        let fields = fmap(&tuple.fields, |field| self.cps_ast(field, effects));
        Ast::Tuple(hir::Tuple { fields })
    }

    fn cps_reinterpret_cast(&mut self, reinterpret_cast: &hir::ReinterpretCast, effects: &EffectStack) -> Ast {
        let lhs = Box::new(self.cps_ast(&reinterpret_cast.lhs, effects));
        let target_type = self.convert_type(&reinterpret_cast.target_type);
        Ast::ReinterpretCast(hir::ReinterpretCast { lhs, target_type })
    }

    fn cps_builtin(&mut self, builtin: &hir::Builtin, effects: &EffectStack) -> Ast {
        let binary_fn = |f: fn(_, _) -> _, context: &mut Context, lhs: &hir::Ast, rhs: &hir::Ast| {
            let lhs = Box::new(context.cps_ast(lhs, effects));
            let rhs = Box::new(context.cps_ast(rhs, effects));
            Ast::Builtin(f(lhs, rhs))
        };

        let unary_fn = |f: fn(_) -> _, context: &mut Self, lhs| {
            Ast::Builtin(f(Box::new(context.cps_ast(lhs, effects))))
        };

        let unary_fn_with_type = |f: fn(_, _) -> _, context: &mut Self, lhs, rhs| {
            let lhs = Box::new(context.cps_ast(lhs, effects));
            let rhs = context.convert_type(rhs);
            Ast::Builtin(f(lhs, rhs))
        };

        use hir::Builtin::*;
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
                let lhs = Box::new(self.cps_ast(lhs, effects));
                let rhs = Box::new(self.cps_ast(rhs, effects));
                let typ = self.convert_type(typ);
                Ast::Builtin(Offset(lhs, rhs, typ))
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
    fn cps_effect(&mut self, effect: &hir::Effect, effects: &EffectStack) -> Ast {
        self.convert_effect(EffectAst::Variable(effect.id), effects)
    }

    /// S(handle c = h in s : t, ts)
    ///   = (fn c => S(s, [ts, t]) @@ (fn x => S(return x, ts))) @@ H(h, [ts, t])
    fn cps_handle(&mut self, handle: &hir::Handle, effects: &EffectStack) -> Ast {
        let mut new_effects = effects.to_vec();
        let result_type = self.convert_type(&handle.result_type);
        new_effects.push((handle.effect.id, result_type.clone()));

        let handler = self.convert_effect(EffectAst::Handle(handle), &new_effects);

        let id = std::iter::once(handle.effect.id);
        let c_type = self.convert_type(&handle.effect.typ);

        let name = Rc::new("handle_expression".to_string());

        let c_lambda = self.new_function(name, id, vec![c_type], true, |this| {
            let expression = this.cps_ast(&handle.expression, &new_effects);

            let k = this.intermediate_function("handle_k", result_type.clone(), true, |this, x| {
                this.cps_return_helper(x, effects, result_type)
            });

            Ast::ct_call1(expression, k)
        });

        Ast::ct_call1(c_lambda, handler)
    }

    /// H(c, ts) = c
    /// H(F(x, k) -> s, [ts, t]) = fn x => fn k => S(s, ts)
    /// H(lift h, [t]) = fn x => fn k => k @@ (H(h, []) @@ x)
    /// H(lift h, [ts, t, t'])
    ///   = fn x => fn k => fn k' => H(h, [ts, t]) @@ x @@ (fn y => k @@ y @@ k')
    fn convert_effect(&mut self, effect: EffectAst, effects: &EffectStack) -> Ast {
        match effect {
            // H(c, ts) = c
            // TODO: implement lift cases
            EffectAst::Variable(id) => {
                let c = self.get_definition(id, effects).unwrap_or_else(|| {
                    panic!("No handler for effect {}", id)
                });
                Ast::Variable(c)
            },
            // H(F(x, k) -> s, [ts, t]) = fn x => fn k => S(s, ts)
            EffectAst::Handle(handle) => {
                // TODO: assert effects.pop() == t
                let mut effects = effects.to_vec();
                effects.pop();

                let xs = fmap(&handle.branch_body.args, |arg| {
                    self.new_local(arg.definition_id, Context::name_of(&arg.name), *arg.typ.clone())
                });

                let k = self.new_local(handle.resume.definition_id, "k", *handle.resume.typ.clone());

                // TODO: Should this be one function with k as the last parameter?
                Context::ct_lambda(xs, {
                    Context::ct_lambda(vec![k], {
                        self.cps_ast(&handle.branch_body.body, &effects)
                    })
                })
            },
        }
    }
}
