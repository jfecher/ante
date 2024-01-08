//! Evaluate any compile-time function applications in the Hir to remove handler abstractions
use std::rc::Rc;

use crate::{hir::{self, Ast, DefinitionId, DecisionTree}, util::fmap};

impl Ast {
    pub fn evaluate_static_calls(self) -> Ast {
        self.evaluate(&im::HashMap::new())
    }
}

type Substitutions = im::HashMap<DefinitionId, Ast>;

/// Evaluate static calls in `self` using the given substitutions
trait Evaluate {
    fn evaluate(self, substitutions: &Substitutions) -> Ast;
}

impl Evaluate for Ast {
    fn evaluate(self, substitutions: &Substitutions) -> Ast {
        dispatch_on_hir!(self, Evaluate::evaluate, substitutions)
    }
}

impl Evaluate for hir::Literal {
    fn evaluate(self, _: &Substitutions) -> Ast {
        Ast::Literal(self)
    }
}

impl Evaluate for hir::Variable {
    fn evaluate(self, substitutions: &Substitutions) -> Ast {
        match substitutions.get(&self.definition_id) {
            Some(ast) => ast.clone(), // Should we recur here?
            None => {
                if let Some(def) = &self.definition {
                    if let Ok(mut def) = def.try_borrow_mut() {
                        if let Some(definition) = def.as_ref() {
                            let new_definition = definition.clone().evaluate(substitutions);
                            *def = Some(new_definition);
                        }
                    }
                }
                Ast::Variable(self)
            },
        }
    }
}

impl Evaluate for Rc<hir::Lambda> {
    // Any variables introduced by the lambda shadow any matching variables in `substitutions`,
    // so make sure to remove them before evaluating the lambda body.
    fn evaluate(self, substitutions: &Substitutions) -> Ast {
        let mut substitutions = substitutions.clone();

        for arg in &self.args {
            substitutions.remove(&arg.definition_id);
        }

        let mut this = self.as_ref().clone();
        this.body = Box::new(this.body.evaluate(&substitutions));
        Ast::Lambda(Rc::new(this))
    }
}

impl Evaluate for hir::FunctionCall{
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        let t = self.clone();
        let function = self.function.evaluate(substitutions);
        let args = fmap(self.args, |arg| arg.evaluate(substitutions));

        // TODO: Need to convert to CPS or ANF first otherwise we're evaluating side-effects twice.
        if let Some(lambda) = try_get_lambda(&function) {
            // TODO: Rc::try_unwrap
            let lambda = lambda.as_ref().clone();

            if lambda.compile_time || self.compile_time {
                let mut new_substitutions = substitutions.clone();
                assert_eq!(lambda.args.len(), args.len());

                for (param, arg) in lambda.args.iter().zip(args) {
                    new_substitutions.insert(param.definition_id, arg);
                }

                let result = lambda.body.clone().evaluate(&new_substitutions);
                println!("Evaluated {} to {}", t, result);

                return lambda.body.evaluate(&new_substitutions).evaluate(substitutions);
            }
        }

        *self.function = function;
        self.args = args;
        Ast::FunctionCall(self)
    }
}

fn try_get_lambda(ast: &Ast) -> Option<Rc<hir::Lambda>> {
    match ast {
        Ast::Lambda(lambda) => Some(lambda.clone()),
        Ast::Variable(variable) => {
            variable.definition.as_ref().and_then(|definition| {
                let def = definition.borrow();
                match def.as_ref() {
                    Some(def) => try_get_lambda(def),
                    None => None,
                }
            })
        }
        _ => None,
    }
}

impl Evaluate for hir::Definition {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        *self.expr = self.expr.evaluate(substitutions);
        Ast::Definition(self)
    }
}

impl Evaluate for hir::If {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        *self.condition = self.condition.evaluate(substitutions);
        *self.then = self.then.evaluate(substitutions);
        *self.otherwise = self.otherwise.evaluate(substitutions);
        Ast::If(self)
    }
}

impl Evaluate for hir::Match {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        self.decision_tree = evaluate_decision_tree(self.decision_tree, substitutions);
        self.branches = fmap(self.branches, |branch| branch.evaluate(substitutions));
        Ast::Match(self)
    }
}

fn evaluate_decision_tree(tree: DecisionTree, substitutions: &Substitutions) -> DecisionTree {
    match tree {
        DecisionTree::Leaf(_) => todo!(),
        DecisionTree::Definition(_, _) => todo!(),
        DecisionTree::Switch { int_to_switch_on, cases, else_case } => todo!(),
    }
}

impl Evaluate for hir::Return {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        *self.expression = self.expression.evaluate(substitutions);
        Ast::Return(self)
    }
}

impl Evaluate for hir::Sequence {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        self.statements = fmap(self.statements, |statement| statement.evaluate(substitutions));
        Ast::Sequence(self)
    }
}

impl Evaluate for hir::Extern {
    fn evaluate(self, _: &Substitutions) -> Ast {
        Ast::Extern(self)
    }
}

impl Evaluate for hir::Assignment {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        *self.lhs = self.lhs.evaluate(substitutions);
        *self.rhs = self.rhs.evaluate(substitutions);
        Ast::Assignment(self)
    }
}

impl Evaluate for hir::MemberAccess {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        *self.lhs = self.lhs.evaluate(substitutions);
        Ast::MemberAccess(self)
    }
}

impl Evaluate for hir::Tuple {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        self.fields = fmap(self.fields, |field| field.evaluate(substitutions));
        Ast::Tuple(self)
    }
}

impl Evaluate for hir::ReinterpretCast {
    fn evaluate(mut self, substitutions: &Substitutions) -> Ast {
        *self.lhs = self.lhs.evaluate(substitutions);
        Ast::ReinterpretCast(self)
    }
}

impl Evaluate for hir::Builtin {
    fn evaluate(self, substitutions: &Substitutions) -> Ast {
        use hir::Builtin;

        let both = |f: fn(_, _) -> Builtin, mut lhs: Box<Ast>, mut rhs: Box<Ast>| {
            *lhs = lhs.evaluate(substitutions);
            *rhs = rhs.evaluate(substitutions);
            Ast::Builtin(f(lhs, rhs))
        };

        let one_with_type = |f: fn(_, _) -> Builtin, mut lhs: Box<Ast>, typ| {
            *lhs = lhs.evaluate(substitutions);
            Ast::Builtin(f(lhs, typ))
        };

        let one = |f: fn(_) -> Builtin, mut lhs: Box<Ast>| {
            *lhs = lhs.evaluate(substitutions);
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
            Builtin::Offset(mut lhs, mut rhs, typ) => {
                *lhs = lhs.evaluate(substitutions);
                *rhs = rhs.evaluate(substitutions);
                Ast::Builtin(Builtin::Offset(lhs, rhs, typ))
            },
        }
    }
}

impl Evaluate for hir::Effect{
    fn evaluate(self, _: &Substitutions) -> Ast {
        unreachable!()
    }
}

impl Evaluate for hir::Handle{
    fn evaluate(self, _: &Substitutions) -> Ast {
        todo!("evaluate")
    }
}
