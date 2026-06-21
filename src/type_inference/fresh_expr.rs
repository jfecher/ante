//! This file contains utilities for creating new expressions in the CST
//! during type-inference. This is most notably used when compiling match expressions
//! where intermediate variables are created to simplify the decision tree structure.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Index,
    sync::Arc,
};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Location,
    name_resolution::{Origin, ResolutionResult},
    parser::{
        cst::{Expr, Name, Path, Pattern},
        desugar_context::DesugarContext,
        ids::{ExprId, IdStore, NameId, NameStore, PathId, PatternId},
    },
    type_inference::{TypeChecker, patterns::DecisionTree, types::Type},
};

/// Extends a [TopLevelContext] with additional expressions, names, and paths
/// from the [TypeChecker] after performing type-checking and match compilation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendedTopLevelContext {
    original: Arc<DesugarContext>,

    /// The TypeChecker may insert new variables into the code, most commonly
    /// during match compilation where each step is broken into a new variable.
    name_origins: BTreeMap<NameId, Origin>,

    /// The TypeChecker also resolves any paths with Origin::TypeResolution to
    /// a more specific origin (a union variant) if possible.
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
    ///
    /// The extra ExprId in the resulting pair refers to an extra Definition
    /// expression created by type-checking meant to be executed before the decision tree is.
    /// The final code should resemble `{ match_var = ...; decision_tree }`
    decision_trees: BTreeMap<ExprId, (ExprId, DecisionTree)>,

    /// Each member access expression translates to a tuple access in the MIR
    /// so the type checker records which field index into the type the member
    /// access refers to to avoid later passes having to repeat this work.
    member_access_indices: BTreeMap<ExprId, u32>,

    /// For each constructor expression, we remember which order its type expects
    /// the fields to be packed into, regardless of the order the fields were given
    /// in the constructor.
    ///
    /// This maps expression to a map from each field name in the Constructor
    /// expresssion to the field's expected index in its type.
    constructor_field_orders: BTreeMap<ExprId, BTreeMap<NameId, u32>>,

    /// Maps each [PathId] that has been instantiated during type inference to
    /// it's non-generic type before instantiation, along with the instantiation bindings that were
    /// later used to instantiate it.
    ///
    /// The post-instantiation type is already associated with the [PathId] in the result of type
    /// inference.
    ///
    /// Non-generic [PathId]s are not in this map.
    instantiations: FxHashMap<PathId, Vec<Type>>,

    /// Any closure capturing an environment will have an entry into this map with the non-empty
    /// set of variables it captures. Free functions are excluded from the map entirely.
    closure_environments: FxHashMap<ExprId, BTreeSet<NameId>>,

    /// Closures declared with the `move` keyword. These capture by value/move instead of
    /// by reference. Used by the MIR builder to determine capture semantics.
    move_closures: FxHashSet<ExprId>,
}

impl<'local, 'innter> TypeChecker<'local, 'innter> {
    pub(super) fn push_expr(&mut self, expr: Expr, typ: Type, location: Location) -> ExprId {
        let id = self.current_extended_context_mut().push_expr(expr, location);
        self.expr_types.insert(id, typ);
        id
    }

    pub(super) fn push_pattern(&mut self, pattern: Pattern, location: Location) -> PatternId {
        self.current_extended_context_mut().push_pattern(pattern, location)
    }

    pub(super) fn push_name(&mut self, name: Name, location: Location) -> NameId {
        self.current_extended_context_mut().push_name(name, location)
    }

    pub(super) fn push_path(&mut self, path: Path, typ: Type, location: Location) -> PathId {
        let id = self.current_extended_context_mut().push_path(path, location);
        self.path_types.insert(id, typ);
        id
    }
}

