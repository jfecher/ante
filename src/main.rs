use clap::{App, Arg};
use std::fs::File;
use std::io::{BufReader, Read};

mod parser;
mod expr;
mod lexer;
mod token;

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
        .arg(Arg::with_name("file").help("The file to compile").required(true))
        .get_matches();

    let filename = args.value_of("file").unwrap();

    let file = match File::open(filename) {
        Ok(file) => file,
        Err(_) => {
            println!("Could not open file {}", filename);
            return Ok(());
        }
    };

    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;

    let result = parser::parse(&contents);
    println!("{:#?}", result);

    Ok(())
}
