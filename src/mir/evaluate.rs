//! Evaluate any compile-time function applications in the Hir to remove handler abstractions
use crate::util::fmap;

use super::ir::{ self as mir, Ast, dispatch_on_mir, DefinitionId, Atom, Mir };

impl Mir {
    pub fn evaluate_static_calls(mut self) -> Mir {
        self.functions = self.functions.iter().filter_map(|(id, (name, function))| {
            if matches!(function, Ast::Atom(Atom::Lambda(lambda)) if lambda.compile_time) {
                None
            } else {
                let function = function.clone();
                let new_function = function.evaluate(&self, &im::HashMap::new());
                Some((*id, (name.clone(), new_function)))
            }
        }).collect();
        self
    }
}

type Substitutions = im::HashMap<DefinitionId, Atom>;

/// Evaluate static calls in `self` using the given substitutions
trait Evaluate<T> {
    fn evaluate(self, mir: &Mir, substitutions: &Substitutions) -> T;
}

impl Evaluate<Ast> for Ast {
    fn evaluate(self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        dispatch_on_mir!(self, Evaluate::evaluate, mir, substitutions)
    }
}

impl Evaluate<Atom> for Atom {
    fn evaluate(self, mir: &Mir, substitutions: &Substitutions) -> Atom {
        dispatch_on_atom!(self, Evaluate::evaluate, mir, substitutions)
    }
}

impl Evaluate<Ast> for Atom {
    fn evaluate(self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        Ast::Atom(self.evaluate(mir, substitutions))
    }
}

impl Evaluate<Atom> for mir::Literal {
    fn evaluate(self, _mir: &Mir, _: &Substitutions) -> Atom {
        Atom::Literal(self)
    }
}

impl Evaluate<Atom> for mir::Variable {
    fn evaluate(self, mir: &Mir, substitutions: &Substitutions) -> Atom {
        match substitutions.get(&self.definition_id) {
            Some(ast) => ast.clone(), // Should we recur here?
            None => {
                if let Some((_, Ast::Atom(Atom::Lambda(lambda)))) = mir.functions.get(&self.definition_id) {
                    if lambda.compile_time {
                        return Atom::Lambda(lambda.clone());
                    }
                }

                Atom::Variable(self)
            }
        }
    }
}

impl Evaluate<Atom> for mir::Lambda {
    // Any variables introduced by the lambda shadow any matching variables in `substitutions`,
    // so make sure to remove them before evaluating the lambda body.
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Atom {
        let mut substitutions = substitutions.clone();

        for arg in &self.args {
            substitutions.remove(&arg.definition_id);
        }

        *self.body = self.body.evaluate(mir, &substitutions);
        Atom::Lambda(self)
    }
}

impl Evaluate<Atom> for mir::Extern {
    fn evaluate(self, _mir: &Mir, _: &Substitutions) -> Atom {
        Atom::Extern(self)
    }
}

impl Evaluate<Ast> for mir::FunctionCall {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        let function = self.function.evaluate(mir, substitutions);
        let args = fmap(self.args, |arg| arg.evaluate(mir, substitutions));

        let args_is_unit = args.len() == 1 && matches!(&args[0], Atom::Literal(mir::Literal::Unit));

        match function {
            Atom::Lambda(lambda) if lambda.compile_time || self.compile_time || args_is_unit => {
                let mut new_substitutions = substitutions.clone();
                assert_eq!(lambda.args.len(), args.len());

                for (param, arg) in lambda.args.iter().zip(args) {
                    new_substitutions.insert(param.definition_id, arg);
                }

                let evaluate_recursive = !args_is_unit;

                let mut result = lambda.body.evaluate(mir, &new_substitutions);
                if evaluate_recursive {
                    result = result.evaluate(mir, substitutions);
                }
                result
            }
            function => {
                self.function = function;
                self.args = args;
                Ast::FunctionCall(self)
            }
        }
    }
}

