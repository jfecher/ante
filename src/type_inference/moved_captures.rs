//! Decide which captures a closure takes by value. Note that `move fn`s try to take
//! all variables by value while normal `fn`s only move in values if that value is
//! moved within the body of the closure.
use std::collections::BTreeSet;

use crate::{
    diagnostics::Location,
    parser::{
        cst::{self, Expr},
        ids::{ExprId, NameId, PatternId},
    },
    type_inference::{Locateable, TypeChecker, patterns::WILDCARD_PATTERN, types::Type},
};

impl TypeChecker<'_, '_> {
    /// Returns each of `captures` that the body of the closure `lambda` may move
    pub(super) fn moved_captures(&mut self, lambda: ExprId, captures: &BTreeSet<NameId>) -> BTreeSet<NameId> {
        let mut moved = BTreeSet::new();
        self.find_in_lambda_body(lambda, captures, &mut moved);
        moved
    }

    fn find_in_lambda_body(&mut self, lambda: ExprId, captures: &BTreeSet<NameId>, moved: &mut BTreeSet<NameId>) {
        if let Expr::Lambda(lambda) = self.resolved_expr(lambda).into_owned() {
            self.find_moved_captures(lambda.body, true, captures, moved);
        }
    }

    /// Add each of `captures` that evaluating `expr` may move to `moved`. `moves` is false if the
    /// value of `expr` is used in place.
    fn find_moved_captures(
        &mut self, expr: ExprId, moves: bool, captures: &BTreeSet<NameId>, moved: &mut BTreeSet<NameId>,
    ) {
        let mut find =
            |this: &mut Self, expr: ExprId, moves: bool| this.find_moved_captures(expr, moves, captures, moved);

        match self.resolved_expr(expr).into_owned() {
            Expr::Sequence(items) => {
                for item in items {
                    find(self, item.expr, moves);
                }
            },
            Expr::Definition(definition) => find(self, definition.rhs, moves),
            Expr::If(if_) => {
                find(self, if_.condition, moves);
                find(self, if_.then, moves);
                if let Some(else_) = if_.else_ {
                    find(self, else_, moves);
                }
            },
            Expr::Chain(chain) => {
                find(self, chain.body, moves);
                find(self, chain.else_, moves);
            },
            Expr::Match(match_) => {
                // Only patterns which bind part of the value move it
                let binds = match_.cases.iter().any(|(pattern, _)| self.pattern_binds_name(*pattern));
                self.find_operand_moved_captures(match_.expression, moves && binds, captures, moved);
                for (_, branch) in match_.cases.iter() {
                    self.find_moved_captures(*branch, moves, captures, moved);
                }
            },
            Expr::Lambda(_) => {
                // A nested closure moves what it captures by value
                let environment = self.current_extended_context().get_closure_environment(expr).cloned();
                for name in environment.unwrap_or_default().intersection(captures) {
                    if self.current_extended_context().captures_by_value(expr, *name) && !self.capture_is_copy(*name) {
                        moved.insert(*name);
                    }
                }
            },
            Expr::Handle(handle) => {
                // The body and branches of a `handle` run in place
                for lambda in std::iter::once(handle.expression).chain(handle.cases.iter().map(|(_, branch)| *branch)) {
                    self.find_in_lambda_body(lambda, captures, moved);
                }
            },
            Expr::Return(return_) => find(self, return_.expression, moves),
            Expr::Assignment(assignment) => {
                // `x += v` reads `x` but `x := v` only writes it
                find(self, assignment.lhs, moves && assignment.op.is_some());
                find(self, assignment.rhs, moves);
                if let Some((_, op_expr)) = assignment.op {
                    find(self, op_expr, moves);
                }
            },
            Expr::Reference(reference) => self.find_operand_moved_captures(reference.rhs, false, captures, moved),
            Expr::MemberAccess(access) => {
                self.find_operand_moved_captures(access.object, false, captures, moved);
                if moves {
                    self.record_moved_capture(expr, captures, moved);
                }
            },
            Expr::Call(call) => {
                // Calling a `once fn` moves it. Calling any other function borrows it.
                let once = self
                    .expr_function_type(call.function)
                    .is_some_and(|function| *self.follow_type(&function.once) == Type::ONCE);
                self.find_operand_moved_captures(call.function, moves && once, captures, moved);
                for argument in call.arguments.iter() {
                    self.find_moved_captures(argument.expr, true, captures, moved);
                }
            },
            Expr::TypeAnnotation(annotation) => find(self, annotation.lhs, moves),
            Expr::Constructor(constructor) => {
                for (_, field) in constructor.fields.iter() {
                    find(self, *field, true);
                }
            },
            Expr::ArrayLiteral(elements) => {
                for element in elements {
                    find(self, element, true);
                }
            },
            Expr::Variable(_) if moves => self.record_moved_capture(expr, captures, moved),
            Expr::Variable(_)
            | Expr::Literal(_)
            | Expr::Error
            | Expr::Extern(_)
            | Expr::Quoted(_)
            | Expr::Fallthrough => (),
            Expr::Is(_)
            | Expr::Do(_)
            | Expr::Loop(_)
            | Expr::While(_)
            | Expr::For(_)
            | Expr::Break
            | Expr::Continue
            | Expr::InterpolatedString(_) => {
                unreachable!("desugared before type inference")
            },
        }
    }

