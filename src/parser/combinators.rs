use crate::lexer::{token::Token, Lexer};
use crate::error::location::{ Location, Locatable };
use super::error::{ ParseError, ParseResult };

macro_rules! seq {
    // monadic bind:
    // 
    // name <- parser;
    // rest
    ( $input:ident $start:ident $location:tt => $name:tt <- $y:expr ; $($rem:tt)* ) => ({
        let ($input, $name) = $y($input)?;
        seq!($input $start $location => $($rem)*)
    });
    // trace point for debugging:
    // 
    // trace arg;
    // rest
    ( $input:ident $start:ident $location:tt => trace $arg:expr ; $($rem:tt)* ) => ({
        println!("trace {} - next = {:?}", $arg, $input.clone().next());
        seq!($input $start $location => $($rem)*)
    });
    // Mark the expression no backtracking for better errors:
    // 
    // name <-! parser;
    // rest
    ( $input:ident $start:ident $location:tt => $name:tt !<- $y:expr ; $($rem:tt)* ) => ({
        let ($input, $name) = no_backtracking($y)($input)?;
        seq!($input $start $location => $($rem)*)
    });
    // Finish the seq by wrapping in an Ok
    ( $input:ident $start:ident $location:tt => $expr:expr ) => ({
        let end = $input.get_end_position();
        let $location = crate::error::location::Location::new(&$input, $start, end);
        Ok(($input, $expr))
    });
}

macro_rules! parser {
    ( $name:ident $location:tt = $($body:tt )* ) => {
        fn $name(input: Lexer) -> AstResult {
            let start = input.get_start_position();
            seq!(input start $location => $($body)*)
        }
    };
}

pub fn or<'a, It, T, F>(functions: It, rule: String) -> impl FnOnce(Lexer<'a>) -> ParseResult<'a, T> where
    It: IntoIterator<Item = F>,
    F: Fn(Lexer<'a>) -> ParseResult<'a, T>
{
    move |input| {
        for f in functions.into_iter() {
            match f(input.clone()) {
                Ok(c) => return Ok(c),
                Err(ParseError::Fatal(c)) => return Err(ParseError::Fatal(c)),
                _ => (),
            }
        }
        Err(ParseError::InRule(rule, input.locate()))
    }
}

macro_rules! choice {
    ( $name:ident = $($body:tt )|* ) => {
        fn $name(input: Lexer) -> AstResult {
            self::or(&[
                $($body),*
            ], stringify!($name).to_string())(input)
        }
    };
}

pub fn expect<'a>(expected: Token<'a>) -> impl Fn(Lexer<'a>) -> ParseResult<'a, Token<'a>> {
    use std::mem::discriminant;
    move |mut input| {
        let start = input.get_start_position();
        match input.next() {
            Some(token) if discriminant(&expected) == discriminant(&token) => Ok((input, token)),
            _ => {
                let end = input.get_end_position();
                let location = Location::new(&input, start, end);
                Err(ParseError::Expected(vec![expected.clone()], location))
            }
        }
    }
}

pub fn expect_any<'a>(expected: &'a [Token<'a>]) -> impl Fn(Lexer<'a>) -> ParseResult<'a, Token<'a>> {
    move |mut input| {
        let start = input.get_start_position();
        match input.next() {
            Some(token) if expected.into_iter().find(|tok| **tok == token).is_some() => Ok((input, token)),
            _ => {
                let end = input.get_end_position();
                let location = Location::new(&input, start, end);
                Err(ParseError::Expected(expected.iter().cloned().collect(), location))
            }
        }
    }
}

