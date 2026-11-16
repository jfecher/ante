//! Inserts a call to `Drop.drop` for each value still owned when it goes out of scope.
//!
//! This is done as part of the borrow-checking pass since only that pass knows which places are
//! still owned at each exit of a scope. Each drop is an ordinary call using the `Drop` impl that
//! inference already found for the value (see [super::value_impls]), so we never search for impls
//! here. Drops are recorded in [Drops](super::fresh_expr::Drops) for the MIR builder to emit.
//!
//! - A binding is dropped at the end of its scope, in reverse declaration order, and at each
//!   `return`, `break`, or `continue` that leaves its scope.
//! - A place moved in only some branches of an `if` or `match` is dropped at the end of the others.
//! - If a place is partially moved, each field it still owns is dropped instead.
//! - The parameter of a `Drop` impl's `drop` is always dropped field by field.
//! - Assigning to a place drops its old value.
//! - A value that is never bound, such as the result of a statement or the operand of a
//!   reference, is dropped at the end of the statement it is evaluated in.
//! - Functions are dropped by dropping their environment, using the `Drop` impl the compiler
//!   provides for them. A `move` closure that moves any of its captures is a `once fn` instead.
//!   Calling it moves the closure into its body, which then drops its remaining captures as it
//!   would its parameters.

use crate::{
    diagnostics::{Diagnostic, ImportSuggestion, Location},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{self, Expr, PlaceSegment},
        ids::{ExprId, NameId, TopLevelName},
    },
    shared_arc::Shared,
    type_inference::{
        Locateable, TypeChecker,
        affine::{MoveScope, prelude_trait},
        borrows::BorrowChecker,
        errors::TypeErrorKind,
        fresh_expr::DropSlot,
        places::{Place, PlaceRoot, ScopeDepth},
        type_body::TypeBody,
        types::{Type, TypeBindings},
        value_impls::{Owned, OwnedRoot, OwnedSegment},
    },
};

/// A union variant as a tuple of (its name within the union's item, its name, its fields)
type Variant = (NameId, cst::Name, Vec<(cst::Name, Type)>);

/// A value we're inserting a drop for, along with what we need to describe it in errors
#[derive(Clone)]
pub(super) struct Dropped {
    name: DroppedName,
    typ: Type,
    location: Location,
    /// The value being dropped. Inference has already found its `Drop` impl.
    owned: Owned,
}

#[derive(Clone)]
enum DroppedName {
    Place(Place),
    /// A field of a union variant. These can only be named through a pattern.
    Field(Place, cst::Name),
    Temporary,
    AssignedOver,
}

/// Why a place is dropped
pub(super) enum DropCause {
    ScopeEnd,
    /// The place is assigned to at this location
    AssignedOver(Location),
}

/// The kind of expression a [DropScope] is for. This determines which jumps leave the scope.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) enum DropScopeKind {
    Function,
    Loop,
    Block,
    /// A single statement in a block. It owns the temporaries evaluated within it but never any bindings.
    Statement,
}

/// Each binding that owns its value within a lexical scope, in declaration order
pub(super) struct DropScope {
    kind: DropScopeKind,
    owned: Vec<NameId>,
}

/// State for a [cst::Chain] whose body we're currently walking
pub(super) struct ChainState {
    /// The number of drop scopes and move scopes enclosing the chain. Any scopes past these are
    /// left by an [Expr::Fallthrough].
    pub(super) drop_scopes: usize,
    pub(super) move_scopes: usize,

    /// Each [Expr::Fallthrough] reached so far, along with what the chain had moved at that point
    pub(super) fallthroughs: Vec<(MoveScope, ExprId)>,
}

/// State for a `move` closure whose body we're currently walking. Since it owns its captures, the
/// closure becomes a `once fn` if its body moves any of them. Calling it then moves the closure
/// into its body, which drops any captures it didn't move. See [BorrowChecker::exit_closure].
pub(super) struct ClosureState {
    /// Each captured variable that isn't `Copy`
    captures: Vec<NameId>,

    /// The number of move scopes enclosing the closure's body
    move_scopes: usize,

    /// Each `return` reached so far, along with what the body had moved at that point
    returns: Vec<(MoveScope, ExprId)>,
}

/// Where to run the drops at the end of a branch
#[derive(Copy, Clone)]
pub(super) enum BranchEnd {
    Slot(DropSlot),
    /// At the end of this lambda, which is the body or a branch of a `handle`
    InLambda(ExprId),
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Returns the Prelude's `Drop` trait and its number of generics, caching them after the first call
    pub(super) fn drop_trait(&mut self) -> (TopLevelName, usize) {
        let compiler = self.compiler;
        *self.drop_trait.get_or_insert_with(|| prelude_trait(compiler, "Drop", 2))
    }

    /// True if `lambda` is the method of the `Drop` impl being checked
    pub(super) fn is_drop_impl_method(&mut self, lambda: ExprId) -> bool {
        let (drop_name, _) = self.drop_trait();
        self.is_impl_method(lambda)
            && self.current_impl.as_ref().is_some_and(|(_, trait_)| trait_.type_name() == drop_name)
    }

