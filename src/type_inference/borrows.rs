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
//! - On each scope exit, we check each variable only mentions places from that scope or prior.
//! - Each mutation is recorded like a move. Using a value with an unstable reference like
//! `elem: ref 'v! t` is an error if `v` was mutated since the value's holder was assigned.
//! - We drop each value that is still owned when it goes out of scope, see [super::drops].

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::{Diagnostic, Location, RepeatedContext, UnownedHolder},
    incremental::{GetItem, GetType},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{self, Expr, KindAnnotation, Pattern},
        ids::{ExprId, NameId, NameStore, PathId, PatternId, TopLevelName},
    },
    type_inference::{
        Locateable, TypeChecker,
        affine::{MoveScope, MoveTracker, PlaceExprRoot},
        drops::{BranchEnd, ChainState, ClosureState, DropScope, DropScopeKind},
        fresh_expr::DropSlot,
        value_impls::{Owned, OwnedRoot},
    },
};

use super::{
    free_variables::FreeVars,
    generics::Generic,
    patterns::WILDCARD_PATTERN,
    places::{EntryRoot, Place, PlaceEntry, PlaceRoot, ScopeDepth},
    types::{GenericSubstitutions, PrimitiveType, Type, TypeBindings},
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_borrows(&mut self, definition: &cst::Definition, depth: ScopeDepth) {
        let mut checker = BorrowChecker {
            tc: self,
            name_depths: FxHashMap::default(),
            function_depth: None,
            reported_escapes: FxHashSet::default(),
            uses: Vec::new(),
            binding_places: FxHashMap::default(),
            borrow_sites: Vec::new(),
            parents: FxHashMap::default(),
            moves: MoveTracker::default(),
            drop_scopes: Vec::new(),
            taken_apart: FxHashSet::default(),
            function_rows: Vec::new(),
            chains: Vec::new(),
            item_names: FxHashSet::default(),
            tail_resumes: FxHashSet::default(),
            closures: Vec::new(),
            loops: Vec::new(),
            diverged: false,
        };
        // Top-level definitions are never dropped so we bind their names before entering any drop scope
        checker.record_name_depths_in_pattern(definition.pattern, depth, true);

        // The drops we insert are type checked after their impls are already found, so there is
        // no need to request them here. See [super::value_impls]
        let requests_value_impls = std::mem::replace(&mut checker.tc.requests_value_impls, false);
        checker.enter_drop_scope(DropScopeKind::Block);
        checker.walk_expr(definition.rhs, depth, Use::Move);
        checker.exit_drop_scope(definition.rhs);
        checker.tc.requests_value_impls = requests_value_impls;
    }
}

/// How the value of a place expression is used
#[derive(Copy, Clone, PartialEq, Eq)]
enum Use {
    /// The value is moved out of the place, unless its type is `Copy`
    Move,
    /// The value is read in place, so it must not have been moved
    Borrow,
    /// Only part of the value is used, or it is written to. We check whether that part was
    /// moved where the part itself is used instead.
    Inspect,
}

impl Use {
    /// True if using a place that was moved out of is an error
    fn checks_moves(self) -> bool {
        self != Use::Inspect
    }

    /// Returns this use but without moving the value out of its place
    fn without_move(self) -> Use {
        if self == Use::Move { Use::Borrow } else { self }
    }
}

/// The kind of lambda being walked
#[derive(Copy, Clone, PartialEq, Eq)]
enum LambdaKind {
    /// A closure. Its body runs when it is called rather than where it is written.
    Closure,
    /// The body of a `handle` with this handler name. This runs in place.
    HandleBody(NameId),
    /// A branch of a `handle`. This may run multiple times if it uses `resume`.
    HandlerBranch { resume: NameId, uses_resume: bool },
}

pub(super) struct BorrowChecker<'a, 'local, 'inner> {
    pub(super) tc: &'a mut TypeChecker<'local, 'inner>,

    /// The scope depth each local name was bound at
    name_depths: FxHashMap<NameId, ScopeDepth>,

    /// The scope depth of the current function's body, used to check `return` statements
    pub(super) function_depth: Option<ScopeDepth>,

    /// Used so we only issue an error for each [Place] once
    reported_escapes: FxHashSet<Place>,

    /// The first use of each holder of unstable references within each enclosing loop or lambda body
    uses: Vec<Uses>,

    /// The places reachable from each binding whose type mentions any.
    /// Bindings with no places are left out to avoid unnecessary iterations over this later.
    pub(super) binding_places: FxHashMap<NameId, Vec<Place>>,

    /// Every expression whose type mentions a place, in walk order. Used to work out the location
    /// of an escaping place after the fact.
    borrow_sites: Vec<ExprId>,

    /// Maps each expression to the one it is nested within, so that [Self::escape_location] can
    /// widen a borrow outwards to the expression which actually extended its lifetime.
    parents: FxHashMap<ExprId, ExprId>,

    pub(super) moves: MoveTracker,

    /// Stack of the bindings owned by each enclosing scope. Innermost scope is last.
    pub(super) drop_scopes: Vec<DropScope>,

    /// Bindings whose fields are dropped in place of the binding itself
    pub(super) taken_apart: FxHashSet<NameId>,

    /// Stack of effect rows for each enclosing lambda. Innermost lambda is last.
    pub(super) function_rows: Vec<Type>,

    /// Names defined by the item being checked. These outlive everything within the item.
    pub(super) item_names: FxHashSet<NameId>,

    /// Stack of each [cst::Chain] whose body is being walked. Innermost chain is last.
    pub(super) chains: Vec<ChainState>,

    /// Each call to `resume` in tail position of its handler branch.
    /// See [Self::drop_before_tail_resume].
    tail_resumes: FxHashSet<ExprId>,

    /// Stack of each lambda whose body is being walked. Innermost lambda is last.
    /// Each entry is `None` unless it is a `move` closure since those are the only closures
    /// that own what they capture.
    pub(super) closures: Vec<Option<ClosureState>>,

    /// Stack of each loop whose body is being walked. Innermost loop is last.
    loops: Vec<LoopState>,

    /// True if control never reaches the point being walked. This happens when an earlier
    /// expression in the same branch jumped away or never returns. We don't drop anything here
    /// since the MIR builder will have already ended the block. Each branch and body restores
    /// this flag when it ends since control may arrive there from elsewhere.
    pub(super) diverged: bool,
}

/// A loop whose body is being walked
struct LoopState {
    /// How many move scopes enclose its body
    move_scopes: usize,

    /// What was moved and mutated within the body before each `break` or `continue`
    jumps: Vec<MoveScope>,
}

