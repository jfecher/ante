//! main.rs - The entry point for the Ante compiler.
//! Handles command-line argument parsing and dataflow between
//! each compiler phase. The compiler as a whole is separated into
//! the following phases (in order):
//!
//! lexing -> parsing -> name resolution -> type inference -> lifetime inference -> codegen
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

#[macro_use]
mod error;
mod cache;
mod nameresolution;
mod types;
mod llvm;

use lexer::Lexer;
use nameresolution::NameResolver;
use cache::ModuleCache;

use clap::{App, Arg};
use std::fs::File;
use std::path::Path;
use std::io::{BufReader, Read};

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Called when the "--check --show-types" command-line flags are given.
/// Iterates through each Definition from the first compiled module (so excluding imports)
/// and prints the type and required traits for each.
fn print_definition_types<'a>(cache: &ModuleCache<'a>) {
    let resolver = cache.name_resolvers.get_mut(0).unwrap();
    let mut definitions = resolver.exports.definitions.iter().collect::<Vec<_>>();

    // Make sure the output has a deterministic order for testing
    definitions.sort();

    for (name, definition_id) in definitions {
        let info = &cache.definition_infos[definition_id.0];
        let typ = info.typ.clone().unwrap_or(types::Type::Primitive(types::PrimitiveType::UnitType));

        print!("{} : ", name);
        types::typeprinter::show_type_and_traits(&typ, &info.required_traits, cache);
    }
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

fn validate_opt_argument(arg: String) -> Result<(), String> {
    match arg.as_str() {
        "0" | "1" | "2" | "3" | "s" | "z" => Ok(()),
        _ => Err("Argument to -O must be one of: 0, 1, 2, 3, s, or z".to_owned()),
    }
}

pub fn main() {
    let args = App::new("ante")
        .version("0.1.1")
        .author("Jake Fecher <jfecher11@gmail.com>")
        .about("Compiler for the Ante programming language")
        .arg(Arg::with_name("lex").long("lex").help("Lex the file and output the resulting list of tokens"))
        .arg(Arg::with_name("parse").long("parse").help("Parse the file and output the resulting Ast"))
        .arg(Arg::with_name("check").long("check").help("Check the file for errors without compiling"))
        .arg(Arg::with_name("run").long("run").help("Run the resulting binary"))
        .arg(Arg::with_name("O").short("O").value_name("level").default_value("0").validator(validate_opt_argument).help("Sets the current optimization level from 0 (no optimization) to 3 (aggressive optimization). Set to s or z to optimize for size."))
        .arg(Arg::with_name("no-color").long("no-color").help("Use plaintext and an indicator line instead of color for pointing out error locations"))
        .arg(Arg::with_name("show-types").long("show-types").help("Print out the type of each definition"))
        .arg(Arg::with_name("emit-llvm").long("emit-llvm").help("Print out the LLVM-IR of the compiled program"))
        .arg(Arg::with_name("delete-binary").long("delete-binary").help("Delete the resulting binary after compiling"))
        .arg(Arg::with_name("show-time").long("show-time").help("Output the time each compiler pass takes for the given program"))
        .arg(Arg::with_name("file").help("The file to compile").required(true))
        .get_matches();

    // Setup the cache and read from the first file
    let filename = Path::new(args.value_of("file").unwrap());
    let file = expect!(File::open(filename), "Could not open file {}\n", filename.display());

    let mut cache = ModuleCache::new(filename.parent().unwrap());

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    expect!(reader.read_to_string(&mut contents), "Failed to read {} into a string\n", filename.display());

    error::color_output(!args.is_present("no-color"));
    util::timing::time_passes(args.is_present("show-time"));

    // Phase 1: Lexing
    util::timing::start_time("Lexing");
    let tokens = Lexer::new(filename, &contents).collect::<Vec<_>>();

    if args.is_present("lex") {
        tokens.iter().for_each(|(token, _)| println!("{}", token));
        return;
    }

    // Phase 2: Parsing
    util::timing::start_time("Parsing");
    let root = expect!(parser::parse(&tokens), "");

    if args.is_present("parse") {
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

    if args.is_present("show-types") {
        print_definition_types(&cache);
    }

    if args.is_present("check") {
        return;
    }

    // Phase 5: Lifetime inference
    // TODO!

    // Phase 6: Codegen
    if error::get_error_count() == 0 {
        llvm::run(&filename, &ast, &mut cache,
                args.is_present("emit-llvm"),
                args.is_present("run"),
                args.is_present("delete-binary"),
                args.value_of("O").unwrap());
    }

    // Print out the time each compiler pass took to complete if the --show-time flag was passed
    util::timing::show_timings();
}