    /// The name to refer to `dropped` by in errors
    fn dropped_name(&self, dropped: &Dropped) -> String {
        let names = self.current_extended_context();
        match &dropped.name {
            DroppedName::Place(place) => place.display_name(names),
            DroppedName::Field(place, field) => format!("{}.{field}", place.display_name(names)),
            DroppedName::Temporary => "This value".to_string(),
            DroppedName::AssignedOver => "The value assigned over".to_string(),
        }
    }

    /// Create an error for `dropped` going out of scope without a `Drop` impl for its type in scope
    pub(super) fn missing_drop(&self, dropped: &Dropped, suggestions: Vec<ImportSuggestion>) -> Diagnostic {
        let name = self.dropped_name(dropped);
        let typ = self.type_to_string(&dropped.typ);
        let is_function = matches!(self.follow_type(&dropped.typ), Type::Function(_));
        Diagnostic::MissingDrop { name, typ, is_function, location: dropped.location.clone(), suggestions }
    }

    /// Build a call to `Drop.drop` with `value`, which is already type checked, and `dictionary`,
    /// the `Drop` impl inference already found for it. Since there is nothing left to infer, we
    /// give the call its type as we build it. Returns the call and the effects it performs.
    fn dictionary_drop_call(&mut self, value: ExprId, dictionary: ExprId, location: Location) -> (ExprId, Type) {
        let function = self.prelude_variable("drop", Type::ERROR, location.clone());
        let Expr::Variable(path) = self.current_extended_context()[function] else { unreachable!() };
        let Some(Origin::TraitMember(member)) = self.path_origin(path) else {
            unreachable!("`drop` is a trait member")
        };
        let Type::Function(method) = self.infer_path(path, &Type::ERROR) else { unreachable!("`drop` is a method") };

        let mut method = method.as_ref().clone();
        let dictionary_type = method.parameters.pop().expect("a trait method takes its dictionary").typ;
        let (dictionary_actual, value_type) = (self.expr_types[&dictionary].clone(), self.expr_types[&value].clone());
        self.unify(&dictionary_type, &dictionary_actual, TypeErrorKind::General, location.clone());
        self.unify(&method.parameters[0].typ, &value_type, TypeErrorKind::General, location.clone());

        let (return_type, effects) = (method.return_type.clone(), method.effects.clone());
        self.trait_member_access(function, dictionary, path, member.index, Type::Function(Shared::new(method)));
        let call = self.call_expr(function, value, location);
        self.expr_types.insert(call, return_type);
        (call, effects)
    }

    /// Create a call to `function` with `argument`. The call still needs to be type checked.
    fn call_expr(&mut self, function: ExprId, argument: ExprId, location: Location) -> ExprId {
        let call = cst::Call { function, arguments: vec![cst::Argument::explicit(argument)] };
        self.push_expr(Expr::Call(call), Type::ERROR, location)
    }

    /// The type of the value held at `place`
    fn place_type(&mut self, place: &Place) -> Option<Type> {
        let PlaceRoot::Variable(root) = place.root else { return None };
        let mut typ = self.name_types.get(&root)?.clone();
        for segment in &place.segments {
            let PlaceSegment::Field(field) = segment else { return None };
            typ = self.get_field_types(&typ, None).remove(field)?.0;
        }
        Some(typ)
    }

    /// Create an expression reading `place`. Its member accesses still need to be type checked.
    fn place_expr(&mut self, place: &Place, location: &Location) -> Option<ExprId> {
        let PlaceRoot::Variable(root) = place.root else { return None };
        let typ = self.name_types.get(&root)?.clone();
        let name = self.current_extended_context()[root].clone();
        let path = self.push_path(cst::Path::ident(name, location.clone()), typ.clone(), location.clone());
        self.current_extended_context_mut().insert_path_origin(path, Origin::Local(root));

        let mut expr = self.push_expr(Expr::Variable(path), typ, location.clone());
        for segment in &place.segments {
            let PlaceSegment::Field(field) = segment else { return None };
            let member = self.push_name(field.clone(), location.clone());
            let access = Expr::MemberAccess(cst::MemberAccess { object: expr, member });
            expr = self.push_expr(access, Type::ERROR, location.clone());
        }
        Some(expr)
    }

    /// Create a call to the Prelude function `name` with `argument`. The call still needs to be
    /// type checked.
    fn prelude_call(&mut self, name: &'static str, argument: ExprId, location: &Location) -> ExprId {
        let function = self.prelude_variable(name, Type::ERROR, location.clone());
        self.call_expr(function, argument, location.clone())
    }
}

impl<'a, 'local, 'inner> BorrowChecker<'a, 'local, 'inner> {
    pub(super) fn enter_drop_scope(&mut self, kind: DropScopeKind) {
        self.drop_scopes.push(DropScope { kind, owned: Vec::new() });
    }

