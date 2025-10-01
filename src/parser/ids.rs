use std::{fmt::Display, hash::Hasher};

use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Location,
    incremental::{DbHandle, Parse},
    name_resolution::namespace::SourceFileId,
};

/// A `TopLevelId` is a 64-bit hash uniquely identifying a particular
/// `TopLevelStatement` node. Since these are attached to each node, and we cache
/// nodes by value, any time an Id changes, the compiler will see the
/// associated node as having changed. For this reason, we want to try
/// to make these Ids as stable as possible when the source program changes.
/// Since Ids must be globally unique (ie. across all files), we usually hash the file path containing
/// the Ast node, in addition to the node itself. This means if a file is renamed
/// every Ast node will be marked as changed but this should be rare enough to be okay.
/// Beyond that, how we hash nodes differs depending on the type of node. See
/// the associated `new` functions for explanations on how each is handled.
///
/// Also note that these Ids are only meant to identify an Ast node - they should
/// not be used to answer the question "has this Ast node changed?" since they
/// do not hash all fields of a node.
///
/// Since the Ast is immutable, this id is also used to associate additional
/// data with an Ast including its Location, and later on its Type.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TopLevelId {
    pub source_file: SourceFileId,
    content_hash: u64,
}

impl TopLevelId {
    /// Top level items are hashed by their name along with a collision counter which increments
    /// for each identical name (in a top-level item) in the same file. Note that definitions may
    /// define a method in a type such as `Vec.len v = ...`, in this case the name is considered
    /// only the `len` portion, and we rely on `collision` to disambiguate any similar definitions
    /// in the same file such as `VecIter.len v = ...`.
    pub fn new(source_file: SourceFileId, content_hash: u64) -> TopLevelId {
        TopLevelId { source_file, content_hash }
    }

    pub(crate) fn location(&self, db: &DbHandle) -> Location {
        let result = db.get(Parse(self.source_file));
        result.top_level_data[self].location.clone()
    }
}

impl Display for TopLevelId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Limit it to 4 digits, otherwise it is too long and hurts debugging
        write!(f, "{}_{}", self.source_file, self.content_hash % 10000)
    }
}

pub fn hash(x: impl std::hash::Hash) -> u64 {
    let mut hasher = deterministic_hash::DeterministicHasher::new(std::hash::DefaultHasher::new());
    x.hash(&mut hasher);
    hasher.finish()
}

/// An ExprId is a bit different from a top-level id in that we make no attempt
/// to keep these stable across minor changes over multiple compilations. Each
/// new expression simply receives the next available ExprId from a counter.
///
/// These are however kept independent from each `TopLevelStatement`. Each `TopLevelStatement`
/// that may contain an expression (definitions and print statements) has its own
/// context where expression ids start from zero. This way, although changing any
/// expression within a top-level statement will cause the entire statement to change,
/// this change is still isolated from any other top-level statement in the program.
///
/// These can afford to be a bit smaller than `TopLevelId`s since they're reset for each
/// `TopLevelStatement` and they're generated from a monotonically-increasing counter
/// rather than a hash.
///
/// Since the Ast is immutable, these ExprIds are used to associate more data with
/// a particular node. For example, name resolution fills out any links to definitions,
/// and type inference associates a type with every ExprId.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExprId(u32);

impl ExprId {
    pub fn new(id: u32) -> ExprId {
        ExprId(id)
    }

    pub fn index(self) -> u32 {
        self.0
    }
}

impl From<ExprId> for usize {
    fn from(value: ExprId) -> Self {
        value.0 as usize
    }
}

impl From<usize> for ExprId {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

impl std::fmt::Display for ExprId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Similar to ExprIds, PatternIds are generated from a monotonically increasing counter,
/// which is reset for each top level item.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PatternId(u32);

impl From<PatternId> for usize {
    fn from(value: PatternId) -> Self {
        value.0 as usize
    }
}

impl From<usize> for PatternId {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

/// Similar to ExprIds, PathIds are generated from a monotonically increasing counter,
/// which is reset for each top level item.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PathId(u32);

impl From<PathId> for usize {
    fn from(value: PathId) -> Self {
        value.0 as usize
    }
}

impl From<usize> for PathId {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

/// Similar to ExprIds, PathIds are generated from a monotonically increasing counter,
/// which is reset for each top level item.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NameId(u32);

impl From<NameId> for usize {
    fn from(value: NameId) -> Self {
        value.0 as usize
    }
}

impl From<usize> for NameId {
    fn from(value: usize) -> Self {
        Self(value as u32)
    }
}

impl Display for NameId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
