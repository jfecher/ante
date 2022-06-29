#![allow(deprecated)]

use std::env;
use std::fs;
use std::path;

fn main() -> std::io::Result<()> {
    // env::home_dir() is deprecated, maybe we should use dirs::home_dir()
    let dot_ante = env::home_dir()
        .expect("failed to get the location of the home dir")
        .to_str()
        .expect("invalid encoding of the home dir name")
        .to_string()
        + "/.ante";
    if !path::Path::new(&dot_ante).exists() {
        fs::create_dir(&dot_ante)
            .expect("failed to create directory \".ante\"");
        fs::create_dir(format!("{dot_ante}/stdlib"))
            .expect("failed to create directory\".ante/stdlib\"");
    }
    println!("cargo:rustc-env=ANTE_STDLIB_DIR={dot_ante}/stdlib");
    let mut stack = vec![path::PathBuf::from("stdlib")];
    while let Some(dir) = stack.pop() {
        for res in fs::read_dir(dir)? {
            let entry = res?;
            let p = entry.path();
            if p.is_dir() {
                let dirname = p.to_str().unwrap();
                if !path::Path::new(&format!("{dot_ante}/{dirname}")).exists() {
                    fs::create_dir(format!("{dot_ante}/{dirname}"))
                        .expect(&format!("failed to create directory \"{dirname}\""));
                }
                stack.push(p);
            } else {
                let filename = p.to_str().unwrap();
                fs::copy(&p, format!("{dot_ante}/{filename}"))
                    .expect(&format!("failed to copy file \"{filename}\""));
            }
        }
    }
    Ok(())
}
