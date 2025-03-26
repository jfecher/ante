use std::env;

fn main() -> std::io::Result<()> {
    if option_env!("ANTE_STDLIB_DIR").is_none() {
        let cur_dir = env::current_dir()?;
        println!("cargo:rustc-env=ANTE_STDLIB_DIR={}/stdlib", cur_dir.to_str().unwrap());
    }
    if option_env!("ANTE_MINICORO_PATH").is_none() {
        let cur_dir = env::current_dir()?;
        println!("cargo:rustc-env=ANTE_MINICORO_PATH={}/minicoro/minicoro.c", cur_dir.to_str().unwrap());
    }
    Ok(())
}
