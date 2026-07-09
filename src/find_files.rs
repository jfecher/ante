use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::builder::OsStr;
use rustc_hash::FxHashSet;

use crate::{
    dependencies::DEPENDENCIES_DIR_NAME,
    files::read_file,
    incremental::{Crate, Db, GetCrateGraph, SourceFile},
    manifest::{MANIFEST_FILE_NAME, Manifest},
    name_resolution::namespace::{CrateId, SourceFileId},
    paths::stdlib_path,
};

pub type CrateGraph = BTreeMap<CrateId, Crate>;

/// Default name of the local crate when its `ante.toml` has no `name` field.
pub const DEFAULT_LOCAL_CRATE_NAME: &str = "Local";

/// Default name of the src folder.
pub(crate) const SRC_FOLDER: &str = "src";
pub(crate) const MAIN_FILE: &str = "main.an";

/// Search the current directory and its ancestors for an Ante manifest.
pub fn find_nearest_project_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;

    loop {
        if dir.join(MANIFEST_FILE_NAME).is_file() {
            return Some(dir);
        }

        if !dir.pop() {
            return None;
        }
    }
}

pub fn find_project_main_file() -> Option<PathBuf> {
    let root = find_nearest_project_root()?;
    Some(root.join(SRC_FOLDER).join(MAIN_FILE))
}

// TODO:
// - Error for cyclic dependencies
// - Handle crate versions
/// Scans the file system for all crates used and populates the Db with their source files.
///
/// `local_crate_root` is the directory whose `src/` subdirectory holds the local crate's
/// sources. It is used to discover files and to normalize `starting_files` paths for [SourceFileId]s.
pub fn populate_crates_and_files(compiler: &mut Db, local_crate_root: &std::path::Path, starting_files: &[PathBuf]) {
    // We must collect all crates and their source files first in this crate graph first
    // before setting them in the Db at the end. If we set them before finding their source
    // files we'd need to needlessly clone them and update the Db twice instead of once.
    let mut crates = CrateGraph::default();

    add_stdlib_crate(&mut crates);
    populate_local_crate_with_starting_files(compiler, &mut crates, local_crate_root, starting_files);

    let mut stack = vec![CrateId::LOCAL, CrateId::STDLIB];
    let mut finished = FxHashSet::default();
    finished.insert(CrateId::STDLIB);

    while let Some(crate_id) = stack.pop() {
        add_source_files_of_crate(compiler, &mut crates, crate_id);

        let dependencies = find_crate_dependencies(&mut crates, crate_id);
        for dependency in &dependencies {
            if finished.insert(*dependency) {
                stack.push(*dependency);
            }
        }

        crates.get_mut(&crate_id).unwrap().dependencies = dependencies;
    }

    set_crate_inputs(compiler, crates);
}

/// The Prelude should always be at a known ID for every file to import it implicitly.
/// We manually populate the stdlib crate with the prelude here to ensure it has that ID.
fn add_stdlib_crate(crates: &mut CrateGraph) {
    crates.insert(CrateId::STDLIB, Crate::new("Std".to_string(), stdlib_path()));
}

/// Create the local crate's Crate entry in the graph and populate it with the given starting files.
fn populate_local_crate_with_starting_files(
    compiler: &mut Db, crates: &mut CrateGraph, local_crate_root: &std::path::Path, starting_files: &[PathBuf],
) {
    let mut source_files = BTreeMap::new();

    for path in starting_files {
        let relative_path = SourceFileId::normalize_path(local_crate_root, path).to_path_buf();
        let id = SourceFileId::new(CrateId::LOCAL, &relative_path);
        let data = read_file_data(path.to_path_buf());
        id.set(compiler, Arc::new(data));
        source_files.insert(Arc::new(relative_path), id);
    }

    let mut crate_ = Crate::new(DEFAULT_LOCAL_CRATE_NAME.to_string(), local_crate_root.to_path_buf());
    crate_.source_files = source_files;
    if let Some(manifest) = Manifest::read(local_crate_root) {
        manifest.apply(&mut crate_);
    }
    crates.insert(CrateId::LOCAL, crate_);
}

/// Set each CrateId -> Crate mapping as an input to the Db
/// We can only do this once each crate's source files have been collected.
fn set_crate_inputs(compiler: &mut Db, crates: CrateGraph) {
    GetCrateGraph.set(compiler, Arc::new(crates));
}

