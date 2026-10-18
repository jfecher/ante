use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> std::io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    let stdlib_dir = match option_env!("ANTE_STDLIB_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let cur_dir = env::current_dir()?;
            println!("cargo:rustc-env=ANTE_STDLIB_DIR={}/stdlib", cur_dir.to_str().unwrap());
            cur_dir.join("stdlib")
        },
    };

    let minicoro_source = match option_env!("ANTE_MINICORO_PATH") {
        Some(path) => PathBuf::from(path),
        None => {
            let cur_dir = env::current_dir()?;
            println!("cargo:rustc-env=ANTE_MINICORO_PATH={}/aminicoro/minicoro.c", cur_dir.to_str().unwrap());
            cur_dir.join("aminicoro/minicoro.c")
        },
    };

    precompile_minicoro(&minicoro_source, &stdlib_dir);
    Ok(())
}

/// Compile the runtime once here to improve compile time of each individual program.
fn precompile_minicoro(source: &Path, stdlib_dir: &Path) {
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed={}", source.with_file_name("minicoro.h").display());

    // Renamed into place so a concurrent link never reads a partial object
    let object = stdlib_dir.join("minicoro.o");
    let temporary = stdlib_dir.join(format!("minicoro.o.tmp{}", std::process::id()));
    let status = Command::new("cc").arg("-c").arg(source).args(["-O0", "-w", "-o"]).arg(&temporary).status();
    if status.is_ok_and(|status| status.success()) && std::fs::rename(&temporary, &object).is_ok() {
        return;
    }

    // precompile failed, remove the files. We'll just recompile aminicoro at runtime
    let _ = std::fs::remove_file(&temporary);
    let _ = std::fs::remove_file(&object);
}
