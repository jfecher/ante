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
use find_files::populate_crates_and_files;
use inc_complete::{Computation, StorageFor};
use incremental::{CompileFile, Db, GetCrateGraph, Parse, Resolve};
use name_resolution::namespace::{CrateId, LocalModuleId, SourceFileId, LOCAL_CRATE};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use crate::{
    diagnostics::{DiagnosticKind, Errors},
    incremental::{DbStorage, TypeCheck},
};

// All the compiler passes:
// (listed out of order because `cargo fmt` alphabetizes them)
mod backend;
mod definition_collection;
mod find_files;
mod lexer;
mod name_resolution;
mod parser;
mod type_inference;

// Util modules:
mod cli;
mod diagnostics;
mod incremental;
mod iterator_extensions;
mod paths;
mod vecmap;

/// Deserialize the compiler from our metadata file, returning it along with the file.
///
/// If we fail, just default to a fresh compiler with no cached compilations.
fn make_compiler(source_files: &[PathBuf], incremental: bool) -> (Db, Option<PathBuf>) {
    if let Some(file) = source_files.first() {
        let metadata_file = file.with_extension("inc");

        if incremental {
            if let Ok(text) = read_file(&metadata_file) {
                return (ron::from_str(&text).unwrap_or_default(), Some(metadata_file));
            }
        }
    }
    (Db::default(), None)
}

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
    let (mut compiler, metadata_file) = make_compiler(&args.files, args.incremental);

    // TODO: If the compiler is created from incremental metadata, any previous input
    // files that are no longer used are never cleared.
    populate_crates_and_files(&mut compiler, &args.files);

    let diagnostics = if args.show_tokens {
        display_tokens(&compiler);
        BTreeSet::new()
    } else if args.show_parse {
        display_parse_tree(&compiler)
    } else if args.show_resolved {
        display_name_resolution(&compiler)
    } else if args.show_types {
        display_type_checking(&compiler)
    } else {
        BTreeSet::new()
    };

    display_diagnostics(diagnostics, &compiler);

    if let Some(metadata_file) = metadata_file {
        if let Err(error) = write_metadata(compiler, &metadata_file) {
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

fn display_type_checking(compiler: &Db) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&LOCAL_CRATE];
    let mut diagnostics = BTreeSet::new();

    for file in local_crate.source_files.values() {
        let parse = Parse(*file).get(compiler);

        for item in &parse.cst.top_level_items {
            let resolve_diagnostics: BTreeSet<_> = compiler.get_accumulated(TypeCheck(item.id));
            diagnostics.extend(resolve_diagnostics);
        }

        println!("{}", parse.cst.display_typed(&parse.top_level_data, compiler))
    }
    diagnostics
}

pub fn path_to_id(crate_id: CrateId, path: &Path) -> SourceFileId {
    let local_module_id = LocalModuleId(parser::ids::hash(path) as u32);
    SourceFileId { crate_id, local_module_id }
}

/// Compile all the files in the set to python files. In a real compiler we may want
/// to compile each as an independent llvm or cranelift module then link them all
/// together at the end.
#[allow(unused)]
fn compile_all(files: BTreeSet<SourceFileId>, compiler: &mut Db) -> Errors {
    files.into_par_iter().flat_map(|file| get_diagnostics_at_step(compiler, CompileFile(file))).collect()
}

/// Retrieve all diagnostics emitted after running the given compiler step
fn get_diagnostics_at_step<C>(compiler: &Db, step: C) -> BTreeSet<Diagnostic>
where
    C: Computation + std::fmt::Debug,
    DbStorage: StorageFor<C>,
{
    compiler.get_accumulated(step)
}

fn write_file(file_name: &Path, text: &str) -> Result<(), String> {
    let mut metadata_file = File::create(file_name)
        .map_err(|error| format!("Failed to create file `{}`:\n{error}", file_name.display()))?;

    let text = text.as_bytes();
    metadata_file
        .write_all(text)
        .map_err(|error| format!("Failed to write to file `{}`:\n{error}", file_name.display()))
}

/// This could be changed so that we only write if the metadata actually
/// changed but to simplify things we just always write.
fn write_metadata(compiler: Db, metadata_file: &Path) -> Result<(), String> {
    // Using `to_writer` here would avoid the intermediate step of creating the string
    let serialized = ron::to_string(&compiler).map_err(|error| format!("Failed to serialize database:\n{error}"))?;
    write_file(metadata_file, &serialized)
}

fn read_file(file_name: &std::path::Path) -> Result<String, String> {
    let mut file =
        File::open(file_name).map_err(|error| format!("Failed to open `{}`:\n{error}", file_name.display()))?;

    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| format!("Failed to read from file `{}`:\n{error}", file_name.display()))?;

    Ok(text)
}
