#[macro_use]
mod parser;
mod lexer;
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

fn print_definition_types<'a>(cache: &ModuleCache<'a>) {
    let mut definitions = vec![];

    for (_, module_id) in cache.modules.iter() {
        let resolver = cache.name_resolvers.get_mut(module_id.0).unwrap();
        definitions.append(&mut resolver.exports.definitions.iter().collect());
    }

    // Make sure the output has a deterministic order for testing
    definitions.sort();

    for (name, definition_id) in definitions {
        let info = &cache.definition_infos[definition_id.0];
        let typ = info.typ.clone().unwrap_or(types::Type::Primitive(types::PrimitiveType::UnitType));

        print!("{} : ", name);
        types::typeprinter::show_type_and_traits(&typ, &info.required_impls, cache);
    }
}

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
    let args = App::new("ante")
        .version("0.1.1")
        .author("Jake Fecher <jfecher11@gmail.com>")
        .about("Compiler for the Ante programming language")
        .arg(Arg::with_name("lex").long("lex").help("Lex the file and output the resulting list of tokens"))
        .arg(Arg::with_name("parse").long("parse").help("Parse the file and output the resulting Ast"))
        .arg(Arg::with_name("check").long("check").help("Check the file for errors without compiling"))
        .arg(Arg::with_name("run").long("run").help("Run the resulting binary"))
        .arg(Arg::with_name("no-color").long("no-color").help("Use plaintext and an indicator line instead of color for pointing out error locations"))
        .arg(Arg::with_name("show-types").long("show-types").help("Print out the type of each definition"))
        .arg(Arg::with_name("show-llvm-ir").long("show-llvm-ir").help("Print out the LLVM-IR of the compiled program"))
        .arg(Arg::with_name("delete-binary").long("delete-binary").help("Delete the resulting binary after compiling. Useful for testing."))
        .arg(Arg::with_name("file").help("The file to compile").required(true))
        .get_matches();

    let filename = Path::new(args.value_of("file").unwrap());
    let file = expect!(File::open(filename), "Could not open file {}\n", filename.display());

    let mut cache = ModuleCache::new(filename.parent().unwrap());

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    expect!(reader.read_to_string(&mut contents), "Failed to read {} into a string\n", filename.display());

    error::color_output(!args.is_present("no-color"));

    let tokens = Lexer::new(filename, &contents).collect::<Vec<_>>();

    if args.is_present("lex") {
        tokens.iter().for_each(|(token, _)| println!("{}", token));
        return;
    }

    let root = expect!(parser::parse(&tokens), "");

    if args.is_present("parse") {
        println!("{}", root);
        return;
    }

    expect!(NameResolver::start(root, &mut cache), "");

    let ast = cache.parse_trees.get_mut(0).unwrap();
    types::typechecker::infer_ast(ast, &mut cache);

    // for defs in cache.definition_infos.iter().filter(|def| def.typ.is_none()) {
    //     warning!(defs.location, "{} is unused and was not typechecked", defs.name);
    // }

    if args.is_present("show-types") {
        print_definition_types(&cache);
    }

    if args.is_present("check") {
        return;
    }

    if error::get_error_count() == 0 {
        llvm::run(&filename, &ast, &mut cache,
                args.is_present("show-llvm-ir"),
                args.is_present("run"),
                args.is_present("delete-binary"));
    }
}
