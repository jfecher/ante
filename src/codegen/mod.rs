use std::process::Command;

use crate::{mir, parser::ids::TopLevelName, paths::aminicoro_path};

pub mod c;
pub mod constant;
#[cfg(feature = "llvm")]
pub mod llvm;

/// Resolve which MIR definition is the binary's entry-point
pub(crate) fn resolve_main_id(selected_main: Option<TopLevelName>) -> Option<mir::DefinitionId> {
    selected_main.and_then(|name| mir::builder::lookup_definition_id(&name))
}

pub fn link_with_cc(object_filename: &str, binary_filename: &str) -> bool {
    let output = format!("-o{}", binary_filename);
    let mut child = Command::new("cc")
        .arg(object_filename)
        .arg(aminicoro_path())
        .arg("-O0")
        .arg("-lm")
        .arg("-w")
        .arg(output)
        .spawn()
        .unwrap();

    // remove the temporary bitcode file
    let status = child.wait().unwrap();
    std::fs::remove_file(object_filename).unwrap();
    status.success()
}
