//! parser/mod.rs - This file defines parsing, the second phase of the compiler.
//! The goal of parsing is to take the `Vec<Token>` output from the lexing phase
//! and validate the grammar/syntax of the program. If the syntax is invalid,
//! a parse error is printed out. Otherwise, the resulting Ast is returned and
//! the compiler moves onto the name resolution pass.
//!
//! This parser itself is built up from parser combinators. The basic combinators
//! (as well as the parser! macro) are defined in the parser/combinators.rs module.
//! These combinators backtrack by default though !<- can be used to prevent backtracking
//! to speed up parsing.
//!
//! This file makes heavy use of the parser! macro which combines parsers in a
//! sequence, threading the `input` parameter between each step, returning early if
//! there was an error, and handles getting the starting and end Locations for the
//! current parse rule, and union-ing them. This resulting Location for the whole
//! rule is accessible via the location/loc parameter.
#[macro_use]
mod combinators;
mod error;

#[macro_use]
pub mod ast;
pub mod pretty_printer;

use crate::lexer::token::Token;
use ast::{ Ast, Type, Trait, TypeDefinitionBody };
use error::{ ParseError, ParseResult };
use crate::error::location::Location;
use combinators::*;

type AstResult<'a, 'b> = ParseResult<'a, 'b, Ast<'b>>;

/// The entry point to parsing. Parses an entire file, printing any
/// error found, or returns the Ast if there was no error.
pub fn parse<'a, 'b>(input: Input<'a, 'b>) -> Result<Ast<'b>, ParseError<'b>> {
    let result = parse_file(input);
    if let Err(error) = &result {
        eprintln!("{}", error);
    }
    result
}

/// A file is a sequence of statements, separated by newlines.
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
        Token::Identifier(_) => or(&[definition, assignment, expression], &"statement")(input),
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
    body !<- block_or_statement;
    ast::Definition {
        pattern: Box::new(name),
        expr: Box::new(Ast::lambda(args, return_type, body, location)),
        mutable: false,
        location,
        level: None,
        info: None,
        typ: None,
    }
);

parser!(varargs location -> 'b () =
    _ <- expect(Token::Range);
    _ <- expect(Token::MemberAccess);
    ()
);

parser!(function_return_type location -> 'b ast::Type<'b> =
    _ <- expect(Token::Colon);
    typ <- parse_type;
    typ
);

parser!(variable_definition location -> 'b ast::Definition<'b> =
    name <- irrefutable_pattern;
    _ <- expect(Token::Equal);
    mutable <- maybe(expect(Token::Mut));
    expr !<- block_or_statement;
    ast::Definition {
        pattern: Box::new(name),
        expr: Box::new(expr),
        mutable: mutable.is_some(),
        location,
        level: None,
        info: None,
        typ: None,
    }
);

parser!(assignment location =
    lhs <- expression;
    _ <- expect(Token::Assignment);
    rhs !<- expression;
    Ast::assignment(lhs, rhs, location)
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
       irrefutable_pair_pattern,
       irrefutable_pattern_argument
    ], &"irrefutable_pattern")(input)
}

parser!(irrefutable_pair_pattern loc =
    first <- irrefutable_pattern_argument;
    _ <- expect(Token::Comma);
    rest !<- irrefutable_pattern;
    Ast::function_call(Ast::operator(Token::Comma, loc), vec![first, rest], loc)
);

fn parenthesized_irrefutable_pattern<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    parenthesized(or(&[operator, irrefutable_pattern], "irrefutable pattern"))(input)
}

fn irrefutable_pattern_argument<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    match input[0].0 {
        Token::ParenthesisLeft => parenthesized_irrefutable_pattern(input),
        Token::UnitLiteral => unit(input),
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
    field_type !<- parse_type_no_pair;
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
    body <- maybe(trait_body);
    Ast::trait_definition(name, args, fundeps, body.unwrap_or(vec![]), loc)
);

parser!(trait_body loc -> 'b Vec<ast::TypeAnnotation<'b>> =
    _ <- expect(Token::With);
    body <- or(&[trait_body_block, trait_body_single], "trait body");
    body
);

parser!(trait_body_single loc -> 'b Vec<ast::TypeAnnotation<'b>> =
    body <- declaration;
    vec![body]
);

