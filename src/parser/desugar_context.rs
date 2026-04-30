use std::{ops::Index, sync::Arc};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Location,
    parser::{
        context::TopLevelContext,
        cst::{Expr, Name, Path, Pattern},
        ids::{ExprId, IdStore, NameId, NameStore, PathId, PatternId},
    },
};

/// Extends a [TopLevelContext] with additional expressions, names, paths, and patterns
/// added during the desugaring pass. Also tracks replacements to existing expression nodes
/// (e.g. `loop`, `|>`, `and`/`or` desugaring overwrite the original expression slot).
///
/// Modeled after [crate::type_inference::fresh_expr::ExtendedTopLevelContext].
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesugarContext {
    original: Arc<TopLevelContext>,

    /// Stores both newly-added items (id >= original length) and overrides to existing
    /// items (id < original length). Reads check this map first before falling back to
    /// `original`.
    more_exprs: FxHashMap<ExprId, Expr>,
    more_patterns: FxHashMap<PatternId, Pattern>,
    more_paths: FxHashMap<PathId, Path>,
    more_names: FxHashMap<NameId, Name>,

    more_expr_locations: FxHashMap<ExprId, Location>,
    more_pattern_locations: FxHashMap<PatternId, Location>,
    more_path_locations: FxHashMap<PathId, Location>,
    more_name_locations: FxHashMap<NameId, Location>,
}

impl DesugarContext {
    pub fn new(original: Arc<TopLevelContext>) -> Self {
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

    /// The total number of expressions (original + newly added). Used by
    /// [crate::type_inference::fresh_expr::ExtendedTopLevelContext] to compute fresh IDs.
    pub fn exprs_len(&self) -> usize {
        self.original.exprs.len() + self.more_exprs.len()
    }

    pub fn patterns_len(&self) -> usize {
        self.original.patterns.len() + self.more_patterns.len()
    }

    pub fn paths_len(&self) -> usize {
        self.original.paths.len() + self.more_paths.len()
    }

    pub fn names_len(&self) -> usize {
        self.original.names.len() + self.more_names.len()
    }

    pub fn push_expr(&mut self, expr: Expr, location: Location) -> ExprId {
        let new_id = ExprId::new(self.exprs_len() as u32);
        self.more_exprs.insert(new_id, expr);
        self.more_expr_locations.insert(new_id, location);
        new_id
    }

    pub fn push_pattern(&mut self, pattern: Pattern, location: Location) -> PatternId {
        let new_id = PatternId::new(self.patterns_len() as u32);
        self.more_patterns.insert(new_id, pattern);
        self.more_pattern_locations.insert(new_id, location);
        new_id
    }

    pub fn push_path(&mut self, path: Path, location: Location) -> PathId {
        let new_id = PathId::new(self.paths_len() as u32);
        self.more_paths.insert(new_id, path);
        self.more_path_locations.insert(new_id, location);
        new_id
    }

    pub fn push_name(&mut self, name: Name, location: Location) -> NameId {
        let new_id = NameId::new(self.names_len() as u32);
        self.more_names.insert(new_id, name);
        self.more_name_locations.insert(new_id, location);
        new_id
    }

    pub fn set_expr(&mut self, id: ExprId, expr: Expr) {
        self.more_exprs.insert(id, expr);
    }

    pub fn expr_location(&self, id: ExprId) -> &Location {
        match self.more_expr_locations.get(&id) {
            Some(loc) => loc,
            None => &self.original.expr_locations[id],
        }
    }

    pub fn pattern_location(&self, id: PatternId) -> &Location {
        match self.more_pattern_locations.get(&id) {
            Some(loc) => loc,
            None => &self.original.pattern_locations[id],
        }
    }

    pub fn path_location(&self, id: PathId) -> &Location {
        match self.more_path_locations.get(&id) {
            Some(loc) => loc,
            None => &self.original.path_locations[id],
        }
    }

    pub fn name_location(&self, id: NameId) -> &Location {
        match self.more_name_locations.get(&id) {
            Some(loc) => loc,
            None => &self.original.name_locations[id],
        }
    }

    /// Returns the location of the entire top-level item this context represents.
    pub fn location(&self) -> &Location {
        &self.original.location
    }

    /// Returns a reference to the underlying [TopLevelContext].
    pub fn as_top_level_context(&self) -> &TopLevelContext {
        &self.original
    }

    pub fn path_locations(&self) -> impl Iterator<Item = (PathId, &Location)> {
        self.original.path_locations.iter().chain(self.more_path_locations.iter().map(|(k, v)| (*k, v)))
    }

    pub fn name_locations(&self) -> impl Iterator<Item = (NameId, &Location)> {
        self.original.name_locations.iter().chain(self.more_name_locations.iter().map(|(k, v)| (*k, v)))
    }

    pub fn pattern_locations(&self) -> impl Iterator<Item = (PatternId, &Location)> {
        self.original.pattern_locations.iter().chain(self.more_pattern_locations.iter().map(|(k, v)| (*k, v)))
    }
}

impl Index<ExprId> for DesugarContext {
    type Output = Expr;

    fn index(&self, index: ExprId) -> &Self::Output {
        match self.more_exprs.get(&index) {
            Some(expr) => expr,
            None => &self.original.exprs[index],
        }
    }
}

impl Index<PatternId> for DesugarContext {
    type Output = Pattern;

    fn index(&self, index: PatternId) -> &Self::Output {
        match self.more_patterns.get(&index) {
            Some(pattern) => pattern,
            None => &self.original.patterns[index],
        }
    }
}

impl Index<PathId> for DesugarContext {
    type Output = Path;

    fn index(&self, index: PathId) -> &Self::Output {
        match self.more_paths.get(&index) {
            Some(path) => path,
            None => &self.original.paths[index],
        }
    }
}

impl Index<NameId> for DesugarContext {
    type Output = Name;

    fn index(&self, index: NameId) -> &Self::Output {
        match self.more_names.get(&index) {
            Some(name) => name,
            None => &self.original.names[index],
        }
    }
}

impl IdStore for DesugarContext {
    fn get_expr(&self, id: ExprId) -> &Expr {
        &self[id]
    }

    fn get_pattern(&self, id: PatternId) -> &Pattern {
        &self[id]
    }

    fn get_path(&self, id: PathId) -> &Path {
        &self[id]
    }
}

impl NameStore for DesugarContext {
    fn get_name(&self, id: NameId) -> &Name {
        &self[id]
    }

    fn try_get_name(&self, id: NameId) -> Option<&Name> {
        self.more_names.get(&id).or_else(|| self.original.names.get(id))
    }
}
