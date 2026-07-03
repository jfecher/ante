use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::{Diagnostic, Location, RepeatedContext},
    incremental::{ExportedTypes, GetItemRaw, VisibleImplicits},
    name_resolution::Origin,
    parser::{
        cst::{Expr, TopLevelItemKind},
        ids::{ExprId, NameId, TopLevelName},
    },
    type_inference::{Locateable, TypeChecker, types::Type},
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
    fn is_descendant_of(&self, ancestor: &MovePath) -> bool {
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

/// Tracks which paths have been moved in the current scope.
/// Used for affine type checking: non-Copy values may only be used once.
#[derive(Clone, Default)]
pub(super) struct MoveTracker {
    moved: FxHashMap<MovePath, Location>,
    errored: FxHashSet<MovePath>,
}

/// A snapshot of a [`MovePath`]'s move record, taken before an expression is inferred so
/// the auto-ref coercion can roll the place back to its pre-inference state.
pub(super) struct SavedMove {
    pub(super) path: MovePath,
    pub(super) location: Option<Location>,
}

impl MoveTracker {
    /// Record that a path has been moved at the given location.
    pub(super) fn record_move(&mut self, path: MovePath, location: Location) {
        self.moved.insert(path, location);
    }

    /// Clear any move record for `path` and its descendants. Called when `path` is being reassigned.
    pub(super) fn clear_moves(&mut self, path: &MovePath) {
        self.moved.remove(path);
        self.moved.retain(|p, _| !p.is_descendant_of(path));
        self.errored.remove(path);
        self.errored.retain(|p| !p.is_descendant_of(path));
    }

    /// Snapshot the move record for `path` so it can be restored after an auto-ref coercion.
    pub(super) fn save_move(&self, path: &MovePath) -> Option<Location> {
        self.moved.get(path).cloned()
    }

    /// Restore a previously saved move record, dropping this expression's own contribution.
    pub(super) fn restore_move(&mut self, path: &MovePath, saved: Option<Location>) {
        match saved {
            Some(location) => self.moved.insert(path.clone(), location),
            None => self.moved.remove(path),
        };
    }

    /// Check if this path or any ancestor is already moved.
    /// Returns the location of the move if found.
    pub(super) fn is_moved(&self, path: &MovePath) -> Option<&Location> {
        if let Some(loc) = self.moved.get(path) {
            return Some(loc);
        }
        match path {
            MovePath::Field(parent, _) => self.is_moved(parent),
            MovePath::Variable(_) => None,
        }
    }

    /// Check if any child (descendant) of this path has been moved.
    /// Returns the first one found, if any.
    pub(super) fn has_child_moved(&self, path: &MovePath) -> Option<(&MovePath, &Location)> {
        self.moved.iter().find(|(moved_path, _)| moved_path.is_descendant_of(path))
    }

    /// Merge move trackers from multiple branches.
    /// A path is considered moved after the branch if it was moved in the base
    /// OR in ANY branch (since one of the branches will execute).
    pub(super) fn merge_branches(base: &MoveTracker, branches: &[MoveTracker]) -> MoveTracker {
        let mut result = base.clone();
        for branch in branches {
            for (path, loc) in &branch.moved {
                if !result.moved.contains_key(path) {
                    result.moved.insert(path.clone(), loc.clone());
                }
            }
            for path in &branch.errored {
                result.errored.insert(path.clone());
            }
        }
        result
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
    /// TODO: Write the actual implicit call to Copy when a copy variable is used.
    pub(super) fn type_is_copy(&mut self, typ: &Type) -> bool {
        let typ = self.follow_type(typ).clone();

        // Fast path: all primitive types are Copy (uniq refs are Type::Applications)
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
            return elems.iter().all(|e| self.type_is_copy(e));
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
        let local_implicits = self.collect_implicits_in_scope();
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
                    matches!(&item.kind, TopLevelItemKind::AbilityDefinition(_))
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

    /// Check if using `path` is valid (not already moved or partially moved).
    /// Emits a diagnostic if the path was already moved.
    /// Only emits the first error per path to avoid noisy duplicate diagnostics.
    pub(super) fn check_use_of_move_path(&mut self, path: &MovePath, locator: impl Locateable) {
        if self.move_tracker.errored.contains(path) {
            return;
        }

        // Check if this exact path or an ancestor was moved
        if let Some(moved_loc) = self.move_tracker.is_moved(path) {
            let name = path.display_name(self.current_extended_context());
            let location = locator.locate(self);
            let moved_in = moved_loc.clone();
            self.compiler.accumulate(Diagnostic::UseOfMovedValue { name: name.clone(), location, moved_in });
            self.move_tracker.errored.insert(path.clone());

        // Check if any child was moved (partial move)
        } else if let Some((_child_path, moved_loc)) = self.move_tracker.has_child_moved(path) {
            let name = path.display_name(self.current_extended_context());
            let location = locator.locate(self);
            let moved_in = moved_loc.clone();
            self.compiler.accumulate(Diagnostic::UseOfMovedValue { name, location, moved_in });
            self.move_tracker.errored.insert(path.clone());
        }
    }

    /// Emit errors for any non-Copy outer variables moved during a context whose
    /// body may run more than once (handler branches, `for` bodies, `while`
    /// condition + body). Call this with `self.move_tracker` set to the scope-local
    /// tracker (started empty via `mem::take`); `outer_names` is the set of NameIds
    /// that existed *before* the scope was entered.
    pub(super) fn check_moves_in_repeated_context(
        &mut self, outer_names: &rustc_hash::FxHashSet<NameId>, context: RepeatedContext,
    ) {
        let outer_moves: Vec<(MovePath, Location)> = self
            .move_tracker
            .moved
            .iter()
            .filter(|(path, _)| outer_names.contains(&path.root_variable()))
            .map(|(p, l)| (p.clone(), l.clone()))
            .collect();

        for (path, location) in outer_moves {
            if !self.type_is_copy(&self.name_types[&path.root_variable()].clone()) {
                let name = path.display_name(self.current_extended_context());
                self.compiler.accumulate(Diagnostic::MoveInRepeatedContext { name, context, location });
            }
        }
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