    /// Leave the innermost scope, dropping anything it still owns after `body` is evaluated
    pub(super) fn exit_drop_scope(&mut self, body: ExprId) {
        let scope = self.drop_scopes.pop().expect("exit_drop_scope called without a scope");
        self.check_temporaries_outlived(&scope);
        // A diverging body never reaches its end, so there's nothing to drop here. Any jumps out
        // of it already drop what they leave.
        if self.diverged {
            return;
        }
        let mut drops = Vec::new();
        self.drop_owned(&scope, &mut drops);
        self.record_drops(DropSlot::After(body), drops);
    }

    /// Record `drops` to be run at `slot`
    fn record_drops(&mut self, slot: DropSlot, drops: Vec<ExprId>) {
        self.tc.current_extended_context_mut().drops_mut().push(slot, drops);
    }

    /// Record that `name` owns its value until the end of the innermost scope
    pub(super) fn own_binding(&mut self, name: NameId) {
        // Names bound within an alias pattern are part of the alias, which owns the whole value
        if self.tc.binding_place(name) != Place::variable(name) {
            return;
        }
        if let Some(typ) = self.tc.name_types.get(&name)
            && self.tc.copy_impl_found(&Owned::new(OwnedRoot::Name(name)), typ)
        {
            return;
        }
        if let Some(scope) = self.block_scope() {
            scope.owned.push(name);
        }
    }

    /// Returns the innermost block scope, skipping over statement scopes
    fn block_scope(&mut self) -> Option<&mut DropScope> {
        self.drop_scopes.iter_mut().rev().find(|scope| scope.kind != DropScopeKind::Statement)
    }

    /// The value of `expr` is used in place rather than moved, so give it a name so that it can
    /// be dropped at the end of its statement. Does nothing if its type is `Copy`.
    pub(super) fn own_temporary(&mut self, expr: ExprId) {
        let Some(typ) = self.tc.expr_types.get(&expr).cloned() else { return };
        if self.drop_scopes.is_empty()
            || self.tc.diverges(&typ)
            || self.tc.copy_impl_found(&Owned::new(OwnedRoot::Value(expr)), &typ)
        {
            return;
        }
        let name = self.name_temporary(expr, "temporary", typ);
        self.drop_scopes.last_mut().expect("checked to be within a scope above").owned.push(name);
    }

    /// If `expr` is a temporary owned by the innermost statement, give ownership of it to the
    /// enclosing block instead
    pub(super) fn extend_temporary(&mut self, expr: ExprId) {
        let Some(name) = self.tc.current_extended_context().drops().temporary(expr) else { return };
        let Some(statement) = self.drop_scopes.last_mut() else { return };
        let Some(index) = statement.owned.iter().position(|owned| *owned == name) else { return };
        statement.owned.remove(index);
        self.block_scope().expect("a statement is always within a block").owned.push(name);
    }

    fn is_temporary(&self, name: NameId) -> bool {
        self.tc.current_extended_context().drops().names_temporary(name)
    }

    /// Returns the name `expr`'s value is bound to so that drops can refer to it, creating one if needed
    fn name_temporary(&mut self, expr: ExprId, prefix: &str, typ: Type) -> NameId {
        if let Some(name) = self.tc.current_extended_context().drops().temporary(expr) {
            return name;
        }
        let (_, name) = self.tc.fresh_variable(prefix, typ.clone(), expr.locate(self.tc));
        self.tc.name_types.insert(name, typ);
        self.tc.current_extended_context_mut().drops_mut().name_temporary(expr, name);
        name
    }

    /// Report any binding still holding a reference to a temporary of `scope` since those
    /// temporaries are about to be dropped
    fn check_temporaries_outlived(&mut self, scope: &DropScope) {
        let drops = self.tc.current_extended_context().drops();
        let temporaries: Vec<ExprId> = scope.owned.iter().filter_map(|name| drops.temporary_expr(*name)).collect();
        if temporaries.is_empty() {
            return;
        }
        let temporary_of = |place: &Place| match place.root {
            PlaceRoot::Anonymous(expr, _) if temporaries.contains(&expr) => Some((place.clone(), expr)),
            _ => None,
        };
        // References returned by the item itself are reported at the return instead
        let bindings = self.binding_places.iter().filter(|(name, _)| !self.item_names.contains(name));
        let dangling: Vec<_> = bindings.flat_map(|(_, places)| places).filter_map(temporary_of).collect();
        for (place, expr) in dangling {
            self.report_escape(place, expr);
        }
    }

    /// Mark the parameter of a `Drop` impl's `drop` to be taken apart rather than dropped.
    /// Otherwise dropping it would call `drop` again.
    pub(super) fn mark_dropped_parameter(&mut self, lambda: ExprId, parameters: &[cst::Parameter]) {
        if self.tc.is_drop_impl_method(lambda) {
            for parameter in parameters {
                if let Some(name) = self.tc.definition_name(parameter.pattern) {
                    self.taken_apart.insert(name);
                }
            }
        }
    }

