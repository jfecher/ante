//! This module contains the escape analysis pass which occurs after type checking (but before
//! definitions are generalized). The purpose of this pass is to:
//! - Issue errors when a reference escapes the scope in which it is valid
//! - Issue errors when a reference is used after a value it may refer to is moved
//!
//! Each reference in Ante has a 'place' which is analogous to a lifetime in Rust. Each place
//! is a set of places a lifetime may refer to. The result of `if rand () then ref foo else ref bar`
//! for example may refer to either `foo` or `bar` so its place is `'(foo, bar)`.
//!
//! Anonymous places are tagged with the stack depth in which they're valid for.
//!
//! This module walks the CST in [EscapeChecker::walk_expr], keeping track of the current scope nesting.
//! - The result type of each block is checked to ensure it contains no places that refer to names
//! declared within the block.
//!
//! In the future moves will be tracked in this pass as well, but they're currently in the
//! larger type checker in cst_traversal.rs.

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::Diagnostic,
    parser::{
        cst::{self, Expr, Pattern},
        ids::{ExprId, NameId, PatternId},
    },
    type_inference::{Locateable, TypeChecker, row::RowEntry},
};

use super::{
    places::{PlaceAtom, ScopeDepth},
    types::{Type, TypeBindings},
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_escaping_references(&self, definition: &cst::Definition) {
        let mut checker = EscapeChecker {
            tc: self,
            name_depths: FxHashMap::default(),
            function_depth: None,
            reference_locations: FxHashMap::default(),
        };
        let depth = self.current_scope_depth();
        checker.record_name_depths_in_pattern(definition.pattern, depth);
        checker.walk_expr(definition.rhs, depth);
    }
}

struct EscapeChecker<'a, 'local, 'inner> {
    tc: &'a TypeChecker<'local, 'inner>,

    /// The scope depth each local name was bound at
    name_depths: FxHashMap<NameId, ScopeDepth>,

    /// The scope depth of the current function's body, used to check `return` statements
    function_depth: Option<ScopeDepth>,

    /// The location of each [PlaceAtom], used for better error messages.
    /// TODO: This should be cleaner, we shouldn't track by [PlaceAtom]. This isn't sufficient when
    /// there are multiple `ref e` expressions to the same `e`.
    reference_locations: FxHashMap<PlaceAtom, ExprId>,
}

impl<'a, 'local, 'inner> EscapeChecker<'a, 'local, 'inner> {
    /// Walk `expr` at `depth`, recursing into every child expression and checking escape
    /// rules wherever a scope closes.
    fn walk_expr(&mut self, expr: ExprId, depth: ScopeDepth) {
        match self.tc.resolved_expr(expr).as_ref() {
            Expr::Sequence(items) => {
                let new_depth = depth.deeper();
                for item in items {
                    self.walk_expr(item.expr, new_depth);
                }
            },
            Expr::Definition(definition) => {
                self.record_name_depths_in_pattern(definition.pattern, depth);
                self.walk_expr(definition.rhs, depth);
            },
            Expr::If(if_) => {
                self.walk_expr(if_.condition, depth);
                let new_depth = depth.deeper();
                self.walk_and_check_escapes(if_.then, new_depth);
                if let Some(else_) = if_.else_ {
                    self.walk_and_check_escapes(else_, new_depth);
                }
            },
            Expr::Match(match_) => {
                // `infer_match` pushes an implicits scope before checking the expression, so its
                // real scope depth is one deeper than the match's own
                let new_depth = depth.deeper();
                self.walk_expr(match_.expression, new_depth);

                let branch_depth = new_depth.deeper();
                for (pattern, branch) in &match_.cases {
                    self.record_name_depths_in_pattern(*pattern, branch_depth);
                    self.walk_and_check_escapes(*branch, branch_depth);
                }
            },
            Expr::Lambda(lambda) => {
                let body_depth = depth.deeper();
                for parameter in &lambda.parameters {
                    self.record_name_depths_in_pattern(parameter.pattern, body_depth);
                }

                let old_function_depth = self.function_depth.replace(body_depth);
                self.walk_and_check_escapes(lambda.body, body_depth);
                self.function_depth = old_function_depth;
            },
            Expr::While(while_) => {
                self.walk_expr(while_.condition, depth);
                self.walk_and_check_escapes(while_.body, depth.deeper());
            },
            Expr::For(for_) => {
                self.walk_expr(for_.start, depth);
                self.walk_expr(for_.end, depth);

                let body_depth = depth.deeper();
                self.name_depths.insert(for_.variable, body_depth);
                self.walk_and_check_escapes(for_.body, body_depth);
            },
            Expr::Return(ret) => {
                self.walk_expr(ret.expression, depth);

                if let Some(boundary) = self.function_depth {
                    self.check_escapes(ret.expression, boundary);
                } else {
                    // return outside of a function, name resolution should already error here
                }
            },
            Expr::Assignment(assignment) => {
                self.walk_expr(assignment.lhs, depth);
                self.walk_expr(assignment.rhs, depth);
                if let Some((_, op_expr)) = assignment.op {
                    self.walk_expr(op_expr, depth);
                }

                let target_depth = self.assignment_target_depth(assignment.lhs).unwrap_or(ScopeDepth(0));
                self.check_escapes(assignment.rhs, target_depth.deeper());
            },
            Expr::Reference(reference) => {
                self.record_reference_location(expr);
                self.walk_expr(reference.rhs, depth);
            },
            Expr::MemberAccess(access) => self.walk_expr(access.object, depth),
            Expr::Call(call) => {
                self.walk_expr(call.function, depth);
                for arg in &call.arguments {
                    self.walk_expr(arg.expr, depth);
                }
            },
            Expr::TypeAnnotation(annotation) => self.walk_expr(annotation.lhs, depth),
            Expr::Constructor(constructor) => {
                for (_, field_expr) in &constructor.fields {
                    self.walk_expr(*field_expr, depth);
                }
            },
            Expr::Handle(handle) => {
                self.walk_expr(handle.expression, depth);
                for (_, branch) in &handle.cases {
                    self.walk_expr(*branch, depth);
                }
            },
            Expr::ArrayLiteral(elements) => {
                for element in elements {
                    self.walk_expr(*element, depth);
                }
            },
            Expr::Literal(_)
            | Expr::Variable(_)
            | Expr::Break
            | Expr::Continue
            | Expr::Error
            | Expr::Extern(_)
            | Expr::Quoted(_) => {},
            Expr::Is(_) | Expr::Do(_) | Expr::Loop(_) | Expr::InterpolatedString(_) => {
                unreachable!("desugared before type inference")
            },
        }
    }