/// Find all Ante source files in the given crate. Currently this is hard coded
/// to only look in the `src` directory.
fn add_source_files_of_crate(compiler: &mut Db, crates: &mut CrateGraph, crate_id: CrateId) {
    let crate_root = crates[&crate_id].path.clone();
    let mut src_folder = crate_root.clone();
    src_folder.push(SRC_FOLDER);

    let mut remaining = vec![src_folder.clone()];
    let mut source_files = BTreeMap::new();

    while let Some(current_dir) = remaining.pop() {
        // We should error in the future when failing to read a directory but for now we want to
        // allow either the local crate or the stdlib to not be present and still compile when
        // we're only working on a single source file. We may want to separate the compile mode
        // more explicitly in the CLI in the future.
        let Ok(dir) = current_dir.read_dir() else {
            continue;
        };

        for file in dir.flatten() {
            let path = file.path();

            if path.is_dir() {
                remaining.push(path);
            } else if path.extension() == Some(&OsStr::from("an")) {
                // All paths are relative to the `src` directory so that they
                // can be reconstructed from only the module names. E.g. `Foo.Bar` = `foo_crate_root/src/bar.an`
                let src_relative = path.strip_prefix(&src_folder).unwrap_or(&path);
                let id = SourceFileId::new(crate_id, src_relative);
                let abs_path =
                    path.canonicalize().unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join(&path));
                let data = read_file_data(abs_path);
                id.set(compiler, Arc::new(data));
                source_files.insert(Arc::new(src_relative.to_path_buf()), id);
            }
        }
    }

    register_directory_modules(compiler, crate_id, &mut source_files);

    // `extend` instead of setting it in case this is LOCAL_CRATE and `populate_local_crate_with_starting_files`
    // populated it with an initial set of files manually specified by the user.
    crates.get_mut(&crate_id).unwrap().source_files.extend(source_files);
}

/// Register a synthetic module for every subdirectory so nested paths like `Crate.Dir.Module`
/// resolve. Each directory module holds a `submodules` map of its direct children.
fn register_directory_modules(
    compiler: &mut Db, crate_id: CrateId, source_files: &mut BTreeMap<Arc<PathBuf>, SourceFileId>,
) {
    // Map each directory (relative to `src`, empty path = crate root) to its direct children.
    let mut dir_children: BTreeMap<PathBuf, BTreeMap<String, SourceFileId>> = BTreeMap::new();

    let files: Vec<(PathBuf, SourceFileId)> = source_files.iter().map(|(path, id)| ((**path).clone(), *id)).collect();

    for (path, id) in &files {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let mut dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        dir_children.entry(dir.clone()).or_default().insert(name, *id);

        // Chain each ancestor directory up to its own parent as a submodule.
        while !dir.as_os_str().is_empty() {
            let parent = dir.parent().map(Path::to_path_buf).unwrap_or_default();
            let dir_name = dir.file_name().unwrap().to_string_lossy().into_owned();
            let dir_id = SourceFileId::new(crate_id, &dir);
            dir_children.entry(parent.clone()).or_default().insert(dir_name, dir_id);
            dir = parent;
        }
    }

    for (dir, children) in dir_children {
        if dir.as_os_str().is_empty() {
            continue; // crate root is resolved specially, it has no backing SourceFile
        }
        let dir_id = SourceFileId::new(crate_id, &dir);
        let mut source_file = SourceFile::new(Arc::new(dir.clone()), String::new());
        source_file.submodules = children;
        dir_id.set(compiler, Arc::new(source_file));
        source_files.insert(Arc::new(dir), dir_id);
    }
}

fn read_file_data(file: PathBuf) -> SourceFile {
    let file = Arc::new(file);
    let text = match read_file(&file) {
        Ok(text) => text,
        Err(_) => {
            // A proper Diagnostic here would be better but there is no source location to use.
            eprintln!("warning: failed to read file {}", file.display());
            String::new()
        },
    };
    SourceFile::new(file, text)
}

