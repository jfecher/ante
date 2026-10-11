use std::path::PathBuf;

/// Returns the default name of the outputted binary file
/// as a result of compiling the program with the given entry module.
pub fn binary_name(module_name: &str) -> PathBuf {
    let path = if cfg!(target_os = "windows") {
        PathBuf::from(module_name).with_extension("exe")
    } else {
        PathBuf::from(module_name).with_extension("")
    };

    std::fs::canonicalize(&path).unwrap_or(path)
}

/// The path to `aminicoro`, the runtime library for coroutines
/// which effects are currently lowered into.
pub fn aminicoro_path() -> PathBuf {
    let object = stdlib_path().join("minicoro.o");
    if object.is_file() {
        return object;
    }

    match option_env!("ANTE_MINICORO_PATH") {
        Some(path) => PathBuf::from(path),
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

/// Returns "Prelude.an"
pub fn prelude_path_relative_to_stdlib_source_folder() -> &'static std::path::Path {
    std::path::Path::new("Prelude.an")
}