    /// Before `jump` (a `return`, `break`, or `continue`), drop what every scope it leaves still
    /// owns. `target` is the kind of the outermost scope it leaves.
    /// Returns the index of that scope, if there is one.
    pub(super) fn drop_before_jump(&mut self, jump: ExprId, target: DropScopeKind) -> Option<usize> {
        let outermost = self.drop_scopes.iter().rposition(|scope| scope.kind == target)?;
        self.drop_scopes_before(DropSlot::BeforeJump(jump), outermost);
        Some(outermost)
    }

    /// Drop what each scope from index `outermost` onward still owns at `slot`, which leaves all
    /// of these scopes
    fn drop_scopes_before(&mut self, slot: DropSlot, outermost: usize) {
        let scopes = self.drop_scopes.split_off(outermost);
        let mut drops = Vec::new();
        for scope in scopes.iter().rev() {
            self.drop_owned(scope, &mut drops);
        }
        self.drop_scopes.extend(scopes);
        self.record_drops(slot, drops);
    }

    /// Called when a condition of the innermost chain fails. Drops what the chain owns before its
    /// `else` runs, and remembers what it had moved so the `else` can be walked with that state.
    pub(super) fn fall_through(&mut self, fallthrough: ExprId) {
        let Some(chain) = self.chains.last() else { return };
        let (drop_scopes, move_scopes) = (chain.drop_scopes, chain.move_scopes);
        self.drop_scopes_before(DropSlot::BeforeJump(fallthrough), drop_scopes);
        let moves = self.moves.state_since(move_scopes);
        self.chains.last_mut().expect("checked above").fallthroughs.push((moves, fallthrough));
    }

    /// A handler branch that calls `resume` in tail position has nothing left to do afterward but
    /// drop what it still owns. We run those drops before `call` instead so that the call is still
    /// the last thing the branch does. Otherwise each handled effect would keep a frame of the
    /// handler alive. What we drop here counts as moved from then on, so any other way out of the
    /// branch drops it the same as anything moved in only some branches.
    ///
    /// The resumed value is evaluated before the drops, so it may not refer to anything the branch owns.
    pub(super) fn drop_before_tail_resume(&mut self, call_expr: ExprId, call: &cst::Call) {
        let Some(boundary) = self.function_depth else { return };
        for argument in &call.arguments {
            self.check_escapes(argument.expr, boundary);
        }

        let Some(outermost) = self.drop_scopes.iter().rposition(|scope| scope.kind == DropScopeKind::Function) else {
            return;
        };
        self.drop_scopes_before(DropSlot::BeforeCall(call_expr), outermost);
        let location = call_expr.locate(self.tc);
        for name in self.drop_scopes[outermost..].iter().flat_map(|scope| &scope.owned) {
            self.moves.record_move(Place::variable(*name), location.clone());
        }
    }

    /// Start walking the body of a lambda. If it is a closure owning any of its captures, track
    /// the non-Copy ones it owns.
    pub(super) fn enter_closure(&mut self, lambda: ExprId, is_closure: bool) {
        let context = self.tc.current_extended_context();
        let owns_captures = is_closure && (context.is_move_closure(lambda) || context.moved_captures(lambda).is_some());
        let state = owns_captures.then(|| {
            let captured = context.get_closure_environment(lambda).into_iter().flatten().copied();
            let captures = captured
                .filter(|name| {
                    context.captures_by_value(lambda, *name)
                        && !self.tc.copy_impl_found(&Owned::new(OwnedRoot::Name(*name)), &self.tc.name_types[name])
                })
                .collect();
            ClosureState { captures, move_scopes: self.moves.scope_count() - 1, returns: Vec::new() }
        });
        self.closures.push(state);
    }

    /// Remember what the innermost closure had moved by the time it reached the `return` expression `jump`
    pub(super) fn return_from_closure(&mut self, jump: ExprId) {
        let Some(Some(state)) = self.closures.last_mut() else { return };
        let moves = self.moves.state_since(state.move_scopes);
        state.returns.push((moves, jump));
    }