impl Evaluate<Ast> for mir::Let<Ast> {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        match self.expr.evaluate(mir, substitutions) {
            Ast::Atom(atom) => {
                let new_substitutions = substitutions.update(self.variable, atom);
                self.body.evaluate(mir, &new_substitutions)
            },
            // Transform `let a = (let b = c in b) in d` into `let a = c in d`
            // Ast::Let(let_) if let_.is_trivial() => {
            //     self.expr = let_.expr;
            //     *self.body = self.body.evaluate(mir, substitutions);
            //     Ast::Let(self)
            // }
            expr => {
                *self.expr = expr;
                *self.body = self.body.evaluate(mir, substitutions);
                Ast::Let(self)
            }
        }
    }
}

impl Evaluate<Ast> for mir::If {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        self.condition = self.condition.evaluate(mir, substitutions);
        *self.then = self.then.evaluate(mir, substitutions);
        *self.otherwise = self.otherwise.evaluate(mir, substitutions);
        Ast::If(self)
    }
}

impl Evaluate<Ast> for mir::Match {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        self.decision_tree = evaluate_decision_tree(self.decision_tree, mir, substitutions);
        self.branches = fmap(self.branches, |branch| branch.evaluate(mir, substitutions));
        Ast::Match(self)
    }
}

// Decision trees should be free of side-effects so we shouldn't expect to find any redexes here
fn evaluate_decision_tree(tree: mir::DecisionTree, mir: &Mir, substitutions: &Substitutions) -> mir::DecisionTree {
    match tree {
        mir::DecisionTree::Leaf(index) => mir::DecisionTree::Leaf(index),
        mir::DecisionTree::Let(mut let_) => {
            *let_.expr = let_.expr.evaluate(mir, substitutions);
            *let_.body = evaluate_decision_tree(*let_.body, mir, substitutions);
            mir::DecisionTree::Let(let_)
        },
        mir::DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
            let int_to_switch_on = int_to_switch_on.evaluate(mir, substitutions);
            let cases = fmap(cases, |(tag, case)| (tag, evaluate_decision_tree(case, mir, substitutions)));
            let else_case = else_case.map(|case| Box::new(evaluate_decision_tree(*case, mir, substitutions)));
            mir::DecisionTree::Switch { int_to_switch_on, cases, else_case }
        },
    }
}

impl Evaluate<Ast> for mir::Return {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        self.expression = self.expression.evaluate(mir, substitutions);
        Ast::Return(self)
    }
}

impl Evaluate<Ast> for mir::Assignment {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        self.lhs = self.lhs.evaluate(mir, substitutions);
        self.rhs = self.rhs.evaluate(mir, substitutions);
        Ast::Assignment(self)
    }
}

impl Evaluate<Ast> for mir::MemberAccess {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        self.lhs = self.lhs.evaluate(mir, substitutions);
        Ast::MemberAccess(self)
    }
}

impl Evaluate<Ast> for mir::Tuple {
    fn evaluate(mut self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        self.fields = fmap(self.fields, |field| field.evaluate(mir, substitutions));
        Ast::Tuple(self)
    }
}

