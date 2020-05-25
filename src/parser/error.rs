use crate::lexer::Lexer;
use crate::lexer::token::{ Token, LexerError };
use crate::error::location::{ Location, Locatable };
use std::fmt::Display;

#[derive(Debug)]
pub enum ParseError<'a> {
    Fatal(Box<ParseError<'a>>),
    Expected(Vec<Token<'a>>, Location<'a>),
    InRule(String, Location<'a>),
    LexerError(LexerError, Location<'a>),
}

pub type ParseResult<'a, T> = Result<(Lexer<'a>, T), ParseError<'a>>;

impl<'a> Locatable<'a> for ParseError<'a> {
    fn locate(&self) -> Location<'a> {
        match self {
            ParseError::Fatal(error) => error.locate(),
            ParseError::Expected(_, location) => *location,
            ParseError::InRule(_, location) => *location,
            ParseError::LexerError(_, location) => *location,
        }
    }
}

impl<'a> Display for ParseError<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            ParseError::Fatal(error) => error.fmt(fmt),
            ParseError::Expected(tokens, location) => {
                if tokens.len() == 1 {
                    location.fmt_error(fmt, format!("parser expected {} here", tokens[0]))
                } else {
                    let expected = tokens.iter().map(|x| format!("{}", x)).collect::<Vec<_>>().join(", ");
                    location.fmt_error(fmt, format!("parser expected one of {}", expected))
                }
            },
            ParseError::InRule(rule, location) => {
                location.fmt_error(fmt, format!("failed trying to parse a {}", rule))
            },
            ParseError::LexerError(error, location) => {
                location.fmt_error(fmt, format!("{}", error))
            },
        }
    }
}