impl ExtendedTopLevelContext {
    pub(crate) fn new(original: Arc<DesugarContext>) -> Self {
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
            member_access_indices: Default::default(),
            constructor_field_orders: Default::default(),
            instantiations: Default::default(),
            closure_environments: Default::default(),
            move_closures: Default::default(),
        }
    }

    /// Inserts an expression with an existing Id, remapping it to a new value
    pub fn insert_expr(&mut self, id: ExprId, expr: Expr) {
        self.more_exprs.insert(id, expr);
    }

    /// Return the given expression only if it is extended, and thus not part
    /// of `Self::original`. This can be used to prevent cloning in some cases.
    pub fn extended_expr(&self, id: ExprId) -> Option<&Expr> {
        self.more_exprs.get(&id)
    }

    /// Mutable version of [`Self::extended_expr`]
    pub fn extended_expr_mut(&mut self, id: ExprId) -> Option<&mut Expr> {
        self.more_exprs.get_mut(&id)
    }

    /// Return the given pattern only if it is extended, and thus not part
    /// of `Self::original`. This can be used to prevent cloning in some cases.
    pub fn extended_pattern(&self, id: PatternId) -> Option<&Pattern> {
        self.more_patterns.get(&id)
    }

    /// Push a new expression to the context
    pub fn push_expr(&mut self, expr: Expr, location: Location) -> ExprId {
        // We assume all expressions are dense and thus no id is skipped
        let new_id = self.original.exprs_len() + self.more_exprs.len();
        let new_id = ExprId::new(new_id as u32);

        self.more_exprs.insert(new_id, expr);
        self.more_expr_locations.insert(new_id, location);
        new_id
    }

    /// Push a new path to the context
    pub fn push_path(&mut self, path: Path, location: Location) -> PathId {
        let new_id = self.original.paths_len() + self.more_paths.len();
        let new_id = PathId::new(new_id as u32);

        self.more_paths.insert(new_id, path);
        self.more_path_locations.insert(new_id, location);
        new_id
    }

    /// Push a new path to the context with the given id
    pub fn push_path_with_id(&mut self, location: Location, make_path: impl FnOnce(PathId) -> Path) -> PathId {
        let new_id = self.original.paths_len() + self.more_paths.len();
        let new_id = PathId::new(new_id as u32);

        self.more_paths.insert(new_id, make_path(new_id));
        self.more_path_locations.insert(new_id, location);
        new_id
    }

    pub fn push_pattern(&mut self, pattern: Pattern, location: Location) -> PatternId {
        // We assume all nameessions are dense and thus no id is skipped
        let new_id = self.original.patterns_len() + self.more_patterns.len();
        let new_id = PatternId::new(new_id as u32);

        self.more_patterns.insert(new_id, pattern);
        self.more_pattern_locations.insert(new_id, location);
        new_id
    }

    /// Push a new name to the context
    pub fn push_name(&mut self, name: Name, location: Location) -> NameId {
        let new_id = self.original.names_len() + self.more_names.len();
        let new_id = NameId::new(new_id as u32);

        self.more_names.insert(new_id, name);
        self.more_name_locations.insert(new_id, location);
        new_id
    }

    /// Retrieve the location of the corresponding [Path] of the given [PathId]
    pub fn path_location(&self, path: PathId) -> Location {
        match self.more_path_locations.get(&path) {
            Some(location) => location.clone(),
            None => self.original.path_location(path).clone(),
        }
    }

    /// Retrieve the location of the corresponding [Expr] of the given [ExprId]
    pub(crate) fn expr_location(&self, expr: ExprId) -> Location {
        match self.more_expr_locations.get(&expr) {
            Some(location) => location.clone(),
            None => self.original.expr_location(expr).clone(),
        }
    }

    /// Retrieve the location of the corresponding [Name] of the given [NameId]
    pub(crate) fn name_location(&self, name: NameId) -> Location {
        match self.more_name_locations.get(&name) {
            Some(location) => location.clone(),
            None => self.original.name_location(name).clone(),
        }
    }

    /// Add each name & path origin from the given [ResolutionResult] to the current extended
    /// context.
    ///
    /// TODO: Restructure type checking so we don't have to clone internally here
    pub(crate) fn extend_from_resolution_result(&mut self, resolution_result: &ResolutionResult) {
        self.name_origins.extend(resolution_result.name_origins.iter().map(|(name, origin)| (*name, *origin)));

        for (path, origin) in resolution_result.path_origins.iter() {
            self.path_origins.entry(*path).or_insert(*origin);
        }
    }

    pub(crate) fn insert_path_origin(&mut self, path_id: PathId, origin: Origin) {
        self.path_origins.insert(path_id, origin);
    }

    pub(crate) fn insert_name_origin(&mut self, name_id: NameId, origin: Origin) {
        self.name_origins.insert(name_id, origin);
    }

    pub(crate) fn path_origin(&self, path_id: PathId) -> Option<Origin> {
        self.path_origins.get(&path_id).copied()
    }

    pub fn name_origin(&self, name_id: NameId) -> Option<Origin> {
        self.name_origins.get(&name_id).copied()
    }

    /// Insert a decision tree, replacing the expression at the given id
    ///
    /// The [match_var_decl_expr] parameter refers to the extra variable definition
    /// created by type checking since the match compiler works only on variables rather than full
    /// expressions. This definition is meant to precede the decision tree when executed.
    ///
    /// Note that because [DecisionTree] is a distinct type, this will not
    /// be checked when indexing the [ExtendedTopLevelContext] with an [ExprId].
    /// Instead, developers must remember to manually check for this case when
    /// retrieving a match expression.
    pub(crate) fn insert_decision_tree(&mut self, expr: ExprId, match_var_decl_expr: ExprId, tree: DecisionTree) {
        self.decision_trees.insert(expr, (match_var_decl_expr, tree));
    }

    /// Retrieve a given tree from the given expression (expected to be a match expression)
    /// or panic if there is none.
    pub(crate) fn decision_tree(&self, expr: ExprId) -> Option<&(ExprId, DecisionTree)> {
        self.decision_trees.get(&expr)
    }

    /// Remember that the field that the MemberAccess at the given [ExprId] refers
    /// to is the Nth field of its type, where N is `field_index`.
    pub(crate) fn push_member_access_index(&mut self, expr: ExprId, field_index: u32) {
        self.member_access_indices.insert(expr, field_index);
    }

    /// Retrieve which field index the member access' field refers to in the object type
    pub fn member_access_index(&self, expr: ExprId) -> Option<u32> {
        self.member_access_indices.get(&expr).copied()
    }

    pub(crate) fn push_constructor_field_order(&mut self, id: ExprId, field_order: BTreeMap<NameId, u32>) {
        self.constructor_field_orders.insert(id, field_order);
    }

    pub fn constructor_field_order(&self, id: ExprId) -> Option<&BTreeMap<NameId, u32>> {
        self.constructor_field_orders.get(&id)
    }

    pub(crate) fn insert_instantiation(&mut self, path: PathId, bindings: Vec<Type>) {
        self.instantiations.insert(path, bindings);
    }

    pub fn get_instantiation(&self, path: PathId) -> Option<&Vec<Type>> {
        self.instantiations.get(&path)
    }

    pub(crate) fn insert_closure_environment(&mut self, expr: ExprId, free_vars: BTreeSet<NameId>) {
        self.closure_environments.insert(expr, free_vars);
    }

    /// Retrieves a closure's captured variables. Returns `None` if no variables are captured.
    pub(crate) fn get_closure_environment(&self, expr: ExprId) -> Option<&BTreeSet<NameId>> {
        self.closure_environments.get(&expr)
    }

    pub(crate) fn mark_move_closure(&mut self, expr: ExprId) {
        self.move_closures.insert(expr);
    }

    /// Copy all per-`ExprId` codegen metadata recorded for `from` onto `to`.
    pub(crate) fn copy_expr_metadata(&mut self, from: ExprId, to: ExprId) {
        if let Some(&index) = self.member_access_indices.get(&from) {
            self.member_access_indices.insert(to, index);
        }
        if let Some(order) = self.constructor_field_orders.get(&from).cloned() {
            self.constructor_field_orders.insert(to, order);
        }
        if let Some(tree) = self.decision_trees.get(&from).cloned() {
            self.decision_trees.insert(to, tree);
        }
        if let Some(env) = self.closure_environments.get(&from).cloned() {
            self.closure_environments.insert(to, env);
        }
        if self.move_closures.contains(&from) {
            self.move_closures.insert(to);
        }
    }

    pub fn is_move_closure(&self, expr: ExprId) -> bool {
        self.move_closures.contains(&expr)
    }
}

