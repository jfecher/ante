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

    // If this fails, the linker compiles aminicoro from source for each program instead
    let cc = match env::var("CC") {
        Ok(cc) if !cc.is_empty() => cc,
        _ if env::var("CARGO_CFG_WINDOWS").is_ok() => "clang".to_string(),
        _ => "cc".to_string(),
    };
    println!("cargo:rerun-if-env-changed=CC");
    compile_into_place(Command::new(cc).arg("-c").arg(source).args(["-O2", "-w"]), &stdlib_dir.join("minicoro.o"));

    // The LLVM backend falls back to `minicoro.o` if this is missing or unreadable.
    compile_into_place(
        Command::new("clang").arg("-c").arg("-emit-llvm").arg(source).args(["-O2", "-w", "-fno-stack-protector"]),
        &stdlib_dir.join("minicoro.bc"),
    );
}

/// Runs `command -o <temporary>`, then renames the result to `output` so a concurrent link can't
/// read a partial file. On failure, `output` is removed.
fn compile_into_place(command: &mut Command, output: &Path) {
    let mut temporary = output.as_os_str().to_owned();
    temporary.push(format!(".tmp{}", std::process::id()));
    let temporary = PathBuf::from(temporary);

    let status = command.arg("-o").arg(&temporary).status();
    if status.is_ok_and(|status| status.success()) && std::fs::rename(&temporary, output).is_ok() {
        return;
    }

    let _ = std::fs::remove_file(&temporary);
    let _ = std::fs::remove_file(output);
}
