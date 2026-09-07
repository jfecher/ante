use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::Location, incremental::{ExportedTypes, GetItemRaw, VisibleImplicits}, name_resolution::Origin, parser::{
        cst::{Expr, TopLevelItemKind},
        ids::{ExprId, NameId, TopLevelName},
    }, type_inference::{types::Type, TypeChecker}
};

use super::fresh_expr::ExtendedTopLevelContext;

use crate::name_resolution::namespace::SourceFileId;

/// A path that can be moved: either a variable or a chain of field accesses.
/// For example, `x` or `x.one.two`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum MovePath {
    Variable(NameId),
    Field(Box<MovePath>, String),
}

impl MovePath {
    /// Project a field out of a parent place, e.g. `whole` + `"a"` becomes `whole.a`.
    pub(super) fn field(parent: MovePath, field: String) -> MovePath {
        MovePath::Field(Box::new(parent), field)
    }

    /// Check if `self` is a proper descendant of `ancestor` (but is not itself the ancestor).
    /// E.g. `x.a.b` is a descendant of `x.a` and `x`, but not of `x.a.b`.
    pub(super) fn is_descendant_of(&self, ancestor: &MovePath) -> bool {
        match self {
            _ if self == ancestor => false,
            MovePath::Field(parent, _) => parent.as_ref() == ancestor || parent.is_descendant_of(ancestor),
            MovePath::Variable(_) => false,
        }
    }

    /// Return the root variable name of this path.
    /// E.g. for `x.one.two`, returns the NameId of `x`.
    pub(super) fn root_variable(&self) -> NameId {
        match self {
            MovePath::Variable(name) => *name,
            MovePath::Field(parent, _) => parent.root_variable(),
        }
    }

    /// Build a display name for error messages, e.g. `"c.one.two"`.
    pub(super) fn display_name(&self, context: &ExtendedTopLevelContext) -> String {
        match self {
            MovePath::Variable(name_id) => context[*name_id].to_string(),
            MovePath::Field(parent, field) => {
                format!("{}.{}", parent.display_name(context), field)
            },
        }
    }
}

