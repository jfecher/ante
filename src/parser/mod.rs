#[macro_use]
mod combinators;
mod error;

#[macro_use]
pub mod ast;
pub mod pretty_printer;

use crate::lexer::token::Token;
use ast::{ Ast, Type, TypeDefinitionBody };
use error::{ ParseError, ParseResult };
use combinators::*;

type AstResult<'a> = ParseResult<'a, Ast<'a>>;

pub fn parse(input: Input) -> Result<Ast, ParseError> {
    let (input, _, _) = maybe_newline(input)?;
    let (input, ast, _) = statement_list(input)?;
    let (input, _, _) = maybe_newline(input)?;
    let _ = expect(Token::EndOfInput)(input)?;
    Ok(ast)
}

fn maybe_newline(input: Input) -> ParseResult<Option<Token>> {
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
        Ast::function_call(Ast::operator(Token::Semicolon, loc), statements, loc)
    }
);

fn statement(input: Input) -> AstResult {
    match input[0].0 {
        Token::ParenthesisLeft |
        Token::Identifier(_) => or(&[definition, expression], "statement".to_string())(input),
        Token::Type => or(&[type_definition, type_alias], "statement".to_string())(input),
        Token::Import => import(input),
        Token::Trait => trait_definition(input),
        Token::Impl => trait_impl(input),
        Token::Return => return_expr(input),
        _ => expression(input),
    }
}

fn definition(input: Input) -> AstResult {
    raw_definition(input).map(|(input, definition, location)|
            (input, Ast::Definition(definition), location))
}

fn raw_definition(input: Input) -> ParseResult<ast::Definition> {
    or(&[function_definition, variable_definition], "definition".to_string())(input)
}

parser!(function_definition location -> ast::Definition =
    name <- irrefutable_pattern;
    args <- many1(variable);
    _ <- expect(Token::Equal);
    body !<- block_or_expression;
    ast::Definition {
        pattern: Box::new(name),
        expr: Box::new(Ast::lambda(args, body, location)),
        location,
        info: None,
        typ: None,
    }
);

parser!(variable_definition location -> ast::Definition =
    name <- irrefutable_pattern;
    _ <- expect(Token::Equal);
    expr !<- block_or_expression;
    ast::Definition {
        pattern: Box::new(name),
        expr: Box::new(expr),
        location,
        info: None,
        typ: None,
    }
);

fn irrefutable_pattern(input: Input) -> AstResult {
    match input[0].0 {
        Token::ParenthesisLeft => parenthsized_operator(input),
        _ => variable(input),
    }
}

parser!(type_definition loc =
    _ <- expect(Token::Type);
    name <- typename;
    args <- many0(identifier);
    _ <- expect(Token::Equal);
    body !<- type_definition_body;
    Ast::type_definition(name, args, body, loc)
);

parser!(type_alias loc =
    _ <- expect(Token::Type);
    name <- typename;
    args <- many0(identifier);
    _ <- expect(Token::Is);
    body !<- parse_type;
    Ast::type_definition(name, args, TypeDefinitionBody::AliasOf(body), loc)
);

fn type_definition_body(input: Input) -> ParseResult<ast::TypeDefinitionBody> {
    match input[0].0 {
        Token::Indent => or(&[union_block_body, struct_block_body], "type_definition_body".to_string())(input),
        Token::Pipe => union_inline_body(input),
        _ => struct_inline_body(input),
    }
}

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
    Ast::import(path, loc)
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
    Ast::trait_definition(name, args, fundeps, body, loc)
);

parser!(trait_function_definition loc -> ast::TypeAnnotation =
    lhs <- irrefutable_pattern;
    _ <- expect(Token::Colon);
    rhs <- parse_type;
    ast::TypeAnnotation { lhs: Box::new(lhs), rhs, location: loc, typ: None }
);

