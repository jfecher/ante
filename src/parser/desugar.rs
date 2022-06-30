use crate::error::location::Location;
use crate::parser::ast::Ast;
use crate::{error::location::Locatable, lexer::token::Token, parser::ast, util::fmap};

/// Turns `(foo _  _ 2)` into `(fn $1 $2 -> (foo $1 $2 2))`
pub fn desugar_explicit_currying<'a, F>(
    function: Ast<'a>, args: Vec<Ast<'a>>, make_function_call: F, loc: Location<'a>,
) -> Ast<'a>
where
    F: FnOnce(Ast<'a>, Vec<Ast<'a>>, Location<'a>) -> Ast<'a>,
{
    if args.iter().any(matches_underscore) {
        curried_function_call(function, args, make_function_call, loc)
    } else {
        make_function_call(function, args, loc)
    }
}

fn curried_function_call<'a, F>(function: Ast<'a>, args: Vec<Ast<'a>>, call_function: F, loc: Location<'a>) -> Ast<'a>
where
    F: FnOnce(Ast<'a>, Vec<Ast<'a>>, Location<'a>) -> Ast<'a>,
{
    let mut curried_args = vec![];
    let mut curried_arg_count = 0;
    let args: Vec<Ast<'a>> = fmap(args, |arg| {
        if matches_underscore(&arg) {
            curried_arg_count += 1;
            let curried_arg = format!("${}", curried_arg_count);
            curried_args.push(Ast::variable(vec![], curried_arg.clone(), arg.locate()));
            Ast::variable(vec![], curried_arg, arg.locate()) // TODO: add correct module prefix
        } else {
            arg
        }
    });

    let function_call = call_function(function, args, loc);
    Ast::lambda(curried_args, None, function_call, loc)
}

fn matches_underscore(arg: &Ast) -> bool {
    matches!(arg, Ast::Variable(ast::Variable{ kind: ast::VariableKind::Identifier(x), ..}) if x == "_")
}

/// Turns:
/// - `bar |> foo` into `foo bar` (applies to <| as well)
/// - `a and b` into `if a then b else false`
/// - `a or b` into `if a then true else b`
///
/// Also handles explicitly curried operators. E.g. `_ or false` will
/// be translated as `fn $1 -> if $1 then true else false`
pub fn desugar_operators<'a>(operator: Token, lhs: Ast<'a>, rhs: Ast<'a>, location: Location<'a>) -> Ast<'a> {
    let call_operator_function = |function: Ast<'a>, mut arguments: Vec<Ast<'a>>, location| {
        let rhs = arguments.pop().unwrap();
        let lhs = arguments.pop().unwrap();

        match function.get_operator() {
            Some(Token::ApplyLeft) => prepend_argument_to_function(lhs, rhs, location),
            Some(Token::ApplyRight) => prepend_argument_to_function(rhs, lhs, location),
            Some(Token::And) => Ast::if_expr(lhs, rhs, Some(Ast::bool_literal(false, location)), location),
            Some(Token::Or) => Ast::if_expr(lhs, Ast::bool_literal(true, location), Some(rhs), location),
            Some(operator_token) => {
                let operator = Ast::operator(operator_token, location);
                Ast::function_call(operator, vec![lhs, rhs], location)
            },
            None => unreachable!(),
        }
    };

    let operator_symbol = Ast::operator(operator, location);
    desugar_explicit_currying(operator_symbol, vec![lhs, rhs], call_operator_function, location)
}

fn prepend_argument_to_function<'a>(f: Ast<'a>, arg: Ast<'a>, location: Location<'a>) -> Ast<'a> {
    match f {
        Ast::FunctionCall(mut call) => {
            call.args.insert(0, arg);
            Ast::FunctionCall(call)
        },
        _ => Ast::function_call(f, vec![arg], location),
    }
}

pub fn desugar_loop<'a>(
    params_defaults: Vec<(Ast<'a>, Option<Ast<'a>>)>, body: Ast<'a>, location: Location<'a>,
) -> Ast<'a> {
    let mut params = Vec::with_capacity(params_defaults.len());
    let mut args = Vec::with_capacity(params_defaults.len());
    for (param, default) in params_defaults {
        match default {
            Some(def) => args.push(def),
            None => args.push(param.clone()),
        }
        params.push(param);
    }

    let recur_name = || Ast::variable(vec![], "recur".to_owned(), location);
    let recur_def = Ast::definition(recur_name(), Ast::lambda(params, None, body, location), location);
    let recur_call = Ast::function_call(recur_name(), args, location);

    Ast::new_scope(Ast::sequence(vec![recur_def, recur_call], location), location)
}