impl Index<ExprId> for ExtendedTopLevelContext {
    type Output = Expr;

    fn index(&self, index: ExprId) -> &Self::Output {
        match self.more_exprs.get(&index) {
            Some(expr) => expr,
            None => &self.original[index],
        }
    }
}

impl Index<PathId> for ExtendedTopLevelContext {
    type Output = Path;

    fn index(&self, index: PathId) -> &Self::Output {
        match self.more_paths.get(&index) {
            Some(path) => path,
            None => &self.original[index],
        }
    }
}

impl Index<NameId> for ExtendedTopLevelContext {
    type Output = Name;

    fn index(&self, index: NameId) -> &Self::Output {
        match self.more_names.get(&index) {
            Some(name) => name,
            None => &self.original[index],
        }
    }
}

impl Index<PatternId> for ExtendedTopLevelContext {
    type Output = Pattern;

    fn index(&self, index: PatternId) -> &Self::Output {
        match self.more_patterns.get(&index) {
            Some(pattern) => pattern,
            None => &self.original[index],
        }
    }
}

impl IdStore for ExtendedTopLevelContext {
    fn get_expr(&self, id: ExprId) -> &crate::parser::cst::Expr {
        &self[id]
    }

    fn get_pattern(&self, id: PatternId) -> &crate::parser::cst::Pattern {
        &self[id]
    }

    fn get_path(&self, id: PathId) -> &crate::parser::cst::Path {
        &self[id]
    }
}

impl NameStore for ExtendedTopLevelContext {
    fn get_name(&self, id: NameId) -> &Name {
        &self[id]
    }

    fn try_get_name(&self, id: NameId) -> Option<&Name> {
        self.more_names.get(&id).or_else(|| self.original.try_get_name(id))
    }
}
