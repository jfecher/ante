#[macro_use]
mod combinators;
pub mod ast;

use crate::lexer::{token::Token, Lexer};
use ast::Expr;

use combinators::*;

type AstResult<'a> = ParseResult<'a, Ast<'a>>;

type Ast<'a> = Expr<'a, ()>;

pub fn parse(lexer: Lexer) -> Result<Ast, ()> {
    let (lexer, ast) = expression(lexer)?;
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

choice!(expression = function_definition
                   | function_call
                   | argument
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
