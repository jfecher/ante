use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::Location,
    incremental::{ExportedTypes, GetItemRaw},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{Expr, TopLevelItemKind},
        ids::{ExprId, NameId, TopLevelName},
    },
    type_inference::{Locateable, TypeChecker, types::Type},
};

use super::places::PlacePath;

use crate::name_resolution::namespace::SourceFileId;

/// The set of paths moved (or reassigned) within one `if`/`match` branch, one loop or lambda
/// body, or the top-level definition body.
#[derive(Default)]
pub(super) struct MoveScope {
    entries: FxHashMap<PlacePath, MoveState>,
}

/// Whether a path was moved (and where) or was explicitly reassigned since, within one scope.
enum MoveState {
    /// The path was moved at this location and has not since been reassigned in this scope.
    Moved(Location),
    /// The path was reassigned in this scope, shadowing any move of it (or of any of its
    /// descendants) recorded in an enclosing scope. See [MoveTracker::clear_moves].
    Cleared,
}

impl MoveScope {
    /// Record a move of `path`, maintaining the invariant that no two entries in one scope are in
    /// an ancestor/descendant relationship: an existing `Moved` ancestor (or an entry for `path`
    /// itself) already covers `path`, so this is a no-op; otherwise any existing entries that are
    /// descendants of `path` are now redundant (moving the whole subsumes moving any one part of
    /// it) and are removed before inserting. This is what collapses e.g. a move of `x` and a
    /// move of `x.y` down to just a move of `x`, regardless of insertion order.
    fn insert_moved(&mut self, path: PlacePath, location: Location) {
        let subsumed = self.entries.iter().any(|(existing, state)| {
            matches!(state, MoveState::Moved(_)) && (*existing == path || path.is_descendant_of(existing))
        });
        if subsumed {
            return;
        }
        self.entries.retain(|existing, _| !existing.is_descendant_of(&path));
        self.entries.insert(path, MoveState::Moved(location));
    }

    /// Record that `path` was reassigned: drop any finer-grained entries this scope had for a
    /// descendant of `path` (now stale) and mark `path` itself `Cleared`.
    fn insert_cleared(&mut self, path: PlacePath) {
        self.entries.retain(|existing, _| !existing.is_descendant_of(&path));
        self.entries.insert(path, MoveState::Cleared);
    }

    /// The most specific entry in this scope that is `key` itself or an ancestor of `key`.
    fn most_specific_containing(&self, key: &PlacePath) -> Option<&MoveState> {
        let mut candidate = key;
        loop {
            if let Some(state) = self.entries.get(candidate) {
                return Some(state);
            }
            match candidate {
                PlacePath::Field(parent, _) => candidate = parent.as_ref(),
                PlacePath::Variable(_) => return None,
            }
        }
    }

    /// Every path in this scope currently recorded as moved (excluding cleared ones).
    pub(super) fn moved_paths(&self) -> impl Iterator<Item = (&PlacePath, &Location)> {
        self.entries.iter().filter_map(|(path, state)| match state {
            MoveState::Moved(location) => Some((path, location)),
            MoveState::Cleared => None,
        })
    }
}

/// Tracks which paths have been moved, as a stack of [MoveScope]s: one scope is pushed for each
/// `if`/`match` branch and each loop/lambda body currently being walked, then popped once that
/// branch/body is finished (and either merged into the parent scope, for branches, or checked
/// against the names declared outside it, for loop/lambda bodies — see `borrows.rs`).
///
/// Reassigning a path (`clear_moves`) never mutates an *enclosing* scope directly: `if`/`match`
/// walk each of their branches against the *same* enclosing scopes (only ever pushing one fresh
/// scope per branch), so mutating an enclosing scope while walking one branch would leak into
/// every sibling branch walked afterward. Instead, a reassignment records a `Cleared` tombstone
/// in the *current* (topmost) scope, which shadows (for lookups only) any move of that path, or
/// of one of its descendants, recorded further down the stack.
pub(super) struct MoveTracker {
    scopes: Vec<MoveScope>,
    /// Paths already reported as used after being moved
    pub(super) errored: FxHashSet<PlacePath>,
}

