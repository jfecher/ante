use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use clap::builder::OsStr;
use rustc_hash::FxHashSet;

use crate::{
    incremental::{Crate, Db, GetCrateGraph, SourceFile},
    name_resolution::namespace::{CrateId, SourceFileId, LOCAL_CRATE, STDLIB_CRATE},
    read_file,
};

const STDLIB_PATH: &str = "stdlib";

pub type CrateGraph = BTreeMap<CrateId, Crate>;

// TODO:
// - Error for cyclic dependencies
// - Handle crate versions
/// Scans the file system for all crates used and populates the Db with their source files
pub fn populate_crates_and_files(compiler: &mut Db, starting_files: &[PathBuf]) {
    // We must collect all crates and their source files first in this crate graph first
    // before setting them in the Db at the end. If we set them before finding their source
    // files we'd need to needlessly clone them and update the Db twice instead of once.
    let mut crates = CrateGraph::default();
    crates.insert(STDLIB_CRATE, Crate::new("Std".to_string(), PathBuf::from(STDLIB_PATH)));

    populate_local_crate_with_starting_files(compiler, &mut crates, starting_files);

    let mut stack = vec![LOCAL_CRATE, STDLIB_CRATE];
    let mut finished = FxHashSet::default();
    finished.insert(STDLIB_CRATE);

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

/// Create the local crate's Crate entry in the graph and populate it with the given starting files.
fn populate_local_crate_with_starting_files(compiler: &mut Db, crates: &mut CrateGraph, starting_files: &[PathBuf]) {
    let mut source_files = BTreeMap::new();

    for path in starting_files {
        let path = path.to_path_buf();
        let id = SourceFileId::new(LOCAL_CRATE, &path);
        let data = read_file_data(path.clone());
        id.set(compiler, Arc::new(data));
        source_files.insert(Arc::new(path), id);
    }

    // TODO: track name for local crate. Currently we only compile single source files
    // but have the infrastructure here to collect source files of crates and their dependencies.
    // We're only missing CLI options.
    let crate_ = Crate { name: "Local".to_string(), path: PathBuf::from("."), dependencies: Vec::new(), source_files };
    crates.insert(LOCAL_CRATE, crate_);
}

/// Set each CrateId -> Crate mapping as an input to the Db
/// We can only do this once each crate's source files have been collected.
fn set_crate_inputs(compiler: &mut Db, crates: CrateGraph) {
    GetCrateGraph.set(compiler, Arc::new(crates));
}

/// Find all Ante source files in the given crate. Currently this is hard coded
/// to only look in the `src` directory.
fn add_source_files_of_crate(compiler: &mut Db, crates: &mut CrateGraph, crate_id: CrateId) {
    let mut src_folder = crates[&crate_id].path.clone();
    src_folder.push("src");

    let mut remaining = vec![src_folder];
    let mut source_files = BTreeMap::new();

    // Push every crate in the `deps` folder as a new crate
    while let Some(src_folder) = remaining.pop() {
        // We should error in the future when failing to read a directory but for now we want to
        // allow either the local crate or the stdlib to not be present and still compile when
        // we're only working on a single source file. We may want to separate the compile mode
        // more explicitly in the CLI in the future.
        let Ok(src_folder) = src_folder.read_dir() else {
            continue;
        };

        for file in src_folder.flatten() {
            let path = file.path();
            if path.is_dir() {
                remaining.push(path);
            } else if path.extension() == Some(&OsStr::from("an")) {
                let id = SourceFileId::new(crate_id, &path);
                let data = read_file_data(path.clone());
                id.set(compiler, Arc::new(data));
                let path = Arc::new(path);
                source_files.insert(path, id);
            }
        }
    }

    // `extend` instead of setting it in case this is LOCAL_CRATE and `populate_local_crate_with_starting_files`
    // populated it with an initial set of files manually specified by the user.
    crates.get_mut(&crate_id).unwrap().source_files.extend(source_files);
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
    deps_folder.push("deps");

    // Every crate currently depends on the stdlib
    let mut dependencies = vec![STDLIB_CRATE];
    let mut remaining = vec![deps_folder];

    // Push every crate in the `deps` folder as a new crate
    while let Some(deps_folder) = remaining.pop() {
        // We should error in the future when failing to read a directory but for now we want to
        // allow either the local crate or the stdlib to not be present and still compile when
        // we're only working on a single source file. We may want to separate the compile mode
        // more explicitly in the CLI in the future.
        let Ok(deps_folder) = deps_folder.read_dir() else {
            continue;
        };

        for dependency in deps_folder.flatten() {
            let path = dependency.path();
            if path.is_dir() {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                let id = new_crate_id(crates, &name, 0);
                dependencies.push(id);

                crates.insert(id, Crate::new(name, path));
            }
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
