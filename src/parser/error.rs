//! parser/error.rs - Defines the ParseError type and the formatting shown
//! when printing this error to stderr.
use crate::lexer::token::{ Token, LexerError };
use crate::error::location::{ Location, Locatable };
use crate::error::ErrorMessage;
use crate::util::join_with;
use super::combinators::Input;
use std::fmt::Display;

#[derive(Debug)]
pub enum ParseError<'a> {
    /// A parsing error may not be fatal if it can be ignored because
    /// e.g. the parser is currently within an `or([...])` combinator that
    /// succeeds if any of the parsers in its array succeed.
    Fatal(Box<ParseError<'a>>),

    /// Expected any of the given tokens, but found... whatever is at the
    /// source Location instead
    Expected(Vec<Token>, Location<'a>),

    /// Failed while in the given parsing rule. E.g. "failed to parse a type".
    /// Due to backtracking this error is somewhat rare since the parser tends
    /// to backtrack trying to parse something else instead of failing in the
    /// rule that parsed the furthest. Proper usage of !<- (or `no_backtracking`)
    /// helps mediate this somewhat.
    InRule(&'static str, Location<'a>),

    /// Found a Token::Invalid issued by the lexer, containing some LexerError.
    /// These errors are always wrapped in a Fatal.
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
                    let expected = join_with(tokens, ", ");
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
