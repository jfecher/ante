//! Evaluate any compile-time function applications in the Mir
//! to remove handler abstractions
use std::collections::HashSet;

use crate::hir::{Literal, Ast};

use super::ir::{Mir, Expr, FunctionId};

impl Ast {
    pub fn evaluate_static_calls(&mut self) {
        todo!("Evaluate hir")
    }
}

impl Mir {
    pub fn evaluate(&mut self) {
        let mut no_progress = HashSet::<FunctionId>::new();
        let mut done = false;

        let mut i = 0;

        while !done {
            eprintln!("==============================================");
            eprintln!("Evaluate iteration {i}");
            eprintln!("{}", self);
            eprintln!("\nScopes: {}", self.find_scopes());
            i += 1;

            done = true;
            let functions = self.functions.keys().cloned().collect::<Vec<_>>();

            for id in functions {
                if !no_progress.contains(&id) {
                    if let Some(function) = self.functions.get_mut(&id) {
                        eprintln!("Evaluating function {id}");
                        let mut body = function.body.clone();

                        let mut changed = false;
                        let mut modified = HashSet::new();
                        body.evaluate(self, &mut changed, &mut modified);

                        // evaluate can mutate functions when beta-reducing calls that only have 1
                        // use. When this happens, these previously simplified functions can have
                        // more reducible expressions.
                        for modified_function in modified {
                            no_progress.remove(&modified_function);
                        }

                        let function = self.functions.get_mut(&id).unwrap();
                        function.body = body;

                        self.remove_unreachable_functions();

                        if changed {
                            done = false;
                        } else {
                            no_progress.insert(id);
                        }
                    }
                }
            }
        }
    }
}

impl Expr {
    fn evaluate(&mut self, mir: &mut Mir, changed: &mut bool, modified: &mut HashSet<FunctionId>) {
        let mut both = |lhs: &mut Expr, rhs: &mut Expr, mir: &mut Mir| {
            lhs.evaluate(mir, changed, modified);
            rhs.evaluate(mir, changed, modified);
        };

        match self {
            Expr::Call(function, args, compile_time) => {
                function.evaluate(mir, changed, modified);

                for arg in args.iter_mut() {
                    arg.evaluate(mir, changed, modified);
                }

                if let Expr::Function(id) = function.as_ref() {
                    if *compile_time || mir.functions[id].compile_time {
                        *changed = true;
                        *self = mir.evaluate_call(id, args.clone(), modified);
                        self.evaluate(mir, changed, modified);
                    }
                }
            },
            Expr::If(c, t, e) => {
                c.evaluate(mir, changed, modified);
                t.evaluate(mir, changed, modified);
                e.evaluate(mir, changed, modified);

                if let Expr::Literal(Literal::Bool(value)) = c.as_ref() {
                    let replacement = if *value { t.as_mut() } else { e.as_mut() };
                    let function =std::mem::replace(replacement, Expr::unit());
                    *changed = true;

                    // Must call then/else since they are represented as functions
                    *self = Expr::rt_call(function, vec![]);
                }
            },
            Expr::Switch(expr, cases, else_case) => {
                expr.evaluate(mir, changed, modified);

                if let Expr::Literal(Literal::Integer(value, _)) = expr.as_ref() {
                    let case = cases.iter().find(|(case, _)| *case as u64 == *value).map(|(_, f)| f);

                    let case = case.or_else(|| else_case.as_ref().clone()).unwrap_or_else(|| {
                        panic!("Expected to find case for constant {}", value)
                    }).clone();

                    *changed = true;
                    *self = Expr::rt_call(Expr::Function(case), vec![]);
                }
            },
            Expr::Literal(_) => (),
            Expr::Parameter(_) => (),
            Expr::Function(_) => (),
            Expr::Extern(_) => (),
            Expr::Tuple(fields) => {
                for field in fields {
                    field.evaluate(mir, changed, modified);
                }
            },
            Expr::MemberAccess(lhs, _, _) => lhs.evaluate(mir, changed, modified),
            Expr::Assign => todo!(),

            // TODO: We could try to evaluate constants here as well
            Expr::AddInt(lhs, rhs) => both(lhs, rhs, mir),
            Expr::AddFloat(lhs, rhs) => both(lhs, rhs, mir),
            Expr::SubInt(lhs, rhs) => both(lhs, rhs, mir),
            Expr::SubFloat(lhs, rhs) => both(lhs, rhs, mir),
            Expr::MulInt(lhs, rhs) => both(lhs, rhs, mir),
            Expr::MulFloat(lhs, rhs) => both(lhs, rhs, mir),
            Expr::DivSigned(lhs, rhs) => both(lhs, rhs, mir),
            Expr::DivUnsigned(lhs, rhs) => both(lhs, rhs, mir),
            Expr::DivFloat(lhs, rhs) => both(lhs, rhs, mir),
            Expr::ModSigned(lhs, rhs) => both(lhs, rhs, mir),
            Expr::ModUnsigned(lhs, rhs) => both(lhs, rhs, mir),
            Expr::ModFloat(lhs, rhs) => both(lhs, rhs, mir),
            Expr::LessSigned(lhs, rhs) => both(lhs, rhs, mir),
            Expr::LessUnsigned(lhs, rhs) => both(lhs, rhs, mir),
            Expr::LessFloat(lhs, rhs) => both(lhs, rhs, mir),
            Expr::EqInt(lhs, rhs) => both(lhs, rhs, mir),
            Expr::EqFloat(lhs, rhs) => both(lhs, rhs, mir),
            Expr::EqChar(lhs, rhs) => both(lhs, rhs, mir),
            Expr::EqBool(lhs, rhs) => both(lhs, rhs, mir),
            Expr::SignExtend(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::ZeroExtend(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::SignedToFloat(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::UnsignedToFloat(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::FloatToSigned(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::FloatToUnsigned(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::FloatPromote(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::FloatDemote(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::BitwiseAnd(lhs, rhs) => both(lhs, rhs, mir),
            Expr::BitwiseOr(lhs, rhs) => both(lhs, rhs, mir),
            Expr::BitwiseXor(lhs, rhs) => both(lhs, rhs, mir),
            Expr::BitwiseNot(lhs) => lhs.evaluate(mir, changed, modified),
            Expr::Truncate(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::Deref(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::Offset(lhs, rhs, _) => both(lhs, rhs, mir),
            Expr::Transmute(lhs, _t) => lhs.evaluate(mir, changed, modified),
            Expr::StackAlloc(lhs) => lhs.evaluate(mir, changed, modified),
        }
    }
}