impl<'a, 'local, 'inner> BorrowChecker<'a, 'local, 'inner> {
    /// Walk `expr` at `depth`, checking that each reference is still valid when used and each
    /// variable is not moved more than once.
    ///
    /// `usage` is how the value of `expr` is used if it is a place such as `x.a.b`.
    /// Arguments and fields are always moved.
    fn walk_expr(&mut self, expr: ExprId, depth: ScopeDepth, usage: Use) {
        for child in self.child_exprs(expr) {
            self.parents.insert(child, expr);
        }

        match self.tc.resolved_expr(expr).into_owned() {
            Expr::Sequence(items) => {
                let new_depth = depth.deeper();
                self.enter_drop_scope(DropScopeKind::Block);
                for (index, item) in items.iter().enumerate() {
                    self.enter_drop_scope(DropScopeKind::Statement);
                    let last = index + 1 == items.len();
                    self.walk_expr(item.expr, new_depth, usage);
                    // Only the last statement's value is used
                    if !last {
                        self.own_temporary(item.expr);
                    }
                    self.exit_drop_scope(item.expr);
                }
                if let Some(tail) = items.last() {
                    self.check_escapes(tail.expr, new_depth);
                }
                self.check_outer_bindings(new_depth);
                self.exit_drop_scope(expr);
            },
            Expr::Definition(definition) => {
                self.walk_expr(definition.rhs, depth, usage);
                // A temporary the definition borrows lives as long as its bindings
                if let Some(borrowed) = self.reference_operand(definition.rhs) {
                    self.extend_temporary(borrowed);
                }
                self.record_name_depths_in_pattern(definition.pattern, depth, true);
            },
            Expr::If(if_) => self.walk_if(&if_, expr, depth, usage),
            Expr::Match(match_) => self.walk_match(&match_, depth, usage),
            Expr::Lambda(_) => {
                self.borrow_sites.push(expr);
                // A closure's body runs when it is called rather than where it is written
                self.walk_lambda(expr, depth, LambdaKind::Closure);
            },
            Expr::While(while_) => self.walk_while(&while_, depth, usage),
            Expr::For(for_) => self.walk_for(&for_, depth, usage),
            Expr::Return(ret) => {
                self.walk_expr(ret.expression, depth, usage);

                if let Some(boundary) = self.function_depth {
                    self.check_escapes(ret.expression, boundary);
                } else {
                    // return outside of a function, name resolution should already error here
                }
                if !self.diverged {
                    self.drop_before_jump(expr, DropScopeKind::Function);
                    self.return_from_closure(expr);
                }
            },
            Expr::Assignment(assignment) => self.walk_assignment(&assignment, expr, depth, usage),
            Expr::Reference(reference) => {
                self.borrow_sites.push(expr);
                if self.walk_operand(reference.rhs, depth, usage.without_move()) {
                    self.own_temporary(reference.rhs);
                }
            },
            Expr::MemberAccess(access) => {
                if self.walk_operand(access.object, depth, Use::Inspect) {
                    self.own_temporary(access.object);
                }
                self.check_member_access_move(&access, expr, usage);
            },
            Expr::Call(call) => {
                // We have to walk a callee that isn't a place first since if it is a lambda,
                // walking its body is what decides whether it is a `once fn`
                let in_place = self.reads_in_place(call.function);
                if !in_place {
                    self.walk_expr(call.function, depth, usage);
                }
                // Calling a `once fn` moves it. Calling any other function borrows it.
                let once = self.decide_callee_once(call.function);
                if in_place {
                    if once {
                        self.check_once_callee_is_owned(call.function);
                    }
                    self.walk_expr(call.function, depth, if once { usage } else { usage.without_move() });
                } else if !once && !self.tc.is_deref_call(call.function) {
                    // Functions called through a reference are still owned by whatever holds them
                    self.own_temporary(call.function);
                }
                for arg in &call.arguments {
                    self.walk_expr(arg.expr, depth, Use::Move);
                }
                let mutated = self.call_mutated_places(call.function);
                self.record_mutation(mutated, expr);
                if self.tail_resumes.contains(&expr) && !self.diverged {
                    self.drop_before_tail_resume(expr, &call);
                }
            },
            Expr::TypeAnnotation(annotation) => self.walk_expr(annotation.lhs, depth, usage),
            Expr::Constructor(constructor) => {
                for (_, field_expr) in &constructor.fields {
                    self.walk_expr(*field_expr, depth, Use::Move);
                }
            },
            Expr::Handle(handle) => self.walk_handle(&handle, depth),
            Expr::ArrayLiteral(elements) => {
                for element in &elements {
                    self.walk_expr(*element, depth, Use::Move);
                }
            },
            Expr::Variable(path) => self.walk_variable(path, expr, usage),
            Expr::Break | Expr::Continue if !self.diverged => {
                self.drop_before_jump(expr, DropScopeKind::Loop);
                self.jump_to_loop();
            },
            Expr::Chain(chain) => self.walk_chain(&chain, depth, usage),
            Expr::Fallthrough if !self.diverged => self.fall_through(expr),
            Expr::Break | Expr::Continue | Expr::Fallthrough => {},
            Expr::Literal(_) | Expr::Error | Expr::Extern(_) | Expr::Quoted(_) => {},
            Expr::Is(_) | Expr::Do(_) | Expr::Loop(_) | Expr::InterpolatedString(_) => {
                unreachable!("desugared before type inference")
            },
        }

        if self.never_returns(expr) {
            self.diverged = true;
        }
    }

    /// True if control never continues after evaluating `expr`. This should match each case
    /// where the MIR builder ends the block after `expr`: jumps and calls to functions
    /// returning `Never`.
    fn never_returns(&self, expr: ExprId) -> bool {
        let diverges = |typ: Option<&Type>| typ.is_some_and(|typ| self.tc.diverges(typ));
        if diverges(self.tc.expr_types.get(&expr)) {
            return true;
        }
        let callee = match self.tc.resolved_expr(expr).as_ref() {
            Expr::Call(call) => call.function,
            _ => return false,
        };
        self.tc.expr_function_type(callee).is_some_and(|function| diverges(Some(&function.return_type)))
    }

    /// Record what the innermost loop's body moved and mutated before reaching a `break` or
    /// `continue`. We need to do this here since the branch being jumped out of never reaches
    /// its end, so its state wouldn't otherwise make it to the end of the loop.
    fn jump_to_loop(&mut self) {
        let Some(state) = self.loops.last_mut() else { return };
        // The body's own scope already reaches the end of the loop either way
        if self.moves.scope_count() > state.move_scopes + 1 {
            state.jumps.push(self.moves.state_since(state.move_scopes + 1));
        }
    }

    /// Start walking the body of a loop. Its move scope should already be entered.
    fn enter_loop(&mut self) {
        self.loops.push(LoopState { move_scopes: self.moves.scope_count() - 1, jumps: Vec::new() });
    }

    /// Leave the body of the innermost loop, merging in the state of each jump out of it
    fn exit_loop(&mut self) {
        let state = self.loops.pop().expect("exit_loop called without a loop");
        for jump in state.jumps {
            self.moves.merge_jump(jump);
        }
    }

    /// Record the scope depth `name` was bound at, along with any places its type may refer to.
    /// `owns` is false if `name` is bound by a pattern that borrows what it matches.
    fn bind_name(&mut self, name: NameId, depth: ScopeDepth, owns: bool) {
        self.name_depths.insert(name, depth);
        if self.drop_scopes.is_empty() {
            self.item_names.insert(name);
        } else if owns {
            self.own_binding(name);
        }

        let Some(typ) = self.tc.name_types.get(&name) else { return };
        let mut entries = Vec::new();
        self.tc.collect_place_entries(typ, &mut entries);

        // Mutations before now can't invalidate this binding's references
        if !entries.is_empty() {
            self.moves.clear_moves(&self.tc.binding_place(name));
        }

        let places: Vec<_> = entries.into_iter().filter_map(PlaceEntry::concrete).collect();
        if !places.is_empty() {
            self.binding_places.insert(name, places);
        }
    }

