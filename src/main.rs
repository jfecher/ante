//! Welcome to this repository! You're in the entry point to the program where we handle
//! command-line arguments and invoke the rest of the compiler.
//!
//! Compared to a traditional pipeline-style compiler, the main difference in architecture
//! of this compiler comes from it being pull-based rather than push-based. So instead of
//! starting by lexing everything, then parsing, name resolution, type inference, etc.,
//! we start by saying "I want a compiled program!" Then the function to get us a compiled
//! program says "well, I need a type-checked Ast for that." Then our type inference pass
//! says "I need a name-resolved ast," and so on. So this compiler still has the same
//! passes you know and love (and listed further down), they're just composed together a
//! bit differently.
//!
//! List of compiler passes and the source file to find more about them in:
//! - Lexing `src/lexer/mod.rs`
//! - Parsing `src/parser/mod.rs`
//! - Name Resolution `src/name_resolution/mod.rs`
//! - Type Inference `src/type_inference/cst_traversal.rs`
//! - MIR Translation `src/mir/builder.rs`
//!
//! Non-passes:
//! - `src/errors.rs`: Defines each error used in the program as well as the `Location` struct
//! - `src/incremental.rs`: Some plumbing for the inc-complete library which also defines
//!   which functions we're caching the result of.
#![allow(mismatched_lifetime_syntaxes)]

use clap::{CommandFactory, Parser};
use cli::{Cli, Completions};
use colored::Colorize;
use diagnostics::Diagnostic;
use inc_complete::{Computation, StorageFor};
use incremental::{Db, GetCrateGraph, Parse, Resolve};
use name_resolution::namespace::{CrateId, LocalModuleId, SourceFileId};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    cli::EmitTarget,
    codegen::llvm::CodegenLlvmResult,
    diagnostics::DiagnosticKind,
    files::{make_compiler, write_metadata},
    incremental::{CodegenLlvm, DbStorage, TypeCheck},
};

// All the compiler passes:
// (listed out of order because `cargo fmt` alphabetizes them)
mod codegen;
mod definition_collection;
mod find_files;
mod lexer;
mod mir;
mod name_resolution;
mod parser;
mod type_inference;

// Util modules:
mod cli;
mod diagnostics;
mod files;
mod incremental;
mod iterator_extensions;
mod paths;
mod vecmap;

fn main() {
    if let Ok(Completions { shell_completion }) = Completions::try_parse() {
        let mut cmd = Cli::command();
        let name = cmd.get_name().to_string();
        clap_complete::generate(shell_completion, &mut cmd, name, &mut std::io::stdout());
    } else {
        compile(Cli::parse())
    }
}

fn compile(args: Cli) {
    let (compiler, metadata_file) = make_compiler(&args.files, args.incremental);

    let diagnostics = match args.emit {
        Some(EmitTarget::Tokens) => {
            display_tokens(&compiler);
            BTreeSet::new()
        },
        Some(EmitTarget::Ast) => display_parse_tree(&compiler),
        Some(EmitTarget::AstR) => display_name_resolution(&compiler),
        Some(EmitTarget::AstT) => display_type_checking(&compiler, true),
        Some(EmitTarget::Mir) => display_mir(&compiler),
        Some(EmitTarget::Ir) => llvm_codegen_separate(&compiler, true).2,
        None => llvm_codegen_all(&compiler, &args.files),
    };

    display_diagnostics(diagnostics, &compiler);

    if let Some(metadata_file) = metadata_file {
        if let Err(error) = write_metadata(&compiler, &metadata_file) {
            eprintln!("\n{error}");
        }
    }
}

/// Returns a pair of (error count, warning count)
fn classify_diagnostics(diagnostics: &BTreeSet<Diagnostic>) -> (usize, usize) {
    let mut error_count = 0;
    let mut warning_count = 0;
    for diagnostic in diagnostics {
        match diagnostic.kind() {
            DiagnosticKind::Error => error_count += 1,
            DiagnosticKind::Warning => warning_count += 1,
            DiagnosticKind::Note => (),
        }
    }
    (error_count, warning_count)
}

fn display_diagnostics(diagnostics: BTreeSet<Diagnostic>, compiler: &Db) {
    let (error_count, warning_count) = classify_diagnostics(&diagnostics);
    for diganostic in diagnostics {
        eprintln!("{}", diganostic.display(true, &compiler));
    }

    if error_count != 0 {
        let error_s = if error_count == 1 { "" } else { "s" };
        let errors = format!("{error_count} error{error_s}").red();

        let warning_s = if warning_count == 1 { "" } else { "s" };
        let warnings = format!("{warning_count} warning{warning_s}");

        if warning_count == 0 {
            eprintln!("Found {errors} and {warnings}");
        } else {
            eprintln!("Found {errors} and {}", warnings.yellow());
        }
    } else if warning_count != 0 {
        let warning_s = if warning_count == 1 { "" } else { "s" };
        let warnings = format!("{warning_count} warning{warning_s}");
        eprintln!("Compiled with {}", warnings.yellow());
    }
}

fn display_tokens(compiler: &Db) {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&CrateId::LOCAL];

    for file_id in local_crate.source_files.values() {
        let file = file_id.get(compiler);
        let tokens = lexer::Lexer::new(&file.contents).collect::<Vec<_>>();
        for (token, _) in tokens {
            println!("{token}");
        }
    }
}