    /// Record the scope depth of every name a pattern binds.
    fn record_name_depths_in_pattern(&mut self, pattern: PatternId, depth: ScopeDepth) {
        match self.tc.pattern_of(pattern).as_ref() {
            Pattern::Variable(name) | Pattern::MethodName { item_name: name, .. } => {
                self.name_depths.insert(*name, depth);
            },
            Pattern::Alias(name, inner) => {
                self.name_depths.insert(*name, depth);
                self.record_name_depths_in_pattern(*inner, depth);
            },
            Pattern::Constructor(_, args) => {
                for arg in args {
                    self.record_name_depths_in_pattern(*arg, depth);
                }
            },
            Pattern::ConstructorRest(_, args, name) => {
                for arg in args {
                    self.record_name_depths_in_pattern(*arg, depth);
                }
                if let Some(name) = name {
                    self.name_depths.insert(*name, depth);
                }
            },
            Pattern::TypeAnnotation(inner, _) => self.record_name_depths_in_pattern(*inner, depth),
            Pattern::Or(alts) => {
                for alt in alts {
                    self.record_name_depths_in_pattern(*alt, depth);
                }
            },
            Pattern::Error | Pattern::Literal(_) => {},
        }
    }

    fn walk_and_check_escapes(&mut self, expr: ExprId, depth: ScopeDepth) {
        self.walk_expr(expr, depth);
        self.check_escapes(expr, depth);
    }

    /// The scope depth `e` in `x := e` must be valid for
    fn assignment_target_depth(&self, lhs: ExprId) -> Option<ScopeDepth> {
        if let Some(depth) = self.reference_target_depth(lhs) {
            return Some(depth);
        }
        if let Expr::Call(call) = self.tc.resolved_expr(lhs).as_ref() {
            if let Some(arg) = call.arguments.first() {
                return self.assignment_target_depth(arg.expr);
            }
        }
        None
    }

    /// If `lhs`'s type is a reference, return the shallowest depth among the places it may reference
    fn reference_target_depth(&self, lhs: ExprId) -> Option<ScopeDepth> {
        let typ = self.tc.expr_types.get(&lhs)?;
        let places = typ.reference_places(&self.tc.bindings)?;
        let mut flattened = Vec::new();
        let places = std::slice::from_ref(&places);
        Type::flatten_places_into(places, &mut flattened, &self.tc.bindings, &TypeBindings::default());
        let depths = flattened.into_iter().filter_map(|entry| match entry {
            Type::PlaceAtom(atom) => self.atom_depth(&atom),
            _ => None,
        });
        depths.min()
    }

    /// The scope depth `atom` is valid for, if known.
    fn atom_depth(&self, atom: &PlaceAtom) -> Option<ScopeDepth> {
        match atom {
            PlaceAtom::Variable(name) => self.name_depths.get(name).copied(),
            PlaceAtom::Anonymous(_, depth) => Some(*depth),
        }
    }