    /// Like [Self::find_moved_captures], but an `expr` which isn't a place produces a new value
    /// and so always moves what it reads
    fn find_operand_moved_captures(
        &mut self, expr: ExprId, moves: bool, captures: &BTreeSet<NameId>, moved: &mut BTreeSet<NameId>,
    ) {
        let in_place = self.classify_place(expr).is_some_and(|place| place.is_named());
        self.find_moved_captures(expr, moves || !in_place, captures, moved);
    }

    /// Moving the place `expr` moves the capture it is rooted at, unless it is read through a
    /// reference or its value is `Copy`
    fn record_moved_capture(&mut self, expr: ExprId, captures: &BTreeSet<NameId>, moved: &mut BTreeSet<NameId>) {
        let Some(name) = self.classify_place(expr).and_then(|place| place.local()) else { return };
        if !captures.contains(&name) || moved.contains(&name) || self.place_is_indirect(expr) {
            return;
        }
        let Some(typ) = self.expr_types.get(&expr).cloned() else { return };
        if !self.may_be_copy(&typ, expr.locate(self)) {
            moved.insert(name);
        }
    }

    fn capture_is_copy(&mut self, name: NameId) -> bool {
        let typ = self.name_types[&name].clone();
        self.may_be_copy(&typ, name.locate(self))
    }

    /// A value whose type isn't known yet, such as an integer literal's, is assumed to be `Copy`.
    /// If it turns out to be moved, the borrow walk reports it.
    fn may_be_copy(&mut self, typ: &Type, location: Location) -> bool {
        matches!(self.follow_type(typ), Type::Variable(_)) || self.type_is_copy(typ, location)
    }

    /// Whether `pattern` binds any part of the value it matches. Wildcards are excluded.
    pub(super) fn pattern_binds_name(&self, pattern: PatternId) -> bool {
        match self.pattern_of(pattern).as_ref() {
            cst::Pattern::Variable(name) => !self.is_wildcard(*name),
            cst::Pattern::Alias(..) => true,
            cst::Pattern::Constructor(_, args) => args.iter().any(|arg| self.pattern_binds_name(*arg)),
            cst::Pattern::ConstructorRest(_, args, name) => {
                name.is_some() || args.iter().any(|arg| self.pattern_binds_name(*arg))
            },
            cst::Pattern::TypeAnnotation(inner, _) => self.pattern_binds_name(*inner),
            cst::Pattern::Or(alts) => alts.iter().any(|alt| self.pattern_binds_name(*alt)),
            cst::Pattern::Error | cst::Pattern::Literal(_) => false,
        }
    }

    pub(super) fn is_wildcard(&self, name: NameId) -> bool {
        self.current_extended_context()[name].as_str() == WILDCARD_PATTERN
    }
}
