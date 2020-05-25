#[macro_use]
mod combinators;
mod error;
pub mod ast;
pub mod pretty_printer;

use crate::lexer::{token::Token, Lexer};
use crate::error::location::{ EndPosition, Location, Locatable };
use ast::Expr;
use error::ParseError;
use combinators::*;

type AstResult<'a> = error::ParseResult<'a, Ast<'a>>;
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

pub fn parse(lexer: Lexer) -> Result<Ast, ParseError> {
    let (lexer, _) = maybe_newline(lexer)?;
    let (lexer, ast) = statement_list(lexer)?;
    let (lexer, _) = maybe_newline(lexer)?;

    let mut lexer = lexer.clone();
    if let Some(token) = lexer.next() {
        // unparsed input
        println!("Partial ast = {}", ast);
        println!("Failed on token: {:#?}", token);
        Err(ParseError::Fatal(Box::new(ParseError::InRule("statement".to_string(), lexer.locate()))))
    } else {
        Ok(ast)
    }
}

fn maybe_newline(input: Lexer) -> Result<(Lexer, Option<Token>), ParseError> {
    maybe(expect(Token::Newline))(input)
}

parser!(statement_list loc =
    first <- statement;
    rest <- many0(pair( expect(Token::Newline), statement ));
    if rest.is_empty() {
        first
    } else {
        let mut statements = vec![first];
        for (_, b) in rest.into_iter() {
            statements.push(b);
        }
        Expr::function_call(Expr::operator(Token::Semicolon, loc, ()), statements, loc, ())
    }
);

choice!(statement = function_definition
                  | variable_definition
                  | expression
);

parser!(function_definition loc =
    name <- variable;
    args <- many1(variable);
    _ <- expect(Token::Equal);
    body !<- block_or_expression;
    Expr::definition(name, Expr::lambda(args, body, loc, ()), loc, ())
);

parser!(variable_definition loc =
    name <- variable;
    _ <- expect(Token::Equal);
    body !<- block_or_expression;
    Expr::definition(name, body, loc, ())
);

choice!(block_or_expression = block
                            | expression
);

parser!(block _ =
    _ <- expect(Token::Indent);
    expr !<- statement_list;
    _ !<- maybe_newline;
    _ !<- expect(Token::Unindent);
    expr
);

fn expression(input: Lexer) -> AstResult {
    expression_chain(0)(input)
}

fn expression_chain(precedence: usize) -> impl Fn(Lexer) -> AstResult {
    move |input| {
        if precedence < OPERATOR_PRECEDENCE.len() - 1 {
            let start = input.get_start_position();
            let (input, lhs) = expression_chain(precedence + 1)(input)?;
            let (input, rhs) = many0(pair(
                expect_any(OPERATOR_PRECEDENCE[precedence]),
                expression_chain(precedence + 1)
            ))(input)?;

            // Parsing the expression is done, now convert it into function calls
            let mut expr = lhs;
            let mut location = Location::new(&input, start, EndPosition::new(start.index));
            for (op, rhs) in rhs {
                location = location.union(rhs.locate());
                expr = Expr::function_call(Expr::operator(op, location, ()), vec![expr, rhs], location, ());
            }
            Ok((input, expr))
        } else {
            expression_argument(input)
        }
    }
}

choice!(expression_argument = function_call
                            | if_expr
                            | function_argument
);

parser!(function_call loc =
    function <- variable;
    args <- many1(function_argument);
    Expr::function_call(function, args, loc, ())
);

parser!(if_expr loc =
    _ <- expect(Token::If);
    condition !<- block_or_expression;
    _ !<- maybe_newline;
    _ !<- expect(Token::Then);
    then !<- block_or_expression;
    otherwise !<- maybe(else_expr);
    Expr::if_expr(condition, then, otherwise, loc, ())
);

parser!(else_expr _ =
    _ <- maybe_newline;
    _ <- expect(Token::Else);
    otherwise !<- block_or_expression;
    otherwise
);

choice!(function_argument = variable
                          | string
                          | integer
                          | float
                          | parse_char
                          | parse_bool
                          | unit
                          | parenthsized_expression
                          | lambda
);

parser!(lambda loc =
    _ <- expect(Token::Backslash);
    args <- many1(variable);
    _ <- expect(Token::Equal);
    body <- block_or_expression;
    Expr::lambda(args, body, loc, ())
);

parser!(parenthsized_expression _ =
    _ <- expect(Token::ParenthesisLeft);
    expr <- expression;
    _ <- expect(Token::ParenthesisRight);
    expr
);

parser!(variable loc =
    name <- identifier;
    Expr::variable(name, loc, ())
);

parser!(string loc =
    contents <- string_literal_token;
    Expr::string(contents, loc, ())
);

parser!(integer loc =
    value <- integer_literal_token;
    Expr::integer(value, loc, ())
);

parser!(float loc =
    value <- float_literal_token;
    Expr::float(value, loc, ())
);

parser!(parse_char loc =
    contents <- char_literal_token;
    Expr::char_literal(contents, loc, ())
);

parser!(parse_bool loc =
    value <- bool_literal_token;
    Expr::bool_literal(value, loc, ())
);

parser!(unit loc =
    _ <- expect(Token::UnitLiteral);
    Expr::unit_literal(loc, ())
);
