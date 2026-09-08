//! This module contains the borrow-checking pass which occurs after type checking (but before
//! definitions are generalized). The purpose of this pass is to:
//! - Issue an error when a reference escapes the scope in which it is valid
//! - Issue an error when a non-`Copy` value is referenced after it has already been moved
//! - Issue an error when a reference is used after a place it may refer to has been moved
//!
//! Each reference in Ante has a 'place' which is analogous to a lifetime in Rust. Each place
//! is a set of places a lifetime may refer to. The result of `if rand () then ref foo else ref bar`
//! for example may refer to either `foo` or `bar` so its place is `'(foo, bar)`.
//!
//! Anonymous places are tagged with the stack depth in which they're valid for.
//!
//! This module walks the CST in [BorrowChecker::walk_expr], keeping track of the current scope nesting.
//! - The result type of each block is checked to ensure it contains no places that refer to names
//! declared within the block.
//! - Each move of a variable is recorded in the current block. We merge these moves when branches
//! are merged and check at the end of each loop to ensure each variable moved was declared within
//! the loop body.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::{Diagnostic, RepeatedContext},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{self, Expr, Pattern},
        ids::{ExprId, NameId, PathId, PatternId},
    },
    type_inference::{
        Locateable, TypeChecker,
        affine::{MoveScope, MoveTracker},
    },
};

use super::{
    free_variables::FreeVars,
    patterns::WILDCARD_PATTERN,
    places::{Place, PlacePath, ScopeDepth},
    types::{Type, TypeBindings},
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_borrows(&mut self, definition: &cst::Definition, depth: ScopeDepth) {
        let mut checker = BorrowChecker {
            tc: self,
            name_depths: FxHashMap::default(),
            function_depth: None,
            reported_escapes: FxHashSet::default(),
            moves: MoveTracker::default(),
        };
        checker.record_name_depths_in_pattern(definition.pattern, depth);
        checker.walk_expr(definition.rhs, depth, true, true);
    }
}

struct BorrowChecker<'a, 'local, 'inner> {
    tc: &'a mut TypeChecker<'local, 'inner>,

    /// The scope depth each local name was bound at
    name_depths: FxHashMap<NameId, ScopeDepth>,

    /// The scope depth of the current function's body, used to check `return` statements
    function_depth: Option<ScopeDepth>,

    /// Used so we only issue an error for each [Place] once
    reported_escapes: FxHashSet<Place>,

    moves: MoveTracker,
}

