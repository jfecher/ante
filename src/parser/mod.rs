#[macro_use]
mod combinators;
mod error;

#[macro_use]
pub mod ast;
pub mod pretty_printer;

use crate::lexer::token::Token;
use ast::{ Ast, Type, TypeDefinitionBody };
use error::{ ParseError, ParseResult };
use crate::error::location::Location;
use combinators::*;

type AstResult<'a, 'b> = ParseResult<'a, 'b, Ast<'b>>;

pub fn parse<'a, 'b>(input: Input<'a, 'b>) -> Result<Ast<'b>, ParseError<'b>> {
    let result = parse_file(input);
    if let Err(error) = &result {
        println!("{}", error);
    }
    result
}

pub fn parse_file<'a, 'b>(input: Input<'a, 'b>) -> Result<Ast<'b>, ParseError<'b>> {
    let (input, _, _) = maybe_newline(input)?;
    let (input, ast, _) = statement_list(input)?;
    let (input, _, _) = maybe_newline(input)?;
    let _ = expect(Token::EndOfInput)(input)?;
    Ok(ast)
}

fn maybe_newline<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, Option<Token>> {
    maybe(expect(Token::Newline))(input)
}

parser!(statement_list loc =
    first <- statement;
    rest <- many0(pair( expect(Token::Newline), statement ));
    if rest.is_empty() {
        first
    } else {
        let mut statements = Vec::with_capacity(rest.len() + 1);
        statements.push(first);
        for (_, b) in rest.into_iter() {
            statements.push(b);
        }
        Ast::sequence(statements, loc)
    }
);

fn statement<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    match input[0].0 {
        Token::ParenthesisLeft |
        Token::Identifier(_) => or(&[definition, expression], &"statement")(input),
        Token::Type => or(&[type_definition, type_alias], &"statement")(input),
        Token::Import => import(input),
        Token::Trait => trait_definition(input),
        Token::Impl => trait_impl(input),
        Token::Return => return_expr(input),
        Token::Extern => parse_extern(input),
        _ => expression(input),
    }
}

fn definition<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    raw_definition(input).map(|(input, definition, location)|
            (input, Ast::Definition(definition), location))
}

fn raw_definition<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, ast::Definition<'b>> {
    or(&[function_definition, variable_definition], &"definition")(input)
}

parser!(function_definition location -> 'b ast::Definition<'b> =
    name <- irrefutable_pattern_argument;
    args <- many1(irrefutable_pattern_argument);
    return_type <- maybe(function_return_type);
    _ <- expect(Token::Equal);
    body !<- block_or_expression;
    ast::Definition {
        pattern: Box::new(name),
        expr: Box::new(Ast::lambda(args, return_type, body, location)),
        location,
        info: None,
        typ: None,
    }
);

parser!(function_return_type location -> 'b ast::Type<'b> =
    _ <- expect(Token::RightArrow);
    typ <- parse_type;
    typ
);

parser!(variable_definition location -> 'b ast::Definition<'b> =
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

parser!(type_annotation_pattern loc =
    lhs <- irrefutable_pattern_argument;
    _ <- expect(Token::Colon);
    rhs <- parse_type;
    Ast::type_annotation(lhs, rhs, loc)
);

fn irrefutable_pattern<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    or(&[
       type_annotation_pattern,
       irrefutable_pattern_argument
    ], &"irrefutable_pattern")(input)
}

fn irrefutable_pattern_argument<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    match input[0].0 {
        Token::ParenthesisLeft =>
            parenthesized(or(&[operator, irrefutable_pattern], &"irrefutable pattern"))(input),
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

fn type_definition_body<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, ast::TypeDefinitionBody<'b>> {
    match input[0].0 {
        Token::Indent => or(&[union_block_body, struct_block_body], &"type_definition_body")(input),
        Token::Pipe => union_inline_body(input),
        _ => struct_inline_body(input),
    }
}

parser!(union_variant loc -> 'b (String, Vec<Type<'b>>, Location<'b>) =
    _ <- expect(Token::Pipe);
    variant !<- typename;
    args !<- many0(basic_type);
    (variant, args, loc)
);