    /// TODO: This is a hack, remove and replace with better location tracking. We shouldn't need
    /// to recur on the type for this.
    fn record_reference_location(&mut self, reference_expr: ExprId) {
        let Some(Type::Application(constructor, args)) = self.tc.expr_types.get(&reference_expr) else { return };
        if args.is_empty() || constructor.reference_constructor(&self.tc.bindings).is_none() {
            return;
        }
        let Some(Some(row)) = Type::as_row(&args[0]) else { return };
        for entry in row.iter() {
            if let Type::PlaceAtom(atom) = entry {
                self.reference_locations.entry(*atom).or_insert(reference_expr);
            }
        }
    }

    /// Check that value's type carries only places that outlive `boundary`.
    fn check_escapes(&mut self, value: ExprId, boundary: ScopeDepth) {
        let Some(typ) = self.tc.expr_types.get(&value) else { return };

        let mut offending = Vec::new();
        self.collect_escaping_places(typ, boundary, &mut offending);

        for atom in offending {
            let Some(&reference_expr) = self.reference_locations.get(&atom) else { continue };
            if !self.is_value_source(value, reference_expr) {
                continue;
            }
            let location = reference_expr.locate(self.tc);
            let (name, declared_at) = match atom {
                PlaceAtom::Variable(name) => {
                    (Some(self.tc.current_extended_context()[name].clone()), name.locate(self.tc))
                },
                PlaceAtom::Anonymous(origin, _) => (None, origin.locate(self.tc)),
            };
            self.tc.compiler.accumulate(Diagnostic::EscapingReference { name, location, declared_at });
        }
    }

    /// True if `target` is reachable from `root` by only stepping through positions that
    /// directly determine `root`'s own value
    /// TODO: Hack, remove
    fn is_value_source(&self, root: ExprId, target: ExprId) -> bool {
        if root == target {
            return true;
        }
        match self.tc.resolved_expr(root).as_ref() {
            Expr::Sequence(items) => items.last().is_some_and(|item| self.is_value_source(item.expr, target)),
            Expr::Call(call) => {
                self.is_value_source(call.function, target)
                    || call.arguments.iter().any(|arg| self.is_value_source(arg.expr, target))
            },
            Expr::If(if_) => {
                self.is_value_source(if_.then, target)
                    || if_.else_.is_some_and(|else_| self.is_value_source(else_, target))
            },
            Expr::Match(match_) => match_.cases.iter().any(|(_, branch)| self.is_value_source(*branch, target)),
            Expr::TypeAnnotation(annotation) => self.is_value_source(annotation.lhs, target),
            Expr::MemberAccess(access) => self.is_value_source(access.object, target),
            _ => false,
        }
    }

    /// Recursively collect every concrete [PlaceAtom] reachable from `typ` whose scope
    /// isn't valid past `boundary`.
    fn collect_escaping_places(&self, typ: &Type, boundary: ScopeDepth, out: &mut Vec<PlaceAtom>) {
        match typ.follow(&self.tc.bindings) {
            Type::Application(constructor, args) => {
                self.collect_escaping_places(constructor, boundary, out);
                for arg in args.iter() {
                    self.collect_escaping_places(arg, boundary, out);
                }
            },
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    self.collect_escaping_places(element, boundary, out);
                }
            },
            typ @ (Type::Places(_) | Type::PlaceAtom(_)) => self.collect_escaping_places_row(&typ, boundary, out),
            Type::Forall(_, typ) => self.collect_escaping_places(typ, boundary, out),
            Type::Effects(effects) => {
                if let Some(effects) = effects.as_ref() {
                    for effect in effects.iter() {
                        self.collect_escaping_places(&effect.typ, boundary, out);
                    }
                }
            },
            Type::Function(function) => {
                for parameter in &function.parameters {
                    self.collect_escaping_places(&parameter.typ, boundary, out);
                }
                self.collect_escaping_places(&function.environment, boundary, out);
                self.collect_escaping_places(&function.return_type, boundary, out);
                self.collect_escaping_places(&function.effects, boundary, out);
            },
            Type::Primitive(_)
            | Type::Generic(_)
            | Type::Variable(_)
            | Type::UserDefined(_)
            | Type::U32(_)
            | Type::EffectId(_) => (),
        }
    }

    /// Collect the concrete places of a reference's places row that don't outlive `boundary`
    fn collect_escaping_places_row(&self, places: &Type, boundary: ScopeDepth, out: &mut Vec<PlaceAtom>) {
        let mut flattened = Vec::new();
        Type::flatten_places_into(
            std::slice::from_ref(places),
            &mut flattened,
            &self.tc.bindings,
            &TypeBindings::default(),
        );
        for entry in flattened {
            let Type::PlaceAtom(atom) = entry else { continue };
            if self.atom_depth(&atom).is_some_and(|depth| depth >= boundary) {
                out.push(atom);
            }
        }
    }
}