    /// Record the scope depth of every name a pattern binds.
    fn record_name_depths_in_pattern(&mut self, pattern: PatternId, depth: ScopeDepth, owns: bool) {
        self.record_names_in_pattern(pattern, depth, owns, false);
    }

    /// Same as [Self::record_name_depths_in_pattern] but `in_or` is set if we're within an
    /// alternative of an or-pattern
    fn record_names_in_pattern(&mut self, pattern: PatternId, depth: ScopeDepth, owns: bool, in_or: bool) {
        match self.tc.pattern_of(pattern).as_ref() {
            Pattern::Variable(name) | Pattern::MethodName { item_name: name, .. } => {
                // Wildcards in one alternative aren't bound by the others, so there's nothing to
                // drop them with once the alternatives join. Whatever they match is leaked.
                let owns = owns && !(in_or && self.is_wildcard(*name));
                self.bind_name(*name, depth, owns);
            },
            Pattern::Alias(name, inner) => {
                self.bind_name(*name, depth, owns);
                self.record_names_in_pattern(*inner, depth, owns, in_or);
            },
            Pattern::Constructor(_, args) => {
                let owns = owns && self.pattern_moves_parts(pattern);
                for arg in args {
                    self.record_names_in_pattern(*arg, depth, owns, in_or);
                }
            },
            Pattern::ConstructorRest(_, args, name) => {
                let owns = owns && self.pattern_moves_parts(pattern);
                for arg in args {
                    self.record_names_in_pattern(*arg, depth, owns, in_or);
                }
                if let Some(name) = name {
                    self.bind_name(*name, depth, owns);
                }
            },
            Pattern::TypeAnnotation(inner, _) => self.record_names_in_pattern(*inner, depth, owns, in_or),
            // Each alternative binds the same names and name resolution links the names in the
            // others to those in the first, so we only need to bind the first alternative's names
            Pattern::Or(alts) => {
                if let Some(alt) = alts.first() {
                    self.record_names_in_pattern(*alt, depth, owns, true);
                }
            },
            Pattern::Error | Pattern::Literal(_) => {},
        }
    }

    fn is_wildcard(&self, name: NameId) -> bool {
        self.tc.current_extended_context()[name].as_str() == WILDCARD_PATTERN
    }

    /// False if `pattern` matches a `Copy` value such as a `shared` type or a reference. In
    /// that case the pattern only copies the value's parts and they stay with its owner.
    fn pattern_moves_parts(&mut self, pattern: PatternId) -> bool {
        let Some(typ) = self.tc.pattern_types.get(&pattern) else { return true };
        !self.tc.copy_impl_found(&Owned::new(OwnedRoot::Pattern(pattern)), typ)
    }

    /// Calling a `once fn` moves it, so unless it is `Copy` we need to be able to move it out
    /// of whatever holds it. This means it can't be a global or reached through a reference or
    /// pointer.
    fn check_once_callee_is_owned(&mut self, callee: ExprId) {
        let Some(holder) = self.unowned_holder(callee) else { return };
        if !self.tc.expr_is_copy(callee, &self.tc.expr_types[&callee]) {
            self.tc.report_unowned_once_call(holder, callee.locate(self.tc));
        }
    }

    /// Returns what holds the place `expr` reads if `expr` cannot be moved out of it
    fn unowned_holder(&self, expr: ExprId) -> Option<UnownedHolder> {
        let place = self.tc.classify_place(expr)?;
        if self.tc.place_is_indirect(expr) {
            Some(UnownedHolder::Reference)
        } else {
            (place.root == PlaceExprRoot::Global).then_some(UnownedHolder::Global)
        }
    }

    /// If `rhs`, the value of a definition, is a reference expression, return what it borrows
    fn reference_operand(&self, rhs: ExprId) -> Option<ExprId> {
        match self.tc.resolved_expr(rhs).as_ref() {
            Expr::Reference(reference) => Some(reference.rhs),
            Expr::TypeAnnotation(annotation) => self.reference_operand(annotation.lhs),
            _ => None,
        }
    }

    /// True if `expr` reads a value held elsewhere rather than producing a new value that
    /// something then needs to own. This is the case for local places and globals, the latter
    /// of which are never moved or dropped.
    fn reads_in_place(&self, expr: ExprId) -> bool {
        self.tc.classify_place(expr).is_some_and(|place| place.is_named())
    }

