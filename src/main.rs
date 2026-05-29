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
//! bit differently. The main advantage of this architecture is that each step and its
//! dependencies can be cached, preventing repeated work. This is important when the
//! compiler is invoked from Ante's language server, ante-ls.
//!
//! List of compiler passes and the source file to find more about them in:
//! - Lexing `src/lexer/mod.rs`
//! - Parsing `src/parser/mod.rs`
//! - Definition Collection `src/definition_collection/mod.rs`
//! - Name Resolution `src/name_resolution/mod.rs`
//! - Type Inference `src/type_inference/cst_traversal.rs`
//! - MIR Translation `src/mir/builder.rs`
//!   There are a number of passes on MIR, some necessary, some merely optimizations:
//!   - Tail-resume Optimization `src/mir/effects/tail_resume_optimization.rs`
//!   - Abort-handler Optimization `src/mir/effects/abort_handler_optimization.rs`
//!   - Effect Lowering `src/mir/effects/effect_lowering.rs`
//!   - Closure Lowering `src/mir/lower_closures.an`
//!   - Remove Unreachable `src/mir/remove_unreachable.an`
//!   - Monomorphization `src/mir/monomorphization/mod.rs`
//!   - Select Largest Variant `src/mir/select_largest_variant.rs`
//! - Backend Codegen - choose your backend:
//!   - LLVM `src/codegen/llvm/mod.rs`
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
use incremental::{Db, GetCrateGraph, Parse, Resolve};
use name_resolution::namespace::{CrateId, LocalModuleId, SourceFileId};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use crate::{
    cli::{EmitTarget, OptLevel},
    codegen::llvm::{CodegenLlvmResult, codegen_llvm},
    diagnostics::{DiagnosticKind, collect_all_diagnostics},
    files::{make_compiler, write_metadata},
    incremental::{TargetPointerSize, TypeCheck, ValidateExports},
    paths::binary_name,
};

mod codegen;
mod definition_collection;
mod find_files;
mod lexer;
mod mir;
mod name_resolution;
mod parser;
mod type_inference;

mod cli;
mod diagnostics;
mod files;
mod incremental;
mod iterator_extensions;
mod paths;
mod timings;
mod vecmap;

use crate::timings::{print_total_time_of_phases, time_phase};

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

    // TODO: Pointer size should be configurable depending on the target machine
    TargetPointerSize.set(&mut compiler, 8);

    if args.show_time {
        eprintln!("Phase timings:");
        print_phase_timings(&mut compiler);
    }

    let opt_level = optimization_level(&args);

    let diagnostics = match args.emit {
        _ if args.check => time_phase("Diagnostics", args.show_time, || collect_all_diagnostics(&mut compiler)),
        Some(EmitTarget::Tokens) => {
            display_tokens(&compiler);
            BTreeSet::new()
        },
        Some(EmitTarget::Ast) => display_parse_tree(&mut compiler, args.emit_all),
        Some(EmitTarget::AstR) => display_name_resolution(&mut compiler, args.emit_all),
        Some(EmitTarget::AstT) => display_type_checking(&mut compiler, true, args.emit_all),
        Some(EmitTarget::Mir) => display_mir(&mut compiler, args.emit_all, false),
        Some(EmitTarget::MirTail) => display_mir(&mut compiler, args.emit_all, true),
        Some(EmitTarget::MirMono) => display_mir_mono(&mut compiler),
        Some(EmitTarget::Ir) => llvm_codegen_separate(&mut compiler, true, args.show_time, opt_level).2,
        None if args.backend == Some(cli::Backend::Llvm) => {
            llvm_codegen_all(&mut compiler, &args.files, !args.build, args.delete_binary, args.show_time, opt_level)
        },
        None => {
            c_codegen_all(&mut compiler, &args.files, !args.build, args.delete_binary, opt_level)
        },
    };

    let (error_count, _) = classify_diagnostics(&diagnostics);
    display_diagnostics(&diagnostics, &compiler, args.no_color);

    if let Some(metadata_file) = metadata_file {
        let result = time_phase("Write metadata", args.show_time, || write_metadata(&compiler, &metadata_file));
        if let Err(error) = result {
            eprintln!("\n{error}");
        }
    }

    if args.show_time {
        print_total_time_of_phases();
    }

    if error_count != 0 {
        std::process::exit(1);
    }
}

