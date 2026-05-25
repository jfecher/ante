use std::process::Command;

use crate::paths::aminicoro_path;

pub mod llvm;

pub fn link_with_cc(object_filename: &str, binary_filename: &str) -> bool {
    let output = format!("-o{}", binary_filename);
    let mut child = Command::new("cc")
        .arg(object_filename)
        .arg(aminicoro_path())
        .arg("-Wno-everything")
        .arg("-O0")
        .arg("-lm")
        .arg(output)
        .spawn()
        .unwrap();

    // remove the temporary bitcode file
    let status = child.wait().unwrap();
    std::fs::remove_file(object_filename).unwrap();
    status.success()
}