parser!(union_block_body _loc -> 'b ast::TypeDefinitionBody<'b> =
    _ <- expect(Token::Indent);
    variants <- delimited_trailing(union_variant, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    TypeDefinitionBody::UnionOf(variants)
);

parser!(union_inline_body _loc -> 'b ast::TypeDefinitionBody<'b> =
    variants <- many1(union_variant);
    TypeDefinitionBody::UnionOf(variants)
);

parser!(struct_field loc -> 'b (String, Type<'b>, Location<'b>) =
    field_name <- identifier;
    _ !<- expect(Token::Colon);
    field_type !<- parse_type;
    (field_name, field_type, loc)
);

parser!(struct_block_body _loc -> 'b ast::TypeDefinitionBody<'b> =
    _ <- expect(Token::Indent);
    fields <- delimited_trailing(struct_field, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    TypeDefinitionBody::StructOf(fields)
);

parser!(struct_inline_body _loc -> 'b ast::TypeDefinitionBody<'b> =
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
    body !<- delimited_trailing(declaration, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    Ast::trait_definition(name, args, fundeps, body, loc)
);

parser!(declaration loc -> 'b ast::TypeAnnotation<'b> =
    lhs <- irrefutable_pattern_argument;
    _ <- expect(Token::Colon);
    rhs <- parse_type;
    ast::TypeAnnotation { lhs: Box::new(lhs), rhs, location: loc, typ: None }
);

parser!(trait_impl loc =
    _ <- expect(Token::Impl);
    name !<- typename;
    args !<- many1(basic_type);
    _ !<- expect(Token::Indent);
    definitions !<- delimited_trailing(raw_definition, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    Ast::trait_impl(name, args, definitions, loc)
);

parser!(return_expr loc =
    _ <- expect(Token::Return);
    expr !<- expression;
    Ast::return_expr(expr, loc)
);

parser!(parse_extern loc =
    _ <- expect(Token::Extern);
    declarations <- or(&[extern_block, extern_single], "extern");
    Ast::extern_expr(declarations, loc)
);

parser!(extern_block _loc -> 'b Vec<ast::TypeAnnotation<'b>>=
    _ <- expect(Token::Indent);
    declarations !<- delimited_trailing(declaration, expect(Token::Newline));
    _ <- expect(Token::Unindent);
    declarations
);

parser!(extern_single _loc -> 'b Vec<ast::TypeAnnotation<'b>> =
    declaration <- declaration;
    vec![declaration]
);

fn block_or_expression<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
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

fn expression<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
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
}

fn term<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    match input[0].0 {
        Token::If => if_expr(input),
        Token::Match => match_expr(input),
        Token::Not => not_expr(input),
        Token::Ampersand => ref_expr(input),
        _ => or(&[
            function_call,
            type_annotation,
            function_argument
        ], &"term")(input),
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

fn parse_type<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, Type<'b>> {
    or(&[
        function_type,
        type_application,
        basic_type
    ], &"type")(input)
}

fn basic_type<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, Type<'b>> {
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
        Token::ParenthesisLeft => parenthesized(parse_type)(input),
        _ => Err(ParseError::InRule(&"type", input[0].1)),
    }
}

parser!(match_branch _loc -> 'b (Ast<'b>, Ast<'b>) =
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

fn function_argument<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
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
            parenthesized(or(&[operator, expression], &"function_argument"))(input)
        },
        Token::TypeName(_) => variant(input),
        _ => Err(ParseError::InRule(&"argument", input[0].1)),
    }
}

parser!(lambda loc =
    _ <- expect(Token::Backslash);
    args !<- many1(irrefutable_pattern_argument);
    return_type <- maybe(function_return_type);
    _ !<- expect(Token::MemberAccess);
    body !<- block_or_expression;
    Ast::lambda(args, return_type, body, loc)
);

parser!(operator loc =
    op <- expect_if("operator", |op| op.is_overloadable_operator());
    Ast::operator(op, loc)
);

parser!(variant loc =
    name <- typename;
    Ast::type_constructor(name, loc)
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

parser!(function_type loc -> 'b Type<'b> =
    args <- many1(basic_type);
    _ <- expect(Token::RightArrow);
    return_type <- parse_type;
    Type::FunctionType(args, Box::new(return_type), loc)
);

parser!(type_application loc -> 'b Type<'b> =
    type_constructor <- basic_type;
    args <- many1(basic_type);
    Type::TypeApplication(Box::new(type_constructor), args, loc)
);

parser!(int_type loc -> 'b Type<'b> =
    _ <- expect(Token::IntegerType);
    Type::IntegerType(loc)
);

parser!(float_type loc -> 'b Type<'b> =
    _ <- expect(Token::FloatType);
    Type::FloatType(loc)
);

parser!(char_type loc -> 'b Type<'b> =
    _ <- expect(Token::CharType);
    Type::CharType(loc)
);

parser!(string_type loc -> 'b Type<'b> =
    _ <- expect(Token::StringType);
    Type::StringType(loc)
);

parser!(boolean_type loc -> 'b Type<'b> =
    _ <- expect(Token::BooleanType);
    Type::BooleanType(loc)
);

parser!(unit_type loc -> 'b Type<'b> =
    _ <- expect(Token::UnitType);
    Type::UnitType(loc)
);

parser!(reference_type loc -> 'b Type<'b> =
    _ <- expect(Token::Ref);
    Type::ReferenceType(loc)
);

parser!(type_variable loc -> 'b Type<'b> =
    name <- identifier;
    Type::TypeVariable(name, loc)
);

parser!(user_defined_type loc -> 'b Type<'b> =
    name <- typename;
    Type::UserDefinedType(name, loc)
);
