use std::path::Path;

/// Expects that the given directory is an existing path
fn rerun_if_stdlib_changes(directory: &Path) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();

        if path.is_dir() {
            rerun_if_stdlib_changes(&path);
        } else {
            // Tell Cargo that if the given file changes, to rerun this build script.
            println!("cargo:rerun-if-changed={}", path.to_string_lossy());
        }
    }
}

fn copy_stdlib(src: &Path, target: &Path) {
    let dest = target.join(src);
    println!("Creating directory {}", dest.to_string_lossy());
    std::fs::create_dir_all(dest).unwrap();

    for entry in std::fs::read_dir(src).unwrap() {
        let path = entry.unwrap().path();
        assert!(path.exists());

        if path.is_dir() {
            rerun_if_stdlib_changes(&path);
        } else {
            let dest = target.join(&path);
            println!("Copying {} to {}", path.to_string_lossy(), dest.to_string_lossy());
            std::fs::copy(path, dest).unwrap();
        }
    }
}

fn main() {
    let stdlib_src_dir = Path::new("stdlib");
    rerun_if_stdlib_changes(stdlib_src_dir);

    let target = dirs::config_dir().unwrap().join("ante");
    copy_stdlib(stdlib_src_dir, &target);
}
