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
//!   - C `src/codegen/c/mod.rs`
//!
//! Non-passes:
//! - `src/errors.rs`: Defines each error used in the program as well as the `Location` struct
//! - `src/incremental.rs`: Some plumbing for the inc-complete library which also defines
//!   which functions we're caching the result of.
#![allow(mismatched_lifetime_syntaxes)]

use clap::{CommandFactory, Parser};
use cli::{Cli, Commands, Completions};
use colored::Colorize;
use diagnostics::Diagnostic;
use incremental::{AllDefinitions, Db, GetCrateGraph, Parse, Resolve};
use name_resolution::namespace::{CrateId, LocalModuleId, SourceFileId};
use parser::ids::TopLevelName;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(feature = "llvm")]
use crate::codegen::llvm::codegen_llvm;
use crate::{
    cli::{EmitTarget, OptLevel},
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

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
    let (run, files, program_name) = match &args.command {
        Some(cmd) => (
            matches!(cmd, Commands::Run),
            crate::find_files::find_project_main_file().map(|path| vec![path]).unwrap_or_default(),
            crate::find_files::find_project_name().unwrap_or_else(|| "a.out".to_string()),
        ),
        None => (!args.build, args.files.clone(), files_to_program_name(&args.files)),
    };
    let (mut compiler, metadata_file) = make_compiler(&files, args.incremental);

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
        Some(EmitTarget::Ir) => display_ir(&mut compiler, resolve_backend(args.backend), args.show_time, opt_level),
        None => codegen_all(
            &mut compiler,
            resolve_backend(args.backend),
            &program_name,
            run,
            args.delete_binary,
            args.show_time,
            opt_level,
            args.bin.as_deref(),
        ),
    };

    let error_count = display_diagnostics(&diagnostics, &compiler, args.no_color);

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

