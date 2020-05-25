use crate::lexer::{ Lexer, token::Token };
use crate::error::location::{ Location, Locatable };
use std::fmt::Display;

#[derive(Debug)]
pub enum ParseError<'a> {
    Fatal(Box<ParseError<'a>>),
    Expected(Vec<Token<'a>>, Location<'a>),
    InRule(String, Location<'a>),
}

pub type ParseResult<'a, T> = Result<(Lexer<'a>, T), ParseError<'a>>;

impl<'a> Locatable<'a> for ParseError<'a> {
    fn locate(&self) -> Location<'a> {
        match self {
            ParseError::Fatal(error) => error.locate(),
            ParseError::Expected(_, location) => *location,
            ParseError::InRule(_, location) => *location,
        }
    }
}

impl<'a> Display for ParseError<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            ParseError::Fatal(error) => error.fmt(fmt),
            ParseError::Expected(tokens, location) => {
                location.fmt_error(fmt, &format!("parser expected one of {:?}", tokens))
            },
            ParseError::InRule(rule, location) => {
                location.fmt_error(fmt, &format!("failed trying to parse a {}", rule))
            },
        }
    }
}