    /// If its type doesn't already say, decide whether the lambda whose body was just walked is a
    /// `once fn`. It is one if it moved any of its captures in `moves`. If so, we drop whatever
    /// captures it still owns at each of its exits.
    ///
    /// `completes` is false if the body never reaches its end.
    pub(super) fn exit_closure(
        &mut self, lambda: ExprId, body: ExprId, moves: &MoveScope, is_closure: bool, completes: bool,
    ) {
        let state = self.closures.pop().expect("exit_closure called without a closure");
        if !is_closure {
            return;
        }
        let Some(function) = self.tc.expr_function_type(lambda) else { return };

        // A capture bound within an alias pattern is moved as part of the alias, so check its place
        // within the alias
        let captures = state.as_ref().map_or(&[][..], |state| &state.captures);
        let places = mapvec(captures, |name| self.tc.binding_place(*name));
        let moved = moves.moved_paths().filter(|(path, _)| places.iter().any(|place| path.overlaps(place)));
        let moved = mapvec(moved, |(path, location)| (path.clone(), location.clone()));

        self.tc.bind_if_unbound(&function.once, if moved.is_empty() { Type::MANY } else { Type::ONCE });
        if *self.tc.follow_type(&function.once) != Type::ONCE {
            for (path, location) in moved {
                let name = path.display_name(self.tc.current_extended_context());
                self.tc.compiler.accumulate(Diagnostic::MoveOutOfFn { name, location });
            }
            return;
        }

        let Some(state) = state else { return };
        let mut exits = mapvec(state.returns, |(moves, jump)| (moves, DropSlot::BeforeJump(jump)));
        if completes {
            exits.push((moves.clone(), DropSlot::After(body)));
        }
        for (moves, slot) in exits {
            let moved_captures = self.moves_of_captured_variables(&state.captures, &moves);
            self.moves.reenter_scope(moves);
            for (path, location) in moved_captures {
                self.moves.record_move(path, location);
            }
            let mut drops = Vec::new();
            for name in state.captures.iter().rev() {
                self.drop_place(&Place::variable(*name), false, &DropCause::ScopeEnd, &mut drops);
            }
            self.moves.pop_scope();
            self.record_drops(slot, drops);
        }
    }

    /// A capture bound within an alias pattern is moved as part of the alias, yet within the
    /// closure it is its own variable, and that variable is what gets dropped. This translates each
    /// move in `moves` of such a capture (or part of one) into a move of that variable.
    fn moves_of_captured_variables(&self, captures: &[NameId], moves: &MoveScope) -> Vec<(Place, Location)> {
        let mut renamed = Vec::new();
        for name in captures {
            let place = self.tc.binding_place(*name);
            let variable = Place::variable(*name);
            if place == variable {
                continue;
            }
            for (path, location) in moves.moved_paths() {
                if place.is_within(path) {
                    renamed.push((variable.clone(), location.clone()));
                } else if path.is_descendant_of(&place) {
                    let part = variable.clone().project(&path.segments[place.segments.len()..]);
                    renamed.push((part, location.clone()));
                }
            }
        }
        renamed
    }

    /// True if calling `callee` moves it since it is a `once fn`. If the function's type doesn't
    /// say yet, we decide here that it may be called any number of times since the call is walked
    /// as a borrow and nothing later can make it a `once fn`.
    pub(super) fn decide_callee_once(&mut self, callee: ExprId) -> bool {
        self.tc.expr_function_type(callee).is_some_and(|function| self.tc.callee_is_once(&function))
    }

    /// For each function parameter of `lambda` whose type doesn't say whether it is a `once fn`,
    /// decide that it is one if the body calls it at most once and does nothing else with it
    pub(super) fn infer_called_once(&mut self, lambda: &cst::Lambda) {
        for parameter in &lambda.parameters {
            let Some(name) = self.tc.definition_name(parameter.pattern) else { continue };
            let Some(Type::Function(function)) = self.tc.name_types.get(&name).map(|typ| self.tc.follow_type(typ))
            else {
                continue;
            };
            let once = function.once.clone();
            if matches!(self.tc.follow_type(&once), Type::Variable(_)) {
                let calls_once = self.most_calls(lambda.body, name) == 1;
                self.tc.bind_if_unbound(&once, if calls_once { Type::ONCE } else { Type::MANY });
            }
        }
    }

    /// Returns the most times evaluating `expr` may call the function `name`: 0, 1, or 2 for
    /// anything more. This is also 2 if `expr` uses `name` in any way other than calling it, or
    /// may call it repeatedly.
    fn most_calls(&self, expr: ExprId, name: NameId) -> usize {
        let is_name = |expr: ExprId| self.tc.local_variable(expr) == Some(name);
        let sum = |exprs: &[ExprId]| exprs.iter().map(|expr| self.most_calls(*expr, name)).sum::<usize>();
        let most = |exprs: &[ExprId]| exprs.iter().map(|expr| self.most_calls(*expr, name)).max().unwrap_or(0);

        let calls = match self.tc.resolved_expr(expr).as_ref() {
            Expr::Variable(_) if is_name(expr) => 2,
            Expr::Call(call) => {
                let arguments = mapvec(&call.arguments, |argument| argument.expr);
                let callee = if is_name(call.function) { 1 } else { self.most_calls(call.function, name) };
                callee + sum(&arguments)
            },
            Expr::If(if_) => {
                let branches: Vec<_> = std::iter::once(if_.then).chain(if_.else_).collect();
                self.most_calls(if_.condition, name) + most(&branches)
            },
            Expr::Match(match_) => {
                let branches = mapvec(&match_.cases, |(_, branch)| *branch);
                self.most_calls(match_.expression, name) + most(&branches)
            },
            // These may run their body any number of times
            Expr::Lambda(_) | Expr::While(_) | Expr::For(_) | Expr::Handle(_) => 2 * sum(&self.child_exprs(expr)),
            _ => sum(&self.child_exprs(expr)),
        };
        calls.min(2)
    }