/// Force the front-end passes (parse, name resolution, type inference) in dependency
/// order so each has its own `--show-time` line. inc-complete caches the results, so
/// the downstream compile mode reuses them.
fn print_phase_timings(compiler: &mut Db) {
    let item_ids = time_phase("Parsing", true, || {
        let crates = GetCrateGraph.get(compiler);
        let mut item_ids = Vec::new();
        for crate_ in crates.values() {
            for file in crate_.source_files.values() {
                let parse = Parse(*file).get(compiler);
                for item in parse.cst.top_level_items.iter() {
                    item_ids.push(item.id);
                }
            }
        }
        item_ids
    });

    // Per-item: rolls up dependent definition-collection queries into this bucket.
    time_phase("Name resolution", true, || {
        for id in &item_ids {
            Resolve(*id).get(&*compiler);
        }
    });

    // Per-item: rolls up the type-check dependency graph and SCC partitioning.
    time_phase("Type inference", true, || {
        for id in &item_ids {
            incremental::TypeCheck(*id).get(&*compiler);
        }
    });
}

/// Translate the CLI optimization flags into a single [`OptLevel`]. Explicit `-O` wins when
/// the user provided a non-default value; otherwise `--release` implies O2.
fn optimization_level(args: &Cli) -> OptLevel {
    match args.opt_level {
        '1' => OptLevel::O1,
        '2' => OptLevel::O2,
        '3' => OptLevel::O3,
        's' => OptLevel::Os,
        'z' => OptLevel::Oz,
        _ if args.release => OptLevel::O2,
        _ => OptLevel::O0,
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

fn display_diagnostics(diagnostics: &BTreeSet<Diagnostic>, compiler: &Db, no_color: bool) {
    let (error_count, warning_count) = classify_diagnostics(&diagnostics);
    for diganostic in diagnostics {
        eprintln!("{}", diganostic.display(!no_color, &compiler));
    }

    if error_count != 0 {
        let error_s = if error_count == 1 { "" } else { "s" };
        let errors = format!("{error_count} error{error_s}");
        let errors = if no_color { errors.into() } else { errors.red() };

        let warning_s = if warning_count == 1 { "" } else { "s" };
        let warnings = format!("{warning_count} warning{warning_s}");
        let warnings = if no_color || warning_count == 0 { warnings.into() } else { warnings.yellow() };

        eprintln!("Found {errors} and {warnings}");
    } else if warning_count != 0 {
        let warning_s = if warning_count == 1 { "" } else { "s" };
        let warnings = format!("{warning_count} warning{warning_s}");
        let warnings = if no_color { warnings.into() } else { warnings.yellow() };
        eprintln!("Compiled with {warnings}");
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

fn display_parse_tree(compiler: &mut Db, emit_all: bool) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let mut diagnostics = BTreeSet::new();

    for (crate_id, crate_) in crates.iter() {
        if emit_all || *crate_id == CrateId::LOCAL {
            for file in crate_.source_files.values() {
                let result = Parse(*file).get(compiler);
                println!("{}", result.cst.display(&result.top_level_data));

                let parse_diagnostics = compiler.get_accumulated_uncached(Parse(*file));
                diagnostics.extend(parse_diagnostics);
            }
        }
    }
    diagnostics
}

fn display_name_resolution(compiler: &mut Db, emit_all: bool) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let mut diagnostics = BTreeSet::new();

    for (crate_id, crate_) in crates.iter() {
        if emit_all || *crate_id == CrateId::LOCAL {
            for file in crate_.source_files.values() {
                let parse = Parse(*file).get(compiler);

                for item in &parse.cst.top_level_items {
                    let resolve_diagnostics = compiler.get_accumulated_uncached(Resolve(item.id));
                    diagnostics.extend(resolve_diagnostics);
                }

                let export_diagnostics = compiler.get_accumulated_uncached(ValidateExports(*file));
                diagnostics.extend(export_diagnostics);

                println!("{}", parse.cst.display_resolved(&parse.top_level_data, compiler))
            }
        }
    }
    diagnostics
}

fn display_type_checking(compiler: &mut Db, show_types: bool, emit_all: bool) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let mut diagnostics = BTreeSet::new();

    for (crate_id, crate_) in crates.iter() {
        if emit_all || *crate_id == CrateId::LOCAL {
            for file in crate_.source_files.values() {
                let parse = Parse(*file).get(compiler);

                for item in &parse.cst.top_level_items {
                    let more_diagnostics = compiler.get_accumulated_uncached(TypeCheck(item.id));
                    diagnostics.extend(more_diagnostics);
                }

                if show_types {
                    println!("{}", parse.cst.display_typed(&parse.top_level_data, compiler))
                }
            }
        }
    }
    diagnostics
}

fn display_mir(compiler: &mut Db, emit_all: bool, optimize_tail_calls: bool) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let mut diagnostics = BTreeSet::new();

    for (crate_id, crate_) in crates.iter() {
        if emit_all || *crate_id == CrateId::LOCAL {
            for file in crate_.source_files.values() {
                let parse = Parse(*file).get(compiler);

                for item in &parse.cst.top_level_items {
                    let item_diagnostics = compiler.get_accumulated_uncached(TypeCheck(item.id));
                    let item_has_errors = item_diagnostics.iter().any(|d| matches!(d.kind(), DiagnosticKind::Error));
                    diagnostics.extend(item_diagnostics);

                    if item_has_errors {
                        continue;
                    }

                    let mir = mir::builder::build_initial_mir_with_shared_map(compiler, item.id);
                    if let Some(mut mir) = mir {
                        if optimize_tail_calls {
                            mir = mir.optimize_tail_resume().optimize_abort_handlers().lower_effects();
                        }

                        print!("{mir}");
                    }
                }
            }
        }
    }
    diagnostics
}

