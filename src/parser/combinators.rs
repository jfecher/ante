use crate::lexer::{token::Token, Lexer};

pub type ParseResult<'a, T> = Result<(Lexer<'a>, T), ()>;

macro_rules! seq {
    ( $input:ident => $name:tt <- $y:expr ; $($rem:tt)* ) => ({
        let ($input, $name) = $y($input)?;
        seq!($input => $($rem)*)
    });
    ( $input:ident => $expr:expr )=>{
        Ok(($input, $expr))
    };
}

macro_rules! parser {
    ( $name:ident = $($body:tt )* ) => {
        fn $name(input: Lexer) -> AstResult {
            seq!(input => $($body)*)
        }
    };
}

pub fn or<'a, It, T, F>(functions: It) -> impl FnOnce(Lexer<'a>) -> ParseResult<'a, T> where
    It: IntoIterator<Item = F>,
    F: Fn(Lexer<'a>) -> ParseResult<'a, T>
{
    move |input| {
        for f in functions.into_iter() {
            if let Ok(c) = f(input.clone()) {
                return Ok(c);
            }
        }
        Err(())
    }
}

macro_rules! choice {
    ( $name:ident = $($body:tt )|* ) => {
        fn $name(input: Lexer) -> AstResult {
            self::or(&[
                $($body),*
            ])(input)
        }
    };
}

pub fn expect<'a>(expected: Token<'a>) -> impl Fn(Lexer<'a>) -> ParseResult<'a, Token<'a>> {
    use std::mem::discriminant;
    move |mut input| {
        match input.next() {
            Some(token) if discriminant(&expected) == discriminant(&token) => Ok((input, token)),
            _ => Err(()),
        }
    }
}

pub fn many0<'a, T, F>(f: F) -> impl Fn(Lexer<'a>) -> ParseResult<'a, Vec<T>>
    where F: Fn(Lexer<'a>) -> ParseResult<'a, T>
{
    move |initial_input| {
        let mut input = initial_input.clone();
        let mut results = Vec::new();
        while let Ok((lexer, t)) = f(input.clone()) {
            input = lexer;
            results.push(t);
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

        while let Ok((lexer, t)) = f(input.clone()) {
            input = lexer;
            results.push(t);
        }
        Ok((input, results))
    }
}

// Basic combinators for extracting the contents of a given token
pub fn identifier(mut lexer: Lexer) -> ParseResult<&str> {
    match lexer.next() {
        Some(Token::Identifier(name)) => Ok((lexer, name)),
        _ => Err(()),
    }
}

pub fn string_literal_token(mut lexer: Lexer) -> ParseResult<String> {
    match lexer.next() {
        Some(Token::StringLiteral(contents)) => Ok((lexer, contents)),
        _ => Err(()),
    }
}

pub fn integer_literal_token(mut lexer: Lexer) -> ParseResult<u64> {
    match lexer.next() {
        Some(Token::IntegerLiteral(int)) => Ok((lexer, int)),
        _ => Err(()),
    }
}

pub fn float_literal_token(mut lexer: Lexer) -> ParseResult<f64> {
    match lexer.next() {
        Some(Token::FloatLiteral(float)) => Ok((lexer, float)),
        _ => Err(()),
    }
}

pub fn char_literal_token(mut lexer: Lexer) -> ParseResult<char> {
    match lexer.next() {
        Some(Token::CharLiteral(contents)) => Ok((lexer, contents)),
        _ => Err(()),
    }
}

pub fn bool_literal_token(mut lexer: Lexer) -> ParseResult<bool> {
    match lexer.next() {
        Some(Token::BooleanLiteral(boolean)) => Ok((lexer, boolean)),
        _ => Err(()),
    }
}