    /// Drop each binding of `scope`, last declared first
    fn drop_owned(&mut self, scope: &DropScope, drops: &mut Vec<ExprId>) {
        for name in scope.owned.iter().rev() {
            let take_apart = self.taken_apart.contains(name);
            self.drop_place(&Place::variable(*name), take_apart, &DropCause::ScopeEnd, drops);
        }
    }

    /// Drop each place moved in one of `branches` at the end of every other branch still owning
    /// it so that no branch leaves it owned. Diverging branches are `None`.
    pub(super) fn balance_branches(
        &mut self, branches: Vec<(Option<MoveScope>, BranchEnd)>, branch_depth: ScopeDepth,
    ) -> Vec<MoveScope> {
        let mut moved: Vec<Place> = Vec::new();
        for (scope, _) in &branches {
            let Some(scope) = scope else { continue };
            for (path, _) in scope.moved_paths() {
                let outer =
                    self.atom_depth(path).is_some_and(|depth| depth < branch_depth) || self.is_open_temporary(path);
                if outer && matches!(path.root, PlaceRoot::Variable(_)) && !moved.contains(path) {
                    moved.push(path.clone());
                }
            }
        }
        // Dropping a place already drops everything within it
        let covered = |path: &Place| moved.iter().any(|other| path.is_descendant_of(other));
        let moved: Vec<Place> = moved.iter().filter(|path| !covered(path)).cloned().collect();

        if moved.is_empty() {
            return branches.into_iter().filter_map(|(scope, _)| scope).collect();
        }

        let mut reaching = Vec::with_capacity(branches.len());
        for (scope, end) in branches {
            let Some(scope) = scope else { continue };
            self.moves.reenter_scope(scope);
            let slot = match end {
                BranchEnd::Slot(slot) => slot,
                BranchEnd::InLambda(lambda) => {
                    self.push_function_row(lambda);
                    DropSlot::After(self.lambda_body(lambda))
                },
            };
            let mut drops = Vec::new();
            for path in &moved {
                self.drop_place(path, false, &DropCause::ScopeEnd, &mut drops);
            }
            if let BranchEnd::InLambda(lambda) = end {
                // The lambda's body may not mention the place it now drops, so capture it for the drop
                for drop in &drops {
                    self.tc.capture_for_drop(lambda, *drop);
                }
                self.function_rows.pop();
            }
            reaching.push(self.moves.pop_scope());
            self.record_drops(slot, drops);
        }
        reaching
    }

    /// True if `path` is a temporary of a scope we're still walking. Temporaries have no depth of
    /// their own, but one evaluated before a branch still outlives that branch.
    fn is_open_temporary(&self, path: &Place) -> bool {
        let PlaceRoot::Variable(root) = path.root else { return false };
        self.is_temporary(root) && self.drop_scopes.iter().any(|scope| scope.owned.contains(&root))
    }

    /// Start walking `lambda`. Drops within it perform their effects in its effect row.
    pub(super) fn push_function_row(&mut self, lambda: ExprId) {
        let effects = self.tc.expr_function_type(lambda).map_or_else(Type::pure, |function| function.effects.clone());
        self.function_rows.push(effects);
    }

    /// Returns the body of `lambda`, which is the body or a branch of a `handle`
    fn lambda_body(&self, lambda: ExprId) -> ExprId {
        match self.tc.resolved_expr(lambda).as_ref() {
            Expr::Lambda(definition) => definition.body,
            _ => unreachable!("lambda_body called on a non-lambda expr"),
        }
    }

    /// Drop the value `assignment` overwrites once its new value is evaluated
    pub(super) fn drop_assigned(&mut self, assignment: &cst::Assignment, expr: ExprId) {
        // A compound assignment passes the old value to its operator, so there's nothing to drop
        if assignment.op.is_some() {
            return;
        }
        let mut drops = Vec::new();
        let owned_place = match self.tc.resolved_expr(assignment.lhs).as_ref() {
            Expr::Reference(reference) if self.tc.owned_place_root(reference.rhs).is_some() => {
                self.tc.try_build_move_path(reference.rhs)
            },
            _ => None,
        };
        match owned_place {
            Some(place) => {
                let cause = DropCause::AssignedOver(assignment.lhs.locate(self.tc));
                self.drop_place(&place, false, &cause, &mut drops);
            },
            None => self.drop_referent(assignment.lhs, &mut drops),
        }
        self.record_drops(DropSlot::BeforeStore(expr), drops);
    }