/// Resolve the effective backend from the user's `--backend` choice, exiting with an error
/// for any backend this compiler cannot run. When `--backend` is omitted the priority is:
/// - debug: cranelift > llvm > c
/// - release: llvm > c
fn resolve_backend(requested: Option<cli::Backend>) -> cli::Backend {
    use cli::Backend;
    match requested {
        Some(Backend::Cranelift) => {
            eprintln!("The cranelift backend is not yet implemented");
            std::process::exit(1);
        },
        Some(Backend::Llvm) if !cfg!(feature = "llvm") => {
            eprintln!(
                "This compiler was built without the 'llvm' feature, so the llvm backend is \
                 unavailable. Rebuild with `cargo build --features llvm` to enable it."
            );
            std::process::exit(1);
        },
        Some(backend) => backend,
        None if cfg!(feature = "llvm") => Backend::Llvm,
        None => Backend::C,
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

/// Print each diagnostic followed by a summary line, returning the error count.
fn display_diagnostics(diagnostics: &BTreeSet<Diagnostic>, compiler: &Db, no_color: bool) -> usize {
    let (error_count, warning_count) = classify_diagnostics(diagnostics);
    for diagnostic in diagnostics {
        eprintln!("{}", diagnostic.display(!no_color, compiler));
    }

    let warnings = format!("{warning_count} warning{}", if warning_count == 1 { "" } else { "s" });
    let warnings = if no_color || warning_count == 0 { warnings.normal() } else { warnings.yellow() };

    if error_count != 0 {
        let errors = format!("{error_count} error{}", if error_count == 1 { "" } else { "s" });
        let errors = if no_color { errors.normal() } else { errors.red() };
        eprintln!("Found {errors} and {warnings}");
    } else if warning_count != 0 {
        eprintln!("Compiled with {warnings}");
    }
    error_count
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

/// Iterate the source files of the local crate (or all crates when `emit_all`), invoking
/// `f` for each and collecting the diagnostics it returns.
fn for_each_emitted_file(
    compiler: &mut Db, emit_all: bool, mut f: impl FnMut(&mut Db, SourceFileId) -> BTreeSet<Diagnostic>,
) -> BTreeSet<Diagnostic> {
    let crates = GetCrateGraph.get(compiler);
    let mut diagnostics = BTreeSet::new();
    for (crate_id, crate_) in crates.iter() {
        if emit_all || *crate_id == CrateId::LOCAL {
            for file in crate_.source_files.values() {
                diagnostics.extend(f(compiler, *file));
            }
        }
    }
    diagnostics
}

fn display_parse_tree(compiler: &mut Db, emit_all: bool) -> BTreeSet<Diagnostic> {
    for_each_emitted_file(compiler, emit_all, |compiler, file| {
        let result = Parse(file).get(compiler);
        println!("{}", result.cst.display(&result.top_level_data));
        compiler.get_accumulated_uncached(Parse(file))
    })
}

fn display_name_resolution(compiler: &mut Db, emit_all: bool) -> BTreeSet<Diagnostic> {
    for_each_emitted_file(compiler, emit_all, |compiler, file| {
        let parse = Parse(file).get(compiler);
        let mut diagnostics = BTreeSet::new();
        for item in &parse.cst.top_level_items {
            diagnostics.extend(compiler.get_accumulated_uncached(Resolve(item.id)));
        }
        diagnostics.extend(compiler.get_accumulated_uncached(ValidateExports(file)));
        println!("{}", parse.cst.display_resolved(&parse.top_level_data, compiler));
        diagnostics
    })
}

fn display_type_checking(compiler: &mut Db, show_types: bool, emit_all: bool) -> BTreeSet<Diagnostic> {
    for_each_emitted_file(compiler, emit_all, |compiler, file| {
        let parse = Parse(file).get(compiler);
        let mut diagnostics = BTreeSet::new();
        for item in &parse.cst.top_level_items {
            diagnostics.extend(compiler.get_accumulated_uncached(TypeCheck(item.id)));
        }
        if show_types {
            println!("{}", parse.cst.display_typed(&parse.top_level_data, compiler));
        }
        diagnostics
    })
}

fn display_mir(compiler: &mut Db, emit_all: bool, optimize_tail_calls: bool) -> BTreeSet<Diagnostic> {
    for_each_emitted_file(compiler, emit_all, |compiler, file| {
        let parse = Parse(file).get(compiler);
        let mut diagnostics = BTreeSet::new();
        for item in &parse.cst.top_level_items {
            let item_diagnostics = compiler.get_accumulated_uncached(TypeCheck(item.id));
            let item_has_errors = item_diagnostics.iter().any(|d| matches!(d.kind(), DiagnosticKind::Error));
            diagnostics.extend(item_diagnostics);

            if item_has_errors {
                continue;
            }

            if let Some(mut mir) = mir::builder::build_initial_mir_with_shared_map(compiler, item.id) {
                if optimize_tail_calls {
                    mir = mir.optimize_tail_resume().optimize_abort_handlers().lower_effects();
                }
                print!("{mir}");
            }
        }
        diagnostics
    })
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

/// Emit the backend IR to stdout: Either llvm-ir or C depending on the backend.
fn display_ir(compiler: &mut Db, backend: cli::Backend, show_time: bool, opt_level: OptLevel) -> BTreeSet<Diagnostic> {
    let diagnostics = time_phase("Diagnostics", show_time, || collect_all_diagnostics(compiler));
    if classify_diagnostics(&diagnostics).0 == 0 {
        match backend {
            #[cfg(feature = "llvm")]
            cli::Backend::Llvm => {
                codegen_llvm(compiler, show_time, opt_level, true, None);
            },
            cli::Backend::C => {
                let _ = opt_level;
                let mir = mir::monomorphization::monomorphize(compiler);
                print!("{}", codegen::c::build_c_file(&mir, None));
            },
            _ => unreachable!("resolve_backend only returns backends this compiler can run"),
        }
    }
    diagnostics
}

/// Check the program, codegen the whole thing through the chosen backend, then optionally run it.
#[allow(clippy::too_many_arguments)]
fn codegen_all(
    compiler: &mut Db, backend: cli::Backend, program_name: &str, run: bool, delete_binary: bool, show_time: bool,
    opt_level: OptLevel, bin: Option<&str>,
) -> BTreeSet<Diagnostic> {
    let (diagnostics, selected_main) = match check_and_select_main(compiler, show_time, bin) {
        Ok(ok) => ok,
        Err(diagnostics) => return diagnostics,
    };

    let ready = match backend {
        #[cfg(feature = "llvm")]
        cli::Backend::Llvm => {
            // Each module is currently the whole program (monomorphization isn't yet incremental).
            let modules = match codegen_llvm(compiler, show_time, opt_level, false, Some(selected_main)) {
                Some(result) => vec![result.object],
                None => Vec::new(),
            };
            codegen::llvm::link(modules, program_name, show_time, opt_level)
        },
        cli::Backend::C => {
            let mir = mir::monomorphization::monomorphize(compiler);
            codegen::c::codegen_c_for_mir(&mir, program_name, opt_level, Some(selected_main));
            true
        },
        _ => unreachable!("resolve_backend only returns backends this compiler can run"),
    };

    if ready && run {
        run_binary(program_name, delete_binary);
    }
    diagnostics
}

/// Collect diagnostics and pick the `main` function to use if there are multiple or 0.
fn check_and_select_main(
    compiler: &mut Db, show_time: bool, bin: Option<&str>,
) -> Result<(BTreeSet<Diagnostic>, TopLevelName), BTreeSet<Diagnostic>> {
    let diagnostics = time_phase("Diagnostics", show_time, || collect_all_diagnostics(compiler));
    if classify_diagnostics(&diagnostics).0 != 0 {
        return Err(diagnostics);
    }
    let selected = select_main(compiler, bin);
    Ok((diagnostics, selected))
}

/// Run the binary then optionally delete it.
fn run_binary(program_name: &str, delete_binary: bool) {
    let binary_path = binary_name(program_name);
    Command::new(&binary_path).spawn().unwrap().wait().unwrap();
    if delete_binary {
        std::fs::remove_file(binary_path).unwrap();
    }
}

/// Return the default name of the program given the source files.
fn files_to_program_name(files: &[PathBuf]) -> String {
    let name = files.first().map_or_else(|| "a.out".into(), |file| file.with_extension(""));
    name.to_string_lossy().into_owned()
}

/// Choose which `main` function to compile into the binary's entry point.
/// This will exit and print to stderr if a `main` function was failed to be selected.
fn select_main(compiler: &Db, bin: Option<&str>) -> TopLevelName {
    let crates = GetCrateGraph.get(compiler);
    let local_crate = &crates[&CrateId::LOCAL];

    let mut mains = BTreeSet::new();
    for (path, file_id) in &local_crate.source_files {
        let definitions = AllDefinitions(*file_id).get(compiler);
        for (name, top_level_name) in &definitions.definitions {
            if name.as_str() == "main" && top_level_name.top_level_item.source_file == *file_id {
                let module = path.to_string_lossy().into_owned();
                mains.insert((module, *top_level_name));
            }
        }
    }

    if mains.is_empty() {
        eprintln!("{}: This program has no main function", "error".red());
        std::process::exit(1);
    } else if mains.len() == 1 {
        mains.into_iter().next().unwrap().1
    } else {
        if let Some(requested) = bin
            && let Some((_, main)) = mains.iter().find(|(module, _)| module == requested)
        {
            return *main;
        }

        eprintln!("{}: This program has multiple main functions. Use --bin to specify which to use:", "error".red());
        for (path, _) in mains {
            eprintln!("  - {path}");
        }
        std::process::exit(1);
    }
}

pub fn path_to_id(crate_id: CrateId, path: &Path) -> SourceFileId {
    let local_module_id = LocalModuleId(parser::ids::hash(path) as u32);
    SourceFileId { crate_id, local_module_id }
}
