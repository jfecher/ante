//! util/mod.rs - Various utility functions used throughout the compiler.
//! Mostly consists of convenience functions for iterators such as `fmap`.
use std::{collections::BTreeSet, fmt::Display, path::PathBuf, process::Command};

#[macro_use]
pub mod logging;
mod id;
pub mod timing;
pub mod trustme;
mod vecmap;

pub use id::Id;
pub use vecmap::VecMap;

/// Equivalent to .iter().map(f).collect()
pub fn fmap<T, U, F>(iterable: T, f: F) -> Vec<U>
where
    T: IntoIterator,
    F: FnMut(T::Item) -> U,
{
    iterable.into_iter().map(f).collect()
}

/// Equivalent to option.as_ref().unwrap().clone()
pub fn unwrap_clone<T: Clone>(option: &Option<T>) -> T {
    option.as_ref().unwrap().clone()
}

/// Convert each element to a String and join them with the given delimiter
pub fn join_with<T: Display>(items: impl IntoIterator<Item = T>, delimiter: &str) -> String {
    fmap(items, |t| format!("{}", t)).join(delimiter)
}

/// Deduplicate the vec without changing the ordering of its elements
pub fn dedup<T: Ord + Copy>(vec: Vec<T>) -> Vec<T> {
    if vec.len() <= 1 {
        vec
    } else {
        let mut seen = BTreeSet::new();
        let mut result = Vec::with_capacity(vec.len());
        for value in vec {
            if !seen.contains(&value) {
                seen.insert(value);
                result.push(value);
            }
        }
        result
    }
}

pub fn link(object_filename: &str, binary_filename: &str) {
    // call gcc to compile the bitcode to a binary
    let output = format!("-o{}", binary_filename);
    let mut child = Command::new("gcc")
        .arg(object_filename)
        .arg(minicoro_path())
        .arg("-Wno-everything")
        .arg("-O0")
        .arg("-lm")
        .arg(output)
        .spawn()
        .unwrap();

    // remove the temporary bitcode file
    child.wait().unwrap();
    std::fs::remove_file(object_filename).unwrap();
}

/// Returns the default name of the outputted binary file
/// as a result of compiling the program with the given entry module.
pub fn binary_name(module_name: &str) -> String {
    if cfg!(target_os = "windows") {
        PathBuf::from(module_name).with_extension("exe").to_string_lossy().into()
    } else {
        PathBuf::from(module_name).with_extension("").to_string_lossy().into()
    }
}

fn minicoro_path() -> &'static str {
    match option_env!("ANTE_MINICORO_PATH") {
        Some(path) => path,
        None => panic!("ANTE_MINICORO_PATH is not set"),
    }
}

pub fn stdlib_dir() -> PathBuf {
    match option_env!("ANTE_STDLIB_DIR") {
        Some(env) => std::fs::canonicalize(env).unwrap(),
        None => panic!("ANTE_STDLIB_DIR is not set"),
    }
}

macro_rules! expect_opt {( $result:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
    match $result {
        Some(t) => t,
        None => panic!($fmt_string $( , $($msg)* )? ),
    }
});}