    /// Drop the value the mutable reference `reference` points to
    fn drop_referent(&mut self, reference: ExprId, drops: &mut Vec<ExprId>) {
        let reference_type = self.tc.expr_types[&reference].clone();
        let Some((_, element)) = reference_type.reference_element(&self.tc.bindings) else { return };
        let location = reference.locate(self.tc);
        let owned = Owned::new(OwnedRoot::Referent(reference));
        let dropped = Dropped { name: DroppedName::AssignedOver, typ: element, location: location.clone(), owned };
        if !self.needs_drop(&dropped) {
            return;
        }

        // The reference should only be evaluated once, so bind it to a name for the drop to read
        let name = self.name_temporary(reference, "assigned", reference_type.clone());
        let Some(pointer) = self.tc.place_expr(&Place::variable(name), &location) else { return };
        let value = self.tc.prelude_call("deref", pointer, &location);
        if let Some(drop) = self.drop_value(value, &dropped) {
            drops.push(drop);
        }
    }

    /// Build a call dropping `value` (described by `dropped`) with the `Drop` impl found for it.
    /// Reports an error if none was found.
    fn drop_value(&mut self, value: ExprId, dropped: &Dropped) -> Option<ExprId> {
        let Some(dictionary) = self.tc.drop_dictionary(&dropped.owned) else {
            self.report_missing_drop(dropped);
            return None;
        };
        self.tc.check_expr(value, &dropped.typ, TypeErrorKind::General);
        let (drop, effects) = self.tc.dictionary_drop_call(value, dictionary, dropped.location.clone());
        self.check_drop_effects(effects, dropped);
        Some(drop)
    }

    /// True if values of `dropped`'s type need to be dropped at all. Reports an error if nothing
    /// is known to drop the type.
    fn needs_drop(&mut self, dropped: &Dropped) -> bool {
        !self.tc.copy_impl_found(&dropped.owned, &dropped.typ) && self.drop_is_known(dropped)
    }

    /// Drop whatever `place` still owns. If `take_apart` is true, its fields are dropped
    /// individually instead of the whole place. This also happens if any part of it was moved.
    fn drop_place(&mut self, place: &Place, take_apart: bool, cause: &DropCause, drops: &mut Vec<ExprId>) {
        if let Some(typ) = self.tc.place_type(place) {
            self.drop_place_of_type(place, typ, take_apart, cause, drops);
        }
    }

    /// [Self::drop_place] for a `place` already known to hold a `typ`
    fn drop_place_of_type(
        &mut self, place: &Place, typ: Type, take_apart: bool, cause: &DropCause, drops: &mut Vec<ExprId>,
    ) {
        let PlaceRoot::Variable(root) = place.root else { return };
        let Some(owned) = self.tc.owned_place(place) else { return };
        let partially_moved = match self.moves.moved(place) {
            Some((moved, _)) if place.is_within(moved) => return,
            Some(_) => true,
            None => false,
        };

        // Types that failed to check have already reported an error
        if typ.contains_error(&self.tc.bindings) || self.tc.copy_impl_found(&owned, &typ) {
            return;
        }
        // Errors from an assignment point to the assignment rather than the place's declaration
        let (name, location) = match cause {
            DropCause::AssignedOver(location) => (DroppedName::AssignedOver, location.clone()),
            DropCause::ScopeEnd if self.is_temporary(root) => (DroppedName::Temporary, root.locate(self.tc)),
            DropCause::ScopeEnd => (DroppedName::Place(place.clone()), root.locate(self.tc)),
        };
        let dropped = Dropped { name, typ, location, owned };
        if !partially_moved && !take_apart {
            if self.drop_is_known(&dropped)
                && let Some(value) = self.tc.place_expr(place, &dropped.location)
                && let Some(drop) = self.drop_value(value, &dropped)
            {
                drops.push(drop);
            }
            return;
        }

        let body = (dropped.typ)
            .definition_head(&self.tc.bindings)
            .map(|(name, arguments)| (name, name.type_body(arguments, self.tc.compiler)));
        if let Some((type_name, TypeBody::Sum(variants))) = body {
            self.drop_variants(place, type_name, variants, &dropped, drops);
        }
        self.drop_fields(place, &dropped.typ, cause, drops);
    }

