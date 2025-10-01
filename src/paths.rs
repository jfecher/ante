use std::{path::PathBuf, process::Command};

#[allow(unused)]
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
#[allow(unused)]
pub fn binary_name(module_name: &str) -> String {
    if cfg!(target_os = "windows") {
        PathBuf::from(module_name).with_extension("exe").to_string_lossy().into()
    } else {
        PathBuf::from(module_name).with_extension("").to_string_lossy().into()
    }
}

#[allow(unused)]
fn minicoro_path() -> &'static str {
    match option_env!("ANTE_MINICORO_PATH") {
        Some(path) => path,
        None => panic!("ANTE_MINICORO_PATH is not set"),
    }
}

pub fn stdlib_path() -> PathBuf {
    match option_env!("ANTE_STDLIB_DIR") {
        Some(env) => match std::fs::canonicalize(env) {
            Ok(env) => env,
            Err(_) => panic!("Failed to canonicalize stdlib path {env} ; does it exist?"),
        },
        None => panic!("ANTE_STDLIB_DIR is not set"),
    }
}

pub fn prelude_path() -> PathBuf {
    let mut path = stdlib_path();
    path.push("src");
    path.push("Prelude.an");
    path
}
