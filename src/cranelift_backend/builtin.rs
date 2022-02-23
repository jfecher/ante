use cranelift::prelude::{Value as CraneliftValue, InstBuilder, IntCC, FloatCC, MemFlags};
use cranelift::codegen::ir::types as cranelift_types;

use crate::parser::ast::Ast;

use super::{Context, Value};

pub fn call_builtin<'a, 'ast, 'c>(args: &[Ast<'c>], context: &mut Context<'a, 'ast, 'c>) -> Value {
    assert!(args.len() == 1);

    use crate::parser::ast::{Literal, LiteralKind::String};
    let arg = match &args[0] {
        Ast::Literal(Literal {
            kind: String(string),
            ..
        }) => string,
        _ => unreachable!(),
    };

    match arg.as_ref() {
        "AddInt" => add_int(context),
        "AddFloat" => add_float(context),

        "SubInt" => sub_int(context),
        "SubFloat" => sub_float(context),

        "MulInt" => mul_int(context),
        "MulFloat" => mul_float(context),

        "DivInt" => div_int(context),
        "DivFloat" => div_float(context),

        "ModInt" => mod_int(context),
        "ModFloat" => mod_float(context),

        "LessInt" => less_int(context),
        "LessFloat" => less_float(context),

        "GreaterInt" => greater_int(context),
        "GreaterFloat" => greater_float(context),

        "EqInt" => eq_int(context),
        "EqFloat" => eq_float(context),
        "EqChar" => eq_char(context),
        "EqBool" => eq_bool(context),

        "deref" => deref(context),
        "transmute" => transmute(context),

        _ => unreachable!("Unknown builtin '{}'", arg),
    }
}

fn add_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().iadd(param1, param2)
}

fn add_float<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().fadd(param1, param2)
}

fn sub_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().isub(param1, param2)
}

fn sub_float<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().fsub(param1, param2)
}

fn mul_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().imul(param1, param2)
}

fn mul_float<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().fmul(param1, param2)
}

fn div_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    // TODO: unsigned
    context.builder.ins().sdiv(param1, param2)
}

fn div_float<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().fdiv(param1, param2)
}

fn mod_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().srem(param1, param2)
}

fn mod_float<'a, 'ast, 'c>(_context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    unimplemented!("cranelift defines no float remainder function")
}

fn less_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().icmp(IntCC::SignedLessThan, param1, param2)
}

fn less_float<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().fcmp(FloatCC::LessThan, param1, param2)
}

fn greater_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().icmp(IntCC::SignedGreaterThan, param1, param2)
}

fn greater_float<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().fcmp(FloatCC::GreaterThan, param1, param2)
}

fn eq_int<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn eq_float<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().fcmp(FloatCC::Equal, param1, param2)
}

fn eq_char<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn eq_bool<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    context.builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn deref<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    dbg!("todo: elem type");
    let element_type = cranelift_types::I64;
    context.builder.ins().load(element_type, MemFlags::new(), param1, 0)
}

fn transmute<'a, 'ast, 'c>(context: &mut Context<'a, 'ast, 'c>) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    // TODO: multiple returns if argument is a struct
    let target_type = context.builder.func.signature.returns[0].value_type;
    context.builder.ins().bitcast(target_type, param1)
}
