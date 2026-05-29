//! This file contains the bulk of the logic for translating Ante's [mir::Mir] IR
//! into C code which can then be compiled & executed by `cc`. For this to be valid,
//! the input [mir::Mir] must be at the end of its pipeline: generics must be removed
//! via either monomorphization or existentialization, largest union variants must be
//! selected, effects must be lowered, etc. See the various [mir::Mir] passes for details.
//!
//! Creating C output is fairly straightforward:
//! - [Builder::build_definition] is called on each definition in the mir to translate
//!   it into a single c function.
//! - The resulting [CFile] artifact is separated into sections so functions and types
//!   can be declared before their first use.
//! - This pass is parallelized by [build_c_file] very simply: for N workers, each worker
//!   compiles definitions with ids given by `id % n = i` where `i` is the worker index.
//! - The entry function [codegen_c_for_mir] will compile & link the resulting C file.
//!
use std::{borrow::Cow, process::Command};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{cli::OptLevel, mir::{self, DefinitionId}};

mod cfile;
use cfile::CFile;

/// Codegen the given Mir into a single C file, then invoke cc to create
/// a object file and a binary. On success, the object file is removed, but
/// the .c file is kept.
pub(crate) fn codegen_c_for_mir(mir: &mir::Mir, binary_name: &str, opt_level: OptLevel) {
    // Create the C file
    let c_file = build_c_file(mir);
    let c_file_name = format!("{binary_name}.c");
    std::fs::write(&c_file_name, c_file).unwrap();

    // Create the .o file
    let o_file_name = format!("{binary_name}.o");
    let mut child = Command::new("cc")
        .arg(&c_file_name)
        .arg(&format!("-o{o_file_name}"))
        .arg(opt_level.as_cc_opt_string())
        .arg("-c")
        .arg("-w")
        .spawn()
        .unwrap();

    // And link it into a binary
    let status = child.wait().unwrap();
    if status.success() {
        super::link_with_cc(&o_file_name, binary_name);
    }
}

/// The main context struct to build a [CFile] from [mir::Mir]
#[derive(Default)]
struct Builder {
    file: CFile,

    /// The current item being worked on - either a type or a function.
    /// This will be appended to the appropriate position in `file` when finished.
    current_item: String,
}

/// Builds a C File for the given [mir::Mir] in-memory. Returns the file contents
fn build_c_file(mir: &mir::Mir) -> String {
    // Split Mir definitions into N groups and compile in parallel.
    // Each worker `i` compiles definitions with id `Id % N = i`
    let n = rayon::current_num_threads() as u32;

    (0..n)
        .into_par_iter()
        .map(|i| c_file_with_definitions_subset(mir, n, i))
        .reduce(CFile::default, CFile::extend)
        .into_contents()
}

/// Create a C file with only definitions of the mir with ids such that `id % n = i`.
/// This is meant to distribute work over `n` workers evenly.
fn c_file_with_definitions_subset(mir: &mir::Mir, n: u32, i: u32) -> CFile {
    let mut builder = Builder::default();

    mir.definitions
        .iter()
        .filter(|(id, _)| id.0 % n == i)
        .for_each(|(_id, definition)| builder.build_definition(definition, mir));

    mir.externals
        .iter()
        .filter(|(id, _)| id.0 % n == i)
        .for_each(|(_id, external)| builder.build_external(external, mir));

    builder.file
}

impl Builder {
    /// Push the given string to `self.current_item`
    fn write(&mut self, s: &str) {
        self.current_item += s;
    }

    /// Build the given definition, adding it as a translating C function when finished
    fn build_definition(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        self.build_fn_signature(definition, mir);

        self.write("{");
        self.build_fn_body(definition);
        self.write("}");
    }

    /// Declare the given item
    fn build_external(&mut self, external: &mir::Extern, mir: &mir::Mir) {
        todo!()
    }

    /// Write a mangled name to `self.current_item`
    fn write_mangled_name(&mut self, name: &str, id: DefinitionId) {
        self.write(name);
        self.write("_");
        self.write(&id.0.to_string());
    }

    fn write_value(&mut self, value: &mir::Value, mir: &mir::Mir) {
        let s = match value {
            mir::Value::Error => unreachable!("Error value found in C codegen"),
            mir::Value::Unit => Cow::Borrowed("(Unit){}"),
            mir::Value::Bool(true) => Cow::Borrowed("true"),
            mir::Value::Bool(false) => Cow::Borrowed("false"),
            mir::Value::Char(_) => todo!(),
            mir::Value::Integer(int) => Cow::Owned(int.to_string()), // TODO: Incorrect suffixes. Should be e.g. `(int16_t) x`
            mir::Value::Float(float) => Cow::Owned(float.to_string()),
            mir::Value::InstructionResult(id) => Cow::Owned(id.to_string()),
            mir::Value::Parameter(block, i) => Cow::Owned(format!("{block}_{i}")),
            mir::Value::Definition(id) => {
                let name = mir.get_name(*id).unwrap();
                // This must match the mangling in [Self::write_mangled_name]
                Cow::Owned(format!("{name}_{}", id.0))
            },
        };
        self.write(&s);
    }

    /// Build the function's signature in `self.current_item` and also push it as a
    /// function declaration.
    fn build_fn_signature(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        // write `ret_t foo(t0 arg0, ..., tN argN);`
        let mir::Type::Function(function_type) = &definition.typ else {
            panic!("Definition is not a function")
        };
        self.write_type(&function_type.return_type);

        self.write(" ");
        self.write_mangled_name(&definition.name, definition.id);
        self.write("(");

        for (parameter, typ) in definition.parameters() {
            self.write_value(&parameter, mir);
            self.write(" ");
            self.write_type(typ);
        }

        self.write(");");

        // `ret_t foo(t0 arg0, ..., tN argN);` written. Forward-declare it then pop the `;`
        self.file.add_function_declaration(&self.current_item);
        self.current_item.pop(); // ;
        self.write(" ");
    }

    /// Write the given C type to `self.current_item`
    fn write_type(&mut self, typ: &mir::Type) {
        todo!()
    }

    /// Return the given type as a String holding a C representation of the type
    fn type_string(&mut self, typ: &mir::Type) -> String {
        todo!()
    }

    /// Iterate over each block and each instruction, inserting them into the function.
    fn build_fn_body(&mut self, definition: &mir::Definition) {
        for block_id in definition.topological_sort() {
            let block = &definition.blocks[block_id];
        }
    }
}