/// TODO: This creates a new dependency for each dependency found.
/// It never checks for duplicates.
fn find_crate_dependencies(crates: &mut CrateGraph, crate_id: CrateId) -> Vec<CrateId> {
    let mut deps_folder = crates[&crate_id].path.clone();
    deps_folder.push(DEPENDENCIES_DIR_NAME);

    // Every crate currently depends on the stdlib
    let mut dependencies = vec![CrateId::STDLIB];

    // We should error in the future when failing to read a directory but for now we want to
    // allow either the local crate or the stdlib to not be present and still compile when
    // we're only working on a single source file. We may want to separate the compile mode
    // more explicitly in the CLI in the future.
    let Ok(deps_folder) = deps_folder.read_dir() else {
        return dependencies;
    };

    // Push every crate in the dependencies directory as a new direct dependency.
    // Transitive dependencies are discovered by `populate_crates_and_files` when these crates
    // are later processed from its stack.
    for dependency in deps_folder.flatten() {
        let path = dependency.path();
        if path.is_dir() {
            let manifest = Manifest::read(&path).unwrap_or_default();
            let fallback_name = path.file_name().unwrap().to_string_lossy().into_owned();

            // `apply` overrides the fallback directory name with the manifest name if present.
            let mut dependency = Crate::new(fallback_name, path);
            manifest.apply(&mut dependency);

            let id = new_crate_id(crates, &dependency.name, 0);
            dependencies.push(id);
            crates.insert(id, dependency);
        }
    }

    dependencies
}

/// Create a new unique CrateId from the crate's name and version
fn new_crate_id(crates: &CrateGraph, name: &String, version: u32) -> CrateId {
    for collisions in 0.. {
        let hash = crate::parser::ids::hash((name, version, collisions));
        let id = CrateId(hash as u32);

        if !crates.contains_key(&id) {
            return id;
        }
    }
    unreachable!("We have somehow had i32::MAX hash collisions")
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let mut path = std::env::temp_dir();
            path.push(format!("ante-{name}-{}-{}", std::process::id(), crate::parser::ids::hash(name)));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            TestDir { path }
        }

        fn create_dependency(&self, dirname: &str, manifest_name: Option<&str>) -> PathBuf {
            let path = self.path.join(DEPENDENCIES_DIR_NAME).join(dirname);
            std::fs::create_dir_all(&path).unwrap();
            if let Some(name) = manifest_name {
                std::fs::write(path.join(MANIFEST_FILE_NAME), format!("name = \"{name}\"\n")).unwrap();
            }
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn local_crate_graph(path: PathBuf) -> CrateGraph {
        let mut crates = CrateGraph::default();
        crates.insert(CrateId::LOCAL, Crate::new(DEFAULT_LOCAL_CRATE_NAME.to_string(), path));
        crates
    }

    #[test]
    fn missing_dependencies_directory_only_depends_on_stdlib() {
        let dir = TestDir::new("missing-deps");
        let mut crates = local_crate_graph(dir.path.clone());

        let dependencies = find_crate_dependencies(&mut crates, CrateId::LOCAL);

        assert_eq!(dependencies, vec![CrateId::STDLIB]);
        assert_eq!(crates.len(), 1);
    }

    #[test]
    fn discovers_direct_dependencies_from_dependencies_directory() {
        let dir = TestDir::new("direct-deps");
        let alpha_path = dir.create_dependency("alpha", Some("Alpha"));
        let beta_path = dir.create_dependency("beta", None);
        let mut crates = local_crate_graph(dir.path.clone());

        let dependencies = find_crate_dependencies(&mut crates, CrateId::LOCAL);

        assert_eq!(dependencies.len(), 3);
        assert_eq!(dependencies[0], CrateId::STDLIB);

        let dependency_crates = dependencies
            .iter()
            .skip(1)
            .map(|id| crates.get(id).unwrap())
            .map(|crate_| (crate_.name.as_str(), crate_.path.as_path()))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(dependency_crates["Alpha"], alpha_path.as_path());
        assert_eq!(dependency_crates["beta"], beta_path.as_path());
    }

    #[test]
    fn transitive_dependencies_are_not_returned_as_direct_dependencies() {
        let dir = TestDir::new("transitive-deps");
        let alpha_path = dir.create_dependency("alpha", Some("Alpha"));
        let nested_path = alpha_path.join(DEPENDENCIES_DIR_NAME).join("nested");
        std::fs::create_dir_all(&nested_path).unwrap();
        std::fs::write(nested_path.join(MANIFEST_FILE_NAME), "name = \"Nested\"\n").unwrap();
        let mut crates = local_crate_graph(dir.path.clone());

        let dependencies = find_crate_dependencies(&mut crates, CrateId::LOCAL);

        assert_eq!(dependencies.len(), 2);
        let alpha = crates.get(&dependencies[1]).unwrap();
        assert_eq!(alpha.name, "Alpha");
        assert_eq!(alpha.path, alpha_path);
        assert!(!crates.values().any(|crate_| crate_.name == "Nested"));
    }
}
