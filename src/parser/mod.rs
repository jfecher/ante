#[macro_use]
mod combinators;
pub mod ast;

use crate::lexer::{token::Token, Lexer};
use ast::Expr;
use combinators::*;

type AstResult<'a> = ParseResult<'a, Ast<'a>>;
type Ast<'a> = Expr<'a, ()>;

// Operator precedence, lowest to highest
const OPERATOR_PRECEDENCE: [&[Token]; 15] = [
    &[Token::Semicolon],
    &[Token::ApplyLeft],
    &[Token::ApplyRight],
    &[Token::Or],
    &[Token::And],
    &[Token::Not],
    &[Token::EqualEqual, Token::Is, Token::Isnt, Token::NotEqual, Token::GreaterThan, Token::LessThan, Token::GreaterThanOrEqual, Token::LessThanOrEqual],
    &[Token::In],
    &[Token::Append],
    &[Token::Range],
    &[Token::Add, Token::Subtract],
    &[Token::Multiply, Token::Divide, Token::Modulus],
    &[Token::Colon],
    &[Token::Index],
    &[Token::As],
];

pub fn parse(lexer: Lexer) -> Result<Ast, ()> {
    let (lexer, _) = many0(expect(Token::Newline))(lexer)?;
    let (lexer, ast) = statement_list(lexer)?;
    let (lexer, _) = many0(expect(Token::Newline))(lexer)?;

    if let Some(token) = lexer.clone().next() {
        // unparsed input
        println!("Partial ast = {:#?}", ast);
        println!("Failed on token: {:#?}", token);
        Err(())
    } else {
        Ok(ast)
    }
}

parser!(statement_list =
    first <- statement;
    rest <- many0(pair( expect(Token::Newline), statement ));
    if rest.is_empty() {
        first
    } else {
        let mut statements = vec![first];
        for (_, b) in rest.into_iter() {
            statements.push(b);
        }
        Expr::function_call(Expr::operator(Token::Semicolon, ()), statements, ())
    }
);

choice!(statement = function_definition
                  | expression
);

fn expression(input: Lexer) -> AstResult {
    expression_chain(0)(input)
}

fn expression_chain(precedence: usize) -> impl Fn(Lexer) -> AstResult {
    move |input| {
        if precedence < OPERATOR_PRECEDENCE.len() - 1 {
            let (input, lhs) = expression_chain(precedence + 1)(input)?;
            let (input, rhs) = many0(pair(
                expect_any(OPERATOR_PRECEDENCE[precedence]),
                expression_chain(precedence + 1)
            ))(input)?;

            // Parsing the expression is done, now convert it into function calls
            let mut expr = lhs;
            for (op, rhs) in rhs {
                expr = Expr::function_call(Expr::operator(op, ()), vec![expr, rhs], ());
            }
            Ok((input, expr))
        } else {
            or(&[
                function_call,
                parenthsized_expression,
                argument
            ])(input)
        }
    }
}

parser!(parenthsized_expression =
    _ <- expect(Token::ParenthesisLeft);
    expr <- expression;
    _ <- expect(Token::ParenthesisRight);
    expr
);

parser!(function_definition =
    name <- variable;
    args <- many1(variable);
    _ <- expect(Token::Equal);
    body <- expression;
    Expr::definition(name, Expr::lambda(args, body, ()), ())
);

parser!(function_call =
    function <- variable;
    args <- many1(argument);
    Expr::function_call(function, args, ())
);

choice!(argument = variable
                 | string
                 | integer
                 | float
                 | parse_char
                 | parse_bool
);

parser!(variable =
    name <- identifier;
    Expr::variable(name, ())
);

parser!(string =
    contents <- string_literal_token;
    Expr::string(contents, ())
);

parser!(integer =
    value <- integer_literal_token;
    Expr::integer(value, ())
);

parser!(float =
    value <- float_literal_token;
    Expr::float(value, ())
);

parser!(parse_char =
    contents <- char_literal_token;
    Expr::char_literal(contents, ())
);

parser!(parse_bool =
    value <- bool_literal_token;
    Expr::bool_literal(value, ())
);