pub fn maybe<'a, F, T>(f: F) -> impl Fn(Lexer<'a>) -> ParseResult<'a, Option<T>>
    where F: Fn(Lexer<'a>) -> ParseResult<'a, T>
{
    move |input| {
        match f(input.clone()) {
            Ok((input, result)) => Ok((input, Some(result))),
            Err(ParseError::Fatal(err)) => Err(ParseError::Fatal(err)),
            Err(_) => Ok((input, None)),
        }
    }
}

pub fn pair<'a, F, G, FResult, GResult>(f: F, g: G) -> impl Fn(Lexer<'a>) -> ParseResult<'a, (FResult, GResult)> where
    F: Fn(Lexer<'a>) -> ParseResult<'a, FResult>,
    G: Fn(Lexer<'a>) -> ParseResult<'a, GResult>
{
    move |input| {
        let (input, fresult) = f(input)?;
        let (input, gresult) = g(input)?;
        Ok((input, (fresult, gresult)))
    }
}

pub fn many0<'a, T, F>(f: F) -> impl Fn(Lexer<'a>) -> ParseResult<'a, Vec<T>>
    where F: Fn(Lexer<'a>) -> ParseResult<'a, T>
{
    move |initial_input| {
        let mut input = initial_input.clone();
        let mut results = Vec::new();
        loop {
            match f(input.clone()) {
                Ok((lexer, t)) => {
                    input = lexer;
                    results.push(t);
                }
                Err(ParseError::Fatal(c)) => return Err(ParseError::Fatal(c)),
                _ => break,
            }
        }
        Ok((input, results))
    }
}

pub fn many1<'a, T, F>(f: F) -> impl Fn(Lexer<'a>) -> ParseResult<'a, Vec<T>>
    where F: Fn(Lexer<'a>) -> ParseResult<'a, T>
{
    move |initial_input| {
        let mut input = initial_input.clone();
        let mut results = Vec::new();

        match f(input.clone()) {
            Ok((lexer, t)) => {
                input = lexer;
                results.push(t);
            },
            Err(e) => return Err(e),
        }

        loop {
            match f(input.clone()) {
                Ok((lexer, t)) => {
                    input = lexer;
                    results.push(t);
                },
                Err(ParseError::Fatal(token)) => return Err(ParseError::Fatal(token)),
                Err(_) => break,
            }
        }
        Ok((input, results))
    }
}

pub fn no_backtracking<'a, T, F>(f: F) -> impl Fn(Lexer<'a>) -> ParseResult<'a, T>
    where F: Fn(Lexer<'a>) -> ParseResult<'a, T>
{
    move |input| {
        f(input).map_err(|e| match e {
            ParseError::Fatal(token) => ParseError::Fatal(token),
            err => ParseError::Fatal(Box::new(err)),
        })
    }
}

// Basic combinators for extracting the contents of a given token
pub fn identifier(mut input: Lexer) -> ParseResult<&str> {
    let start = input.get_start_position();
    match input.next() {
        Some(Token::Identifier(name)) => Ok((input, name)),
        Some(Token::Invalid(c)) => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        _ => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Expected(vec![Token::Identifier("")], location))
        },
    }
}

pub fn string_literal_token(mut input: Lexer) -> ParseResult<String> {
    let start = input.get_start_position();
    match input.next() {
        Some(Token::StringLiteral(contents)) => Ok((input, contents)),
        Some(Token::Invalid(c)) => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        _ => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Expected(vec![Token::StringLiteral("".to_owned())], location))
        },
    }
}

pub fn integer_literal_token(mut input: Lexer) -> ParseResult<u64> {
    let start = input.get_start_position();
    match input.next() {
        Some(Token::IntegerLiteral(int)) => Ok((input, int)),
        Some(Token::Invalid(c)) => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        _ => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Expected(vec![Token::IntegerLiteral(0)], location))
        },
    }
}

pub fn float_literal_token(mut input: Lexer) -> ParseResult<f64> {
    let start = input.get_start_position();
    match input.next() {
        Some(Token::FloatLiteral(float)) => Ok((input, float)),
        Some(Token::Invalid(c)) => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        _ => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Expected(vec![Token::FloatLiteral(0.0)], location))
        },
    }
}

pub fn char_literal_token(mut input: Lexer) -> ParseResult<char> {
    let start = input.get_start_position();
    match input.next() {
        Some(Token::CharLiteral(contents)) => Ok((input, contents)),
        Some(Token::Invalid(c)) => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        _ => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Expected(vec![Token::CharLiteral(' ')], location))
        },
    }
}

pub fn bool_literal_token(mut input: Lexer) -> ParseResult<bool> {
    let start = input.get_start_position();
    match input.next() {
        Some(Token::BooleanLiteral(boolean)) => Ok((input, boolean)),
        Some(Token::Invalid(c)) => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        _ => {
            let end = input.get_end_position();
            let location = Location::new(&input, start, end);
            Err(ParseError::Expected(vec![Token::BooleanLiteral(true)], location))
        },
    }
}