impl Default for MoveTracker {
    fn default() -> Self {
        MoveTracker { scopes: vec![MoveScope::default()], errored: FxHashSet::default() }
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
    pub(super) fn record_move(&mut self, path: PlacePath, location: Location) {
        self.scopes.last_mut().unwrap().insert_moved(path, location);
    }

    /// Clear any move record for `path` and its descendants. Called when `path` is being
    /// reassigned. Only ever touches the current (topmost) scope; see the [MoveTracker] doc
    /// comment for why.
    pub(super) fn clear_moves(&mut self, path: &PlacePath) {
        self.scopes.last_mut().unwrap().insert_cleared(path.clone());
        self.errored.remove(path);
        self.errored.retain(|p| !p.is_descendant_of(path));
    }

    /// The most specific entry, anywhere in the scope stack, that is `key` itself or an ancestor
    /// of `key`: scopes are searched innermost-to-outermost, and within each scope the most
    /// specific (closest to `key`) entry wins; the first scope with *any* matching entry
    /// determines the answer. This means a `Cleared` entry for an ancestor of `key` recorded in
    /// an inner scope correctly shadows a `Moved` entry for `key` (or an ancestor of it) recorded
    /// in an outer scope, without ever mutating that outer scope.
    fn effective_state(&self, key: &PlacePath) -> Option<&MoveState> {
        self.scopes.iter().rev().find_map(|scope| scope.most_specific_containing(key))
    }

    /// Check if this path or any ancestor is currently moved (and not since reassigned).
    pub(super) fn is_moved(&self, path: &PlacePath) -> Option<&Location> {
        match self.effective_state(path) {
            Some(MoveState::Moved(location)) => Some(location),
            _ => None,
        }
    }

    /// Check if any descendant of this path is currently moved (and not since reassigned).
    /// Returns the first one found, if any. Only meaningful once `is_moved(path)` returns `None`.
    pub(super) fn has_child_moved(&self, path: &PlacePath) -> Option<(&PlacePath, &Location)> {
        let mut candidates: Vec<&PlacePath> = Vec::new();
        for scope in &self.scopes {
            for key in scope.entries.keys() {
                if key.is_descendant_of(path) && !candidates.contains(&key) {
                    candidates.push(key);
                }
            }
        }
        candidates.into_iter().find_map(|key| match self.effective_state(key) {
            Some(MoveState::Moved(location)) => Some((key, location)),
            _ => None,
        })
    }

    /// Merge each non-diverging branch into the current scope:
    /// - A path reassigned in every branch is reassigned after the whole `if`/`match`.
    /// - A path moved in any branch is moved after the whole `if`/`match`.
    ///
    /// Reassignments are applied first so that a branch which reassigns `x` merged with one
    /// that moves `x` ends with `x` being moved in the parent.
    pub(super) fn merge_branches_into_parent(&mut self, branches: Vec<MoveScope>) {
        let cleared_in =
            |scope: &MoveScope, path: &PlacePath| matches!(scope.entries.get(path), Some(MoveState::Cleared));

        let cleared_everywhere = branches.first().map_or(Vec::new(), |first| {
            let cleared = first.entries.iter().filter(|(path, _)| cleared_in(first, path));
            let cleared = cleared.filter(|(path, _)| branches[1..].iter().all(|branch| cleared_in(branch, path)));
            mapvec(cleared, |(path, _)| path.clone())
        });

        for path in &cleared_everywhere {
            self.clear_moves(path);
        }

        let parent = self.scopes.last_mut().unwrap();
        for branch in branches {
            for (path, state) in branch.entries {
                if let MoveState::Moved(location) = state {
                    parent.insert_moved(path, location);
                }
            }
        }
    }
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Returns the TopLevelName for the Prelude's `Copy` type, caching it.
    fn get_copy_type_name(&mut self) -> TopLevelName {
        if let Some(name) = self.copy_type_name {
            return name;
        }
        let exported_types = ExportedTypes(SourceFileId::prelude()).get(self.compiler);
        let top_level_name = exported_types.get(&Arc::new("Copy".to_string())).expect("Copy type not found in Prelude");
        self.copy_type_name = Some(*top_level_name);
        *top_level_name
    }

    /// Types that are `Copy` without needing to find a `Copy` impl
    fn is_trivially_copy(&self, typ: &Type) -> bool {
        match typ.follow(&self.bindings) {
            Type::Primitive(_) => true,
            // TODO: This isn't always true, but we also can't define the proper Copy impls
            // for functions in the stdlib because we can't manually access a closure's environment
            // and we can't define every copy impl for every possible parameter count.
            Type::Function(_) => true,
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
        Type::Application(Arc::new(copy_constructor), Arc::new(vec![typ]))
    }

    /// Returns true if `typ` is known to implement `Copy`.
    /// FIXME: Types that aren't fully known yet are optimistically assumed to be `Copy`.
    pub(super) fn type_is_copy(&mut self, typ: &Type, location: Location) -> bool {
        if self.is_trivially_copy(typ) || typ.has_unbound_type_variables(&self.bindings) {
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
    /// FIXME: Values whose types aren't fully known are optimistically assumed to be `Copy`.
    pub(super) fn expr_is_copy(&self, expr: ExprId, typ: &Type) -> bool {
        self.is_trivially_copy(typ)
            || typ.has_unbound_type_variables(&self.bindings)
            || self.copy_witnesses.get(&expr).is_some_and(|witness| self.implicit_was_found(*witness))
    }

    fn is_ability(&self, typ: &Type) -> bool {
        match typ.follow(&self.bindings) {
            // Type aliases are expanded away during `from_cst_type`, so no `UserDefined`
            // here can refer to an alias
            Type::Application(constructor, _) => self.is_ability(constructor),
            Type::UserDefined(origin) => match origin {
                Origin::TopLevelDefinition(name) => {
                    let (item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
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
                let (item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
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
    pub(super) fn binding_place(&self, name: NameId) -> PlacePath {
        self.binding_places.get(&name).cloned().unwrap_or(PlacePath::Variable(name))
    }

    /// Try to build a PlacePath from an expression by walking through
    /// variable references and member access chains.
    /// Returns None if the expression is not a simple path.
    pub(super) fn try_build_move_path(&self, expr: ExprId) -> Option<PlacePath> {
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
                Some(PlacePath::field(parent, access.member.clone()))
            },
            _ => None,
        }
    }
}
