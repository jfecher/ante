#[macro_use]
mod combinators;
mod error;

#[macro_use]
pub mod ast;
pub mod pretty_printer;

use crate::lexer::{token::Token, Lexer};
use crate::error::location::Locatable;
use ast::{ Expr, Type, TypeDefinitionBody };
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
    let (mut lexer, _) = maybe_newline(lexer)?;

    if let Some(token) = lexer.next() {
        // unparsed input
        println!("Partial ast = {}", ast);
        println!("Failed on token: {:#?}", token);
        Err(ParseError::Fatal(Box::new(ParseError::InRule("statement".to_string(), lexer.locate()))))
    } else {
        Ok(ast)
    }
}

fn maybe_newline(input: Lexer) -> error::ParseResult<Option<Token>> {
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
                  | type_definition
                  | type_alias
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

parser!(type_definition loc =
    _ <- expect(Token::Type);
    name <- typename;
    args <- many0(identifier);
    _ <- expect(Token::Equal);
    body !<- type_definition_body;
    Expr::type_definition(name, args, body, loc, ())
);

parser!(type_alias loc =
    _ <- expect(Token::Type);
    name <- typename;
    args <- many0(identifier);
    _ <- expect(Token::Is);
    body !<- parse_type;
    Expr::type_definition(name, args, TypeDefinitionBody::AliasOf(body), loc, ())
);

choice!(type_definition_body -> ast::TypeDefinitionBody =
    union_block_body
    | union_inline_body
    | struct_block_body
    | struct_inline_body
);

parser!(union_variant loc -> Type =
    _ <- expect(Token::Pipe);
    variant !<- typename;
    args !<- many0(parse_type);
    if args.is_empty() {
        Type::UserDefinedType(variant, loc)
    } else {
        Type::TypeApplication(Box::new(Type::UserDefinedType(variant, loc)), args, loc)
    }
);

parser!(union_block_body _ -> ast::TypeDefinitionBody =
    _ <- expect(Token::Indent);
    variants <- delimited(union_variant, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    TypeDefinitionBody::UnionOf(variants)
);

parser!(union_inline_body _ -> ast::TypeDefinitionBody =
    variants <- many1(union_variant);
    TypeDefinitionBody::UnionOf(variants)
);

parser!(struct_field _ -> (&str, Type) =
    field_name <- identifier;
    _ !<- expect(Token::Colon);
    field_type !<- parse_type;
    (field_name, field_type)
);

parser!(struct_block_body _ -> ast::TypeDefinitionBody =
    _ <- expect(Token::Indent);
    fields <- delimited(struct_field, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    TypeDefinitionBody::StructOf(fields)
);

parser!(struct_inline_body _ -> ast::TypeDefinitionBody =
    fields <- delimited(struct_field, expect(Token::Comma));
    TypeDefinitionBody::StructOf(fields)
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
            let mut location = input.locate();
            let (input, lhs) = expression_chain(precedence + 1)(input)?;
            let (input, rhs) = many0(pair(
                expect_any(OPERATOR_PRECEDENCE[precedence]),
                no_backtracking(expression_chain(precedence + 1))
            ))(input)?;

            // Parsing the expression is done, now convert it into function calls
            let mut expr = lhs;
            for (op, rhs) in rhs {
                location = location.union(rhs.locate());
                expr = Expr::function_call(Expr::operator(op, location, ()), vec![expr, rhs], location, ());
            }
            Ok((input, expr))
        } else {
            term(input)
        }
    }
}

choice!(term = function_call
             | if_expr
             | match_expr
             | type_annotation
             | function_argument
);

parser!(function_call loc =
    function <- function_argument;
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

parser!(match_expr loc =
    _ <- expect(Token::Match);
    expression !<- block_or_expression;
    _ !<- maybe_newline;
    _ !<- expect(Token::With);
    branches !<- many0(match_branch);
    Expr::match_expr(expression, branches, loc, ())
);

parser!(type_annotation loc =
    lhs <- function_argument;
    _ <- expect(Token::Colon);
    rhs <- parse_type;
    Expr::type_annotation(lhs, rhs, loc, ())
);

choice!(parse_type -> ast::Type =
    type_application | basic_type
);

choice!(basic_type -> ast::Type =
    int_type
    | float_type
    | char_type
    | string_type
    | boolean_type
    | unit_type
    | reference_type
    | type_variable
    | user_defined_type
    | parenthsized_type
);

parser!(match_branch _ -> (Ast, Ast) =
    _ <- maybe_newline;
    _ <- expect(Token::Pipe);
    pattern !<- expression;
    _ !<- expect(Token::RightArrow);
    branch !<- block_or_expression;
    (pattern, branch)
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

parser!(type_application loc -> Type =
    type_constructor <- basic_type;
    args <- many1(basic_type);
    Type::TypeApplication(Box::new(type_constructor), args, loc)
);

parser!(int_type loc -> Type =
    _ <- expect(Token::IntegerType);
    Type::IntegerType(loc)
);

parser!(float_type loc -> Type =
    _ <- expect(Token::FloatType);
    Type::FloatType(loc)
);

parser!(char_type loc -> Type =
    _ <- expect(Token::CharType);
    Type::CharType(loc)
);

parser!(string_type loc -> Type =
    _ <- expect(Token::StringType);
    Type::StringType(loc)
);

parser!(boolean_type loc -> Type =
    _ <- expect(Token::BooleanType);
    Type::BooleanType(loc)
);

parser!(unit_type loc -> Type =
    _ <- expect(Token::UnitType);
    Type::UnitType(loc)
);

parser!(reference_type loc -> Type =
    _ <- expect(Token::Ref);
    Type::ReferenceType(loc)
);

parser!(type_variable loc -> Type =
    name <- identifier;
    Type::TypeVariable(name, loc)
);

parser!(user_defined_type loc -> Type =
    name <- typename;
    Type::UserDefinedType(name, loc)
);

parser!(parenthsized_type _ -> Type =
    _ <- expect(Token::ParenthesisLeft);
    inner_type <- parse_type;
    _ <- expect(Token::ParenthesisRight);
    inner_type
);
