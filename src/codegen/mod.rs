use std::process::Command;

pub mod llvm;

pub fn link_with_gcc(object_filename: &str, binary_filename: &str) {
    // call gcc to compile the bitcode to a binary
    let output = format!("-o{}", binary_filename);
    let mut child = Command::new("gcc")
        .arg(object_filename)
        //.arg(minicoro_path())
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
