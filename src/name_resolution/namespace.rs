use serde::{Deserialize, Serialize};

use crate::{find_files::SRC_FOLDER, parser::ids::TopLevelId, paths::prelude_path_relative_to_stdlib_source_folder};

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
    /// Create a new [SourceFileId]. Note that _all_ [SourceFileId]s are created
    /// with a hash of the file path, excluding the `src` directory, if present.
    /// E.g. `foo/bar/module_root/src/baz/qux.an` is an invalid path, it should
    /// be abbreviated to `baz/qux.an`
    pub fn new(crate_id: CrateId, path: &std::path::Path) -> SourceFileId {
        let local_module_id = LocalModuleId(crate::parser::ids::hash(path) as u32);
        SourceFileId { crate_id, local_module_id }
    }

    pub fn new_in_local_crate(path: &std::path::Path) -> SourceFileId {
        SourceFileId::new(CrateId::LOCAL, path)
    }

    /// Normalize the given path and create a SourceFileId from it
    pub fn for_local_path(root: &std::path::Path, path: &std::path::Path) -> SourceFileId {
        let path = Self::normalize_path(root, path);
        SourceFileId::new(CrateId::LOCAL, path)
    }

    /// Normalizes the path so any SourceFileIds created from it are consistent:
    /// - Remove the `root` prefix if present
    /// - Remove a `src` directory prefix
    pub fn normalize_path<'a>(root: &'a std::path::Path, path: &'a std::path::Path) -> &'a std::path::Path {
        let relative = path.strip_prefix(root).unwrap_or(path);
        relative.strip_prefix(SRC_FOLDER).unwrap_or(relative)
    }

    pub fn prelude() -> SourceFileId {
        Self::new(CrateId::STDLIB, prelude_path_relative_to_stdlib_source_folder())
    }
}

/// A crate's id is a hash of its name and its version.
/// Crate ids are expected to be globally unique.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CrateId(pub u32);

impl CrateId {
    /// `Std` always has id 0
    pub const STDLIB: CrateId = CrateId(0);

    /// The local crate always has id 1.
    /// It is the root of the crate graph currently being compiled.
    pub const LOCAL: CrateId = CrateId(1);
}

/// A local module id is a hash of the module path from the crate root.
/// Module ids are expected to be unique only within the same crate.
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalModuleId(pub u32);

/// A crate's root module always has ID 0
pub const CRATE_ROOT_MODULE: LocalModuleId = LocalModuleId(0);