impl<'a, 'local, 'inner> BorrowChecker<'a, 'local, 'inner> {
    /// Walk `expr` at `depth`, checking that each reference is still valid when used and each
    /// variable is not moved more than once.
    ///
    /// `check_move` is false on assignment's lhs and a member access' object.
    /// `record_move` is false on the rhs of reference expressions.
    fn walk_expr(&mut self, expr: ExprId, depth: ScopeDepth, check_move: bool, record_move: bool) {
        match self.tc.resolved_expr(expr).into_owned() {
            Expr::Sequence(items) => {
                let new_depth = depth.deeper();
                for item in &items {
                    self.walk_expr(item.expr, new_depth, check_move, record_move);
                }
                if let Some(tail) = items.last() {
                    self.check_escapes(tail.expr, new_depth);
                }
            },
            Expr::Definition(definition) => {
                self.record_name_depths_in_pattern(definition.pattern, depth);
                self.walk_expr(definition.rhs, depth, check_move, record_move);
            },
            Expr::If(if_) => self.walk_if(&if_, depth, check_move, record_move),
            Expr::Match(match_) => self.walk_match(&match_, depth, check_move, record_move),
            Expr::Lambda(_) => self.walk_lambda(expr, depth, None, None),
            Expr::While(while_) => self.walk_while(&while_, depth, check_move, record_move),
            Expr::For(for_) => self.walk_for(&for_, depth, check_move, record_move),
            Expr::Return(ret) => {
                self.walk_expr(ret.expression, depth, check_move, record_move);

                if let Some(boundary) = self.function_depth {
                    self.check_escapes(ret.expression, boundary);
                } else {
                    // return outside of a function, name resolution should already error here
                }
            },
            Expr::Assignment(assignment) => self.walk_assignment(&assignment, depth, check_move, record_move),
            Expr::Reference(reference) => self.walk_expr(reference.rhs, depth, check_move, false),
            Expr::MemberAccess(access) => {
                self.walk_expr(access.object, depth, false, false);
                self.check_member_access_move(&access, expr, check_move, record_move);
            },
            Expr::Call(call) => {
                self.walk_expr(call.function, depth, check_move, record_move);
                for arg in &call.arguments {
                    self.walk_expr(arg.expr, depth, check_move, record_move);
                }
            },
            Expr::TypeAnnotation(annotation) => self.walk_expr(annotation.lhs, depth, check_move, record_move),
            Expr::Constructor(constructor) => {
                for (_, field_expr) in &constructor.fields {
                    self.walk_expr(*field_expr, depth, check_move, record_move);
                }
            },
            Expr::Handle(handle) => self.walk_handle(&handle, depth),
            Expr::ArrayLiteral(elements) => {
                for element in &elements {
                    self.walk_expr(*element, depth, check_move, record_move);
                }
            },
            Expr::Variable(path) => self.walk_variable(path, expr, check_move, record_move),
            Expr::Literal(_) | Expr::Break | Expr::Continue | Expr::Error | Expr::Extern(_) | Expr::Quoted(_) => {},
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

    /// Whether `pattern` binds any part of the value it matches. Wildcards are excluded.
    fn pattern_binds_name(&self, pattern: PatternId) -> bool {
        match self.tc.pattern_of(pattern).as_ref() {
            Pattern::Variable(name) => self.tc.current_extended_context()[*name].as_str() != WILDCARD_PATTERN,
            Pattern::MethodName { .. } | Pattern::Alias(..) => true,
            Pattern::Constructor(_, args) => args.iter().any(|arg| self.pattern_binds_name(*arg)),
            Pattern::ConstructorRest(_, args, name) => {
                name.is_some() || args.iter().any(|arg| self.pattern_binds_name(*arg))
            },
            Pattern::TypeAnnotation(inner, _) => self.pattern_binds_name(*inner),
            Pattern::Or(alts) => alts.iter().any(|alt| self.pattern_binds_name(*alt)),
            Pattern::Error | Pattern::Literal(_) => false,
        }
    }

    fn walk_and_check_escapes(&mut self, expr: ExprId, depth: ScopeDepth, check_move: bool, record_move: bool) {
        self.walk_expr(expr, depth, check_move, record_move);
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
            Type::Place(atom) => self.atom_depth(&atom),
            _ => None,
        });
        depths.min()
    }

    /// The scope depth `atom` is valid for, if known.
    fn atom_depth(&self, atom: &Place) -> Option<ScopeDepth> {
        match atom {
            Place::Path(path) => self.name_depths.get(&path.root_variable()).copied(),
            Place::Anonymous(_, depth) => Some(*depth),
        }
    }

    /// Check that `value`'s type carries only places that outlive `boundary`.
    fn check_escapes(&mut self, value: ExprId, boundary: ScopeDepth) {
        let Some(typ) = self.tc.expr_types.get(&value) else { return };

        let mut offending = Vec::new();
        self.collect_place_atoms(typ, &mut offending);
        offending.retain(|atom| self.atom_depth(atom).is_some_and(|depth| depth >= boundary));

        for atom in offending {
            if !self.reported_escapes.insert(atom.clone()) {
                continue;
            }
            let location = self.narrow_value_expr(value).locate(self.tc);
            let (name, declared_at) = match atom {
                Place::Path(path) => {
                    let name = path.display_name(self.tc.current_extended_context());
                    (Some(name), path.root_variable().locate(self.tc))
                },
                Place::Anonymous(origin, _) => (None, origin.locate(self.tc)),
            };
            self.tc.compiler.accumulate(Diagnostic::EscapingReference { name, location, declared_at });
        }
    }

    /// Make an expr's location a bit more precise by returning the last expr of a block
    /// or the lhs of a type annotation.
    fn narrow_value_expr(&self, value: ExprId) -> ExprId {
        match self.tc.resolved_expr(value).as_ref() {
            Expr::Sequence(items) => match items.last() {
                Some(item) => self.narrow_value_expr(item.expr),
                None => value,
            },
            Expr::TypeAnnotation(annotation) => self.narrow_value_expr(annotation.lhs),
            _ => value,
        }
    }