impl Evaluate<Ast> for mir::Builtin {
    fn evaluate(self, mir: &Mir, substitutions: &Substitutions) -> Ast {
        use mir::Builtin;

        let both = |f: fn(_, _) -> Builtin, lhs: Atom, rhs: Atom| {
            let lhs = lhs.evaluate(mir, substitutions);
            let rhs = rhs.evaluate(mir, substitutions);
            Ast::Builtin(f(lhs, rhs))
        };

        let one_with_type = |f: fn(_, _) -> Builtin, lhs: Atom, typ| {
            let lhs = lhs.evaluate(mir, substitutions);
            Ast::Builtin(f(lhs, typ))
        };

        let one = |f: fn(_) -> Builtin, lhs: Atom| {
            let lhs = lhs.evaluate(mir, substitutions);
            Ast::Builtin(f(lhs))
        };

        match self {
            Builtin::AddInt(lhs, rhs) => both(Builtin::AddInt, lhs, rhs),
            Builtin::AddFloat(lhs, rhs) => both(Builtin::AddFloat, lhs, rhs),
            Builtin::SubInt(lhs, rhs) => both(Builtin::SubInt, lhs, rhs),
            Builtin::SubFloat(lhs, rhs) => both(Builtin::SubFloat, lhs, rhs),
            Builtin::MulInt(lhs, rhs) => both(Builtin::MulInt, lhs, rhs),
            Builtin::MulFloat(lhs, rhs) => both(Builtin::MulFloat, lhs, rhs),
            Builtin::DivSigned(lhs, rhs) => both(Builtin::DivSigned, lhs, rhs),
            Builtin::DivUnsigned(lhs, rhs) => both(Builtin::DivUnsigned, lhs, rhs),
            Builtin::DivFloat(lhs, rhs) => both(Builtin::DivFloat, lhs, rhs),
            Builtin::ModSigned(lhs, rhs) => both(Builtin::ModSigned, lhs, rhs),
            Builtin::ModUnsigned(lhs, rhs) => both(Builtin::ModUnsigned, lhs, rhs),
            Builtin::ModFloat(lhs, rhs) => both(Builtin::ModFloat, lhs, rhs),
            Builtin::LessSigned(lhs, rhs) => both(Builtin::LessSigned, lhs, rhs),
            Builtin::LessUnsigned(lhs, rhs) => both(Builtin::LessUnsigned, lhs, rhs),
            Builtin::LessFloat(lhs, rhs) => both(Builtin::LessFloat, lhs, rhs),
            Builtin::EqInt(lhs, rhs) => both(Builtin::EqInt, lhs, rhs),
            Builtin::EqFloat(lhs, rhs) => both(Builtin::EqFloat, lhs, rhs),
            Builtin::EqChar(lhs, rhs) => both(Builtin::EqChar, lhs, rhs),
            Builtin::EqBool(lhs, rhs) => both(Builtin::EqBool, lhs, rhs),
            Builtin::SignExtend(lhs, rhs) => one_with_type(Builtin::SignExtend, lhs, rhs),
            Builtin::ZeroExtend(lhs, rhs) => one_with_type(Builtin::ZeroExtend, lhs, rhs),
            Builtin::SignedToFloat(lhs, rhs) => one_with_type(Builtin::SignedToFloat, lhs, rhs),
            Builtin::UnsignedToFloat(lhs, rhs) => one_with_type(Builtin::UnsignedToFloat, lhs, rhs),
            Builtin::FloatToSigned(lhs, rhs) => one_with_type(Builtin::FloatToSigned, lhs, rhs),
            Builtin::FloatToUnsigned(lhs, rhs) => one_with_type(Builtin::FloatToUnsigned, lhs, rhs),
            Builtin::FloatPromote(lhs, rhs) => one_with_type(Builtin::FloatPromote, lhs, rhs),
            Builtin::FloatDemote(lhs, rhs) => one_with_type(Builtin::FloatDemote, lhs, rhs),
            Builtin::BitwiseAnd(lhs, rhs) => both(Builtin::BitwiseAnd, lhs, rhs),
            Builtin::BitwiseOr(lhs, rhs) => both(Builtin::BitwiseOr, lhs, rhs),
            Builtin::BitwiseXor(lhs, rhs) => both(Builtin::BitwiseXor, lhs, rhs),
            Builtin::BitwiseNot(lhs) => one(Builtin::BitwiseNot, lhs),
            Builtin::StackAlloc(lhs) => one(Builtin::StackAlloc, lhs),
            Builtin::Truncate(lhs, rhs) => one_with_type(Builtin::Truncate, lhs, rhs),
            Builtin::Deref(lhs, rhs) => one_with_type(Builtin::Deref, lhs, rhs),
            Builtin::Transmute(lhs, rhs) => one_with_type(Builtin::Transmute, lhs, rhs),
            Builtin::Offset(lhs, rhs, typ) => {
                let lhs = lhs.evaluate(mir, substitutions);
                let rhs = rhs.evaluate(mir, substitutions);
                Ast::Builtin(Builtin::Offset(lhs, rhs, typ))
            },
        }
    }
}

impl Evaluate<Atom> for mir::Effect {
    fn evaluate(self, _mir: &Mir, _: &Substitutions) -> Atom {
        unreachable!("Effect nodes should be removed by the mir-cps pass before evaluation")
    }
}

impl Evaluate<Ast> for mir::Handle {
    fn evaluate(self, _mir: &Mir, _: &Substitutions) -> Ast {
        unreachable!("Handle expressions should be removed by the mir-cps pass before evaluation")
    }
}
