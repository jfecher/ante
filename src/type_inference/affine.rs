use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::{DbHandle, GetItemRaw, RawItem},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{Expr, TopLevelItemKind},
        get_item::TraitDefinition,
        ids::{ExprId, NameId, PatternId, TopLevelName},
    },
    shared_arc::Shared,
    type_inference::{
        Locateable, TypeChecker, prelude_type_name,
        types::Type,
        value_impls::{Owned, OwnedRoot},
    },
};

use super::places::{Place, PlaceEntry};

/// The set of paths moved (or reassigned) within one `if`/`match` branch, one loop or lambda
/// body, or the top-level definition body, along with each mutation performed within it.
#[derive(Default, Clone)]
pub(super) struct MoveScope {
    entries: FxHashMap<Place, MoveState>,
    mutations: Vec<Mutation>,
}

/// Whether a path was moved (and where) or was explicitly reassigned since, within one scope.
#[derive(Clone)]
enum MoveState {
    /// The path was moved at this location and has not since been reassigned in this scope.
    Moved(Location),
    /// The path was reassigned in this scope at this tick, shadowing any move of it (or of any of
    /// its descendants) recorded in an enclosing scope. See [MoveTracker::clear_moves].
    Cleared(u32),
}

/// A mutation of `place`, which invalidates references assigned before `tick`
#[derive(Clone)]
struct Mutation {
    place: PlaceEntry,
    location: Location,
    tick: u32,
}

impl MoveScope {
    /// Record a move of `path`, maintaining the invariant that no two entries in one scope are in
    /// an ancestor/descendant relationship: an existing `Moved` ancestor (or an entry for `path`
    /// itself) already covers `path`, so this is a no-op; otherwise any existing entries that are
    /// descendants of `path` are now redundant (moving the whole subsumes moving any one part of
    /// it) and are removed before inserting. This is what collapses e.g. a move of `x` and a
    /// move of `x.y` down to just a move of `x`, regardless of insertion order.
    fn insert_moved(&mut self, path: Place, location: Location) {
        let subsumed = self
            .entries
            .iter()
            .any(|(existing, state)| matches!(state, MoveState::Moved(_)) && path.is_within(existing));
        if subsumed {
            return;
        }
        self.entries.retain(|existing, _| !existing.is_descendant_of(&path));
        self.entries.insert(path, MoveState::Moved(location));
    }

    /// Record that `path` was reassigned at `tick`: drop any finer-grained entries this scope had
    /// for a descendant of `path` (now stale) and mark `path` itself `Cleared`.
    fn insert_cleared(&mut self, path: Place, tick: u32) {
        self.entries.retain(|existing, _| !existing.is_descendant_of(&path));
        self.entries.insert(path, MoveState::Cleared(tick));
    }

    /// The most specific entry in this scope that is `key` itself or an ancestor of `key`.
    fn most_specific_containing(&self, key: &Place) -> Option<(&Place, &MoveState)> {
        if self.entries.is_empty() {
            return None;
        }
        let mut candidate = key.clone();
        loop {
            if let Some(entry) = self.entries.get_key_value(&candidate) {
                return Some(entry);
            }
            candidate.segments.pop()?;
        }
    }

    /// Every path in this scope currently recorded as moved (excluding cleared ones).
    pub(super) fn moved_paths(&self) -> impl Iterator<Item = (&Place, &Location)> {
        self.entries.iter().filter_map(|(path, state)| match state {
            MoveState::Moved(location) => Some((path, location)),
            MoveState::Cleared(_) => None,
        })
    }

    /// Where a mutation in this scope after `since` may have invalidated a reference to `place`
    fn invalidating(&self, place: &PlaceEntry, since: u32) -> Option<&Location> {
        let mut mutations = self.mutations.iter();
        let mutation = mutations.find(|mutation| mutation.tick > since && place.invalidated_by(&mutation.place));
        mutation.map(|mutation| &mutation.location)
    }
}

