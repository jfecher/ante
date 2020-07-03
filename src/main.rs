#[macro_use]
mod parser;
mod lexer;
mod util;

#[macro_use]
mod error;
mod nameresolution;
mod types;

use lexer::Lexer;
use nameresolution::{ NameResolver, modulecache::ModuleCache };

use clap::{App, Arg};
use std::fs::File;
use std::path::Path;
use std::io::{BufReader, Read};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug)]
pub enum Error {
    Unrecoverable,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::Unrecoverable
    }
}

fn print_definition_types<'a>(cache: &ModuleCache<'a>) {
    for (_, module_id) in cache.modules.iter() {
        let resolver = cache.name_resolvers.get_mut(module_id.0).unwrap();
        for (name, definition_id) in resolver.exports.definitions.iter() {
            let info = &cache.definition_infos[definition_id.0];
            let typ = info.typ.clone().unwrap();
            println!("{} : {}", name, typ.debug(&cache));
            if !info.required_impls.is_empty() {
                print!("  given");
                for trait_impl in info.required_impls.iter() {
                    print!(", {}", trait_impl.debug(&cache));
                }
                println!("");
            }
        }
    }
}

pub fn main() -> Result<(), Error> {
    let args = App::new("ante")
        .version("0.0.1")
        .author("Jake Fecher <jfecher11@gmail.com>")
        .about("Compiler for the Ante programming language")
        .arg(Arg::with_name("lex").long("lex").help("Parse the file and output the resulting Ast"))
        .arg(Arg::with_name("parse").long("parse").help("Parse the file and output the resulting Ast"))
        .arg(Arg::with_name("check").long("check").help("Check the file for errors without compiling"))
        .arg(Arg::with_name("show types").long("show-types").help("Print out the type of each definition"))
        .arg(Arg::with_name("no color").long("no-color").help("Use plaintext for errors and an indicator line instead of color for pointing out error locations"))
        .arg(Arg::with_name("file").help("The file to compile").required(true))
        .get_matches();

    let filename = Path::new(args.value_of("file").unwrap());

    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            println!("Could not open file {}", filename.display());
            return Err(Error::Unrecoverable);
        }
    };

    let mut cache = ModuleCache::new(filename.parent().unwrap());

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    error::color_output(!args.is_present("no color"));

    let tokens = Lexer::new(filename, &contents).collect::<Vec<_>>();

    if args.is_present("lex") {
        tokens.iter().for_each(|(token, _)| println!("{}", token));
    } else if args.is_present("parse") {
        let result = parser::parse(&tokens);
        match result {
            Ok(tree) => println!("{}", tree),
            Err(e) => println!("{}", e),
        }
    } else if args.is_present("check") {
        let root = parser::parse(&tokens)
            .map_err(|e| { println!("{}", e); Error::Unrecoverable })?;

        NameResolver::start(root, &mut cache);

        if error::get_error_count() == 0 {
            let ast = cache.parse_trees.get_mut(0).unwrap();
            types::typechecker::infer_ast(ast, &mut cache);

            for defs in cache.definition_infos.iter().filter(|def| def.typ.is_none()) {
                warning!(defs.location, "{} is unused and was not typechecked", defs.name);
            }

            if args.is_present("show types") {
                print_definition_types(&cache);
            }
        }
    } else {
        unimplemented!("Compiling is currently unimplemented")
    }

    Ok(())
}