    /// Drop the fields of whichever variant the union at `place` holds. We do this by matching on
    /// a reference to it, with each case dropping the fields of its variant. The union's `with`
    /// fields belong to the union itself, so they're dropped like any other place's fields.
    ///
    /// The match is type checked before the drops are added to its cases since we only know what
    /// its patterns bind after that.
    fn drop_variants(
        &mut self, place: &Place, type_name: TopLevelName, variants: Vec<Variant>, union: &Dropped,
        drops: &mut Vec<ExprId>,
    ) {
        let location = &union.location;
        let common_fields = type_name.common_field_count(self.tc.compiler);

        let variants = mapvec(variants, |(variant, name, fields)| {
            let payload = fields.len().saturating_sub(common_fields);
            let fields = mapvec(fields.into_iter().enumerate(), |(index, (field_name, field_type))| {
                let dropped = Dropped {
                    name: DroppedName::Field(place.clone(), field_name),
                    typ: field_type,
                    location: location.clone(),
                    owned: union.owned.part(OwnedSegment::VariantField(variant, index)),
                };
                let needs_drop = index < payload && self.needs_drop(&dropped);
                needs_drop.then_some(dropped)
            });
            (variant, name, fields)
        });
        if !variants.iter().any(|(_, _, fields)| fields.iter().any(Option::is_some)) {
            return;
        }
        let Some(value) = self.tc.place_expr(place, location) else { return };

        let mut bodies = Vec::new();
        let cases = mapvec(variants, |(variant, name, fields)| {
            let path = self.tc.push_path(cst::Path::ident(name, location.clone()), Type::ERROR, location.clone());
            let origin = Origin::TopLevelDefinition(TopLevelName::new(type_name.top_level_item, variant));
            self.tc.current_extended_context_mut().insert_path_origin(path, origin);

            let mut dropped_fields = Vec::new();
            let fields = mapvec(fields, |dropped| {
                let (field_path, field) = self.tc.fresh_variable("field", Type::ERROR, location.clone());
                if let Some(dropped) = dropped {
                    dropped_fields.push((field_path, dropped));
                }
                self.tc.push_pattern(cst::Pattern::Variable(field), location.clone())
            });
            let pattern = self.tc.push_pattern(cst::Pattern::Constructor(path, fields), location.clone());
            let body = self.tc.push_expr(Expr::Literal(cst::Literal::Unit), Type::UNIT, location.clone());
            bodies.push((body, dropped_fields));
            (pattern, body)
        });

        let reference = cst::Reference { kind: cst::ReferenceKind::Ref, rhs: value, implicit: false };
        let expression = self.tc.push_expr(Expr::Reference(reference), Type::ERROR, location.clone());
        let match_ = self.tc.push_expr(Expr::Match(cst::Match { expression, cases }), Type::ERROR, location.clone());
        self.tc.check_expr(match_, &Type::UNIT, TypeErrorKind::General);

        for (body, fields) in bodies {
            let mut statements = Vec::new();
            for (field_path, dropped) in fields {
                let reference = self.tc.push_expr(Expr::Variable(field_path), Type::ERROR, location.clone());
                let value = self.tc.prelude_call("deref", reference, location);
                if let Some(drop) = self.drop_value(value, &dropped) {
                    statements.push(cst::SequenceItem { comments: Vec::new(), expr: drop });
                }
            }
            if !statements.is_empty() {
                self.tc.current_extended_context_mut().insert_expr(body, Expr::Sequence(statements));
            }
        }
        drops.push(match_);
    }

    /// Drop each field of `place`, which is a product type or a union with `with` fields
    fn drop_fields(&mut self, place: &Place, typ: &Type, cause: &DropCause, drops: &mut Vec<ExprId>) {
        let mut fields: Vec<_> = self.tc.get_field_types(typ, None).into_iter().collect();
        fields.sort_by_key(|(_, (_, index))| *index);
        for (field, (field_type, _)) in fields {
            self.drop_place_of_type(&place.clone().field(field), field_type, false, cause, drops);
        }
    }

    /// Returns false and reports an error if the type of `dropped` is unknown. Such a type is
    /// generalized without a `Drop` constraint, so nothing is known to drop it.
    fn drop_is_known(&mut self, dropped: &Dropped) -> bool {
        if !matches!(self.tc.follow_type(&dropped.typ), Type::Variable(_)) {
            return true;
        }
        self.report_missing_drop(dropped);
        false
    }

    /// Report that no `Drop` impl was found for `dropped`
    fn report_missing_drop(&mut self, dropped: &Dropped) {
        let target = self.tc.drop_type(dropped.typ.clone());
        let suggestions = self.tc.collect_import_suggestions(&target);
        let error = self.tc.missing_drop(dropped, suggestions);
        self.tc.compiler.accumulate(error);
    }

    /// Add `effects`, the effects `drop` performs on `dropped`, to the innermost function being
    /// walked, or report an error if they aren't allowed there
    fn check_drop_effects(&mut self, effects: Type, dropped: &Dropped) {
        let row = self.current_row();
        // `try_unify` rather than `unify` so that a mismatch is reported as a disallowed drop
        // effect rather than a type error
        if let Ok(bindings) = self.tc.try_unify(&effects, &row) {
            self.tc.bindings.extend(bindings);
            return;
        }
        let none = TypeBindings::default();
        let mut effects = self.tc.collect_effects(&effects, &none);
        effects.retain(|effect| !matches!(effect, Type::Variable(_)));
        let effects = self.tc.type_to_string(&Type::effects(&effects, &self.tc.bindings, &none));
        let name = self.tc.dropped_name(dropped);
        let typ = self.tc.type_to_string(&dropped.typ);
        let location = dropped.location.clone();
        self.tc.compiler.accumulate(Diagnostic::DropEffectNotAllowed { name, typ, effects, location });
    }

    /// The effect row of the innermost function being walked
    fn current_row(&self) -> Type {
        self.function_rows.last().cloned().unwrap_or_else(Type::pure)
    }
}
