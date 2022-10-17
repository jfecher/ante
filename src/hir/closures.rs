use crate::hir::{self, Ast};
use crate::parser::ast;
use crate::util::fmap;

use super::monomorphisation::Context;
use super::DecisionTree;

impl<'c> Context<'c> {
    /// Find all recursive calls of this closure and change the environment
    /// to be the parameter environment rather than that of the callsite.
    ///
    /// If the given argument is not a closure this will do nothing
    pub fn fix_recursive_closure_calls(
        &mut self, expr: Ast, definition: &ast::Definition<'c>, definition_id: hir::DefinitionId,
    ) -> Ast {
        match definition.expr.as_ref() {
            ast::Ast::Lambda(lambda) if !lambda.closure_environment.is_empty() => {
                let (f, outer_env) = unwrap_closure(expr);
                let inner_env = get_env_parameter(&f);
                let inner_f = self.fresh_variable();

                let new_f = replace_env(f, &inner_env, definition_id, &inner_f);
                let def = Ast::Definition(hir::Definition {
                    variable: inner_f.definition_id,
                    name: None,
                    expr: Box::new(new_f),
                });

                let seq = Ast::Sequence(hir::Sequence { statements: vec![def, inner_f.into()] });
                Ast::Tuple(hir::Tuple { fields: vec![seq, outer_env] })
            },
            _ => expr,
        }
    }
}

/// Given a closure, return (f, env)
fn unwrap_closure(expr: Ast) -> (Ast, Ast) {
    match expr {
        Ast::Tuple(mut elems) => {
            // Extract the env parameter
            assert_eq!(elems.fields.len(), 2);
            let env = elems.fields.pop().unwrap();
            let f = elems.fields.pop().unwrap();
            (f, env)
        },
        other => unreachable!("Expected a closure tuple, found:\n  {}", other),
    }
}

fn get_env_parameter(expr: &Ast) -> Ast {
    // A closure's environment is always its last argument
    match expr {
        Ast::Lambda(lambda) => lambda.args.last().unwrap().clone().into(),
        other => unreachable!("Expected a lambda within the closure, found:\n  {}", other),
    }
}

fn replace_env(expr: Ast, env: &Ast, definition_id: hir::DefinitionId, f: &hir::Variable) -> Ast {
    match expr {
        Ast::Variable(var) if var.definition_id == definition_id => {
            let env = env.clone();
            let new_var = Ast::Variable(f.clone());
            Ast::Tuple(hir::Tuple { fields: vec![new_var, env] })
        },
        Ast::Literal(_) => expr,
        Ast::Variable(_) => expr,
        Ast::Extern(_) => expr,
        Ast::Lambda(mut lambda) => {
            lambda.body = Box::new(replace_env(*lambda.body, env, definition_id, f));
            Ast::Lambda(lambda)
        },
        Ast::FunctionCall(mut call) => {
            call.function = Box::new(replace_env(*call.function, env, definition_id, f));
            call.args = fmap(call.args, |arg| replace_env(arg, env, definition_id, f));
            Ast::FunctionCall(call)
        },
        Ast::Definition(mut def) => {
            def.expr = Box::new(replace_env(*def.expr, env, definition_id, f));
            Ast::Definition(def)
        },
        Ast::If(mut if_expr) => {
            if_expr.condition = Box::new(replace_env(*if_expr.condition, env, definition_id, f));
            if_expr.then = Box::new(replace_env(*if_expr.then, env, definition_id, f));
            if let Some(otherwise) = if_expr.otherwise {
                if_expr.otherwise = Some(Box::new(replace_env(*otherwise, env, definition_id, f)));
            }
            Ast::If(if_expr)
        },
        Ast::Sequence(mut seq) => {
            seq.statements = fmap(seq.statements, |stmt| replace_env(stmt, env, definition_id, f));
            Ast::Sequence(seq)
        },
        Ast::MemberAccess(mut access) => {
            *access.lhs = replace_env(*access.lhs, env, definition_id, f);
            Ast::MemberAccess(access)
        },
        Ast::Tuple(mut tuple) => {
            tuple.fields = fmap(tuple.fields, |field| replace_env(field, env, definition_id, f));
            Ast::Tuple(tuple)
        },
        Ast::Assignment(mut assignment) => {
            assignment.lhs = Box::new(replace_env(*assignment.lhs, env, definition_id, f));
            assignment.rhs = Box::new(replace_env(*assignment.rhs, env, definition_id, f));
            Ast::Assignment(assignment)
        },
        Ast::Match(mut match_expr) => {
            match_expr.branches = fmap(match_expr.branches, |branch| replace_env(branch, env, definition_id, f));
            match_expr.decision_tree = replace_env_decision_tree(match_expr.decision_tree, env, definition_id, f);
            Ast::Match(match_expr)
        },
        Ast::Return(mut ret) => {
            *ret.expression = replace_env(*ret.expression, env, definition_id, f);
            Ast::Return(ret)
        },
        Ast::ReinterpretCast(mut cast) => {
            *cast.lhs = replace_env(*cast.lhs, env, definition_id, f);
            Ast::ReinterpretCast(cast)
        },
        Ast::Builtin(builtin) => Ast::Builtin(replace_env_builtin(builtin, env, definition_id, f)),
    }
}