fn display_parse_tree(compiler: &Db) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&CrateId::LOCAL];
    let mut diagnostics = BTreeSet::new();

    for file in local_crate.source_files.values() {
        let result = Parse(*file).get(compiler);
        println!("{}", result.cst.display(&result.top_level_data));

        let parse_diagnostics: BTreeSet<_> = compiler.get_accumulated(Parse(*file));
        diagnostics.extend(parse_diagnostics);
    }
    diagnostics
}

fn display_name_resolution(compiler: &Db) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&CrateId::LOCAL];
    let mut diagnostics = BTreeSet::new();

    for file in local_crate.source_files.values() {
        let parse = Parse(*file).get(compiler);

        for item in &parse.cst.top_level_items {
            let resolve_diagnostics: BTreeSet<_> = compiler.get_accumulated(Resolve(item.id));
            diagnostics.extend(resolve_diagnostics);
        }

        println!("{}", parse.cst.display_resolved(&parse.top_level_data, compiler))
    }
    diagnostics
}

fn display_type_checking(compiler: &Db, show_types: bool) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&CrateId::LOCAL];
    let mut diagnostics = BTreeSet::new();

    for file in local_crate.source_files.values() {
        let parse = Parse(*file).get(compiler);

        for item in &parse.cst.top_level_items {
            let more_diagnostics: BTreeSet<_> = compiler.get_accumulated(TypeCheck(item.id));
            diagnostics.extend(more_diagnostics);
        }

        if show_types {
            println!("{}", parse.cst.display_typed(&parse.top_level_data, compiler))
        }
    }
    diagnostics
}

fn display_mir(compiler: &Db) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&CrateId::LOCAL];
    let mut diagnostics = BTreeSet::new();

    for file in local_crate.source_files.values() {
        let parse = Parse(*file).get(compiler);

        for item in &parse.cst.top_level_items {
            let mir = mir::builder::build_initial_mir(compiler, item.id);
            if let Some(mir) = mir {
                for definition in mir.definitions.into_values() {
                    println!("{definition}\n");
                }
            }
            let more_diagnostics: BTreeSet<_> = compiler.get_accumulated(TypeCheck(item.id));
            diagnostics.extend(more_diagnostics);
        }
    }
    diagnostics
}

/// Codegen each item as a separate llvm module
/// Returns (module strings, true if there are any errors, diagnostics)
fn llvm_codegen_separate(compiler: &Db, display_ir: bool) -> (Vec<Arc<Vec<u8>>>, bool, BTreeSet<Diagnostic>) {
    let crates = GetCrateGraph.get(compiler);
    let mut diagnostics = BTreeSet::new();
    crate::codegen::llvm::initialize_native_target();

    let mut modules = Vec::new();
    let mut has_errors = false;

    // TODO: This could be parallel
    for (crate_id, crate_) in crates.iter() {
        for file in crate_.source_files.values() {
            let parse = Parse(*file).get(compiler);

            for item in &parse.cst.top_level_items {
                let more_diagnostics: BTreeSet<_> = compiler.get_accumulated(TypeCheck(item.id));
                let error_count = classify_diagnostics(&more_diagnostics).0;
                has_errors |= error_count != 0;

                // We can't codegen if there were errors
                // TODO: We should have this check be inside the CodegenLlvm pass itself but we
                // can't call get_accumulated with only a `DbHandle`. If this limitation in
                // inc-complete can't be fixed then we'd need to add a `has_errors: bool` field
                // onto most compiler passes.
                if !has_errors {
                    if let Some(result) = CodegenLlvm(item.id).get(compiler) {
                        if display_ir && *crate_id == CrateId::LOCAL {
                            let context = &parse.top_level_data[&item.id];
                            let name = item.kind.name().to_string(context);
                            display_llvm_bitcode(&result, name);
                        }
                        modules.push(result.module_bitcode);
                    }
                }

                diagnostics.extend(more_diagnostics);
            }
        }
    }
    (modules, has_errors, diagnostics)
}

fn display_llvm_bitcode(result: &CodegenLlvmResult, module_name: String) {
    let buffer = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range(&result.module_bitcode, &module_name);
    let context = inkwell::context::Context::create();
    let new_module = inkwell::module::Module::parse_bitcode_from_buffer(&buffer, &context).expect("Failed to parse llvm module bitcode");
    let module = new_module.print_to_string();
    let module = module.to_string_lossy();
    println!("{module}");
}

/// Codegen everything, linking together each separate llvm module
fn llvm_codegen_all(compiler: &Db, files: &[PathBuf]) -> BTreeSet<Diagnostic> {
    let (modules, has_errors, diagnostics) = llvm_codegen_separate(compiler, false);
    if has_errors {
        return diagnostics;
    }

    let module_name = files.first().map_or_else(|| "a.out".into(), |file| file.with_extension(""));
    let module_name = module_name.to_string_lossy();

    codegen::llvm::link(modules, &module_name);
    diagnostics
}

pub fn path_to_id(crate_id: CrateId, path: &Path) -> SourceFileId {
    let local_module_id = LocalModuleId(parser::ids::hash(path) as u32);
    SourceFileId { crate_id, local_module_id }
}

/// Retrieve all diagnostics emitted after running the given compiler step
#[allow(unused)]
fn get_diagnostics_at_step<C>(compiler: &Db, step: C) -> BTreeSet<Diagnostic>
where
    C: Computation + std::fmt::Debug,
    DbStorage: StorageFor<C>,
{
    compiler.get_accumulated(step)
}
