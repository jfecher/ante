use serde::{Deserialize, Serialize};

use crate::parser::ids::TopLevelId;

#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) enum Namespace {
    /// A local namespace within an expression with possibly both
    /// locals and globals visible. The actual module will should
    /// always match `Resolver::namespace()`
    Local,

    /// A module within a crate
    #[allow(unused)]
    Module(SourceFileId),

    /// A type's namespace containing its methods
    #[allow(unused)]
    Type(TopLevelId),
}

impl Namespace {
    pub(super) fn crate_(crate_id: CrateId) -> Self {
        Namespace::Module(SourceFileId { crate_id, local_module_id: CRATE_ROOT_MODULE })
    }
}

/// A SourceFileId corresponds to one source file in a project
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceFileId {
    pub crate_id: CrateId,
    pub local_module_id: LocalModuleId,
}

impl SourceFileId {
    pub fn new(crate_id: CrateId, path: &std::path::Path) -> SourceFileId {
        let local_module_id = LocalModuleId(crate::parser::ids::hash(path) as u32);
        SourceFileId { crate_id, local_module_id }
    }
}

/// A crate's id is a hash of its name and its version.
/// Crate ids are expected to be globally unique.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CrateId(pub u32);

/// `Std` always has id 0
pub const STDLIB_CRATE: CrateId = CrateId(0);

/// The local crate always has id 1
pub const LOCAL_CRATE: CrateId = CrateId(1);

/// A local module id is a hash of the module path from the crate root.
/// Module ids are expected to be unique only within the same crate.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalModuleId(pub u32);

/// A crate's root module always has ID 0
pub const CRATE_ROOT_MODULE: LocalModuleId = LocalModuleId(0);
