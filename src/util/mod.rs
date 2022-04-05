//! util/mod.rs - Various utility functions used throughout the compiler.
//! Mostly consists of convenience functions for iterators such as `fmap`.
use std::{fmt::Display, process::Command, path::PathBuf};

#[macro_use]
pub mod logging;
pub mod timing;
pub mod trustme;

/// Equivalent to .iter().map(f).collect()
pub fn fmap<T, U, F>(iterable: T, f: F) -> Vec<U>
    where 
    T: IntoIterator,
    F: FnMut(T::Item) -> U
{
    iterable.into_iter().map(f).collect()
}

/// What a name! Iterate the array, mapping each element with a function that returns a pair
/// of a value and a vector. Accumulate the results in two separate vectors, the second of
/// which is flattened from all the second-element vectors found so far.
pub fn fmap_mut_pair_flatten_second<T, Ret1, Ret2, F>(array: &mut [T], mut f: F) -> (Vec<Ret1>, Vec<Ret2>)
    where F: FnMut(&mut T) -> (Ret1, Vec<Ret2>)
{
    let mut ret1 = Vec::with_capacity(array.len());
    let mut ret2 = Vec::with_capacity(array.len());
    for elem in array.iter_mut() {
        let (elem1, mut vec) = f(elem);
        ret1.push(elem1);
        ret2.append(&mut vec);
    }
    (ret1, ret2)
}

/// Equivalent to option.as_ref().unwrap().clone()
pub fn unwrap_clone<T: Clone>(option: &Option<T>) -> T {
    option.as_ref().unwrap().clone()
}

/// Convert each element to a String and join them with the given delimiter
pub fn join_with<T: Display>(vec: &[T], delimiter: &str) -> String {
    fmap(vec, |t| format!("{}", t)).join(delimiter)
}

pub fn link(object_filename: &str, binary_filename: &str) {
    // call gcc to compile the bitcode to a binary
    let output = format!("-o{}", binary_filename);
    let mut child = Command::new("gcc")
        .arg(object_filename)
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
        PathBuf::from(module_name)
            .with_extension("exe")
            .to_string_lossy()
            .into()
    } else {
        PathBuf::from(module_name)
            .with_extension("")
            .to_string_lossy()
            .into()
    }
}
