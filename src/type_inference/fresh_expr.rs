//! This file contains utilities for creating new expressions in the CST
//! during type-inference. This is most notably used when compiling match expressions
//! where intermediate variables are created to simplify the decision tree structure.

use std::{collections::BTreeMap, ops::Index, sync::Arc};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Location,
    name_resolution::{Origin, ResolutionResult},
    parser::{
        context::TopLevelContext,
        cst::{Expr, Name, Path, Pattern},
        ids::{ExprId, NameId, PathId, PatternId},
    },
    type_inference::{TypeChecker, patterns::DecisionTree, type_id::TypeId},
};

/// Extends a [TopLevelContext] with additional expressions, names, and paths.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedTopLevelContext {
    original: Arc<TopLevelContext>,

    name_origins: BTreeMap<NameId, Origin>,
    path_origins: BTreeMap<PathId, Origin>,

    more_exprs: FxHashMap<ExprId, Expr>,
    more_patterns: FxHashMap<PatternId, Pattern>,
    more_paths: FxHashMap<PathId, Path>,
    more_names: FxHashMap<NameId, Name>,

    more_expr_locations: FxHashMap<ExprId, Location>,
    more_pattern_locations: FxHashMap<PatternId, Location>,
    more_path_locations: FxHashMap<PathId, Location>,
    more_name_locations: FxHashMap<NameId, Location>,

    /// Type checking translates match expressions into decision trees,
    /// which need to be stored here for later passes to use.
    decision_trees: BTreeMap<ExprId, DecisionTree>,
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

    pub(super) fn push_name(&mut self, name: Name, location: Location) -> NameId {
        self.current_extended_context_mut().push_name(name, location)
    }
}

impl ExtendedTopLevelContext {
    pub(crate) fn new(original: Arc<TopLevelContext>) -> Self {
        Self {
            original,
            name_origins: Default::default(),
            path_origins: Default::default(),
            more_exprs: Default::default(),
            more_patterns: Default::default(),
            more_paths: Default::default(),
            more_names: Default::default(),
            more_expr_locations: Default::default(),
            more_pattern_locations: Default::default(),
            more_path_locations: Default::default(),
            more_name_locations: Default::default(),
            decision_trees: Default::default(),
        }
    }

    /// Inserts an expression with an existing Id, remapping it to a new value
    pub fn insert_expr(&mut self, id: ExprId, expr: Expr) {
        self.more_exprs.insert(id, expr);
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

    pub fn push_pattern(&mut self, pattern: Pattern, location: Location) -> PatternId {
        // We assume all nameessions are dense and thus no id is skipped
        let new_id = self.original.patterns.len() + self.more_patterns.len();
        let new_id = PatternId::new(new_id as u32);

        self.more_patterns.insert(new_id, pattern);
        self.more_pattern_locations.insert(new_id, location);
        new_id
    }

    /// Push a new name to the context
    pub fn push_name(&mut self, name: Name, location: Location) -> NameId {
        let new_id = self.original.names.len() + self.more_names.len();
        let new_id = NameId::new(new_id as u32);

        self.more_names.insert(new_id, name);
        self.more_name_locations.insert(new_id, location);
        new_id
    }

    /// Retrieve the location of the corresponding [Path] of the given [PathId]
    pub fn path_location(&self, path: PathId) -> Location {
        match self.original.path_locations.get(path) {
            Some(location) => location.clone(),
            None => self.more_path_locations[&path].clone(),
        }
    }

    /// Retrieve the location of the corresponding [Expr] of the given [ExprId]
    pub(crate) fn expr_location(&self, expr: ExprId) -> Location {
        match self.original.expr_locations.get(expr) {
            Some(location) => location.clone(),
            None => self.more_expr_locations[&expr].clone(),
        }
    }

    /// Add each name & path origin from the given [ResolutionResult] to the current extended
    /// context.
    ///
    /// TODO: Restructure type checking so we don't have to clone internally here
    pub(crate) fn extend_from_resolution_result(&mut self, resolution_result: &ResolutionResult) {
        self.name_origins.extend(resolution_result.name_origins.iter().map(|(name, origin)| (*name, *origin)));
        self.path_origins.extend(resolution_result.path_origins.iter().map(|(path, origin)| (*path, *origin)));
    }

    #[allow(unused)]
    pub(crate) fn path_origin(&self, path_id: PathId) -> Origin {
        self.path_origins[&path_id]
    }

    #[allow(unused)]
    pub(crate) fn name_origin(&self, name_id: NameId) -> Origin {
        self.name_origins[&name_id]
    }

    /// Insert a decision tree, replacing the expression at the given id
    ///
    /// Note that because [DecisionTree] is a distinct type, this will not
    /// be checked when indexing the [ExtendedTopLevelContext] with an [ExprId].
    /// Instead, developers must remember to manually check for this case when
    /// retrieving a match expression.
    pub(crate) fn insert_decision_tree(&mut self, expr: ExprId, tree: DecisionTree) {
        self.decision_trees.insert(expr, tree);
    }

    /// Retrieve a given tree from the given expression (expected to be a match expression)
    /// or panic if there is none.
    #[allow(unused)]
    pub(crate) fn decision_tree(&self, expr: ExprId) -> &DecisionTree {
        &self.decision_trees[&expr]
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

impl Index<PatternId> for ExtendedTopLevelContext {
    type Output = Pattern;

    fn index(&self, index: PatternId) -> &Self::Output {
        match self.original.patterns.get(index) {
            Some(pattern) => pattern,
            None => &self.more_patterns[&index],
        }
    }
}