fn display_mir_mono(compiler: &mut Db) -> BTreeSet<Diagnostic> {
    let diagnostics = collect_all_diagnostics(compiler);
    let (errors, _) = classify_diagnostics(&diagnostics);
    if errors == 0 {
        let mir = mir::monomorphization::monomorphize(compiler);
        println!("{mir}");
    }
    diagnostics
}

/// Codegen each item as a separate llvm module
/// Returns (module strings, true if there are any errors, diagnostics)
fn llvm_codegen_separate(
    compiler: &mut Db, display_ir: bool, show_time: bool, opt_level: OptLevel,
) -> (Vec<Arc<Vec<u8>>>, bool, BTreeSet<Diagnostic>) {
    let diagnostics = time_phase("Diagnostics", show_time, || collect_all_diagnostics(compiler));
    let (errors, _) = classify_diagnostics(&diagnostics);
    if errors != 0 {
        return (Vec::new(), true, diagnostics);
    }

    let modules = if let Some(result) = codegen_llvm(compiler, show_time, opt_level) {
        if display_ir {
            display_llvm_bitcode(&result, "program");
        }
        vec![result.module_bitcode]
    } else {
        Vec::new()
    };
    (modules, false, diagnostics)
}

fn display_llvm_bitcode(result: &CodegenLlvmResult, module_name: &str) {
    let buffer = inkwell::memory_buffer::MemoryBuffer::create_from_memory_range(&result.module_bitcode, module_name);
    let context = inkwell::context::Context::create();
    let new_module = inkwell::module::Module::parse_bitcode_from_buffer(&buffer, &context)
        .expect("Failed to parse llvm module bitcode");
    let module = new_module.print_to_string();
    let module = module.to_string_lossy();
    println!("{module}");
}

/// Codegen everything, linking together each separate llvm module
fn llvm_codegen_all(
    compiler: &mut Db, files: &[PathBuf], run: bool, delete_binary: bool, show_time: bool, opt_level: OptLevel,
) -> BTreeSet<Diagnostic> {
    let (mut modules, has_errors, diagnostics) = llvm_codegen_separate(compiler, false, show_time, opt_level);
    if has_errors {
        return diagnostics;
    }

    // Each module is currently the whole program (monomorphization isn't yet incremental).
    modules.truncate(1);

    let program_name = files_to_program_name(files);

    let link_succeeded = codegen::llvm::link(modules, &program_name, show_time, opt_level);
    if !link_succeeded {
        return diagnostics;
    }

    if run {
        // Use an absolute path so the binary can be found regardless of PATH.
        let binary_path = binary_name(&program_name);

        Command::new(&binary_path).spawn().unwrap().wait().unwrap();
        if delete_binary {
            std::fs::remove_file(binary_path).unwrap();
        }
    }

    diagnostics
}

/// Monomorphize and codegen the whole program through the C backend, then optionally run it.
fn c_codegen_all(
    compiler: &mut Db, files: &[PathBuf], run: bool, delete_binary: bool, opt_level: OptLevel,
) -> BTreeSet<Diagnostic> {
    let diagnostics = collect_all_diagnostics(compiler);
    let (errors, _) = classify_diagnostics(&diagnostics);
    if errors != 0 {
        return diagnostics;
    }

    let program_name = files_to_program_name(files);
    let mir = mir::monomorphization::monomorphize(compiler);
    codegen::c::codegen_c_for_mir(&mir, &program_name, opt_level);

    if run {
        let binary_path = binary_name(&program_name);
        Command::new(&binary_path).spawn().unwrap().wait().unwrap();
        if delete_binary {
            std::fs::remove_file(binary_path).unwrap();
        }
    }

    diagnostics
}

/// Return the default name of the program given the source files.
fn files_to_program_name(files: &[PathBuf]) -> String {
    let name = files.first().map_or_else(|| "a.out".into(), |file| file.with_extension(""));
    name.to_string_lossy().into_owned()
}

pub fn path_to_id(crate_id: CrateId, path: &Path) -> SourceFileId {
    let local_module_id = LocalModuleId(parser::ids::hash(path) as u32);
    SourceFileId { crate_id, local_module_id }
}
