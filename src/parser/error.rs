use crate::lexer::token::{ Token, LexerError };
use crate::error::location::{ Location, Locatable };
use crate::error::ErrorMessage;
use crate::util::join_with;
use super::combinators::Input;
use std::fmt::Display;

#[derive(Debug)]
pub enum ParseError<'a> {
    Fatal(Box<ParseError<'a>>),
    Expected(Vec<Token>, Location<'a>),
    InRule(String, Location<'a>),
    LexerError(LexerError, Location<'a>),
}

pub type ParseResult<'local, 'cache, T> = Result<(Input<'local, 'cache>, T, Location<'cache>), ParseError<'cache>>;

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
                    let msg = format!("parser expected {} here", tokens[0]);
                    write!(fmt, "{}", ErrorMessage::error(&msg[..], *location))
                } else {
                    let expected = join_with(&tokens, ", ");
                    let msg = format!("parser expected one of {}", expected);
                    write!(fmt, "{}", ErrorMessage::error(&msg[..], *location))
                }
            },
            ParseError::InRule(rule, location) => {
                let msg = format!("failed trying to parse a {}", rule);
                write!(fmt, "{}", ErrorMessage::error(&msg[..], *location))
            },
            ParseError::LexerError(error, location) => {
                write!(fmt, "{}", ErrorMessage::error(&error.to_string()[..], *location))
            },
        }
    }
}
