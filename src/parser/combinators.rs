use crate::lexer::token::Token;
use crate::error::location::Location;
use super::error::{ ParseError, ParseResult };

pub type Input<'local, 'cache> = &'local[(Token, Location<'cache>)];

/// Helper macro for parser!
macro_rules! seq {
    // monadic bind:
    // 
    // name <- parser;
    // rest
    ( $input:ident $location:tt => $name:tt <- $y:expr ; $($rem:tt)* ) => ({
        let ($input, $name, start) = $y($input)?;
        seq!($input start start $location => $($rem)*)
    });
    ( $input:ident $start:ident $e:ident $location:tt => $name:tt <- $y:expr ; $($rem:tt)* ) => ({
        let ($input, $name, _end) = $y($input)?;
        seq!($input $start _end $location => $($rem)*)
    });
    // trace point for debugging:
    // 
    // trace arg;
    // rest
    ( $input:ident $start:ident $end:ident $location:tt => trace $arg:expr ; $($rem:tt)* ) => ({
        println!("trace {} - next = {:?}", $arg, $input[0].clone());
        seq!($input $start $end $location => $($rem)*)
    });
    // Mark the expression no backtracking for better errors:
    // 
    // name <-! parser;
    // rest
    ( $input:ident $start:ident $e:ident $location:tt => $name:tt !<- $y:expr ; $($rem:tt)* ) => ({
        let ($input, $name, _end) = no_backtracking($y)($input)?;
        seq!($input $start _end $location => $($rem)*)
    });
    // Finish the seq by wrapping in an Ok
    ( $input:ident $start:ident $end:ident $location:tt => $expr:expr ) => ({
        let $location = $start.union($end);
        Ok(($input, $expr, $location))
    });
}

/// Defines a sequenced parser function with do-notation, threading
/// the input at each step and unwrapping the result with `?`.
/// In addition to `lhs <- rhs;` performing the monadic bind, there
/// is `lhs !<- rhs;` which is equivalent to `lhs <- no_backtracking(rhs);`.
/// The final expression given is wrapped in an `Ok((input, expr))`
///
/// for example:
/// ```
/// parser!(basic_definition loc =
///     name <- variable;
///     _ <- expect(Token::Equal);
///     value !<- expression;
///     Expr::definition(name, value, loc, ())
/// )
/// ```
macro_rules! parser {
    ( $name:ident $location:tt -> $lt:tt $return_type:ty = $($body:tt )* ) => {
        fn $name<'a, $lt>(input: $crate::parser::combinators::Input<'a, $lt>) -> error::ParseResult<'a, $lt, $return_type> {
            seq!(input $location => $($body)*)
        }
    };
    // Variant with implicit return type of ParseResult<Ast>
    ( $name:ident $location:tt = $($body:tt )* ) => {
        parser!($name $location -> 'b Ast<'b> = $($body)* );
    };
}

/// Matches the input if any of the given parsers matches.
/// This backtracks after each parse so for better error messages, no_backtracking
/// should be used in each contained parser once it is sure that parser's rule
/// should be matched. For example, in an if expression, everything after the initial `if`
/// should be marked as no_backtracking.
pub fn or<'local, 'cache: 'local, It, T, F>(functions: It, rule: &'static str) -> impl FnOnce(Input<'local, 'cache>) -> ParseResult<'local, 'cache, T> where
    It: IntoIterator<Item = F>,
    F: Fn(Input<'local, 'cache>) -> ParseResult<'local, 'cache, T>
{
    move |input| {
        for f in functions.into_iter() {
            match f(input) {
                Ok(c) => return Ok(c),
                Err(ParseError::Fatal(c)) => return Err(ParseError::Fatal(c)),
                _ => (),
            }
        }

        assert!(!input.is_empty());

        match input[0] {
            (Token::Invalid(err), location) => Err(ParseError::Fatal(Box::new(ParseError::LexerError(err, location)))),
            (_, location) => Err(ParseError::InRule(rule, location))
        }
    }
}

/// Fail if the next token in the stream is not the given expected token
pub fn expect<'a, 'b: 'a>(expected: Token) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, Token> {
    use std::mem::discriminant;
    move |input| {
        if discriminant(&expected) == discriminant(&input[0].0) {
            Ok((&input[1..], input[0].0.clone(), input[0].1))
        } else if let Token::Invalid(err) = input[0].0 {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(err, input[0].1))))
        } else {
            Err(ParseError::Expected(vec![expected.clone()], input[0].1))
        }
    }
}

