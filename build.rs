use std::env;

fn main() -> std::io::Result<()> {
    if option_env!("ANTE_STDLIB_DIR").is_none() {
        let cur_dir = env::current_dir()?;
        println!("cargo:rustc-env=ANTE_STDLIB_DIR={}/stdlib", cur_dir.to_str().unwrap());
    }
    Ok(())
}
