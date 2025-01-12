//! main.rs - The entry point for the Ante compiler.
//! Handles command-line argument parsing and dataflow between
//! each compiler phase. The compiler as a whole is separated into
//! the following phases (in order):
//!
//! lexing -> parsing -> name resolution -> type inference -> monomorphisation -> codegen
//!
//! Each phase corresponds to a source folder with roughly the same name (though the codegen
//! folder is named "llvm"), and each phase after parsing operates by traversing the AST.
//! This AST traversal is usually defined in the mod.rs file for that phase and is a good
//! place to start if you're trying to learn how that phase works. An exception is type
//! inference which has its AST pass defined in types/typechecker.rs rather than types/mod.rs.
//! Note that sometimes "phases" are sometimes called "passes" and vice-versa - the terms are
//! interchangeable.
#[macro_use]
mod parser;
mod lexer;

#[macro_use]
mod util;
mod frontend;

#[macro_use]
mod error;
mod cache;
mod cli;

#[macro_use]
mod hir;
mod cranelift_backend;
mod lifetimes;
mod nameresolution;
mod types;

#[cfg(feature = "llvm")]
mod llvm;

use cache::ModuleCache;
use cli::{Backend, Cli, Completions, EmitTarget};
use frontend::{check, FrontendPhase, FrontendResult};

use clap::{CommandFactory, Parser};
use clap_complete as clap_cmp;
use std::collections::HashMap;
use std::fs::File;
use std::io::{stdout, BufReader, Read};
use std::path::Path;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Called when the "--check --show-types" command-line flags are given.
/// Iterates through each Definition from the first compiled module (so excluding imports)
/// and prints the type and required traits for each.
fn print_definition_types(cache: &ModuleCache) {
    let resolver = cache.name_resolvers.get_mut(0).unwrap();
    let mut definitions = resolver.exports.definitions.iter().collect::<Vec<_>>();

    // Make sure the output has a deterministic order for testing
    definitions.sort();

    for (name, definition_id) in definitions {
        let info = &cache[*definition_id];

        if let Some(typ) = &info.typ {
            let type_string =
                types::typeprinter::show_type_and_traits(name, typ, &info.required_traits, &info.trait_info, cache, true);
            println!("{}", type_string);
        } else {
            println!("{} : (none)", name);
        }
    }
}

fn print_completions<G: clap_cmp::Generator>(gen: G) {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_cmp::generate(gen, &mut cmd, name, &mut stdout());
}

/// Convenience macro for unwrapping a Result or printing an error message and returning () on Err.
macro_rules! expect {( $result:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
    match $result {
        Ok(t) => t,
        Err(_) => {
            print!($fmt_string $( , $($msg)* )? );
            return ();
        },
    }
});}

pub fn main() {
    if let Ok(Completions { shell_completion }) = Completions::try_parse() {
        print_completions(shell_completion);
    } else {
        compile(Cli::parse())
    }
}

fn compile(args: Cli) {
    // Setup the cache and read from the first file
    let filename = Path::new(&args.file);
    let file = File::open(filename);
    let file = expect!(file, "Could not open file {}\n", filename.display());
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    expect!(reader.read_to_string(&mut contents), "Failed to read {} into a string\n", filename.display());

    let filename = if filename.is_relative() {
        let cwd = expect!(std::env::current_dir(), "Could not get current directory\n");
        expect!(cwd.join(filename).canonicalize(), "Could not canonicalize {}\n", filename.display())
    } else {
        expect!(filename.canonicalize(), "Could not canonicalize {}\n", filename.display())
    };
    let parent = filename.parent().unwrap();

    let file_cache = HashMap::from([(filename.clone(), contents.clone())]);

    let mut cache = ModuleCache::new(parent, file_cache);

    error::color_output(!args.no_color);

    let phase = if args.lex {
        FrontendPhase::Lex
    } else if args.parse {
        FrontendPhase::Parse
    } else {
        FrontendPhase::TypeCheck
    };

    match check(&filename, contents, &mut cache, phase, args.show_time) {
        FrontendResult::Done => return,
        FrontendResult::ContinueCompilation => (),
        FrontendResult::Errors => {
            cache.display_diagnostics();

            if args.show_types {
                print_definition_types(&cache);
            }
            return;
        },
    }

    let ast = cache.parse_trees.get_mut(0).unwrap();

    if args.show_types {
        print_definition_types(&cache);
    }

    if args.check || cache.error_count() != 0 {
        return;
    }

    let hir = hir::monomorphise(ast, cache);
    if args.emit == Some(EmitTarget::Hir) {
        println!("{}", hir);
        return;
    }

    // Phase 5: Lifetime inference
    // util::timing::start_time("Lifetime Inference");
    // lifetimes::infer(ast, &mut cache);

    // if args.show_lifetimes {
    //     println!("{}", ast);
    // }

    // Phase 6: Codegen
    let default_backend = if args.opt_level == '0' { Backend::Cranelift } else { Backend::Llvm };
    let backend = args.backend.unwrap_or(default_backend);

    match backend {
        Backend::Cranelift => cranelift_backend::run(&filename, hir, &args),
        Backend::Llvm => {
            if cfg!(feature = "llvm") {
                #[cfg(feature = "llvm")]
                llvm::run(&filename, hir, &args);
            } else {
                eprintln!("The llvm backend is required for non-debug builds. Recompile ante with --features 'llvm' to enable optimized builds.");
            }
        },
    }

    // Print out the time each compiler pass took to complete if the --show-time flag was passed
    util::timing::show_timings();
}