/// Fail if the next token in the stream is not the given expected token
pub fn expect_if<'a, 'b: 'a, F>(rule: &'static str, f: F) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, Token>
    where F: Fn(&Token) -> bool
{
    move |input| {
        if f(&input[0].0) {
            Ok((&input[1..], input[0].0.clone(), input[0].1))
        } else if let Token::Invalid(err) = input[0].0 {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(err, input[0].1))))
        } else {
            Err(ParseError::InRule(rule, input[0].1))
        }
    }
}

/// Matches the input 0 or 1 times. Only fails if a ParseError::Fatal is found
pub fn maybe<'a, 'b: 'a, F, T>(f: F) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, Option<T>>
    where F: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, T>
{
    move |input| {
        match f(input) {
            Ok((input, result, loc)) => Ok((input, Some(result), loc)),
            Err(ParseError::Fatal(err)) => Err(ParseError::Fatal(err)),
            Err(_) => Ok((input, None, input[0].1)),
        }
    }
}

/// Parse the two functions in a sequence, returning a pair of their results
pub fn pair<'a, 'b: 'a, F, G, FResult, GResult>(f: F, g: G) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, (FResult, GResult)> where
    F: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, FResult>,
    G: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, GResult>
{
    move |input| {
        let (input, fresult, loc1) = f(input)?;
        let (input, gresult, loc2) = g(input)?;
        Ok((input, (fresult, gresult), loc1.union(loc2)))
    }
}
/// Match f at least once, then match many0(g, f)
pub fn delimited<'a, 'b: 'a, F, G, FResult, GResult>(f: F, g: G) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, Vec<FResult>> where
    F: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, FResult>,
    G: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, GResult>
{
    move |mut input| {
        let mut results = Vec::new();
        let start = input[0].1;
        let mut end;

        match f(input) {
            Ok((new_input, t, location)) => {
                input = new_input;
                end = location;
                results.push(t);
            },
            Err(e) => return Err(e),
        }

        loop {
            match g(input) {
                Ok((new_input, _, _)) => input = new_input,
                Err(ParseError::Fatal(token)) => return Err(ParseError::Fatal(token)),
                Err(_) => break,
            }
            match f(input) {
                Ok((new_input, t, location)) => {
                    input = new_input;
                    end = location;
                    results.push(t);
                },
                Err(ParseError::Fatal(token)) => return Err(ParseError::Fatal(token)),
                Err(e) => return Err(e),
            }
        }

        let location = start.union(end);
        Ok((input, results, location))
    }
}

/// Match delimited(f, g) followed by an optional trailing g
pub fn delimited_trailing<'a, 'b: 'a, F, G, FResult, GResult>(f: F, g: G) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, Vec<FResult>> where
    F: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, FResult>,
    G: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, GResult>
{
    move |mut input| {
        let mut results = Vec::new();
        let start = input[0].1;
        let mut end;

        match f(input) {
            Ok((new_input, t, location)) => {
                input = new_input;
                end = location;
                results.push(t);
            },
            Err(e) => return Err(e),
        }

        loop {
            match g(input) {
                Ok((new_input, _, _)) => input = new_input,
                Err(ParseError::Fatal(token)) => return Err(ParseError::Fatal(token)),
                Err(_) => break,
            }
            match f(input) {
                Ok((new_input, t, location)) => {
                    input = new_input;
                    end = location;
                    results.push(t);
                },
                Err(ParseError::Fatal(token)) => return Err(ParseError::Fatal(token)),
                Err(_) => break,
            }
        }

        let location = start.union(end);
        Ok((input, results, location))
    }
}

/// Match begin, middle, then end in a sequence.
pub fn bounded<'a, 'b: 'a, F, FResult>(begin: Token, f: F, end: Token) -> impl FnOnce(Input<'a, 'b>) -> ParseResult<'a, 'b, FResult> where
    F: FnOnce(Input<'a, 'b>) -> ParseResult<'a, 'b, FResult>,
{
    move |input| {
        let (input, _, _) = expect(begin)(input)?;
        let (input, result, location) = f(input)?;
        let (input, _, _) = expect(end)(input)?;
        Ok((input, result, location))
    }
}

/// parenthesized f = bounded '(' f ')'
pub fn parenthesized<'a, 'b: 'a, F, FResult>(f: F) -> impl FnOnce(Input<'a, 'b>) -> ParseResult<'a, 'b, FResult> where
    F: FnOnce(Input<'a, 'b>) -> ParseResult<'a, 'b, FResult>,
{
    bounded(Token::ParenthesisLeft, f, Token::ParenthesisRight)
}

