use clap::{App, Arg};
use std::fs::File;
use std::path::Path;
use std::io::{BufReader, Read};

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

#[derive(Debug)]
enum Error {
    Unrecoverable,
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::Unrecoverable
    }
}

fn main() -> Result<(), Error> {
    let args = App::new("ante")
        .version("0.0.1")
        .author("Jake Fecher <jfecher11@gmail.com>")
        .about("Compiler for the Ante programming language")
        .arg(Arg::with_name("lex").long("lex").help("Parse the file and output the resulting Ast"))
        .arg(Arg::with_name("parse").long("parse").help("Parse the file and output the resulting Ast"))
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

    let tokens = Lexer::new(filename, &contents).collect::<Vec<_>>();

    if args.is_present("lex") {
        tokens.iter().for_each(|(token, _)| println!("{}", token));
    } else if args.is_present("parse") {
        let result = parser::parse(&tokens);
        match result {
            Ok(tree) => println!("{}", tree),
            Err(e) => println!("{}", e),
        }
    } else {
        let result = parser::parse(&tokens);
        match result {
            Ok(root) => {
                NameResolver::start(root, &mut cache);
                let ast = cache.parse_trees.get_mut(0).unwrap();
                println!("{}", ast);
                let (typ, _) = types::typechecker::infer(ast, &mut cache);
                println!("{}", typ.display(&mut cache));
                for defs in cache.definition_infos.iter().filter(|def| def.typ.is_none()) {
                    warning!(defs.location, "{} is unused and was not typechecked", defs.name);
                }
                for (_, module_id) in cache.modules.iter() {
                    let resolver = cache.name_resolvers.get_mut(module_id.0).unwrap();
                    for (name, definition_id) in resolver.exports.definitions.iter() {
                        let typ = cache.definition_infos[definition_id.0].typ.clone().unwrap();
                        println!("{} : {}", name, typ.display(&cache));
                    }
                }
            },
            Err(e) => {
                println!("{}", e);
            },
        }
    }

    Ok(())
}
