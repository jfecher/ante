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

    #[cfg(feature = "llvm")]
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

/// Link `object_filenames` into a binary. `link_runtime` if set links aminicoro.
pub fn link_with_cc(
    object_filenames: &[String], binary_filename: &str, link_options: &LinkOptions, link_runtime: bool,
) -> bool {
    let output = format!("-o{}", crate::paths::binary_name(binary_filename).display());
    let mut command = Command::new(crate::paths::c_compiler());
    command.args(object_filenames);
    if link_runtime {
        command.arg(aminicoro_path()).arg("-O2");
    }
    // libm and libpthread are part of the C runtime on Windows
    if !cfg!(target_os = "windows") {
        command.arg("-lm").arg("-lpthread");
    }
    command.arg("-w").arg(output);

    for path in &link_options.search_paths {
        command.arg(format!("-L{path}"));
    }
    for lib in &link_options.libs {
        command.arg(format!("-l{lib}"));
    }

    let mut child = command.spawn().unwrap();

    // remove the temporary object files
    let status = child.wait().unwrap();
    for object_filename in object_filenames {
        std::fs::remove_file(object_filename).unwrap();
    }
    status.success()
}