fn replace_env_decision_tree(
    expr: DecisionTree, env: &Ast, definition_id: hir::DefinitionId, f: &hir::Variable,
) -> DecisionTree {
    match expr {
        DecisionTree::Leaf(_) => expr,
        DecisionTree::Definition(mut definition, tree) => {
            definition.expr = Box::new(replace_env(*definition.expr, env, definition_id, f));
            let tree = Box::new(replace_env_decision_tree(*tree, env, definition_id, f));
            DecisionTree::Definition(definition, tree)
        },
        DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
            let int_to_switch_on = Box::new(replace_env(*int_to_switch_on, env, definition_id, f));
            let cases = fmap(cases, |(tag, case)| {
                let case = replace_env_decision_tree(case, env, definition_id, f);
                (tag, case)
            });
            let else_case = else_case.map(|case| Box::new(replace_env_decision_tree(*case, env, definition_id, f)));
            DecisionTree::Switch { int_to_switch_on, cases, else_case }
        },
    }
}

fn replace_env_builtin(
    builtin: hir::Builtin, env: &Ast, definition_id: hir::DefinitionId, f: &hir::DefinitionInfo,
) -> hir::Builtin {
    use hir::Builtin::*;

    let f = |mut ast: Box<Ast>| {
        *ast = replace_env(*ast, env, definition_id, f);
        ast
    };

    // Nothing special, just recur on all Asts
    match builtin {
        AddInt(lhs, rhs) => AddInt(f(lhs), f(rhs)),
        AddFloat(lhs, rhs) => AddFloat(f(lhs), f(rhs)),
        SubInt(lhs, rhs) => SubInt(f(lhs), f(rhs)),
        SubFloat(lhs, rhs) => SubFloat(f(lhs), f(rhs)),
        MulInt(lhs, rhs) => MulInt(f(lhs), f(rhs)),
        MulFloat(lhs, rhs) => MulFloat(f(lhs), f(rhs)),
        DivSigned(lhs, rhs) => DivSigned(f(lhs), f(rhs)),
        DivUnsigned(lhs, rhs) => DivUnsigned(f(lhs), f(rhs)),
        DivFloat(lhs, rhs) => DivFloat(f(lhs), f(rhs)),
        ModSigned(lhs, rhs) => ModSigned(f(lhs), f(rhs)),
        ModUnsigned(lhs, rhs) => ModUnsigned(f(lhs), f(rhs)),
        ModFloat(lhs, rhs) => ModFloat(f(lhs), f(rhs)),
        LessSigned(lhs, rhs) => LessSigned(f(lhs), f(rhs)),
        LessUnsigned(lhs, rhs) => LessUnsigned(f(lhs), f(rhs)),
        LessFloat(lhs, rhs) => LessFloat(f(lhs), f(rhs)),
        EqInt(lhs, rhs) => EqInt(f(lhs), f(rhs)),
        EqFloat(lhs, rhs) => EqFloat(f(lhs), f(rhs)),
        EqChar(lhs, rhs) => EqChar(f(lhs), f(rhs)),
        EqBool(lhs, rhs) => EqBool(f(lhs), f(rhs)),
        SignExtend(lhs, t) => SignExtend(f(lhs), t),
        ZeroExtend(lhs, t) => ZeroExtend(f(lhs), t),
        SignedToFloat(lhs, t) => SignedToFloat(f(lhs), t),
        UnsignedToFloat(lhs, t) => UnsignedToFloat(f(lhs), t),
        FloatToSigned(lhs, t) => FloatToSigned(f(lhs), t),
        FloatToUnsigned(lhs, t) => FloatToUnsigned(f(lhs), t),
        FloatPromote(lhs) => FloatPromote(f(lhs)),
        FloatDemote(lhs) => FloatDemote(f(lhs)),
        BitwiseAnd(lhs, rhs) => BitwiseAnd(f(lhs), f(rhs)),
        BitwiseOr(lhs, rhs) => BitwiseOr(f(lhs), f(rhs)),
        BitwiseXor(lhs, rhs) => BitwiseXor(f(lhs), f(rhs)),
        BitwiseNot(lhs) => BitwiseNot(f(lhs)),
        Truncate(lhs, t) => Truncate(f(lhs), t),
        Deref(lhs, t) => Deref(f(lhs), t),
        Offset(lhs, rhs, size) => Offset(f(lhs), f(rhs), size),
        Transmute(lhs, t) => Transmute(f(lhs), t),
        StackAlloc(lhs) => StackAlloc(f(lhs)),
    }
}