parser!(trait_impl loc =
    _ <- expect(Token::Impl);
    name !<- typename;
    args !<- many1(parse_type);
    _ !<- expect(Token::Indent);
    definitions !<- delimited(raw_definition, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    Ast::trait_impl(name, args, definitions, loc)
);

parser!(return_expr loc =
    _ <- expect(Token::Return);
    expr !<- expression;
    Ast::return_expr(expr, loc)
);

fn block_or_expression(input: Input) -> AstResult {
    match input[0].0 {
        Token::Indent => block(input),
        _ => expression(input),
    }
}

parser!(block _loc =
    _ <- expect(Token::Indent);
    expr !<- statement_list;
    _ !<- maybe_newline;
    _ !<- expect(Token::Unindent);
    expr
);

fn expression(input: Input) -> AstResult {
    shunting_yard(input)
}

fn precedence(token: &Token) -> Option<i8> {
    match token {
        Token::Semicolon => Some(0),
        Token::ApplyLeft => Some(1),
        Token::ApplyRight => Some(2),
        Token::Or => Some(3),
        Token::And => Some(4),
        Token::Not => Some(5),
        Token::EqualEqual | Token::Is | Token::Isnt | Token::NotEqual | Token::GreaterThan | Token::LessThan | Token::GreaterThanOrEqual | Token::LessThanOrEqual => Some(6),
        Token::In => Some(7),
        Token::Append => Some(8),
        Token::Range => Some(9),
        Token::Add | Token::Subtract => Some(10),
        Token::Multiply | Token::Divide | Token::Modulus => Some(11),
        Token::Colon => Some(12),
        Token::Index => Some(13),
        Token::As => Some(14),
        _ => None,
    }
}

fn shunting_yard(input: Input) -> AstResult {
    let (mut input, value, location) = term(input)?;

    let mut operator_stack = vec![];
    let mut results = vec![(value, location)];

    // loop while the next token is an operator
    while let Some(prec) = precedence(&input[0].0) {
        while !operator_stack.is_empty() && precedence(operator_stack[operator_stack.len()- 1]).unwrap() >= prec {
            let (rhs, rhs_location) = results.pop().unwrap();
            let (lhs, lhs_location) = results.pop().unwrap();
            let location = lhs_location.union(rhs_location);
            let operator = Ast::operator(operator_stack.pop().unwrap().clone(), location);
            let call = Ast::function_call(operator, vec![lhs, rhs], location);
            results.push((call, location));
        }

        operator_stack.push(&input[0].0);
        input = &input[1..];

        let (new_input, value, location) = no_backtracking(term)(input)?;
        results.push((value, location));
        input = new_input;
    }

    while !operator_stack.is_empty() {
        assert!(results.len() >= 2);
        let (rhs, rhs_location) = results.pop().unwrap();
        let (lhs, lhs_location) = results.pop().unwrap();
        let location = lhs_location.union(rhs_location);
        let operator = Ast::operator(operator_stack.pop().unwrap().clone(), location);
        let call = Ast::function_call(operator, vec![lhs, rhs], location);
        results.push((call, location));
    }

    assert!(operator_stack.is_empty());
    assert!(results.len() == 1);
    let (value, location) = results.pop().unwrap();
    Ok((input, value, location))

    // let mut lhs_precedence = 0;
    // if precedence < OPERATOR_PRECEDENCE.len() - 1 {
    //     let mut location = input[0].1;
    //     let (input, lhs, _) = expression_chain(precedence + 1)(input)?;
    //     let (input, rhs, _) = many0(pair(
    //         expect_any(OPERATOR_PRECEDENCE[precedence]),
    //         no_backtracking(expression_chain(precedence + 1))
    //     ))(input)?;

    //     // Parsing the expression is done, now convert it into function calls
    //     let mut expr = lhs;
    //     for (op, rhs) in rhs {
    //         location = location.union(rhs.locate());
    //         expr = Expr::function_call(Expr::operator(op, location), vec![expr, rhs], location);
    //     }
    //     Ok((input, expr, location))
    // } else {
    //     term(input)
    // }
}

fn term(input: Input) -> AstResult {
    match input[0].0 {
        Token::If => if_expr(input),
        Token::Match => match_expr(input),
        Token::Not => not_expr(input),
        Token::Ampersand => ref_expr(input),
        _ => or(&[
            function_call,
            type_annotation,
            function_argument
        ], "term".to_string())(input),
    }
}

parser!(function_call loc =
    function <- function_argument;
    args <- many1(function_argument);
    Ast::function_call(function, args, loc)
);

parser!(if_expr loc =
    _ <- expect(Token::If);
    condition !<- block_or_expression;
    _ !<- maybe_newline;
    _ !<- expect(Token::Then);
    then !<- block_or_expression;
    otherwise !<- maybe(else_expr);
    Ast::if_expr(condition, then, otherwise, loc)
);

parser!(match_expr loc =
    _ <- expect(Token::Match);
    expression !<- block_or_expression;
    _ !<- maybe_newline;
    _ !<- expect(Token::With);
    branches !<- many0(match_branch);
    Ast::match_expr(expression, branches, loc)
);

parser!(not_expr loc =
    not <- expect(Token::Not);
    expr !<- term;
    Ast::function_call(Ast::operator(not, loc), vec![expr], loc)
);

parser!(ref_expr loc =
    token <- expect(Token::Ampersand);
    expr !<- term;
    Ast::function_call(Ast::operator(token, loc), vec![expr], loc)
);

parser!(type_annotation loc =
    lhs <- function_argument;
    _ <- expect(Token::Colon);
    rhs <- parse_type;
    Ast::type_annotation(lhs, rhs, loc)
);

fn parse_type(input: Input) -> ParseResult<ast::Type> {
    or(&[
        function_type,
        type_application,
        basic_type
    ], "type".to_string())(input)
}

fn basic_type(input: Input) -> ParseResult<ast::Type> {
    match input[0].0 {
        Token::IntegerType => int_type(input),
        Token::FloatType => float_type(input),
        Token::CharType => char_type(input),
        Token::StringType => string_type(input),
        Token::BooleanType => boolean_type(input),
        Token::UnitType => unit_type(input),
        Token::Ref => reference_type(input),
        Token::Identifier(_) => type_variable(input),
        Token::TypeName(_) => user_defined_type(input),
        Token::ParenthesisLeft => parenthsized_type(input),
        _ => Err(ParseError::InRule("type".to_string(), input[0].1)),
    }
}

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

fn function_argument(input: Input) -> AstResult {
    match input[0].0 {
        Token::Identifier(_) => variable(input),
        Token::StringLiteral(_) => string(input),
        Token::IntegerLiteral(_) => integer(input),
        Token::FloatLiteral(_) => float(input),
        Token::CharLiteral(_) => parse_char(input),
        Token::BooleanLiteral(_) => parse_bool(input),
        Token::UnitLiteral => unit(input),
        Token::Backslash => lambda(input),
        Token::ParenthesisLeft => {
            if input[1].0.is_overloadable_operator() {
                parenthsized_operator(input)
            } else {
                parenthsized_expression(input)
            }
        },
        _ => Err(ParseError::InRule("argument".to_string(), input[0].1)),
    }
}

parser!(lambda loc =
    _ <- expect(Token::Backslash);
    args <- many1(variable);
    _ <- expect(Token::MemberAccess);
    body <- block_or_expression;
    Ast::lambda(args, body, loc)
);

parser!(parenthsized_operator loc =
    _ <- expect(Token::ParenthesisLeft);
    op <- expect_if("parenthsized_operator", |op| op.is_overloadable_operator());
    _ <- expect(Token::ParenthesisRight);
    Ast::operator(op, loc)
);

parser!(parenthsized_expression loc =
    _ <- expect(Token::ParenthesisLeft);
    expr <- expression;
    _ <- expect(Token::ParenthesisRight);
    expr
);

parser!(variable loc =
    name <- identifier;
    Ast::variable(name, loc)
);

parser!(string loc =
    contents <- string_literal_token;
    Ast::string(contents, loc)
);

parser!(integer loc =
    value <- integer_literal_token;
    Ast::integer(value, loc)
);

parser!(float loc =
    value <- float_literal_token;
    Ast::float(value, loc)
);

parser!(parse_char loc =
    contents <- char_literal_token;
    Ast::char_literal(contents, loc)
);

parser!(parse_bool loc =
    value <- bool_literal_token;
    Ast::bool_literal(value, loc)
);

parser!(unit loc =
    _ <- expect(Token::UnitLiteral);
    Ast::unit_literal(loc)
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
