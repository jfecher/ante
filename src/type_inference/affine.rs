use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::Location,
    incremental::{GetItemRaw, RawItem},
    name_resolution::Origin,
    parser::{
        cst::{Expr, ReferenceKind, TopLevelItemKind},
        ids::{ExprId, NameId, TopLevelName},
    },
    shared_arc::Shared,
    type_inference::{
        Locateable, TypeChecker, prelude_type_name,
        types::{PrimitiveType, Type},
    },
};

use super::places::{Place, PlaceEntry};

/// The set of paths moved (or reassigned) within one `if`/`match` branch, one loop or lambda
/// body, or the top-level definition body, along with each mutation performed within it.
#[derive(Default)]
pub(super) struct MoveScope {
    entries: FxHashMap<Place, MoveState>,
    mutations: Vec<Mutation>,
}

/// Whether a path was moved (and where) or was explicitly reassigned since, within one scope.
enum MoveState {
    /// The path was moved at this location and has not since been reassigned in this scope.
    Moved(Location),
    /// The path was reassigned in this scope at this tick, shadowing any move of it (or of any of
    /// its descendants) recorded in an enclosing scope. See [MoveTracker::clear_moves].
    Cleared(u32),
}

/// A mutation of `place`, which invalidates references assigned before `tick`
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

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Returns the TopLevelName for the Prelude's `Copy` type, caching it.
    fn get_copy_type_name(&mut self) -> TopLevelName {
        if let Some(name) = self.copy_type_name {
            return name;
        }
        let name = prelude_type_name(self.compiler, "Copy");
        self.copy_type_name = Some(name);
        name
    }

    /// Types that are `Copy` without needing to find a `Copy` impl
    fn is_trivially_copy(&self, typ: &Type) -> bool {
        match typ.follow(&self.bindings) {
            Type::Primitive(_) => true,
            // TODO: This isn't always true, but we also can't define the proper Copy impls
            // for functions in the stdlib because we can't manually access a closure's environment
            // and we can't define every copy impl for every possible parameter count.
            Type::Function(_) => true,
            // `ref`, `mut`, and `imm` references are always Copy, see `copy_ref`, `copy_mut`, and `copy_imm`
            Type::Application(constructor, _)
                if matches!(
                    constructor.follow(&self.bindings),
                    Type::Primitive(PrimitiveType::Reference(
                        ReferenceKind::Ref | ReferenceKind::Mut | ReferenceKind::Imm
                    ))
                ) =>
            {
                true
            },
            // TODO: Actually require abilities only capture `Copy` types
            typ if self.is_ability(typ) => true,
            // `shared` types are pointer-wrapped in MIR and are always Copy.
            typ => self.is_shared_user_defined(typ),
        }
    }

    /// The type `Copy typ`
    fn copy_type(&mut self, typ: Type) -> Type {
        let copy_name = self.get_copy_type_name();
        let copy_constructor = Type::UserDefined(Origin::TopLevelDefinition(copy_name));
        Type::Application(Shared::new(copy_constructor), Shared::new(vec![typ]))
    }

    /// Returns true if `typ` is known to implement `Copy`.
    pub(super) fn type_is_copy(&mut self, typ: &Type, location: Location) -> bool {
        if self.is_trivially_copy(typ) {
            return true;
        }
        let copy = self.copy_type(typ.clone());
        self.implicit_exists_now(copy, location)
    }

    /// Request a `Copy` impl for the value of `expr` so the borrowing pass can later tell
    /// whether it is a move or not.
    ///
    /// TODO: Use the found impl to write the actual `Copy` call when a copy variable is used.
    pub(super) fn request_copy_witness(&mut self, expr: ExprId, typ: &Type) {
        if self.copy_witnesses.contains_key(&expr) || self.is_trivially_copy(typ) {
            return;
        }
        let copy = self.copy_type(typ.clone());
        let witness = self.request_optional_implicit(copy, expr.locate(self));
        self.copy_witnesses.insert(expr, witness);
    }

    /// Whether evaluating `expr` of type `typ` copies rather than moves.
    pub(super) fn expr_is_copy(&self, expr: ExprId, typ: &Type) -> bool {
        self.is_trivially_copy(typ)
            || self.copy_witnesses.get(&expr).is_some_and(|witness| self.implicit_was_found(*witness))
    }

    fn is_ability(&self, typ: &Type) -> bool {
        match typ.follow(&self.bindings) {
            // Type aliases are expanded away during `from_cst_type`, so no `UserDefined`
            // here can refer to an alias
            Type::Application(constructor, _) => self.is_ability(constructor),
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
        self.binding_places.get(&name).cloned().unwrap_or_else(|| Place::variable(name))
    }

    /// Try to build a Place from an expression by walking through
    /// variable references and member access chains.
    /// Returns None if the expression is not a simple path.
    pub(super) fn try_build_move_path(&self, expr: ExprId) -> Option<Place> {
        match &self.current_extended_context()[expr] {
            Expr::Variable(path) => {
                if let Some(Origin::Local(name)) = self.path_origin(*path) {
                    Some(self.binding_place(name))
                } else {
                    None
                }
            },
            Expr::MemberAccess(access) => {
                let parent = self.try_build_move_path(access.object)?;
                Some(parent.field(access.member.clone()))
            },
            _ => None,
        }
    }
}
