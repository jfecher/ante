use clap::{App, Arg};
use std::fs::File;
use std::path::{ Path, PathBuf };
use std::io::{BufReader, Read};

#[macro_use]
mod parser;
mod lexer;

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
    let args = App::new("antec")
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
            Ok(mut root) => {
                NameResolver::resolve(&mut root, &mut cache);
            },
            Err(e) => {
                println!("{}", e);
            },
        }
    }

    Ok(())
}