/// Tracks which paths have been moved, as a stack of [MoveScope]s: one scope is pushed for each
/// `if`/`match` branch and each loop/lambda body currently being walked, then popped once that
/// branch/body is finished.
///
/// Reassigning a path never mutates an enclosing scope directly: `if`/`match`
/// walk each of their branches against the same enclosing scopes, so mutating an enclosing scope
/// while walking one branch would leak into every sibling branch walked afterward. Instead, a
/// reassignment records a `Cleared` tombstone in the current scope, which shadows any move of
/// that path, or of one of its descendants, recorded further down the stack.
///
/// Mutations are recorded alongside moves, ticked so they can be ordered against reassignments.
pub(super) struct MoveTracker {
    scopes: Vec<MoveScope>,
    /// Paths already reported as used after being moved
    pub(super) errored: FxHashSet<Place>,
    /// The tick of the latest mutation
    clock: u32,
}

impl Default for MoveTracker {
    fn default() -> Self {
        MoveTracker { scopes: vec![MoveScope::default()], errored: FxHashSet::default(), clock: 0 }
    }
}

impl MoveTracker {
    /// The number of scopes being walked
    pub(super) fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    /// Merge every scope from index `base` onward into a single scope containing everything
    /// moved, reassigned, or mutated in them. This is the state a jump out of those scopes
    /// leaves them in.
    pub(super) fn state_since(&self, base: usize) -> MoveScope {
        let mut state = MoveScope::default();
        for scope in &self.scopes[base..] {
            for (path, entry) in &scope.entries {
                match entry {
                    MoveState::Moved(location) => state.insert_moved(path.clone(), location.clone()),
                    MoveState::Cleared(tick) => state.insert_cleared(path.clone(), *tick),
                }
            }
            state.mutations.extend(scope.mutations.iter().cloned());
        }
        state
    }

    /// Push a previously popped scope back on so we can query the state its branch ended in
    pub(super) fn reenter_scope(&mut self, scope: MoveScope) {
        self.scopes.push(scope);
    }

    /// Push a fresh, empty scope, e.g. when entering an `if`/`match` branch or a loop/lambda body.
    pub(super) fn push_scope(&mut self) {
        self.scopes.push(MoveScope::default());
    }

    /// Pop and return the current topmost scope. Every `push_scope` must be paired with exactly
    /// one `pop_scope`; a fresh tracker always starts with one scope already on the stack (see
    /// `Default`), so this should never panic in practice.
    pub(super) fn pop_scope(&mut self) -> MoveScope {
        self.scopes.pop().expect("MoveTracker::pop_scope called with no scope on the stack")
    }

    /// Record that a path has been moved at the given location, in the current scope.
    pub(super) fn record_move(&mut self, path: Place, location: Location) {
        self.scopes.last_mut().unwrap().insert_moved(path, location);
    }

    /// Record that each of `places` was mutated at `location`, in the current scope.
    pub(super) fn record_mutation(&mut self, places: Vec<PlaceEntry>, location: Location) {
        self.clock += 1;
        let tick = self.clock;
        let mutations = places.into_iter().map(|place| Mutation { place, location: location.clone(), tick });
        self.scopes.last_mut().unwrap().mutations.extend(mutations);
    }

    /// Move a finished body's mutations into the current scope, since the body ran in place
    pub(super) fn absorb_mutations(&mut self, body: MoveScope) {
        self.scopes.last_mut().unwrap().mutations.extend(body.mutations);
    }

    /// Record a jump out of the current scope, where `jump` holds what was moved and mutated
    /// before the jump (see [Self::state_since]). Since wherever the jump leads to comes after
    /// this scope, those moves and mutations need to be visible there as well.
    pub(super) fn merge_jump(&mut self, jump: MoveScope) {
        let scope = self.scopes.last_mut().unwrap();
        for (path, state) in jump.entries {
            if let MoveState::Moved(location) = state {
                scope.insert_moved(path, location);
            }
        }
        scope.mutations.extend(jump.mutations);
    }

    /// Clear any move record for `path` and its descendants. Called when `path` is being
    /// reassigned. Only ever touches the current (topmost) scope; see the [MoveTracker] doc
    /// comment for why.
    pub(super) fn clear_moves(&mut self, path: &Place) {
        self.clear_moves_at(path, self.clock);
    }

