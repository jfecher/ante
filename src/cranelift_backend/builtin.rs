use cranelift::frontend::FunctionBuilder;
use cranelift::prelude::{Value as CraneliftValue, InstBuilder, IntCC, FloatCC, MemFlags};
use cranelift::codegen::ir::types as cranelift_types;

use crate::parser::ast::Ast;

use super::{Context, Value};

pub fn call_builtin<'c>(args: &[Ast<'c>], context: &mut Context, builder: &mut FunctionBuilder) -> Value {
    assert_eq!(args.len(), 1);

    use crate::parser::ast::{Literal, LiteralKind::String};
    let arg = match &args[0] {
        Ast::Literal(Literal {
            kind: String(string),
            ..
        }) => string,
        _ => unreachable!(),
    };

    let result = match arg.as_ref() {
        "AddInt" => add_int(context, builder),
        "AddFloat" => add_float(context, builder),

        "SubInt" => sub_int(context, builder),
        "SubFloat" => sub_float(context, builder),

        "MulInt" => mul_int(context, builder),
        "MulFloat" => mul_float(context, builder),

        "DivInt" => div_int(context, builder),
        "DivFloat" => div_float(context, builder),

        "ModInt" => mod_int(context, builder),
        "ModFloat" => mod_float(context, builder),

        "LessInt" => less_int(context, builder),
        "LessFloat" => less_float(context, builder),

        "GreaterInt" => greater_int(context, builder),
        "GreaterFloat" => greater_float(context, builder),

        "EqInt" => eq_int(context, builder),
        "EqFloat" => eq_float(context, builder),
        "EqChar" => eq_char(context, builder),
        "EqBool" => eq_bool(context, builder),

        "sign_extend" => sign_extend(context, builder),
        "zero_extend" => zero_extend(context, builder),
        "truncate" => truncate(context, builder),

        "deref" => deref(context, builder),
        "transmute" => transmute(context, builder),

        _ => unreachable!("Unknown builtin '{}'", arg),
    };

    Value::Normal(result)
}

fn add_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().iadd(param1, param2)
}

fn add_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().fadd(param1, param2)
}

fn sub_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().isub(param1, param2)
}

fn sub_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().fsub(param1, param2)
}

fn mul_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().imul(param1, param2)
}

fn mul_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().fmul(param1, param2)
}

fn div_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    // TODO: unsigned
    builder.ins().sdiv(param1, param2)
}

fn div_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().fdiv(param1, param2)
}

fn mod_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().srem(param1, param2)
}

fn mod_float(_context: &mut Context, _builder: &mut FunctionBuilder) -> CraneliftValue {
    unimplemented!("cranelift defines no float remainder function")
}

fn less_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().icmp(IntCC::SignedLessThan, param1, param2)
}

fn less_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().fcmp(FloatCC::LessThan, param1, param2)
}

fn greater_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().icmp(IntCC::SignedGreaterThan, param1, param2)
}

fn greater_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().fcmp(FloatCC::GreaterThan, param1, param2)
}

fn eq_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn eq_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().fcmp(FloatCC::Equal, param1, param2)
}

fn eq_char(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn eq_bool(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn deref(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    dbg!("todo: elem type");
    let element_type = cranelift_types::I64;
    builder.ins().load(element_type, MemFlags::new(), param1, 0)
}

fn transmute(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().bitcast(target_type, param1)
}

fn sign_extend(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().sextend(target_type, param1)
}

fn zero_extend(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().uextend(target_type, param1)
}

fn truncate(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().ireduce(target_type, param1)
}
