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
mod util;
mod cache;
mod cli;

#[macro_use]
mod error;
mod lexer;

#[macro_use]
mod parser;
mod nameresolution;
mod types;

#[macro_use]
mod hir;
mod mir;
mod lifetimes;

#[cfg(feature = "llvm")]
mod llvm;
mod cranelift_backend;

use cache::ModuleCache;
use lexer::Lexer;
use nameresolution::NameResolver;

use clap::{CommandFactory, Parser};
use clap_complete as clap_cmp;
use std::fs::File;
use std::io::{stdout, BufReader, Read};
use std::path::Path;

use crate::cli::{Backend, Cli, Completions, EmitTarget};

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
            let (t, traits) =
                types::typeprinter::show_type_and_traits(typ, &info.required_traits, &info.trait_info, cache);
            println!("{} : {}", name, t);
            if !traits.is_empty() {
                println!("  given {}", traits.join(", "));
            }
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

    let mut cache = ModuleCache::new(filename.parent().unwrap());

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    expect!(reader.read_to_string(&mut contents), "Failed to read {} into a string\n", filename.display());

    error::color_output(!args.no_color);
    util::timing::time_passes(args.show_time);

    // Phase 1: Lexing
    util::timing::start_time("Lexing");
    let tokens = Lexer::new(filename, &contents).collect::<Vec<_>>();

    if args.lex {
        tokens.iter().for_each(|(token, _)| println!("{}", token));
        return;
    }

    // Phase 2: Parsing
    util::timing::start_time("Parsing");
    let root = expect!(parser::parse(&tokens), "");

    if args.parse {
        println!("{}", root);
        return;
    }

    // Phase 3: Name resolution
    // Timing for name resolution is within the start method to
    // break up the declare and define passes
    expect!(NameResolver::start(root, &mut cache), "");

    // Phase 4: Type inference
    util::timing::start_time("Type Inference");
    let ast = cache.parse_trees.get_mut(0).unwrap();
    types::typechecker::infer_ast(ast, &mut cache);

    if args.show_types {
        print_definition_types(&cache);
    }

    if args.check || error::get_error_count() != 0 {
        return;
    }

    let hir = hir::monomorphise(ast, cache);
    if args.emit == Some(EmitTarget::Hir) {
        println!("{}", hir);
        return;
    }

    // Phase 5: CPS Conversion
    let mir = mir::convert_to_mir(hir);
    eprintln!("{mir}");

    if true {
        mir.debug_print_control_flow_graph();
        mir.interpret();
        return;
    }

    // Phase 6: Codegen
    // let default_backend = if args.opt_level == '0' { Backend::Cranelift } else { Backend::Llvm };
    // let backend = args.backend.unwrap_or(default_backend);

    // match backend {
    //     Backend::Cranelift => cranelift_backend::run(filename, hir, &args),
    //     Backend::Llvm => {
    //         if cfg!(feature = "llvm") {
    //             #[cfg(feature = "llvm")]
    //             llvm::run(filename, hir, &args);
    //         } else {
    //             eprintln!("The llvm backend is required for non-debug builds. Recompile ante with --features 'llvm' to enable optimized builds.");
    //         }
    //     },
    // }

    // Print out the time each compiler pass took to complete if the --show-time flag was passed
    util::timing::show_timings();
}