    fn clear_moves_at(&mut self, path: &Place, tick: u32) {
        self.scopes.last_mut().unwrap().insert_cleared(path.clone(), tick);
        self.errored.remove(path);
        self.errored.retain(|p| !p.is_descendant_of(path));
    }

    /// The most specific entry, anywhere in the scope stack, that is `key` itself or an ancestor
    /// of `key`: scopes are searched innermost-to-outermost, and within each scope the most
    /// specific (closest to `key`) entry wins; the first scope with *any* matching entry
    /// determines the answer. This means a `Cleared` entry for an ancestor of `key` recorded in
    /// an inner scope correctly shadows a `Moved` entry for `key` (or an ancestor of it) recorded
    /// in an outer scope, without ever mutating that outer scope.
    fn effective_state(&self, key: &Place) -> Option<(&Place, &MoveState)> {
        self.scopes.iter().rev().find_map(|scope| scope.most_specific_containing(key))
    }

    /// The tick `path` was last assigned at
    fn assigned_at(&self, path: &Place) -> u32 {
        match self.effective_state(path) {
            Some((_, MoveState::Cleared(tick))) => *tick,
            _ => 0,
        }
    }

    /// Where a mutation since `holder` was assigned may have invalidated one of its references to `entries`
    pub(super) fn invalidated(&self, holder: &Place, entries: &[PlaceEntry]) -> Option<&Location> {
        let since = self.assigned_at(holder);
        entries.iter().find_map(|entry| self.scopes.iter().find_map(|scope| scope.invalidating(entry, since)))
    }

    /// The moved path within or containing `path`, if any, checking ancestors first
    pub(super) fn moved(&self, path: &Place) -> Option<(&Place, &Location)> {
        match self.effective_state(path) {
            Some((moved, MoveState::Moved(location))) => Some((moved, location)),
            _ => self.has_child_moved(path),
        }
    }

    /// Check if any descendant of this path is currently moved (and not since reassigned).
    /// Returns the first one found, if any. Only meaningful once `path` itself isn't moved.
    fn has_child_moved(&self, path: &Place) -> Option<(&Place, &Location)> {
        let mut candidates: Vec<&Place> = Vec::new();
        for scope in &self.scopes {
            for key in scope.entries.keys() {
                if key.is_descendant_of(path) && !candidates.contains(&key) {
                    candidates.push(key);
                }
            }
        }
        candidates.into_iter().find_map(|key| match self.effective_state(key) {
            Some((_, MoveState::Moved(location))) => Some((key, location)),
            _ => None,
        })
    }

    /// Merge each non-diverging branch into the current scope:
    /// - A path reassigned in every branch is reassigned after the whole `if`/`match`, as of the
    ///   earliest of those reassignments.
    /// - A path moved in any branch is moved after the whole `if`/`match`.
    /// - A mutation in any branch is a mutation after the whole `if`/`match`.
    ///
    /// Reassignments are applied first so that a branch which reassigns `x` merged with one
    /// that moves `x` ends with `x` being moved in the parent.
    pub(super) fn merge_branches_into_parent(&mut self, branches: Vec<MoveScope>) {
        let cleared_in = |scope: &MoveScope, path: &Place| match scope.entries.get(path) {
            Some(MoveState::Cleared(tick)) => Some(*tick),
            _ => None,
        };

        let Some(first) = branches.first() else { return };
        let cleared = first.entries.keys().filter_map(|path| {
            // `None` sorts first, so this is only `Some` if every branch reassigned `path`
            let earliest = branches.iter().map(|branch| cleared_in(branch, path)).min()??;
            Some((path.clone(), earliest))
        });
        let cleared_everywhere: Vec<_> = cleared.collect();

        for (path, tick) in &cleared_everywhere {
            self.clear_moves_at(path, *tick);
        }

        let parent = self.scopes.last_mut().unwrap();
        for branch in branches {
            for (path, state) in branch.entries {
                if let MoveState::Moved(location) = state {
                    parent.insert_moved(path, location);
                }
            }
            parent.mutations.extend(branch.mutations);
        }
    }
}

