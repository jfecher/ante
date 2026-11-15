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

/// The C compiler used to compile C output and link binaries. Uses `$CC` if set, otherwise
/// `clang` on Windows and `cc` elsewhere.
pub fn c_compiler() -> String {
    match std::env::var("CC") {
        Ok(cc) if !cc.is_empty() => cc,
        _ if cfg!(target_os = "windows") => "clang".to_string(),
        _ => "cc".to_string(),
    }
}

/// The path to `aminicoro`, the runtime library for coroutines
/// which effects are currently lowered into.
pub fn aminicoro_path() -> PathBuf {
    let object = stdlib_path().join("minicoro.o");
    if object.is_file() {
        return object;
    }

    match std::env::var_os("ANTE_MINICORO_PATH") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env!("ANTE_MINICORO_PATH")),
    }
}

/// The path to `aminicoro` compiled to LLVM bitcode. If `clang` wasn't found when the compiler
/// was built, this will not exist.
pub fn aminicoro_bitcode_path() -> Option<PathBuf> {
    Some(stdlib_path().join("minicoro.bc")).filter(|path| path.is_file())
}

pub fn stdlib_path() -> PathBuf {
    let raw_path = std::env::var_os("ANTE_STDLIB_DIR").unwrap_or(env!("ANTE_STDLIB_DIR").into());
    std::fs::canonicalize(&raw_path).unwrap_or_else(|err| {
        let raw_path = raw_path.display();
        panic!("Couldn't canonicalize stdlib path {raw_path} : {err}");
    })
}

/// Returns "Prelude.an"
pub fn prelude_path_relative_to_stdlib_source_folder() -> &'static std::path::Path {
    std::path::Path::new("Prelude.an")
}
