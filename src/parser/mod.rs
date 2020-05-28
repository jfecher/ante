#[macro_use]
mod combinators;
mod error;

#[macro_use]
pub mod ast;
pub mod pretty_printer;

use crate::lexer::token::Token;
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

pub fn parse(input: Input) -> Result<Ast, ParseError> {
    let (input, _, _) = maybe_newline(input)?;
    let (input, ast, _) = statement_list(input)?;
    let (input, _, _) = maybe_newline(input)?;
    let _ = expect(Token::EndOfInput)(input)?;
    Ok(ast)
}

fn maybe_newline(input: Input) -> error::ParseResult<Option<Token>> {
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

choice!(statement = definition
                  | type_definition
                  | type_alias
                  | import
                  | trait_definition
                  | trait_impl
                  | expression
);

fn definition(input: Input) -> AstResult {
    raw_definition(input).map(|(input, definition, location)|
            (input, Expr::Definition(definition), location))
}

choice!(raw_definition -> ast::Definition<()> = function_definition | variable_definition);

parser!(function_definition location -> ast::Definition<()> =
    name <- variable;
    args <- many1(variable);
    _ <- expect(Token::Equal);
    body !<- block_or_expression;
    ast::Definition {
        pattern: Box::new(name),
        expr: Box::new(Expr::lambda(args, body, location, ())),
        location,
        data: ()
    }
);

parser!(variable_definition location -> ast::Definition<()> =
    name <- variable;
    _ <- expect(Token::Equal);
    expr !<- block_or_expression;
    ast::Definition {
        pattern: Box::new(name),
        expr: Box::new(expr),
        location,
        data: ()
    }
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

parser!(union_block_body _loc -> ast::TypeDefinitionBody =
    _ <- expect(Token::Indent);
    variants <- delimited(union_variant, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    TypeDefinitionBody::UnionOf(variants)
);

parser!(union_inline_body _loc -> ast::TypeDefinitionBody =
    variants <- many1(union_variant);
    TypeDefinitionBody::UnionOf(variants)
);

parser!(struct_field _loc -> (&str, Type) =
    field_name <- identifier;
    _ !<- expect(Token::Colon);
    field_type !<- parse_type;
    (field_name, field_type)
);

parser!(struct_block_body _loc -> ast::TypeDefinitionBody =
    _ <- expect(Token::Indent);
    fields <- delimited(struct_field, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    TypeDefinitionBody::StructOf(fields)
);

parser!(struct_inline_body _loc -> ast::TypeDefinitionBody =
    fields <- delimited(struct_field, expect(Token::Comma));
    TypeDefinitionBody::StructOf(fields)
);

parser!(import loc =
    _ <- expect(Token::Import);
    path <- delimited(typename, expect(Token::MemberAccess));
    Expr::import(path, loc, ())
);

parser!(trait_definition loc =
    _ <- expect(Token::Trait);
    name !<- typename;
    args !<- many1(identifier);
    _ !<- maybe(expect(Token::RightArrow));
    fundeps !<- many0(identifier);
    _ !<- expect(Token::Indent);
    body !<- delimited(trait_function_definition, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    Expr::trait_definition(name, args, fundeps, body, loc, ())
);

parser!(trait_function_definition loc -> ast::TypeAnnotation<()> =
    lhs <- function_argument;
    _ <- expect(Token::Colon);
    rhs <- parse_type;
    ast::TypeAnnotation { lhs: Box::new(lhs), rhs, location: loc, data: () }
);

parser!(trait_impl loc =
    _ <- expect(Token::Impl);
    name !<- typename;
    args !<- many1(parse_type);
    _ !<- expect(Token::Indent);
    definitions !<- delimited(raw_definition, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    Expr::trait_impl(name, args, definitions, loc, ())
);

choice!(block_or_expression = block
                            | expression
);

parser!(block _loc =
    _ <- expect(Token::Indent);
    expr !<- statement_list;
    _ !<- maybe_newline;
    _ !<- expect(Token::Unindent);
    expr
);

fn expression(input: Input) -> AstResult {
    expression_chain(0)(input)
}

fn expression_chain(precedence: usize) -> impl Fn(Input) -> AstResult {
    move |input| {
        if precedence < OPERATOR_PRECEDENCE.len() - 1 {
            let mut location = input[0].1;
            let (input, lhs, _) = expression_chain(precedence + 1)(input)?;
            let (input, rhs, _) = many0(pair(
                expect_any(OPERATOR_PRECEDENCE[precedence]),
                no_backtracking(expression_chain(precedence + 1))
            ))(input)?;

            // Parsing the expression is done, now convert it into function calls
            let mut expr = lhs;
            for (op, rhs) in rhs {
                location = location.union(rhs.locate());
                expr = Expr::function_call(Expr::operator(op, location, ()), vec![expr, rhs], location, ());
            }
            Ok((input, expr, location))
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
    function_type
    | type_application
    | basic_type
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

parser!(match_branch _loc -> (Ast, Ast) =
    _ <- maybe_newline;
    _ <- expect(Token::Pipe);
    pattern !<- expression;
    _ !<- expect(Token::RightArrow);
    branch !<- block_or_expression;
    (pattern, branch)
);

parser!(else_expr _loc =
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
    _ <- expect(Token::MemberAccess);
    body <- block_or_expression;
    Expr::lambda(args, body, loc, ())
);

parser!(parenthsized_expression _loc =
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

parser!(function_type loc -> Type =
    args <- many1(basic_type);
    _ <- expect(Token::RightArrow);
    return_type <- parse_type;
    Type::FunctionType(args, Box::new(return_type), loc)
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

parser!(parenthsized_type _loc -> Type =
    _ <- expect(Token::ParenthesisLeft);
    inner_type <- parse_type;
    _ <- expect(Token::ParenthesisRight);
    inner_type
);
