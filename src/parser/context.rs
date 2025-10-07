use serde::{Deserialize, Serialize};

use crate::{diagnostics::{Location, LocationData}, name_resolution::namespace::SourceFileId, parser::{cst::{Expr, Name, Path, Pattern}, ids::{ExprId, NameId, PathId, PatternId}}, vecmap::VecMap};

/// Metadata associated with a top level statement
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct TopLevelContext {
    pub location: Location,
    pub exprs: VecMap<ExprId, Expr>,
    pub patterns: VecMap<PatternId, Pattern>,
    pub paths: VecMap<PathId, Path>,
    pub names: VecMap<NameId, Name>,

    pub expr_locations: VecMap<ExprId, Location>,
    pub pattern_locations: VecMap<PatternId, Location>,
    pub path_locations: VecMap<PathId, Location>,
    pub name_locations: VecMap<NameId, Location>,
}

impl TopLevelContext {
    pub fn new(file_id: SourceFileId) -> Self {
        Self {
            location: LocationData::placeholder(file_id),
            exprs: VecMap::default(),
            patterns: VecMap::default(),
            expr_locations: VecMap::default(),
            pattern_locations: VecMap::default(),
            paths: VecMap::default(),
            names: VecMap::default(),
            path_locations: VecMap::default(),
            name_locations: VecMap::default(),
        }
    }
}
