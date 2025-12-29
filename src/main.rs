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
use name_resolution::namespace::{CrateId, LOCAL_CRATE, LocalModuleId, SourceFileId};
use std::{collections::BTreeSet, path::Path};

use crate::{
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

    let diagnostics = if args.show_tokens {
        display_tokens(&compiler);
        BTreeSet::new()
    } else if args.show_parse {
        display_parse_tree(&compiler)
    } else if args.show_resolved {
        display_name_resolution(&compiler)
    } else if args.show_types || args.check {
        display_type_checking(&compiler, args.show_types)
    } else if args.show_mir {
        display_mir(&compiler)
    } else {
        llvm_codegen(&compiler)
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
    let local_crate = &crates[&LOCAL_CRATE];

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
    let local_crate = &crates[&LOCAL_CRATE];
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
    let local_crate = &crates[&LOCAL_CRATE];
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
    let local_crate = &crates[&LOCAL_CRATE];
    let mut diagnostics = BTreeSet::new();

    for file in local_crate.source_files.values() {
        let parse = Parse(*file).get(compiler);

        for item in &parse.cst.top_level_items {
            let resolve_diagnostics: BTreeSet<_> = compiler.get_accumulated(TypeCheck(item.id));
            diagnostics.extend(resolve_diagnostics);
        }

        if show_types {
            println!("{}", parse.cst.display_typed(&parse.top_level_data, compiler))
        }
    }
    diagnostics
}

fn display_mir(compiler: &Db) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&LOCAL_CRATE];
    let mut diagnostics = BTreeSet::new();

    for file in local_crate.source_files.values() {
        let parse = Parse(*file).get(compiler);

        for item in &parse.cst.top_level_items {
            let mir = mir::builder::build_initial_mir(compiler, item.id);
            if let Some(mir) = mir {
                for function in mir.functions.into_values() {
                    println!("{function}\n");
                }
            }
            let resolve_diagnostics: BTreeSet<_> = compiler.get_accumulated(TypeCheck(item.id));
            diagnostics.extend(resolve_diagnostics);
        }
    }
    diagnostics
}

fn llvm_codegen(compiler: &Db) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let mut diagnostics = BTreeSet::new();
    crate::codegen::llvm::initialize_native_target();

    // TODO: This could be parallel
    for (_, crate_) in crates.iter() {
        for file in crate_.source_files.values() {
            let parse = Parse(*file).get(compiler);

            for item in &parse.cst.top_level_items {
                let llvm_result = CodegenLlvm(item.id).get(compiler);

                if let Some(llvm) = &llvm_result.module_string {
                    println!("{llvm}");
                }
                let resolve_diagnostics: BTreeSet<_> = compiler.get_accumulated(CodegenLlvm(item.id));
                diagnostics.extend(resolve_diagnostics);
            }
        }
    }
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