parser!(trait_body_block loc -> 'b Vec<ast::TypeAnnotation<'b>> =
    _ <- expect(Token::Indent);
    body !<- delimited_trailing(declaration, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    body
);

parser!(declaration loc -> 'b ast::TypeAnnotation<'b> =
    lhs <- irrefutable_pattern_argument;
    _ <- expect(Token::Colon);
    rhs !<- parse_type;
    ast::TypeAnnotation { lhs: Box::new(lhs), rhs, location: loc, typ: None }
);

parser!(trait_impl loc =
    _ <- expect(Token::Impl);
    name !<- typename;
    args !<- many1(basic_type);
    given !<- maybe(given);
    definitions !<- maybe(impl_body);
    Ast::trait_impl(name, args, given.unwrap_or(vec![]), definitions.unwrap_or(vec![]), loc)
);

parser!(impl_body loc -> 'b Vec<ast::Definition<'b>> =
    _ <- expect(Token::With);
    definitions <- or(&[impl_body_block, impl_body_single], "impl body");
    definitions
);

parser!(impl_body_single loc -> 'b Vec<ast::Definition<'b>> =
    definition <- raw_definition;
    vec![definition]
);

parser!(impl_body_block loc -> 'b Vec<ast::Definition<'b>> =
    _ <- expect(Token::Indent);
    definitions !<- delimited_trailing(raw_definition, expect(Token::Newline));
    _ !<- expect(Token::Unindent);
    definitions
);

parser!(given loc -> 'b Vec<Trait<'b>> =
    _ <- expect(Token::Given);
    traits <- delimited(required_trait, expect(Token::Comma));
    traits
);

parser!(required_trait location -> 'b Trait<'b> =
    name <- typename;
    args <- many1(basic_type);
    Trait { name, args, location }
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
    _ !<- expect(Token::Unindent);
    declarations
);

parser!(extern_single _loc -> 'b Vec<ast::TypeAnnotation<'b>> =
    declaration <- declaration;
    vec![declaration]
);

fn block_or_statement<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    match input[0].0 {
        Token::Indent => block(input),
        _ => statement(input),
    }
}

parser!(block _loc =
    _ <- expect(Token::Indent);
    expr !<- statement_list;
    _ !<- maybe_newline;
    _ !<- expect(Token::Unindent);
    expr
);

/// Returns the precedence of an operator along with
/// whether or not it is right-associative.
/// Returns None if the given Token is not an operator
fn precedence(token: &Token) -> Option<(i8, bool)> {
    match token {
        Token::Semicolon => Some((0, false)),
        Token::Comma => Some((1, true)),
        Token::ApplyLeft => Some((2, true)),
        Token::ApplyRight => Some((3, false)),
        Token::Or => Some((4, false)),
        Token::And => Some((5, false)),
        Token::EqualEqual | Token::Is | Token::Isnt | Token::NotEqual | Token::GreaterThan | Token::LessThan | Token::GreaterThanOrEqual | Token::LessThanOrEqual => Some((7, false)),
        Token::In => Some((8, false)),
        Token::Append => Some((9, false)),
        Token::Range => Some((10, false)),
        Token::Add | Token::Subtract => Some((11, false)),
        Token::Multiply | Token::Divide | Token::Modulus => Some((12, false)),
        Token::Colon => Some((13, false)),
        Token::Index => Some((14, false)),
        Token::As => Some((15, false)),
        _ => None,
    }
}

/// Should we push this operator onto our operator stack and keep parsing our expression?
/// This handles the operator precedence and associativity parts of the shunting-yard algorithm.
fn should_continue(operator_on_stack: &Token, r_prec: i8, r_is_right_assoc: bool) -> bool {
    let (l_prec, _) = precedence(operator_on_stack).unwrap();

    l_prec > r_prec
    || (l_prec == r_prec && !r_is_right_assoc)
}

fn pop_operator<'c>(operator_stack: &mut Vec<&Token>, results: &mut Vec<(Ast<'c>, Location<'c>)>) {
    let (rhs, rhs_location) = results.pop().unwrap();
    let (lhs, lhs_location) = results.pop().unwrap();
    let location = lhs_location.union(rhs_location);
    let operator = Ast::operator(operator_stack.pop().unwrap().clone(), location);
    let call = Ast::function_call(operator, vec![lhs, rhs], location);
    results.push((call, location));
}

/// Parse an arbitrary expression using the shunting-yard algorithm
fn expression<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    let (mut input, value, location) = term(input)?;

    let mut operator_stack = vec![];
    let mut results = vec![(value, location)];

    // loop while the next token is an operator
    while let Some((prec, right_associative)) = precedence(&input[0].0) {
        while !operator_stack.is_empty()
            && should_continue(operator_stack[operator_stack.len()- 1], prec, right_associative)
        {
            pop_operator(&mut operator_stack, &mut results);
        }

        operator_stack.push(&input[0].0);
        input = &input[1..];

        let (new_input, value, location) = no_backtracking(term)(input)?;
        results.push((value, location));
        input = new_input;
    }

    while !operator_stack.is_empty() {
        assert!(results.len() >= 2);
        pop_operator(&mut operator_stack, &mut results);
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
        _ => or(&[
            function_call,
            type_annotation,
            function_argument
        ], &"term")(input),
    }
}

parser!(function_call loc =
    function <- member_access;
    args <- many1(function_argument);
    Ast::function_call(function, args, loc)
);

