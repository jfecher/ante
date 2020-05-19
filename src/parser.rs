use nom::IResult;
use nom::character::complete::{one_of, none_of, char, alphanumeric0, digit1};
use nom::number::complete::double;
use nom::multi::{many0, many1, many_m_n};
use nom::combinator::value;
use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, tag};

use std::iter::FromIterator;

use crate::expr::*;

type ParseResult<'a> = IResult<&'a str, Expr<()>>;

pub fn parse(input: &str) -> Result<Module<()>, nom::Err<(&str, nom::error::ErrorKind)>> {
    let (input, call) = statement(input)?;
    let (input, _) = many0(eof_whitespace)(input)?;
    nom::combinator::not(none_of(""))(input)?;
    Ok(Module {
        definitions: vec![],
        contents: Box::new(call),
        data: (),
    })
}

fn statement(input: &str) -> ParseResult {
    alt((function_definition, function_call, argument))(input)
}

fn function_definition(input: &str) -> ParseResult {
    let (input, function) = variable(input)?;
    let (input, args) = many1(whitespace_and(variable))(input)?;
    let (input, _) = many0(whitespace)(input)?;
    let (input, _) = char('=')(input)?;
    let (input, body) = whitespace_and(statement)(input)?;
    Ok((input, Expr::definition(function, Expr::lambda(args, body, ()), ())))
}

fn function_call(input: &str) -> ParseResult {
    let (input, function) = variable(input)?;
    let (input, args) = many1(whitespace_and(argument))(input)?;
    Ok((input, Expr::function_call(function, args, ())))
}

fn whitespace_and(f: impl Fn(&str) -> ParseResult) -> impl Fn(&str) -> ParseResult {
    move |input| {
        let (input, _) = whitespace(input)?;
        f(input)
    }
}

fn argument(input: &str) -> ParseResult {
    alt((variable, integer, float, string))(input)
}

fn whitespace(input: &str) -> IResult<&str, Vec<char>> {
    many1(one_of(" \t\r"))(input)
}

fn eof_whitespace(input: &str) -> IResult<&str, Vec<char>> {
    many1(one_of(" \t\r\n"))(input)
}

fn variable(input: &str) -> ParseResult {
    let lowercase_char = one_of("abcdefghijklmnopqrstuvwxyz_");
    let (input, first_char) = many_m_n(1, 1, lowercase_char)(input)?;
    let (input, rest) = alphanumeric0(input)?;
    Ok((input, Expr::variable(first_char[0].to_string() + rest, ())))
}

fn integer(input: &str) -> ParseResult {
    let (input, num) = digit1(input)?;
    Ok((input, Expr::integer(num.parse().unwrap(), ())))
}

fn float(input: &str) -> ParseResult {
    let (input, num) = double(input)?;
    Ok((input, Expr::float(num, ())))
}

fn string(input: &str) -> ParseResult {
    let (input, _) = char('"')(input)?;
    let (input, contents) = many0(escaped_transform(none_of("\""), '\\', alt((
        value("\\", tag("\\")),
        value("\"", tag("\"")),
        value("n", tag("\n")),
        value("r", tag("\r")),
        value("t", tag("\t")),
        value("$", tag("$")),
    ))))(input)?;
    let (input, _) = char('"')(input)?;
    Ok((input, Expr::string(String::from_iter(contents), ())))
}
