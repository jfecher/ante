use clap::{App, Arg};
use std::fs::File;
use std::io::{BufReader, Read};

mod parser;
mod lexer;
mod error;

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
        .arg(Arg::with_name("lex").long("lex").help("Lex the file and output the lexed tokens"))
        .arg(Arg::with_name("parse").long("parse").help("Parse the file and output the resulting Ast"))
        .arg(Arg::with_name("file").help("The file to compile").required(true))
        .get_matches();

    let filename = args.value_of("file").unwrap();

    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            println!("Could not open file {}", filename);
            return Err(Error::Unrecoverable);
        }
    };

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    let file = lexer::File { filename, contents: &contents };
    let keywords = lexer::Lexer::get_keywords();
    let tokens = lexer::Lexer::new(file, &keywords);

    if args.is_present("lex") {
        tokens.for_each(|token| println!("{}", token));
    } else if args.is_present("parse") {
        let result = parser::parse(tokens);
        match result {
            Ok(tree) => println!("{}", tree),
            Err(e) => println!("{}", e),
        }
    }

    Ok(())
}
