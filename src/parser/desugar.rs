
use crate::{error::location::Locatable, lexer::token::Token, parser::ast, util::fmap};
use crate::parser::ast::Ast;
use crate::error::location::Location;

/// Turns `(foo _  _ 2)` into `(fn $1 $2 -> (foo $1 $2 2))`
pub fn desugar_explicit_currying<'a>(function: Ast<'a>, args: Vec<Ast<'a>>, loc: Location<'a>) -> Ast<'a> {
    if matches_not_typeconstructor(&function) && args.iter().any(matches_underscore) {
        return curried_function_call(function, args, loc)
    }

    Ast::function_call(function, args, loc)
}

fn curried_function_call<'a>(function: Ast<'a>, args: Vec<Ast<'a>>, loc: Location<'a>) -> Ast<'a> {
    let mut curried_args = vec![];
    let mut curried_arg_count = 0;
    let args: Vec<Ast<'a>> = fmap(args, |arg| {
        if matches_underscore(&arg) {
            curried_arg_count += 1;
            let curried_arg = format!("${}", curried_arg_count);
            curried_args.push(Ast::variable(curried_arg.clone(), arg.locate()));
            Ast::variable(curried_arg, arg.locate())
        } else {
            arg
        }
    });

    let function_call = Ast::function_call(function, args, loc);
    Ast::lambda(curried_args, None, function_call, loc)
}

fn matches_underscore(arg: &Ast) -> bool {
    matches!(arg, Ast::Variable(ast::Variable{ kind: ast::VariableKind::Identifier(x), ..}) if x == "_")
}

fn matches_not_typeconstructor(function: &Ast) -> bool {
    !matches!(function, Ast::Variable(ast::Variable{ kind: ast::VariableKind::TypeConstructor(_), ..}))
}

/// Turns `bar |> foo` into `foo bar`
pub fn desugar_apply_operator<'a>(operator: Token, lhs: Ast<'a>, rhs: Ast<'a>, location: Location<'a>) -> Ast<'a> {
    match operator {
        Token::ApplyLeft  => prepend_argument_to_function(lhs, rhs, location),
        Token::ApplyRight => prepend_argument_to_function(rhs, lhs, location),
        _ => {
            let operator = Ast::operator(operator, location);
            Ast::function_call(operator, vec![lhs, rhs], location)
        }
    }
}

fn prepend_argument_to_function<'a>(f: Ast<'a>, arg: Ast<'a>, location: Location<'a>) -> Ast<'a> {
    match f {
        Ast::FunctionCall(mut call) => {
            call.args.insert(0, arg);
            Ast::FunctionCall(call)
        },
        _ => Ast::function_call(f, vec![arg], location)
    }
}
