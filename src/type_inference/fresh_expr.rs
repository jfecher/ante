//! This file contains utilities for creating new expressions in the CST
//! during type-inference. This is most notably used when compiling match expressions
//! where intermediate variables are created to simplify the decision tree structure.

use std::{ops::Index, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::Location,
    parser::{
        context::TopLevelContext,
        cst::{Expr, Name, Path, Pattern},
        ids::{ExprId, NameId, PathId, PatternId},
    },
    type_inference::{TypeChecker, type_id::TypeId},
};

/// Extends a [TopLevelContext] with additional expressions, names, and paths.
pub struct ExtendedTopLevelContext {
    original: Arc<TopLevelContext>,

    more_exprs: FxHashMap<ExprId, Expr>,
    more_patterns: FxHashMap<PatternId, Pattern>,
    more_paths: FxHashMap<PathId, Path>,
    more_names: FxHashMap<NameId, Name>,

    more_expr_locations: FxHashMap<ExprId, Location>,
    more_pattern_locations: FxHashMap<PatternId, Location>,
    more_path_locations: FxHashMap<PathId, Location>,
    more_name_locations: FxHashMap<NameId, Location>,
}

impl<'local, 'innter> TypeChecker<'local, 'innter> {
    pub(super) fn push_expr(&mut self, expr: Expr, typ: TypeId, location: Location) -> ExprId {
        let id = self.current_extended_context_mut().push_expr(expr, location);
        self.expr_types.insert(id, typ);
        id
    }

    pub(super) fn push_pattern(&mut self, pattern: Pattern, location: Location) -> PatternId {
        self.current_extended_context_mut().push_pattern(pattern, location)
    }

    pub(super) fn push_path(&mut self, path: Path, location: Location) -> PathId {
        self.current_extended_context_mut().push_path(path, location)
    }
}

impl ExtendedTopLevelContext {
    pub(crate) fn new(original: Arc<TopLevelContext>) -> Self {
        Self {
            original,
            more_exprs: Default::default(),
            more_patterns: Default::default(),
            more_paths: Default::default(),
            more_names: Default::default(),
            more_expr_locations: Default::default(),
            more_pattern_locations: Default::default(),
            more_path_locations: Default::default(),
            more_name_locations: Default::default(),
        }
    }

    /// Push a new expression to the context
    pub fn push_expr(&mut self, expr: Expr, location: Location) -> ExprId {
        // We assume all expressions are dense and thus no id is skipped
        let new_id = self.original.exprs.len() + self.more_exprs.len();
        let new_id = ExprId::new(new_id as u32);

        self.more_exprs.insert(new_id, expr);
        self.more_expr_locations.insert(new_id, location);
        new_id
    }

    /// Push a new path to the context
    pub fn push_path(&mut self, path: Path, location: Location) -> PathId {
        let new_id = self.original.paths.len() + self.more_paths.len();
        let new_id = PathId::new(new_id as u32);

        self.more_paths.insert(new_id, path);
        self.more_path_locations.insert(new_id, location);
        new_id
    }

    pub(crate) fn push_pattern(&mut self, pattern: Pattern, location: Location) -> PatternId {
        // We assume all nameessions are dense and thus no id is skipped
        let new_id = self.original.patterns.len() + self.more_patterns.len();
        let new_id = PatternId::new(new_id as u32);

        self.more_patterns.insert(new_id, pattern);
        self.more_pattern_locations.insert(new_id, location);
        new_id
    }

    /// Retrieve the location of the corresponding [Path] of the given [PathId]
    pub fn path_location(&self, path: PathId) -> Location {
        match self.original.path_locations.get(path) {
            Some(location) => location.clone(),
            None => self.more_path_locations[&path].clone(),
        }
    }
}

impl Index<ExprId> for ExtendedTopLevelContext {
    type Output = Expr;

    fn index(&self, index: ExprId) -> &Self::Output {
        match self.original.exprs.get(index) {
            Some(expr) => expr,
            None => &self.more_exprs[&index],
        }
    }
}

impl Index<PathId> for ExtendedTopLevelContext {
    type Output = Path;

    fn index(&self, index: PathId) -> &Self::Output {
        match self.original.paths.get(index) {
            Some(path) => path,
            None => &self.more_paths[&index],
        }
    }
}

impl Index<NameId> for ExtendedTopLevelContext {
    type Output = Name;

    fn index(&self, index: NameId) -> &Self::Output {
        match self.original.names.get(index) {
            Some(name) => name,
            None => &self.more_names[&index],
        }
    }
}