/// Returns the Prelude trait `name` along with its number of generics.
/// If the trait failed to parse, `default_generics` is used as the number of generics instead.
pub(super) fn prelude_trait(compiler: &DbHandle, name: &str, default_generics: usize) -> (TopLevelName, usize) {
    let name = prelude_type_name(compiler, name);
    let generics =
        TraitDefinition::get(name, compiler).map_or(default_generics, |trait_| trait_.definition().generics.len());
    (name, generics)
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// The Prelude's `Copy` trait and its number of generics, caching them
    pub(super) fn copy_trait(&mut self) -> (TopLevelName, usize) {
        let compiler = self.compiler;
        *self.copy_trait.get_or_insert_with(|| prelude_trait(compiler, "Copy", 1))
    }

    /// Returns the type `trait_ typ`, filling in each of the trait's other arguments with a fresh
    /// type variable
    pub(super) fn trait_type(&mut self, (trait_, generic_count): (TopLevelName, usize), typ: Type) -> Type {
        let constructor = Type::UserDefined(Origin::TopLevelDefinition(trait_));
        let mut args = vec![typ];
        args.extend(self.fresh_arguments(&constructor, 1..generic_count));
        Type::Application(Shared::new(constructor), Shared::new(args))
    }

    /// Types that are `Copy` without needing to find a `Copy` impl
    pub(super) fn is_trivially_copy(&self, typ: &Type) -> bool {
        match typ.follow(&self.bindings) {
            // TODO: Remove once traits are no longer passed by value
            typ if typ.impl_data_name(&self.bindings).is_some() => true,
            Type::Primitive(_) => true,
            // A function is copied by copying what it captured
            Type::Function(function) => self.is_trivially_copy_environment(&function.environment),
            Type::Application(constructor, _) if constructor.is_reference_constructor(&self.bindings) => true,
            // TODO: Actually require abilities only capture `Copy` types
            typ if self.is_trait_or_effect(typ) => true,
            // `shared` types are pointer-wrapped in MIR and are always Copy.
            typ => self.is_shared_user_defined(typ),
        }
    }

    /// The Prelude's pair type `(,)`, caching it
    pub(super) fn pair_type(&self) -> TopLevelName {
        *self.pair_type.get_or_init(|| super::prelude_type_name(self.compiler, ","))
    }

    /// True if a closure's environment is `Copy` without needing to search for an impl. This is
    /// the case when every capture is trivially `Copy`, e.g. when everything is captured by reference.
    fn is_trivially_copy_environment(&self, environment: &Type) -> bool {
        match environment.follow(&self.bindings) {
            Type::Application(constructor, captures) if self.is_pair(constructor) => {
                captures.iter().all(|capture| self.is_trivially_copy_environment(capture))
            },
            Type::Tuple(captures) => captures.iter().all(|capture| self.is_trivially_copy(capture)),
            capture => self.is_trivially_copy(capture),
        }
    }

    /// True if `typ` is a closure environment with more than one capture, all trivially `Copy`
    pub(super) fn is_trivially_copy_environment_pair(&self, typ: &Type) -> bool {
        matches!(typ.follow(&self.bindings), Type::Application(constructor, _) if self.is_pair(constructor))
            && self.is_trivially_copy_environment(typ)
    }

    fn is_pair(&self, constructor: &Type) -> bool {
        match constructor.follow(&self.bindings) {
            Type::UserDefined(Origin::TopLevelDefinition(name)) => *name == self.pair_type(),
            _ => false,
        }
    }

    /// The type `Copy typ`, with a fresh type variable for each of its omitted optional arguments
    pub(super) fn copy_type(&mut self, typ: Type) -> Type {
        let copy = self.copy_trait();
        self.trait_type(copy, typ)
    }

    /// Returns true if `typ` is known to implement `Copy`.
    pub(super) fn type_is_copy(&mut self, typ: &Type, location: Location) -> bool {
        let in_scope = self.collect_implicits_in_scope();
        self.type_is_copy_in(typ, location, &in_scope)
    }

    /// Returns true if `typ` is known to implement `Copy` using the local implicits `in_scope`
    pub(super) fn type_is_copy_in(&mut self, typ: &Type, location: Location, in_scope: &[NameId]) -> bool {
        if self.is_trivially_copy(typ) {
            return true;
        }
        match self.follow_type(typ).clone() {
            // A function is copied by copying what it captured
            Type::Function(function) => self.type_is_copy_in(&function.environment, location, in_scope),
            _ => {
                // Cache the result since we ask about the same types repeatedly: once per value,
                // again when a `Drop` search falls back to `Copy`, and for each closure
                // environment holding them
                let subject = typ.follow_all(&self.bindings);
                let reusable = !subject.contains_unbound_variable(&self.bindings)
                    && in_scope.iter().all(|name| !self.name_types[name].contains_unbound_variable(&self.bindings));
                let key = reusable.then(|| (in_scope.to_vec(), subject));
                if let Some(found) = key.as_ref().and_then(|key| self.item.copy_memo.get(key)) {
                    return *found;
                }
                let scc_candidates_tried = self.item.scc_candidates_tried;
                let copy = self.copy_type(typ.clone());
                let found = self.implicit_exists_now(copy, location, in_scope);
                if let Some(key) = key
                    && self.item.scc_candidates_tried == scc_candidates_tried
                {
                    self.item.copy_memo.insert(key, found);
                }
                found
            },
        }
    }

    /// Request a `Copy` impl for each of an impl's `parameters`, whose types are
    /// `parameter_types`. Returns only the parameters that actually need one.
    pub(super) fn request_impl_parameter_copy_impls(
        &mut self, parameters: &[PatternId], parameter_types: &[Type],
    ) -> Vec<PatternId> {
        let unresolved = |typ: &Type| matches!(typ.follow(&self.bindings), Type::Application(constructor, _) if constructor.follow(&self.bindings).is_error());
        let parameters = parameters.iter().zip(parameter_types);
        let needed = parameters.filter(|(_, typ)| !unresolved(typ) && !self.is_trivially_copy(typ)).collect::<Vec<_>>();
        mapvec(needed, |(pattern, _)| {
            self.request_value_impls(Owned::new(OwnedRoot::Pattern(*pattern)), false);
            *pattern
        })
    }

    /// Report each parameter of the impl being checked which isn't `Copy`.
    /// Its impls must already be resolved.
    pub(super) fn check_impl_parameters_are_copy(&mut self) {
        let Some(impl_) = &self.item.impl_ else { return };
        for pattern in impl_.copy_parameters.clone() {
            let typ = &self.pattern_types[&pattern];
            if !self.copy_impl_found(&Owned::new(OwnedRoot::Pattern(pattern)), typ) {
                let typ = self.type_to_string(typ);
                let location = pattern.locate(self);
                self.compiler.accumulate(Diagnostic::ImplParameterNotCopy { typ, location });
            }
        }
    }

    /// Whether evaluating `expr` of type `typ` copies rather than moves.
    pub(super) fn expr_is_copy(&self, expr: ExprId, typ: &Type) -> bool {
        self.copy_impl_found(&self.owned_value(expr), typ)
    }

    fn is_trait_or_effect(&self, typ: &Type) -> bool {
        match typ.follow(&self.bindings) {
            // Type aliases are expanded away during `from_cst_type`, so no `UserDefined`
            // here can refer to an alias
            Type::Application(constructor, _) => self.is_trait_or_effect(constructor),
            Type::UserDefined(origin) => match origin {
                Origin::TopLevelDefinition(name) => {
                    let RawItem(item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
                    matches!(&item.kind, TopLevelItemKind::TraitDefinition(_) | TopLevelItemKind::EffectDefinition(_))
                },
                _ => false,
            },
            _ => false,
        }
    }

    /// Returns the `(shared, mutable)` flags if `typ` resolves to a user-defined type definition.
    fn shared_type_flags(&self, typ: &Type) -> Option<(bool, bool)> {
        match typ.follow(&self.bindings) {
            Type::Application(constructor, _) => self.shared_type_flags(constructor),
            Type::UserDefined(Origin::TopLevelDefinition(name)) => {
                let RawItem(item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
                match &item.kind {
                    TopLevelItemKind::TypeDefinition(td) => Some((td.shared, td.mutable)),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    fn is_shared_user_defined(&self, typ: &Type) -> bool {
        matches!(self.shared_type_flags(typ), Some((true, _)))
    }

    pub(super) fn is_shared_mut_user_defined(&self, typ: &Type) -> bool {
        matches!(self.shared_type_flags(typ), Some((true, true)))
    }

    /// The place a binding denotes: a recorded sub-place, or its own variable by default.
    pub(super) fn binding_place(&self, name: NameId) -> Place {
        self.item.binding_places.get(&name).cloned().unwrap_or_else(|| Place::variable(name))
    }

    /// Returns `expr` as a place if it reads a value held elsewhere rather than producing a new
    /// one. Places are variables or a chain of field accesses on a variable.
    pub(super) fn classify_place(&self, expr: ExprId) -> Option<PlaceExpr> {
        match self.resolved_expr(expr).as_ref() {
            Expr::Variable(path) => {
                let root = match self.path_origin(*path)? {
                    Origin::Local(name) => PlaceExprRoot::Local(name),
                    Origin::TopLevelDefinition(_) | Origin::Builtin(_) => PlaceExprRoot::Global,
                    _ => return None,
                };
                Some(PlaceExpr { root, fields: Vec::new() })
            },
            Expr::TypeAnnotation(annotation) => self.classify_place(annotation.lhs),
            Expr::MemberAccess(access) => {
                let mut place = self.classify_place(access.object)?;
                place.fields.push(self.member_name(access).clone());
                Some(place)
            },
            // The borrow checker names some temporaries so that any parts of them that weren't
            // moved out can still be dropped
            _ => {
                let name = self.current_extended_context().drops().temporary(expr)?;
                Some(PlaceExpr { root: PlaceExprRoot::Temporary(name), fields: Vec::new() })
            },
        }
    }

    /// True if the place `expr` accesses a field through a reference, pointer, or `shared mut`
    /// type. In this case the place's root doesn't own the value being read.
    /// See [Self::classify_place].
    pub(super) fn place_is_indirect(&self, expr: ExprId) -> bool {
        match self.resolved_expr(expr).as_ref() {
            Expr::TypeAnnotation(annotation) => self.place_is_indirect(annotation.lhs),
            Expr::MemberAccess(access) => {
                self.expr_types.get(&access.object).is_some_and(|typ| {
                    typ.reference_or_pointer_element(&self.bindings).is_some() || self.is_shared_mut_user_defined(typ)
                }) || self.place_is_indirect(access.object)
            },
            _ => false,
        }
    }

    /// Returns the [Place] `expr` reads if it is a local place or a temporary.
    /// See [Self::classify_place].
    pub(super) fn try_build_move_path(&self, expr: ExprId) -> Option<Place> {
        let place = self.classify_place(expr)?;
        let root = match place.root {
            PlaceExprRoot::Local(name) => self.binding_place(name),
            PlaceExprRoot::Temporary(name) => Place::variable(name),
            PlaceExprRoot::Global => return None,
        };
        Some(place.fields.into_iter().fold(root, Place::field))
    }
}

/// An expression that reads a value held elsewhere, see [TypeChecker::classify_place]
pub(super) struct PlaceExpr {
    pub(super) root: PlaceExprRoot,

    /// Each field accessed starting from the root. The outermost field is last.
    pub(super) fields: Vec<crate::parser::cst::Name>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) enum PlaceExprRoot {
    Local(NameId),
    /// A top-level definition or builtin. These are never moved or dropped.
    Global,
    /// An expression's value which the borrow checker gave a name to
    Temporary(NameId),
}

impl PlaceExpr {
    /// Returns the local variable this place is rooted at, if any
    pub(super) fn local(&self) -> Option<NameId> {
        match self.root {
            PlaceExprRoot::Local(name) => Some(name),
            _ => None,
        }
    }

    /// True if the place is held by something that still exists after the expression is evaluated
    pub(super) fn is_named(&self) -> bool {
        !matches!(self.root, PlaceExprRoot::Temporary(_))
    }
}