    /// Whether `pattern` binds any part of the value it matches. Wildcards are excluded.
    fn pattern_binds_name(&self, pattern: PatternId) -> bool {
        match self.tc.pattern_of(pattern).as_ref() {
            Pattern::Variable(name) => !self.is_wildcard(*name),
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

    fn walk_and_check_escapes(&mut self, expr: ExprId, depth: ScopeDepth, usage: Use) {
        self.walk_expr(expr, depth, usage);
        self.check_escapes(expr, depth);
        self.check_outer_bindings(depth);
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
        let mut atoms = Vec::new();
        self.collect_place_atoms(&places, &mut atoms);
        atoms.iter().filter_map(|atom| self.atom_depth(atom)).min()
    }

    /// The scope depth `atom` is valid for, if known.
    pub(super) fn atom_depth(&self, atom: &Place) -> Option<ScopeDepth> {
        match atom.root {
            PlaceRoot::Variable(name) => self.name_depths.get(&name).copied(),
            PlaceRoot::Anonymous(_, depth) => Some(depth),
        }
    }

    /// Check that `value`'s type carries only places that outlive `boundary`.
    pub(super) fn check_escapes(&mut self, value: ExprId, boundary: ScopeDepth) {
        let Some(typ) = self.tc.expr_types.get(&value) else { return };
        let mut atoms = Vec::new();
        self.collect_place_atoms(typ, &mut atoms);
        atoms.retain(|atom| self.atom_depth(atom).is_some_and(|depth| depth >= boundary));

        let location = self.narrow_value_expr(value);
        for atom in atoms {
            self.report_escape(atom, location);
        }
    }

    /// A binding declared outside the scope being left may have been widened, by a store through a
    /// `mut` reference or an assignment, to refer to a place declared inside it. The block tail and
    /// branch checks only look at one value's type, so nothing else catches this.
    fn check_outer_bindings(&mut self, boundary: ScopeDepth) {
        let mut escapes = Vec::new();
        for (name, places) in &self.binding_places {
            if self.name_depths.get(name).is_none_or(|depth| *depth >= boundary) {
                continue;
            }
            // Skipping already-reported places here rather than in report_escape keeps us from
            // searching for an error location we would only throw away.
            let offending = places.iter().filter(|atom| {
                !self.reported_escapes.contains(atom) && self.atom_depth(atom).is_some_and(|d| d >= boundary)
            });
            escapes.extend(offending.cloned().map(|atom| (atom, *name)));
        }

        for (atom, name) in escapes {
            let carrier = self.tc.name_types.get(&name).cloned();
            match carrier.and_then(|carrier| self.escape_location(&atom, name, &carrier)) {
                Some(location) => self.report_escape(atom, location),
                None => self.report_escape(atom, name),
            }
        }

        // Drop the bindings this scope owned so they aren't rechecked later
        let Self { binding_places, name_depths, .. } = self;
        binding_places.retain(|name, _| name_depths.get(name).is_some_and(|depth| *depth < boundary));
    }

    /// The expression to blame for `atom` escaping out of `binding`, if one is known.
    fn escape_location(&self, atom: &Place, binding: NameId, carrier: &Type) -> Option<ExprId> {
        if let PlaceRoot::Anonymous(origin, _) = atom.root {
            return Some(origin);
        }

        let target = self.row_carrying(carrier, atom)?;

        if let Some(location) = self.widening_argument(atom, binding, &target) {
            return Some(location);
        }

        let mut last_origin = None;
        for origin in self.borrow_sites.iter().rev().copied() {
            if !self.places_originated_by(origin).contains(atom) {
                continue;
            }
            last_origin.get_or_insert(origin);

            let mut current = Some(origin);
            while let Some(expr) = current {
                let row = self.tc.expr_types.get(&expr).and_then(|typ| self.row_carrying(typ, atom));
                if row.is_some_and(|row| is_subrow(&row, &target)) {
                    return Some(expr);
                }
                current = self.parents.get(&expr).copied();
            }
        }

        last_origin
    }

    /// Find a call which is passed a borrow of `binding`, and return the argument beside it which
    /// carries `atom` in `target`.
    fn widening_argument(&self, atom: &Place, binding: NameId, target: &[Type]) -> Option<ExprId> {
        let binding_place = self.tc.binding_place(binding);

        for borrow in self.borrow_sites.iter().rev().copied() {
            if !self.places_originated_by(borrow).contains(&binding_place) {
                continue;
            }

            let Some(parent) = self.parents.get(&borrow).copied() else { continue };
            let Expr::Call(call) = self.tc.resolved_expr(parent).into_owned() else { continue };

            let stored = call.arguments.iter().map(|arg| arg.expr).find(|arg| {
                *arg != borrow
                    && self
                        .tc
                        .expr_types
                        .get(arg)
                        .and_then(|typ| self.row_carrying(typ, atom))
                        .is_some_and(|row| is_subrow(&row, target))
            });

            if stored.is_some() {
                return stored;
            }
        }

        None
    }

    /// The places `expr` creates, as opposed to just carries.
    fn places_originated_by(&self, expr: ExprId) -> Vec<Place> {
        let mut places = Vec::new();
        match self.tc.resolved_expr(expr).as_ref() {
            Expr::Reference(reference) => {
                let row = self.tc.infer_place(reference.rhs);
                self.collect_place_atoms(&row, &mut places);
            },
            Expr::Lambda(_) => {
                if let Some(typ) = self.tc.expr_types.get(&expr)
                    && let Type::Function(function) = typ.follow(&self.tc.bindings)
                {
                    self.collect_place_atoms(&function.environment, &mut places);
                }
            },
            _ => (),
        }
        places
    }

    /// The canonicalized places row within `typ` which carries `atom`, if any.
    fn row_carrying(&self, typ: &Type, atom: &Place) -> Option<Vec<Type>> {
        match typ.follow(&self.tc.bindings) {
            Type::Application(constructor, args) => self
                .row_carrying(constructor, atom)
                .or_else(|| args.iter().find_map(|arg| self.row_carrying(arg, atom))),
            Type::Tuple(elements) => elements.iter().find_map(|element| self.row_carrying(element, atom)),
            Type::Places(_) | Type::Place(..) | Type::PlaceProjection(..) => {
                let row =
                    Type::canonicalize_places(std::slice::from_ref(typ), &self.tc.bindings, &TypeBindings::default());
                row.iter().any(|entry| matches!(entry, Type::Place(place, _) if place == atom)).then_some(row)
            },
            Type::Forall(_, typ) => self.row_carrying(typ, atom),
            Type::Effects(effects) => {
                effects.as_ref().and_then(|effects| effects.iter().find_map(|effect| self.row_carrying(effect, atom)))
            },
            Type::Function(function) => self
                .row_carrying(&function.environment, atom)
                .or_else(|| self.row_carrying(&function.effects, atom))
                .or_else(|| self.row_carrying(&function.return_type, atom))
                .or_else(|| function.parameters.iter().find_map(|p| self.row_carrying(&p.typ, atom))),
            Type::Primitive(_) | Type::Generic(_) | Type::Variable(_) | Type::UserDefined(_) | Type::U32(_) => None,
        }
    }

    /// Every expression directly nested within `expr`
    pub(super) fn child_exprs(&self, expr: ExprId) -> Vec<ExprId> {
        match self.tc.resolved_expr(expr).as_ref() {
            Expr::Sequence(items) => mapvec(items, |item| item.expr),
            Expr::Definition(definition) => vec![definition.rhs],
            Expr::MemberAccess(access) => vec![access.object],
            Expr::Call(call) => {
                let mut children = vec![call.function];
                children.extend(call.arguments.iter().map(|arg| arg.expr));
                children
            },
            Expr::Lambda(lambda) => vec![lambda.body],
            Expr::If(if_) => {
                let mut children = vec![if_.condition, if_.then];
                children.extend(if_.else_);
                children
            },
            Expr::Match(match_) => {
                let mut children = vec![match_.expression];
                children.extend(match_.cases.iter().map(|(_, branch)| *branch));
                children
            },
            Expr::Handle(handle) => {
                let mut children = vec![handle.expression];
                children.extend(handle.cases.iter().map(|(_, branch)| *branch));
                children
            },
            Expr::Chain(chain) => vec![chain.body, chain.else_],
            Expr::Reference(reference) => vec![reference.rhs],
            Expr::TypeAnnotation(annotation) => vec![annotation.lhs],
            Expr::Constructor(constructor) => constructor.fields.iter().map(|(_, field)| *field).collect(),
            Expr::While(while_) => vec![while_.condition, while_.body],
            Expr::For(for_) => vec![for_.start, for_.end, for_.body],
            Expr::Return(ret) => vec![ret.expression],
            Expr::Assignment(assignment) => {
                let mut children = vec![assignment.lhs, assignment.rhs];
                children.extend(assignment.op.map(|(_, op_expr)| op_expr));
                children
            },
            Expr::ArrayLiteral(elements) => elements.clone(),
            Expr::Literal(_)
            | Expr::Variable(_)
            | Expr::Break
            | Expr::Continue
            | Expr::Fallthrough
            | Expr::Error
            | Expr::Extern(_)
            | Expr::Quoted(_) => Vec::new(),
            Expr::Is(_) | Expr::Do(_) | Expr::Loop(_) | Expr::InterpolatedString(_) => {
                unreachable!("desugared before type inference")
            },
        }
    }

    /// Issue an escaping reference error for `atom`, avoiding multiple errors for the same place
    pub(super) fn report_escape(&mut self, atom: Place, location: impl Locateable) {
        if !self.reported_escapes.insert(atom.clone()) {
            return;
        }
        let location = location.locate(self.tc);
        let (name, declared_at) = match atom.root {
            PlaceRoot::Variable(root) => {
                let name = atom.display_name(self.tc.current_extended_context());
                (Some(name), root.locate(self.tc))
            },
            PlaceRoot::Anonymous(origin, _) => (None, origin.locate(self.tc)),
        };
        self.tc.compiler.accumulate(Diagnostic::EscapingReference { name, location, declared_at });
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
        let mut entries = Vec::new();
        self.tc.collect_place_entries(typ, &mut entries);
        out.extend(entries.into_iter().filter_map(PlaceEntry::concrete));
    }

    /// Record that `mutated` was mutated at `locator`
    fn record_mutation(&mut self, mutated: Vec<PlaceEntry>, locator: impl Locateable) {
        if !mutated.is_empty() {
            let location = locator.locate(self.tc);
            self.moves.record_mutation(mutated, location);
        }
    }

    /// The places in each `Mutate` effect calling `function` performs
    fn call_mutated_places(&self, function: ExprId) -> Vec<PlaceEntry> {
        self.tc
            .expr_function_type(function)
            .map_or_else(Vec::new, |function| self.tc.mutated_entries(&function.effects))
    }

    /// Error if a place `name` mutates is instantiated at `path` to one overlapping another of its place generics
    fn check_mutated_places_distinct(&mut self, path: PathId, name: TopLevelName) {
        let Some(instantiation) = self.tc.current_extended_context().get_instantiation(path) else { return };
        let Type::Forall(generics, body) = GetType(name).get(self.tc.compiler) else { return };
        let Type::Function(function) = body.as_ref() else { return };
        let substitutions: GenericSubstitutions = generics.iter().copied().zip(instantiation.iter().cloned()).collect();

        let mut mutated = Vec::new();
        let mut effect_mutations = Vec::new();
        for effect in self.tc.collect_effects(&function.effects, &TypeBindings::default()) {
            match (self.tc.mutated_places(&effect), effect) {
                (Some(places), _) => self.collect_place_generics_in_row(&places, &mut mutated),
                (None, Type::Generic(generic)) => {
                    let places = self.tc.mutated_entries(&substitutions[&generic]);
                    if !places.is_empty() {
                        effect_mutations.push((generic, places));
                    }
                },
                (None, _) => (),
            }
        }
        if mutated.is_empty() && effect_mutations.is_empty() {
            return;
        }

        let mut place_generics = Vec::new();
        for parameter in &function.parameters {
            self.collect_place_generics(&parameter.typ, &mut place_generics);
        }
        self.collect_place_generics(&function.return_type, &mut place_generics);
        let place_generics =
            mapvec(place_generics, |generic| (generic, self.tc.place_entries(&substitutions[&generic])));

        let (_, names) = GetItem(name.top_level_item).get(self.tc.compiler);
        let named = |generic| match generic {
            Generic::Named(Origin::Local(name)) => Some(name),
            _ => None,
        };
        let place_name =
            |generic| named(generic).map_or("an inferred place".to_string(), |name| format!("'{}", names[name]));
        let effect_name = |generic| named(generic).map_or("its arguments".to_string(), |name| names[name].to_string());
        let function_name = &names[name.local_name_id];

        for (i, (first, first_places)) in place_generics.iter().enumerate() {
            for (second, second_places) in &place_generics[i + 1..] {
                if (mutated.contains(first) || mutated.contains(second))
                    && let Some(place) = overlapping(first_places, second_places)
                {
                    self.report_alias(function_name, place_name(*first), place_name(*second), place, path);
                }
            }

            for (effect, mutated) in &effect_mutations {
                if let Some(place) = overlapping(mutated, first_places) {
                    let effect = format!("a place mutated by {}", effect_name(*effect));
                    self.report_alias(function_name, place_name(*first), effect, place, path);
                }
            }
        }
    }

    fn report_alias(
        &self, function: &str, first: String, second: String, place: &PlaceEntry, locator: impl Locateable,
    ) {
        let function = function.to_string();
        let place = self.entry_display_name(place);
        let location = locator.locate(self.tc);
        self.tc.compiler.accumulate(Diagnostic::PlacesMayAlias { function, first, second, place, location });
    }

    /// The name of a place in the current item for error messages, if it has one
    fn entry_display_name(&self, place: &PlaceEntry) -> Option<String> {
        let names = self.tc.current_extended_context();
        let root = match place.root {
            EntryRoot::Concrete(root) => Place { root, segments: Vec::new() }.display_name(names),
            EntryRoot::Generic(Generic::Named(Origin::Local(name))) => names.try_get_name(name)?.to_string(),
            EntryRoot::Generic(_) | EntryRoot::Unbound(_) => return None,
        };
        Some(format!("'{root}{}", cst::Segments(&place.segments)))
    }

    /// Collect each generic used as a place within the generalized type `typ`, except in function parameters
    fn collect_place_generics(&self, typ: &Type, out: &mut Vec<Generic>) {
        match typ {
            Type::Application(constructor, args) => {
                let place_parameters = match constructor.as_ref() {
                    Type::Primitive(PrimitiveType::Reference) => vec![true, false],
                    Type::UserDefined(Origin::TopLevelDefinition(name)) => {
                        let (item, _) = GetItem(name.top_level_item).get(self.tc.compiler);
                        match &item.kind {
                            cst::TopLevelItemKind::TypeDefinition(definition) => {
                                mapvec(&definition.generics, |param| param.kind == Some(KindAnnotation::Place))
                            },
                            _ => Vec::new(),
                        }
                    },
                    _ => Vec::new(),
                };
                for (i, arg) in args.iter().enumerate() {
                    if place_parameters.get(i).copied().unwrap_or(false) {
                        self.collect_place_generics_in_row(arg, out);
                    } else {
                        self.collect_place_generics(arg, out);
                    }
                }
            },
            Type::Places(_) | Type::Place(..) | Type::PlaceProjection(..) => {
                self.collect_place_generics_in_row(typ, out)
            },
            // A function parameter's places are given by the callee rather than to it
            Type::Function(function) => {
                self.collect_place_generics(&function.environment, out);
                self.collect_place_generics(&function.return_type, out);
                self.collect_place_generics(&function.effects, out);
            },
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    self.collect_place_generics(element, out);
                }
            },
            Type::Effects(Some(effects)) => {
                for effect in effects.iter() {
                    self.collect_place_generics(effect, out);
                }
            },
            Type::Forall(_, typ) => self.collect_place_generics(typ, out),
            Type::Effects(None)
            | Type::Primitive(_)
            | Type::Generic(_)
            | Type::Variable(_)
            | Type::UserDefined(_)
            | Type::U32(_) => (),
        }
    }

