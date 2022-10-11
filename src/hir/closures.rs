use crate::hir::{self, Ast};
use crate::parser::ast;
use crate::util::fmap;

use super::monomorphisation::Context;

impl<'c> Context<'c> {
    /// Find all recursive calls of this closure and change the environment
    /// to be the parameter environment rather than that of the callsite.
    ///
    /// If the given argument is not a closure this will do nothing
    pub fn fix_recursive_closure_calls(&mut self, expr: Ast, definition: &ast::Definition<'c>, definition_id: hir::DefinitionId) -> Ast {
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
            }
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
            access.lhs = Box::new(replace_env(*access.lhs, env, definition_id, f));
            Ast::MemberAccess(access)
        },
        Ast::Tuple(mut tuple) => {
            tuple.fields = fmap(tuple.fields, |field| replace_env(field, env, definition_id, f));
            Ast::Tuple(tuple)
        },
        Ast::Match(_) => todo!(),
        Ast::Return(_) => todo!(),
        Ast::Extern(_) => todo!(),
        Ast::Assignment(_) => todo!(),
        Ast::ReinterpretCast(_) => todo!(),
        Ast::Builtin(_) => todo!(),
    }
}