/// Runs the parser 0 or more times until it errors, then returns a Vec of the successes.
/// Will only return Err when a ParseError::Fatal is found
pub fn many0<'a, 'b: 'a, T, F>(f: F) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, Vec<T>>
    where F: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, T>
{
    move |mut input| {
        let mut results = Vec::new();
        let start = input[0].1;
        let mut end = start;

        loop {
            match f(input) {
                Ok((new_input, t, location)) => {
                    input = new_input;
                    end = location;
                    results.push(t);
                }
                Err(ParseError::Fatal(c)) => return Err(ParseError::Fatal(c)),
                _ => break,
            }
        }
        Ok((input, results, start.union(end)))
    }
}

/// Runs the parser 1 or more times until it errors, then returns a Vec of the successes.
/// Will return Err if the parser fails the first time or a ParseError::Fatal is found
pub fn many1<'a, 'b: 'a, T, F>(f: F) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, Vec<T>>
    where F: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, T>
{
    move |mut input| {
        let mut results = Vec::new();
        let start = input[0].1;
        let mut end;

        match f(input) {
            Ok((new_input, t, location)) => {
                input = new_input;
                end = location;
                results.push(t);
            },
            Err(e) => return Err(e),
        }

        loop {
            match f(input) {
                Ok((new_input, t, location)) => {
                    input = new_input;
                    end = location;
                    results.push(t);
                },
                Err(ParseError::Fatal(token)) => return Err(ParseError::Fatal(token)),
                Err(_) => break,
            }
        }
        Ok((input, results, start.union(end)))
    }
}

/// Wraps the parser in a ParseError::Fatal if it fails. Used for better error reporting
/// around `or` and similar combinators to prevent backtracking away from an error.
pub fn no_backtracking<'a, 'b: 'a, T, F>(f: F) -> impl Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, T>
    where F: Fn(Input<'a, 'b>) -> ParseResult<'a, 'b, T>
{
    move |input| {
        f(input).map_err(|e| match e {
            ParseError::Fatal(token) => ParseError::Fatal(token),
            err => ParseError::Fatal(Box::new(err)),
        })
    }
}

// Basic combinators for extracting the contents of a given token
pub fn identifier<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, String> {
    match &input[0] {
        (Token::Identifier(name), location) => Ok((&input[1..], name.clone(), *location)),
        (Token::Invalid(c), location) => {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(*c, *location))))
        },
        (_, location) => {
            Err(ParseError::Expected(vec![Token::Identifier("identifier".to_owned())], *location))
        },
    }
}

pub fn typename<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, String> {
    match &input[0] {
        (Token::TypeName(name), location) => Ok((&input[1..], name.clone(), *location)),
        (Token::Invalid(c), location) => {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(*c, *location))))
        },
        (_, location) => {
            Err(ParseError::Expected(vec![Token::TypeName("type name".to_owned())], *location))
        },
    }
}

pub fn string_literal_token<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, String> {
    match &input[0] {
        (Token::StringLiteral(contents), location) => Ok((&input[1..], contents.clone(), *location)),
        (Token::Invalid(c), location) => {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(*c, *location))))
        },
        (_, location) => {
            Err(ParseError::Expected(vec![Token::StringLiteral("".to_owned())], *location))
        },
    }
}

pub fn integer_literal_token<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, u64> {
    match input[0] {
        (Token::IntegerLiteral(int), location) => Ok((&input[1..], int, location)),
        (Token::Invalid(c), location) => {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        (_, location) => {
            Err(ParseError::Expected(vec![Token::IntegerLiteral(0)], location))
        },
    }
}

pub fn float_literal_token<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, f64> {
    match input[0] {
        (Token::FloatLiteral(float), location) => Ok((&input[1..], float, location)),
        (Token::Invalid(c), location) => {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        (_, location) => {
            Err(ParseError::Expected(vec![Token::FloatLiteral(0.0)], location))
        },
    }
}

pub fn char_literal_token<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, char> {
    match input[0] {
        (Token::CharLiteral(contents), location) => Ok((&input[1..], contents, location)),
        (Token::Invalid(c), location) => {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        (_, location) => {
            Err(ParseError::Expected(vec![Token::CharLiteral(' ')], location))
        },
    }
}

pub fn bool_literal_token<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, bool> {
    match input[0] {
        (Token::BooleanLiteral(boolean), location) => Ok((&input[1..], boolean, location)),
        (Token::Invalid(c), location) => {
            Err(ParseError::Fatal(Box::new(ParseError::LexerError(c, location))))
        },
        (_, location) => {
            Err(ParseError::Expected(vec![Token::BooleanLiteral(true)], location))
        },
    }
}