    /// Collect each generic in the generalized places row `places`
    fn collect_place_generics_in_row(&self, places: &Type, out: &mut Vec<Generic>) {
        for entry in self.tc.place_entries(places) {
            if let EntryRoot::Generic(generic) = entry.root
                && !out.contains(&generic)
            {
                out.push(generic);
            }
        }
    }

    /// Check that no place `typ` may refer to was moved, or mutated since `holder` was assigned
    fn check_referents(&mut self, typ: &Type, locator: impl Locateable, holder: Option<&Place>) {
        let mut entries = Vec::new();
        self.tc.collect_place_entries(typ, &mut entries);

        for entry in &entries {
            let EntryRoot::Concrete(root @ PlaceRoot::Variable(_)) = entry.root else { continue };
            let path = Place { root, segments: entry.segments.clone() }.stable_prefix();

            if let Some((moved_path, moved_in)) = self.moves.moved(&path)
                && !self.moves.errored.contains(moved_path)
            {
                let (moved_path, moved_in) = (moved_path.clone(), moved_in.clone());
                let name = moved_path.display_name(self.tc.current_extended_context());
                let location = locator.locate(self.tc);
                self.tc.compiler.accumulate(Diagnostic::ReferenceToMovedValue { name, location, moved_in });
                self.moves.errored.insert(moved_path);
            }
        }

        let unstable: Vec<_> = entries.into_iter().filter(PlaceEntry::is_unstable).collect();
        let Some(holder) = holder.filter(|_| !unstable.is_empty()) else { return };
        let location = locator.locate(self.tc);

        if let Some((name, invalidated_at)) = self.invalidation(holder, &unstable) {
            let location = location.clone();
            self.tc.compiler.accumulate(Diagnostic::UseOfInvalidatedReference { name, location, invalidated_at });
        }

        if let Some(uses) = self.uses.last_mut() {
            uses.entry(holder.clone()).or_insert((location, unstable));
        }
    }