parser!(if_expr loc =
    _ <- expect(Token::If);
    condition !<- block_or_statement;
    _ !<- maybe_newline;
    _ !<- expect(Token::Then);
    then !<- block_or_statement;
    otherwise !<- maybe(else_expr);
    Ast::if_expr(condition, then, otherwise, loc)
);

parser!(match_expr loc =
    _ <- expect(Token::Match);
    expression !<- block_or_statement;
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

parser!(at_expr loc =
    token <- expect(Token::At);
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
        pair_type,
        basic_type
    ], &"type")(input)
}

fn parse_type_no_pair<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, Type<'b>> {
    or(&[
        function_type,
        type_application,
        basic_type
    ], &"type")(input)
}

fn basic_type<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, Type<'b>> {
    match input[0].0 {
        Token::IntegerType(_) => int_type(input),
        Token::FloatType => float_type(input),
        Token::CharType => char_type(input),
        Token::StringType => string_type(input),
        Token::BooleanType => boolean_type(input),
        Token::UnitType => unit_type(input),
        Token::Ref => reference_type(input),
        Token::Identifier(_) => type_variable(input),
        Token::TypeName(_) => user_defined_type(input),
        Token::ParenthesisLeft => parenthesized_type(input),
        _ => Err(ParseError::InRule(&"type", input[0].1)),
    }
}

fn parenthesized_type<'a, 'b>(input: Input<'a, 'b>) -> ParseResult<'a, 'b, Type<'b>> {
    parenthesized(parse_type)(input)
}

parser!(match_branch _loc -> 'b (Ast<'b>, Ast<'b>) =
    _ <- maybe_newline;
    _ <- expect(Token::Pipe);
    pattern !<- expression;
    _ !<- expect(Token::RightArrow);
    branch !<- block_or_statement;
    (pattern, branch)
);

parser!(else_expr _loc =
    _ <- maybe_newline;
    _ <- expect(Token::Else);
    otherwise !<- block_or_statement;
    otherwise
);

/// A function_argument is a unary expr or a member_access of
/// 1-n arguments.
fn function_argument<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    match input[0].0 {
        Token::Not => not_expr(input),
        Token::Ampersand => ref_expr(input),
        Token::At => at_expr(input),
        _ => member_access(input),
    }
}

/// member_access = argument ('.' identifier)*
fn member_access<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    let (mut input, mut arg, mut location) = argument(input)?;

    while input[0].0 == Token::MemberAccess {
        input = &input[1..];

        let (new_input, field, field_location) = no_backtracking(identifier)(input)?;
        input = new_input;
        location = location.union(field_location);
        arg = Ast::member_access(arg, field, location);
    }

    Ok((input, arg, location))
}

fn argument<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    match input[0].0 {
        Token::Identifier(_) => variable(input),
        Token::StringLiteral(_) => string(input),
        Token::IntegerLiteral(_, _) => integer(input),
        Token::FloatLiteral(_) => float(input),
        Token::CharLiteral(_) => parse_char(input),
        Token::BooleanLiteral(_) => parse_bool(input),
        Token::UnitLiteral => unit(input),
        Token::Fn => lambda(input),
        Token::ParenthesisLeft => parenthesized_expression(input),
        Token::TypeName(_) => variant(input),
        _ => Err(ParseError::InRule(&"argument", input[0].1)),
    }
}

parser!(lambda loc =
    _ <- expect(Token::Fn);
    args !<- many1(irrefutable_pattern_argument);
    return_type <- maybe(function_return_type);
    _ !<- expect(Token::MemberAccess);
    body !<- block_or_statement;
    Ast::lambda(args, return_type, body, loc)
);

parser!(operator loc =
    op <- expect_if("operator", |op| op.is_overloadable_operator());
    Ast::operator(op, loc)
);

fn parenthesized_expression<'a, 'b>(input: Input<'a, 'b>) -> AstResult<'a, 'b> {
    parenthesized(or(&[expression, operator], &"argument"))(input)
}

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
    Ast::integer(value.0, value.1, loc)
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
    varargs <- maybe(varargs);
    _ <- expect(Token::RightArrow);
    return_type <- parse_type;
    Type::FunctionType(args, Box::new(return_type), varargs.is_some(), loc)
);

parser!(type_application loc -> 'b Type<'b> =
    type_constructor <- basic_type;
    args <- many1(basic_type);
    Type::TypeApplication(Box::new(type_constructor), args, loc)
);

parser!(pair_type loc -> 'b Type<'b> =
    first <- basic_type;
    _ <- expect(Token::Comma);
    rest !<- parse_type;
    Type::PairType(Box::new(first), Box::new(rest), loc)
);

parser!(int_type loc -> 'b Type<'b> =
    kind <- int_type_token;
    Type::IntegerType(kind, loc)
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