    /// Recursively collect every concrete [Place] reachable from `typ`
    fn collect_place_atoms(&self, typ: &Type, out: &mut Vec<Place>) {
        match typ.follow(&self.tc.bindings) {
            Type::Application(constructor, args) => {
                self.collect_place_atoms(constructor, out);
                for arg in args.iter() {
                    self.collect_place_atoms(arg, out);
                }
            },
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    self.collect_place_atoms(element, out);
                }
            },
            Type::Places(_) | Type::Place(_) => self.collect_place_atoms_row(&typ, out),
            Type::Forall(_, typ) => self.collect_place_atoms(typ, out),
            Type::Effects(effects) => {
                if let Some(effects) = effects.as_ref() {
                    for effect in effects.iter() {
                        self.collect_place_atoms(&effect.typ, out);
                    }
                }
            },
            // A function holds its environment's places plus whatever its result may refer to
            Type::Function(function) => {
                self.collect_place_atoms(&function.environment, out);
                self.collect_place_atoms(&function.effects, out);

                // TODO: This keeps tests working but seems like a hack, investigate further
                let mut supplied = Vec::new();
                for parameter in &function.parameters {
                    self.collect_place_atoms(&parameter.typ, &mut supplied);
                }
                let mut returned = Vec::new();
                self.collect_place_atoms(&function.return_type, &mut returned);
                out.extend(returned.into_iter().filter(|atom| !supplied.contains(atom)));
            },
            Type::Primitive(_)
            | Type::Generic(_)
            | Type::Variable(_)
            | Type::UserDefined(_)
            | Type::U32(_)
            | Type::EffectId(_) => (),
        }
    }

    /// Collect the concrete places of a reference's places row
    fn collect_place_atoms_row(&self, places: &Type, out: &mut Vec<Place>) {
        let mut flattened = Vec::new();
        Type::flatten_places_into(
            std::slice::from_ref(places),
            &mut flattened,
            &self.tc.bindings,
            &TypeBindings::default(),
        );
        for entry in flattened {
            if let Type::Place(atom) = entry {
                out.push(atom);
            }
        }
    }

    /// Check that no place a value of type `typ` may refer to has been moved
    fn check_referents_moved(&mut self, typ: &Type, locator: impl Locateable) {
        let mut atoms = Vec::new();
        self.collect_place_atoms(typ, &mut atoms);

        for atom in atoms {
            let Place::Path(path) = atom else { continue };

            let moved = if let Some(moved_in) = self.moves.is_moved(&path) {
                Some((path, moved_in.clone()))
            } else {
                self.moves.has_child_moved(&path).map(|(child, moved_in)| (child.clone(), moved_in.clone()))
            };

            if let Some((moved_path, moved_in)) = moved
                && !self.moves.errored.contains(&moved_path)
            {
                let name = moved_path.display_name(self.tc.current_extended_context());
                let location = locator.locate(self.tc);
                self.tc.compiler.accumulate(Diagnostic::ReferenceToMovedValue { name, location, moved_in });
                self.moves.errored.insert(moved_path);
            }
        }
    }

    /// Check if using `path` is valid (not already moved or partially moved).
    /// Emits a diagnostic if the path was already moved.
    /// Only emits the first error per path to avoid noisy duplicate diagnostics.
    fn check_use_of_move_path(&mut self, path: &PlacePath, locator: impl Locateable) {
        if self.moves.errored.contains(path) {
            return;
        }

        // Check if this exact path or an ancestor was moved
        if let Some(moved_loc) = self.moves.is_moved(path) {
            let name = path.display_name(self.tc.current_extended_context());
            let location = locator.locate(self.tc);
            let moved_in = moved_loc.clone();
            self.tc.compiler.accumulate(Diagnostic::UseOfMovedValue { name: name.clone(), location, moved_in });
            self.moves.errored.insert(path.clone());

        // Check if any child was moved (partial move)
        } else if let Some((_child_path, moved_loc)) = self.moves.has_child_moved(path) {
            let name = path.display_name(self.tc.current_extended_context());
            let location = locator.locate(self.tc);
            let moved_in = moved_loc.clone();
            self.tc.compiler.accumulate(Diagnostic::UseOfMovedValue { name, location, moved_in });
            self.moves.errored.insert(path.clone());
        }
    }

    /// Emit errors for any variable declared outside `body_scope` that was moved within it
    fn check_moves_in_repeated_context(
        &mut self, body_scope: &MoveScope, body_depth: ScopeDepth, context: RepeatedContext,
    ) {
        let moved = body_scope.moved_paths().filter(|(path, _)| self.name_depths[&path.root_variable()] < body_depth);
        let outer_moves = mapvec(moved, |(path, location)| (path.clone(), location.clone()));

        for (path, location) in outer_moves {
            let name = path.display_name(self.tc.current_extended_context());
            self.tc.compiler.accumulate(Diagnostic::MoveInRepeatedContext { name, context, location });
        }
    }

    /// A `move` closure captures its free variables by value. Record a move for each
    /// non-Copy free variable against the outer move tracker.
    fn record_move_captures(&mut self, id: ExprId, self_name: Option<NameId>) {
        let mut context = FreeVars::default();
        if let Some(name) = self_name {
            context.defined_in_fn.insert(name);
        }
        context.find_free_variables(id, self.tc);

        let location = id.locate(self.tc);
        for name in &context.free_vars {
            let typ = self.tc.name_types[name].clone();
            if !self.tc.expr_is_copy(context.use_sites[name], &typ) {
                let move_path = self.tc.binding_place(*name);
                self.moves.record_move(move_path, location.clone());
            }
        }
    }

    fn move_path_of_assignment_lhs(&self, lhs: ExprId) -> Option<PlacePath> {
        match self.tc.resolved_expr(lhs).as_ref() {
            Expr::Reference(reference) => self.tc.try_build_move_path(reference.rhs),
            _ => self.tc.try_build_move_path(lhs),
        }
    }

    fn walk_variable(&mut self, path: PathId, expr: ExprId, check_move: bool, record_move: bool) {
        let Some(Origin::Local(name)) = self.tc.path_origin(path) else { return };

        let move_path = self.tc.binding_place(name);
        let typ = self.tc.path_types[&path].clone();
        if check_move {
            self.check_use_of_move_path(&move_path, path);
            self.check_referents_moved(&typ, path);
        }

        if record_move && !self.tc.expr_is_copy(expr, &typ) {
            let location = path.locate(self.tc);
            self.moves.record_move(move_path, location);
        }
    }

    /// The field path move check for a member access. `access.object` is walked separately, with moves
    /// fully suppressed.
    fn check_member_access_move(
        &mut self, access: &cst::MemberAccess, expr: ExprId, check_move: bool, record_move: bool,
    ) {
        if check_move {
            let typ = self.tc.expr_types[&expr].clone();
            self.check_referents_moved(&typ, expr);
        }

        let Some(parent_path) = self.tc.try_build_move_path(access.object) else { return };

        let struct_type = self.tc.expr_types[&access.object].clone();
        let indirect = struct_type.reference_element(&self.tc.bindings).is_some()
            || struct_type.pointer_element(&self.tc.bindings).is_some();
        if indirect {
            return;
        }

        let move_path = PlacePath::field(parent_path, access.member.clone());
        if check_move {
            self.check_use_of_move_path(&move_path, expr);
        }

        if record_move {
            let field_type = self.tc.expr_types[&expr].clone();
            if !self.tc.expr_is_copy(expr, &field_type) {
                let location = expr.locate(self.tc);
                self.moves.record_move(move_path, location);
            }
        }
    }

    fn walk_if(&mut self, if_: &cst::If, depth: ScopeDepth, check_move: bool, record_move: bool) {
        self.walk_expr(if_.condition, depth, check_move, record_move);
        let new_depth = depth.deeper();

        self.moves.push_scope();
        self.walk_and_check_escapes(if_.then, new_depth, check_move, record_move);
        let then_scope = self.moves.pop_scope();
        let then_type = self.tc.expr_types[&if_.then].clone();
        let then_scope = (!self.tc.diverges(&then_type)).then_some(then_scope);

        let else_scope = match if_.else_ {
            Some(else_) => {
                self.moves.push_scope();
                self.walk_and_check_escapes(else_, new_depth, check_move, record_move);
                let else_scope = self.moves.pop_scope();
                let else_type = self.tc.expr_types[&else_].clone();
                (!self.tc.diverges(&else_type)).then_some(else_scope)
            },
            None => Some(MoveScope::default()),
        };

        let branches = then_scope.into_iter().chain(else_scope).collect();
        self.moves.merge_branches_into_parent(branches);
    }

    fn walk_match(&mut self, match_: &cst::Match, depth: ScopeDepth, check_move: bool, record_move: bool) {
        // `infer_match` pushes an implicits scope before checking the expression, so its
        // real scope depth is one deeper than the match's own
        let new_depth = depth.deeper();

        // Only patterns which bind part of the pattern move it, e.g. `x is None` does not move `x`
        let binds = match_.cases.iter().any(|(pattern, _)| self.pattern_binds_name(*pattern));
        self.walk_expr(match_.expression, new_depth, check_move, record_move && binds);

        let branch_depth = new_depth.deeper();
        let mut branches = Vec::with_capacity(match_.cases.len());
        for (pattern, branch) in &match_.cases {
            self.moves.push_scope();
            self.record_name_depths_in_pattern(*pattern, branch_depth);
            self.walk_and_check_escapes(*branch, branch_depth, check_move, record_move);
            branches.push(self.moves.pop_scope());
        }
        self.moves.merge_branches_into_parent(branches);
    }

    fn walk_while(&mut self, while_: &cst::While, depth: ScopeDepth, check_move: bool, record_move: bool) {
        let body_depth = depth.deeper();
        self.moves.push_scope();
        self.walk_expr(while_.condition, depth, check_move, record_move);
        self.walk_and_check_escapes(while_.body, body_depth, check_move, record_move);
        let body_scope = self.moves.pop_scope();

        self.check_moves_in_repeated_context(&body_scope, body_depth, RepeatedContext::WhileLoop);
    }

    fn walk_for(&mut self, for_: &cst::For, depth: ScopeDepth, check_move: bool, record_move: bool) {
        self.walk_expr(for_.start, depth, check_move, record_move);
        self.walk_expr(for_.end, depth, check_move, record_move);

        let body_depth = depth.deeper();
        self.name_depths.insert(for_.variable, body_depth);

        self.moves.push_scope();
        self.walk_and_check_escapes(for_.body, body_depth, check_move, record_move);
        let body_scope = self.moves.pop_scope();

        self.check_moves_in_repeated_context(&body_scope, body_depth, RepeatedContext::ForLoop);
    }

    fn walk_assignment(
        &mut self, assignment: &cst::Assignment, depth: ScopeDepth, check_move: bool, record_move: bool,
    ) {
        // Allow `x := v` to use `x` even if moved, but `x += v` cannot since it reads `x`
        if assignment.op.is_some() {
            self.walk_expr(assignment.lhs, depth, check_move, record_move);
        } else {
            self.walk_expr(assignment.lhs, depth, false, false);
        }
        self.walk_expr(assignment.rhs, depth, check_move, record_move);
        if let Some((_, op_expr)) = assignment.op {
            self.walk_expr(op_expr, depth, check_move, record_move);
        }

        let target_depth = self.assignment_target_depth(assignment.lhs).unwrap_or(ScopeDepth(0));
        self.check_escapes(assignment.rhs, target_depth.deeper());

        // The LHS always holds a value after an assignment.
        if let Some(path) = self.move_path_of_assignment_lhs(assignment.lhs) {
            self.moves.clear_moves(&path);
        }
    }

    /// Both the handled body and each branch are lambdas, so both go through
    /// `walk_lambda` which will record that branch's `resume` parameter
    /// into `name_depths` so we don't need to do so here.
    fn walk_handle(&mut self, handle: &cst::Handle, depth: ScopeDepth) {
        self.walk_lambda(handle.expression, depth, Some(handle.handler_name), None);

        for (pattern, branch) in &handle.cases {
            // Only allow moving variables into this branch if `resume` is never mentioned.
            // This notably keeps handlers like `try_or` working.
            let uses_resume = self.tc.handler_branch_uses_resume(pattern.resume_name, *branch);
            let repeated_context = uses_resume.then_some(RepeatedContext::HandlerBranch);
            self.walk_lambda(*branch, depth, None, repeated_context);
        }
    }

    fn walk_lambda(
        &mut self, lambda_expr: ExprId, depth: ScopeDepth, self_name: Option<NameId>,
        repeated_context: Option<RepeatedContext>,
    ) {
        let Expr::Lambda(lambda) = self.tc.resolved_expr(lambda_expr).into_owned() else {
            unreachable!("walk_lambda called on a non-lambda expr")
        };

        let body_depth = depth.deeper();
        for parameter in &lambda.parameters {
            self.record_name_depths_in_pattern(parameter.pattern, body_depth);
        }

        let old_function_depth = self.function_depth.replace(body_depth);
        self.moves.push_scope();
        self.walk_and_check_escapes(lambda.body, body_depth, true, true);
        let body_scope = self.moves.pop_scope();

        if let Some(context) = repeated_context {
            self.check_moves_in_repeated_context(&body_scope, body_depth, context);
        }
        self.function_depth = old_function_depth;

        if lambda.is_move {
            self.record_move_captures(lambda_expr, self_name);
        }
    }
}