    /// The name of `holder` and where it was invalidated, if a mutation since it was assigned invalidates `entries`
    fn invalidation(&mut self, holder: &Place, entries: &[PlaceEntry]) -> Option<(String, Location)> {
        let invalidated_at = self.moves.invalidated(holder, entries)?.clone();
        let names = self.tc.current_extended_context();
        self.moves.errored.insert(holder.clone()).then(|| (holder.display_name(names), invalidated_at))
    }

    /// Check if using `path` is valid (not already moved or partially moved).
    /// Emits a diagnostic if the path was already moved.
    /// Only emits the first error per path to avoid noisy duplicate diagnostics.
    fn check_use_of_move_path(&mut self, path: &Place, locator: impl Locateable) {
        if self.moves.errored.contains(path) {
            return;
        }

        if let Some((_, moved_in)) = self.moves.moved(path) {
            let name = path.display_name(self.tc.current_extended_context());
            let location = locator.locate(self.tc);
            let moved_in = moved_in.clone();
            self.tc.compiler.accumulate(Diagnostic::UseOfMovedValue { name, location, moved_in });
            self.moves.errored.insert(path.clone());
        }
    }

    /// Emit errors for any variable declared outside `body_scope` that was moved within it
    fn check_moves_in_repeated_context(
        &mut self, body_scope: &MoveScope, body_depth: ScopeDepth, context: RepeatedContext,
    ) {
        self.report_outer_moves(body_scope, body_depth, |name, location| Diagnostic::MoveInRepeatedContext {
            name,
            context,
            location,
        });
    }

    /// Emit `diagnostic` for each place declared outside `body_scope` that was moved within it
    fn report_outer_moves(
        &mut self, body_scope: &MoveScope, body_depth: ScopeDepth, diagnostic: impl Fn(String, Location) -> Diagnostic,
    ) {
        let moved =
            body_scope.moved_paths().filter(|(path, _)| self.atom_depth(path).is_some_and(|depth| depth < body_depth));
        for (path, location) in mapvec(moved, |(path, location)| (path.clone(), location.clone())) {
            let name = path.display_name(self.tc.current_extended_context());
            self.tc.compiler.accumulate(diagnostic(name, location));
        }
    }

    /// Closures other than `move` closures capture by reference, so each captured variable still
    /// owns its value. Emit an error for each such variable moved within the closure's body.
    fn check_moves_out_of_reference_captures(&mut self, body_scope: &MoveScope, body_depth: ScopeDepth) {
        self.report_outer_moves(body_scope, body_depth, |name, location| Diagnostic::MoveOutOfReferenceCapture {
            name,
            location,
        });
    }