/// The set of paths moved (or reassigned) within one `if`/`match` branch, one loop or lambda
/// body, or the top-level definition body.
#[derive(Default)]
pub(super) struct MoveScope {
    entries: FxHashMap<MovePath, MoveState>,
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
    fn insert_moved(&mut self, path: MovePath, location: Location) {
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
    fn insert_cleared(&mut self, path: MovePath) {
        self.entries.retain(|existing, _| !existing.is_descendant_of(&path));
        self.entries.insert(path, MoveState::Cleared);
    }

    /// The most specific entry in this scope that is `key` itself or an ancestor of `key`.
    fn most_specific_containing(&self, key: &MovePath) -> Option<&MoveState> {
        let mut candidate = key;
        loop {
            if let Some(state) = self.entries.get(candidate) {
                return Some(state);
            }
            match candidate {
                MovePath::Field(parent, _) => candidate = parent.as_ref(),
                MovePath::Variable(_) => return None,
            }
        }
    }

    /// Every path in this scope currently recorded as moved (excluding cleared ones).
    pub(super) fn moved_paths(&self) -> impl Iterator<Item = (&MovePath, &Location)> {
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
    pub(super) errored: FxHashSet<MovePath>,
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
    pub(super) fn record_move(&mut self, path: MovePath, location: Location) {
        self.scopes.last_mut().unwrap().insert_moved(path, location);
    }

    /// Clear any move record for `path` and its descendants. Called when `path` is being
    /// reassigned. Only ever touches the current (topmost) scope; see the [MoveTracker] doc
    /// comment for why.
    pub(super) fn clear_moves(&mut self, path: &MovePath) {
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
    fn effective_state(&self, key: &MovePath) -> Option<&MoveState> {
        self.scopes.iter().rev().find_map(|scope| scope.most_specific_containing(key))
    }

    /// Check if this path or any ancestor is currently moved (and not since reassigned).
    pub(super) fn is_moved(&self, path: &MovePath) -> Option<&Location> {
        match self.effective_state(path) {
            Some(MoveState::Moved(location)) => Some(location),
            _ => None,
        }
    }

    /// Check if any descendant of this path is currently moved (and not since reassigned).
    /// Returns the first one found, if any. Only meaningful once `is_moved(path)` returns `None`.
    pub(super) fn has_child_moved(&self, path: &MovePath) -> Option<(&MovePath, &Location)> {
        let mut candidates: Vec<&MovePath> = Vec::new();
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

    /// Union each branch's moves into the current (parent) scope: a path is considered moved
    /// after the whole `if`/`match` if it was moved in *any* non-diverging branch.
    /// Branch-local reassignments (`Cleared` entries) are intentionally not propagated: e.g. if a
    /// path is moved before an `if` and reassigned in every branch, it is still considered moved
    /// afterward. This is a known, pre-existing limitation kept as-is (not introduced by this
    /// change).
    ///
    /// Takes an iterator rather than a `Vec` so callers don't need to allocate to collect branch
    /// scopes first — `walk_if` chains two `Option<MoveScope>`s, and `walk_match` merges each
    /// case's scope immediately (via `std::iter::once`) rather than buffering all of them.
    pub(super) fn merge_branches_into_parent(&mut self, branches: impl IntoIterator<Item = MoveScope>) {
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

    /// Returns true if the given type implements Copy.
    ///
    /// `extra_implicits` are additional local implicit names to check beyond whatever is
    /// currently in [Self::collect_implicits_in_scope].
    ///
    /// TODO: Write the actual implicit call to Copy when a copy variable is used.
    pub(super) fn type_is_copy(&mut self, typ: &Type, extra_implicits: &[NameId]) -> bool {
        let typ = self.follow_type(typ).clone();

        if matches!(&typ, Type::Primitive(_)) {
            return true;
        }

        // TODO: This isn't always true, but we also can't define the proper Copy impls
        // for functions in the stdlib because we can't manually access a closure's environment
        // and we can't define every copy impl for every possible parameter count.
        if matches!(&typ, Type::Function(_)) {
            return true;
        }

        // Tuple types are Copy if all elements are Copy
        if let Type::Tuple(elems) = &typ {
            return elems.iter().all(|e| self.type_is_copy(e, extra_implicits));
        }

        // TODO: Actually require abilities only capture `Copy` types
        if self.is_ability(&typ) {
            return true;
        }

        // `shared` types are pointer-wrapped in MIR and are always Copy.
        if self.is_shared_user_defined(&typ) {
            return true;
        }

        let copy_name = self.get_copy_type_name();
        let copy_constructor = Type::UserDefined(Origin::TopLevelDefinition(copy_name));

        let copy_of_t = Type::Application(Arc::new(copy_constructor), Arc::new(vec![typ.clone()]));

        // Check local implicits in scope
        let mut local_implicits = self.collect_implicits_in_scope();
        local_implicits.extend_from_slice(extra_implicits);
        for name in &local_implicits {
            let name_type = self.name_types[name].follow_all(&self.bindings);
            if self.try_unify(&name_type, &copy_of_t).is_ok() {
                return true;
            }
        }

        // Check global implicits
        if let Some(item) = self.current_item {
            let visible_implicits = VisibleImplicits(item.source_file).get(self.compiler);
            let mut found = false;
            visible_implicits.iter_possibly_matching_impls(&copy_of_t, |_name, name_id| {
                let (name_type, _) = self.type_and_bindings_of_top_level_name(name_id);
                if self.try_unify(&name_type, &copy_of_t).is_ok() {
                    found = true;
                    return true;
                }
                // Also check if it's a function whose return type matches
                if let Type::Function(f) = &name_type
                    && self.try_unify(&f.return_type, &copy_of_t).is_ok()
                {
                    found = true;
                    return true;
                }
                false
            });
            if found {
                return true;
            }
        }

        false
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
    pub(super) fn binding_place(&self, name: NameId) -> MovePath {
        self.binding_places.get(&name).cloned().unwrap_or(MovePath::Variable(name))
    }

    /// Try to build a MovePath from an expression by walking through
    /// variable references and member access chains.
    /// Returns None if the expression is not a simple path.
    pub(super) fn try_build_move_path(&self, expr: ExprId) -> Option<MovePath> {
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
                Some(MovePath::field(parent, access.member.clone()))
            },
            _ => None,
        }
    }
}
