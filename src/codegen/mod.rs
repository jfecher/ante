use std::process::Command;

use crate::{mir, parser::ids::TopLevelName, paths::aminicoro_path};

pub mod c;
pub mod constant;
#[cfg(feature = "llvm")]
pub mod llvm;

#[derive(Clone, Copy)]
pub(crate) enum OverflowingIntOp {
    Add,
    Sub,
    Mul,
}

impl OverflowingIntOp {
    pub(crate) fn c_builtin_name(self) -> &'static str {
        match self {
            OverflowingIntOp::Add => "__builtin_add_overflow",
            OverflowingIntOp::Sub => "__builtin_sub_overflow",
            OverflowingIntOp::Mul => "__builtin_mul_overflow",
        }
    }

    pub(crate) fn llvm_name_part(self) -> &'static str {
        match self {
            OverflowingIntOp::Add => "add",
            OverflowingIntOp::Sub => "sub",
            OverflowingIntOp::Mul => "mul",
        }
    }
}

/// Resolve which MIR definition is the binary's entry-point
pub(crate) fn resolve_main_id(selected_main: Option<TopLevelName>) -> Option<mir::DefinitionId> {
    selected_main.and_then(|name| mir::builder::lookup_definition_id(&name))
}

/// Native libraries and search paths to link a binary against.
#[derive(Default, Clone)]
pub struct LinkOptions {
    pub libs: Vec<String>,
    pub search_paths: Vec<String>,
}

pub fn link_with_cc(object_filename: &str, binary_filename: &str, link_options: &LinkOptions) -> bool {
    let output = format!("-o{}", binary_filename);
    let mut command = Command::new("cc");
    command.arg(object_filename).arg(aminicoro_path()).arg("-O0").arg("-lm").arg("-w").arg(output);

    for path in &link_options.search_paths {
        command.arg(format!("-L{path}"));
    }
    for lib in &link_options.libs {
        command.arg(format!("-l{lib}"));
    }

    let mut child = command.spawn().unwrap();

    // remove the temporary bitcode file
    let status = child.wait().unwrap();
    std::fs::remove_file(object_filename).unwrap();
    status.success()
}