    /// Emit errors for any outer holder the current body uses then may invalidate before running again
    fn check_invalidations_in_repeated_context(
        &mut self, uses: &Uses, body_depth: ScopeDepth, context: RepeatedContext,
    ) {
        for (holder, (location, entries)) in uses {
            if self.atom_depth(holder).is_none_or(|depth| depth >= body_depth) {
                continue;
            }
            if let Some((name, invalidated_at)) = self.invalidation(holder, entries) {
                let location = location.clone();
                let diagnostic = Diagnostic::InvalidatedInRepeatedContext { name, context, location, invalidated_at };
                self.tc.compiler.accumulate(diagnostic);
            }
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

    fn move_path_of_assignment_lhs(&self, lhs: ExprId) -> Option<Place> {
        match self.tc.resolved_expr(lhs).as_ref() {
            Expr::Reference(reference) => self.tc.try_build_move_path(reference.rhs),
            _ => self.tc.try_build_move_path(lhs),
        }
    }

    fn walk_variable(&mut self, path: PathId, expr: ExprId, usage: Use) {
        let name = match self.tc.path_origin(path) {
            Some(Origin::Local(name)) => name,
            Some(Origin::TopLevelDefinition(name)) => return self.check_mutated_places_distinct(path, name),
            _ => return,
        };

        let move_path = self.tc.binding_place(name);
        let typ = self.tc.path_types[&path].clone();
        if usage.checks_moves() {
            self.check_use_of_move_path(&move_path, path);
            self.check_referents(&typ, path, Some(&move_path));
        }

        if usage == Use::Move && !self.tc.expr_is_copy(expr, &typ) {
            let location = path.locate(self.tc);
            self.moves.record_move(move_path, location);
        }
    }

    /// The field path move check for a member access. `access.object` is walked separately, with moves
    /// fully suppressed.
    fn check_member_access_move(&mut self, access: &cst::MemberAccess, expr: ExprId, usage: Use) {
        let struct_type = self.tc.expr_types[&access.object].clone();
        let indirect = struct_type.reference_element(&self.tc.bindings).is_some()
            || struct_type.pointer_element(&self.tc.bindings).is_some();

        // A field reached through a reference is held by the object rather than the field
        let parent_path = self.tc.try_build_move_path(access.object);
        let holder =
            parent_path.map(|parent| if indirect { parent } else { parent.field(self.tc.member_name(access).clone()) });

        if usage.checks_moves() {
            let typ = self.tc.expr_types[&expr].clone();
            self.check_referents(&typ, expr, holder.as_ref());
        }

        let Some(move_path) = holder.filter(|_| !indirect) else { return };
        if usage.checks_moves() {
            self.check_use_of_move_path(&move_path, expr);
        }

        if usage == Use::Move {
            let field_type = self.tc.expr_types[&expr].clone();
            if !self.tc.expr_is_copy(expr, &field_type) {
                let location = expr.locate(self.tc);
                self.moves.record_move(move_path, location);
            }
        }
    }

    fn walk_if(&mut self, if_: &cst::If, expr: ExprId, depth: ScopeDepth, usage: Use) {
        self.walk_expr(if_.condition, depth, usage);
        let new_depth = depth.deeper();

        // Without an `else` the value of the `then` branch is unused
        let then_scope = self.walk_branch(if_.then, new_depth, usage, if_.else_.is_none());
        let else_ = match if_.else_ {
            Some(else_) => {
                let scope = self.walk_branch(else_, new_depth, usage, false);
                (scope, BranchEnd::Slot(DropSlot::After(else_)))
            },
            None => (Some(MoveScope::default()), BranchEnd::Slot(DropSlot::Otherwise(expr))),
        };

        let branches = vec![(then_scope, BranchEnd::Slot(DropSlot::After(if_.then))), else_];
        let branches = self.balance_branches(branches, new_depth);
        self.moves.merge_branches_into_parent(branches);
    }

    /// Walk the conditions and `then` of a chain, followed by its `else` once. The `else` is
    /// walked as if reached from every condition that may fail. See [cst::Chain].
    fn walk_chain(&mut self, chain: &cst::Chain, depth: ScopeDepth, usage: Use) {
        let new_depth = depth.deeper();

        self.moves.push_scope();
        let state = ChainState {
            drop_scopes: self.drop_scopes.len(),
            move_scopes: self.moves.scope_count() - 1,
            fallthroughs: Vec::new(),
        };
        self.chains.push(state);
        self.enter_drop_scope(DropScopeKind::Block);
        let body = self.finish_branch(chain.body, new_depth, usage, false);
        let state = self.chains.pop().expect("the chain pushed above");

        // Balance the failures so that each one owns the same values before the `else` runs
        let failures =
            mapvec(state.fallthroughs, |(moves, expr)| (Some(moves), BranchEnd::Slot(DropSlot::BeforeJump(expr))));
        let failures = self.balance_branches(failures, new_depth);
        self.moves.push_scope();
        self.moves.merge_branches_into_parent(failures);
        self.enter_drop_scope(DropScopeKind::Block);
        let else_ = self.finish_branch(chain.else_, new_depth, usage, false);

        let branches = vec![
            (body, BranchEnd::Slot(DropSlot::After(chain.body))),
            (else_, BranchEnd::Slot(DropSlot::After(chain.else_))),
        ];
        let branches = self.balance_branches(branches, new_depth);
        self.moves.merge_branches_into_parent(branches);
    }

    /// Walk `expr`, using it in place if it is a place. Otherwise `expr` produces a new value,
    /// so it always moves out of whatever it reads and we return true so the caller can own it.
    fn walk_operand(&mut self, expr: ExprId, depth: ScopeDepth, usage: Use) -> bool {
        let place = self.reads_in_place(expr);
        self.walk_expr(expr, depth, if place { usage } else { Use::Move });
        !place
    }

    /// Walk one branch of an `if` in its own scope, returning what it moved unless it diverges
    fn walk_branch(&mut self, branch: ExprId, depth: ScopeDepth, usage: Use, value_unused: bool) -> Option<MoveScope> {
        self.moves.push_scope();
        self.enter_drop_scope(DropScopeKind::Block);
        self.finish_branch(branch, depth, usage, value_unused)
    }

    /// Walk `branch` then exit the scopes entered for it, returning what it moved unless it diverges
    fn finish_branch(
        &mut self, branch: ExprId, depth: ScopeDepth, usage: Use, value_unused: bool,
    ) -> Option<MoveScope> {
        let diverged = self.diverged;
        self.walk_and_check_escapes(branch, depth, usage);
        if value_unused {
            self.own_temporary(branch);
        }
        self.exit_drop_scope(branch);
        let scope = self.moves.pop_scope();
        let reaches_end = !std::mem::replace(&mut self.diverged, diverged);
        reaches_end.then_some(scope)
    }

    fn walk_match(&mut self, match_: &cst::Match, depth: ScopeDepth, usage: Use) {
        // `infer_match` pushes an implicits scope before checking the expression, so its
        // real scope depth is one deeper than the match's own
        let new_depth = depth.deeper();

        // Only patterns which bind part of the pattern move it, e.g. `x is None` does not move `x`
        let binds = match_.cases.iter().any(|(pattern, _)| self.pattern_binds_name(*pattern));
        let scrutinee_usage = if binds { usage } else { usage.without_move() };
        if self.walk_operand(match_.expression, new_depth, scrutinee_usage) && !binds {
            self.own_temporary(match_.expression);
        }

        // Patterns that bind nothing leave what they match with its owner. The same goes for
        // patterns matching a `Copy` value such as a `shared` type since they only copy its parts.
        let patterns_own = binds && !self.tc.expr_is_copy(match_.expression, &self.tc.expr_types[&match_.expression]);

        let branch_depth = new_depth.deeper();
        let mut branches = Vec::with_capacity(match_.cases.len());
        for (pattern, branch) in &match_.cases {
            self.moves.push_scope();
            self.enter_drop_scope(DropScopeKind::Block);

            self.record_name_depths_in_pattern(*pattern, branch_depth, patterns_own);

            let scope = self.finish_branch(*branch, branch_depth, usage, false);
            branches.push((scope, BranchEnd::Slot(DropSlot::After(*branch))));
        }
        let branches = self.balance_branches(branches, branch_depth);
        self.moves.merge_branches_into_parent(branches);
    }

    fn walk_while(&mut self, while_: &cst::While, depth: ScopeDepth, usage: Use) {
        let body_depth = depth.deeper();
        let diverged = self.diverged;
        self.enter_body();
        self.enter_loop();
        // The condition's temporaries are dropped each time it is evaluated
        self.enter_drop_scope(DropScopeKind::Statement);
        self.walk_expr(while_.condition, depth, usage);
        self.exit_drop_scope(while_.condition);
        self.enter_drop_scope(DropScopeKind::Loop);
        self.walk_and_check_escapes(while_.body, body_depth, usage);
        self.exit_drop_scope(while_.body);
        self.exit_loop();
        self.diverged = diverged;
        let body = self.finish_body(body_depth, Some(RepeatedContext::WhileLoop));
        self.moves.absorb_mutations(body);
    }

    fn walk_for(&mut self, for_: &cst::For, depth: ScopeDepth, usage: Use) {
        self.walk_expr(for_.start, depth, usage);
        self.walk_expr(for_.end, depth, usage);

        let body_depth = depth.deeper();
        let diverged = self.diverged;
        self.enter_body();
        self.enter_loop();
        self.enter_drop_scope(DropScopeKind::Loop);
        self.bind_name(for_.variable, body_depth, true);
        self.walk_and_check_escapes(for_.body, body_depth, usage);
        self.exit_drop_scope(for_.body);
        self.exit_loop();
        self.diverged = diverged;
        let body = self.finish_body(body_depth, Some(RepeatedContext::ForLoop));
        self.moves.absorb_mutations(body);
    }

    /// Track a body's moves, mutations, and uses of unstable references
    fn enter_body(&mut self) {
        self.moves.push_scope();
        self.uses.push(FxHashMap::default());
    }

    /// Check what a body moves or invalidates outside of itself, erroring if `context` says it
    /// may run more than once. Returns what the body moved and mutated.
    fn finish_body(&mut self, body_depth: ScopeDepth, context: Option<RepeatedContext>) -> MoveScope {
        let uses = self.uses.pop().unwrap();
        if let Some(context) = context {
            self.check_invalidations_in_repeated_context(&uses, body_depth, context);
        }

        let body_scope = self.moves.pop_scope();
        // Closures are handled elsewhere. A `move` closure moving what it owns just makes it a
        // `once fn` (see [Self::exit_closure]), and other closures are checked by
        // [Self::check_moves_out_of_reference_captures].
        if let Some(context) = context.filter(|context| *context != RepeatedContext::Closure) {
            self.check_moves_in_repeated_context(&body_scope, body_depth, context);
        }

        if let Some(parent) = self.uses.last_mut() {
            for (holder, used) in uses {
                parent.entry(holder).or_insert(used);
            }
        }
        body_scope
    }

    fn walk_assignment(&mut self, assignment: &cst::Assignment, expr: ExprId, depth: ScopeDepth, usage: Use) {
        // Allow `x := v` to use `x` even if moved, but `x += v` cannot since it reads `x`.
        // Writing through an existing reference also uses that reference.
        if assignment.op.is_some() {
            self.walk_expr(assignment.lhs, depth, usage);
        } else {
            let writes_through = !matches!(self.tc.resolved_expr(assignment.lhs).as_ref(), Expr::Reference(_));
            let usage = if writes_through { usage.without_move() } else { Use::Inspect };
            self.walk_expr(assignment.lhs, depth, usage);
        }
        self.walk_expr(assignment.rhs, depth, usage);
        if let Some((_, op_expr)) = assignment.op {
            self.walk_expr(op_expr, depth, usage);
        }

        let target_depth = self.assignment_target_depth(assignment.lhs).unwrap_or(ScopeDepth(0));
        self.check_escapes(assignment.rhs, target_depth.deeper());

        if let Some(mutated) = self.tc.assignment_mutated_places(assignment.lhs) {
            let mutated = self.tc.place_entries(&mutated);
            self.record_mutation(mutated, expr);
        }

        if !self.diverged {
            self.drop_assigned(assignment, expr);
        }

        // The LHS always holds a value after an assignment.
        if let Some(path) = self.move_path_of_assignment_lhs(assignment.lhs) {
            self.moves.clear_moves(&path);
        }
    }

    /// Both the handled body and each branch are lambdas, so both go through
    /// `walk_lambda` which will record that branch's `resume` parameter
    /// into `name_depths` so we don't need to do so here.
    fn walk_handle(&mut self, handle: &cst::Handle, depth: ScopeDepth) {
        // The handled body runs in place, so anything it moves is moved for each branch and
        // everything afterward as well
        let (body, completes) = self.walk_lambda(handle.expression, depth, LambdaKind::HandleBody(handle.handler_name));
        self.moves.merge_branches_into_parent(vec![body]);

        // Either the body completes or a branch that never resumes runs in its place
        let mut ends = vec![(completes.then(MoveScope::default), BranchEnd::InLambda(handle.expression))];
        for (pattern, branch) in &handle.cases {
            // Only allow moving variables into this branch if `resume` is never mentioned.
            // This notably keeps handlers like `try_or` working.
            let uses_resume = self.tc.handler_branch_uses_resume(pattern.resume_name, *branch);
            let kind = LambdaKind::HandlerBranch { resume: pattern.resume_name, uses_resume };
            let (scope, completes) = self.walk_lambda(*branch, depth, kind);
            if uses_resume {
                // The body completes within `resume`, which is where it drops whatever another branch moved
                self.moves.absorb_mutations(scope);
            } else {
                ends.push((completes.then_some(scope), BranchEnd::InLambda(*branch)));
            }
        }
        let ends = self.balance_branches(ends, depth.deeper());
        self.moves.merge_branches_into_parent(ends);

        self.tc.bind_handle_environment(handle.expression);
        for (_, branch) in &handle.cases {
            self.tc.bind_handle_environment(*branch);
        }
    }

    /// Walk a lambda's body. Returns what it moved and mutated outside of itself along with
    /// whether control reaches the end of the body.
    fn walk_lambda(&mut self, lambda_expr: ExprId, depth: ScopeDepth, kind: LambdaKind) -> (MoveScope, bool) {
        // `Some(handler_name)` exempts the handler's own name from being a capture
        let (self_name, context, resume) = match kind {
            LambdaKind::Closure => (None, Some(RepeatedContext::Closure), None),
            LambdaKind::HandleBody(handler_name) => (Some(handler_name), None, None),
            LambdaKind::HandlerBranch { resume, uses_resume } => {
                (None, uses_resume.then_some(RepeatedContext::HandlerBranch), Some(resume))
            },
        };
        let Expr::Lambda(lambda) = self.tc.resolved_expr(lambda_expr).into_owned() else {
            unreachable!("walk_lambda called on a non-lambda expr")
        };

        let body_depth = depth.deeper();
        self.enter_drop_scope(DropScopeKind::Function);
        self.mark_dropped_parameter(lambda_expr, &lambda.parameters);
        for parameter in &lambda.parameters {
            self.record_name_depths_in_pattern(parameter.pattern, body_depth, true);
        }
        self.push_function_row(lambda_expr);

        let old_function_depth = self.function_depth.replace(body_depth);
        self.enter_body();
        let is_closure = kind == LambdaKind::Closure && !self.tc.is_impl_method(lambda_expr);
        self.enter_closure(lambda_expr, is_closure && lambda.is_move);
        if is_closure {
            self.infer_called_once(&lambda);
        }
        if let Some(resume) = resume {
            let mut tail_resumes = Vec::new();
            self.collect_tail_resumes(lambda.body, resume, &mut tail_resumes);
            self.tail_resumes.extend(tail_resumes);
        }
        let diverged = self.diverged;
        self.walk_and_check_escapes(lambda.body, body_depth, Use::Move);
        self.exit_drop_scope(lambda.body);
        let completes = !std::mem::replace(&mut self.diverged, diverged);
        self.function_depth = old_function_depth;
        let body = self.finish_body(body_depth, context);
        self.exit_closure(lambda_expr, lambda.body, &body, is_closure, completes);
        self.function_rows.pop();

        if lambda.is_move {
            self.record_move_captures(lambda_expr, self_name);
        } else if is_closure {
            self.check_moves_out_of_reference_captures(&body, body_depth);
        }
        (body, completes)
    }

    /// Collect each call of `resume` in tail position of `expr`
    fn collect_tail_resumes(&self, expr: ExprId, resume: NameId, calls: &mut Vec<ExprId>) {
        match self.tc.resolved_expr(expr).as_ref() {
            Expr::Sequence(items) => {
                if let Some(last) = items.last() {
                    self.collect_tail_resumes(last.expr, resume, calls);
                }
            },
            Expr::If(if_) => {
                self.collect_tail_resumes(if_.then, resume, calls);
                if let Some(else_) = if_.else_ {
                    self.collect_tail_resumes(else_, resume, calls);
                }
            },
            Expr::Match(match_) => {
                for (_, branch) in &match_.cases {
                    self.collect_tail_resumes(*branch, resume, calls);
                }
            },
            Expr::Chain(chain) => {
                self.collect_tail_resumes(chain.body, resume, calls);
                self.collect_tail_resumes(chain.else_, resume, calls);
            },
            Expr::Call(call) => {
                if self.tc.local_variable(call.function) == Some(resume) {
                    calls.push(expr);
                }
            },
            _ => (),
        }
    }
}

/// True if every entry of the canonical row `row` is in `target`, as when `row` flowed into it
fn is_subrow(row: &[Type], target: &[Type]) -> bool {
    row.iter().all(|entry| target.contains(entry))
}

/// The first use of each holder of unstable references in a body, and those references
type Uses = FxHashMap<Place, (Location, Vec<PlaceEntry>)>;

/// An entry of `a` overlapping one of `b`
fn overlapping<'a>(a: &'a [PlaceEntry], b: &[PlaceEntry]) -> Option<&'a PlaceEntry> {
    a.iter().find(|a| b.iter().any(|b| a.overlaps(b)))
}
