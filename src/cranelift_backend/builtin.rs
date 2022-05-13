use cranelift::frontend::FunctionBuilder;
use cranelift::prelude::{FloatCC, InstBuilder, IntCC, MemFlags, Value as CraneliftValue};

use crate::hir::Builtin;

use super::context::{int_pointer_type, pointer_type};
use super::{Context, Value};

pub fn call_builtin(builtin: Builtin, context: &mut Context, builder: &mut FunctionBuilder) -> Value {
    let result = match builtin {
        Builtin::AddInt => add_int(context, builder),
        Builtin::AddFloat => add_float(context, builder),

        Builtin::SubInt => sub_int(context, builder),
        Builtin::SubFloat => sub_float(context, builder),

        Builtin::MulInt => mul_int(context, builder),
        Builtin::MulFloat => mul_float(context, builder),

        Builtin::DivSigned => div_signed(context, builder),
        Builtin::DivUnsigned => div_unsigned(context, builder),
        Builtin::DivFloat => div_float(context, builder),

        Builtin::ModSigned => mod_signed(context, builder),
        Builtin::ModUnsigned => mod_unsigned(context, builder),
        Builtin::ModFloat => mod_float(context, builder),

        Builtin::LessSigned => less_signed(context, builder),
        Builtin::LessUnsigned => less_unsigned(context, builder),
        Builtin::LessFloat => less_float(context, builder),

        Builtin::EqInt => eq_int(context, builder),
        Builtin::EqFloat => eq_float(context, builder),
        Builtin::EqChar => eq_char(context, builder),
        Builtin::EqBool => eq_bool(context, builder),

        Builtin::SignExtend => sign_extend(context, builder),
        Builtin::ZeroExtend => zero_extend(context, builder),

        Builtin::SignedToFloat => signed_to_float(context, builder),
        Builtin::UnsignedToFloat => unsigned_to_float(context, builder),
        Builtin::FloatToSigned => float_to_signed(context, builder),
        Builtin::FloatToUnsigned => float_to_unsigned(context, builder),

        Builtin::Truncate => truncate(context, builder),

        Builtin::Deref => deref(context, builder),
        Builtin::Offset => offset(context, builder),
        Builtin::Transmute => transmute(context, builder),
    };

    Value::Normal(result)
}

fn binary_function<F>(context: &mut Context, builder: &mut FunctionBuilder, f: F) -> CraneliftValue
where
    F: FnOnce(CraneliftValue, CraneliftValue, &mut FunctionBuilder) -> CraneliftValue,
{
    let param1 = context.current_function_parameters[0];
    let param2 = context.current_function_parameters[1];
    f(param1, param2, builder)
}

fn add_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().iadd(param1, param2))
}

fn add_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().fadd(param1, param2))
}

fn sub_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().isub(param1, param2))
}

fn sub_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().fsub(param1, param2))
}

fn mul_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().imul(param1, param2))
}

fn mul_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().fmul(param1, param2))
}

fn div_signed(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().sdiv(param1, param2))
}

fn div_unsigned(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().udiv(param1, param2))
}

fn div_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().fdiv(param1, param2))
}

fn mod_signed(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().srem(param1, param2))
}

fn mod_unsigned(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().urem(param1, param2))
}

fn mod_float(_context: &mut Context, _builder: &mut FunctionBuilder) -> CraneliftValue {
    unimplemented!("cranelift defines no float remainder function")
}

fn less_signed(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| {
        builder.ins().icmp(IntCC::SignedLessThan, param1, param2)
    })
}

fn less_unsigned(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| {
        builder.ins().icmp(IntCC::UnsignedLessThan, param1, param2)
    })
}

fn less_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().fcmp(FloatCC::LessThan, param1, param2))
}

fn eq_int(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().icmp(IntCC::Equal, param1, param2))
}

fn eq_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().fcmp(FloatCC::Equal, param1, param2))
}

fn eq_char(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().icmp(IntCC::Equal, param1, param2))
}

fn eq_bool(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    binary_function(context, builder, |param1, param2, builder| builder.ins().icmp(IntCC::Equal, param1, param2))
}

fn deref(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let param1 = context.current_function_parameters[0];
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().load(target_type, MemFlags::new(), param1, 0)
}

fn transmute(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    // TODO: struct types
    let param1 = context.current_function_parameters[0];
    let target_type = builder.func.signature.returns[0].value_type;
    let start_type = builder.func.dfg.value_type(param1);

    if start_type != target_type {
        builder.ins().bitcast(target_type, param1)
    } else {
        param1
    }
}

fn offset(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let address = context.current_function_parameters[0];
    let offset = context.current_function_parameters[1];
    let return_types = &builder.func.signature.returns;

    let usize_type = int_pointer_type();
    let pointer_type = pointer_type();

    let type_size = return_types.iter().map(|p| p.value_type.bytes()).sum::<u32>() as i64;
    let size = builder.ins().iconst(int_pointer_type(), type_size);
    let offset = builder.ins().imul(offset, size);

    let address = builder.ins().bitcast(usize_type, address);
    let new_address = builder.ins().iadd(address, offset);
    builder.ins().bitcast(pointer_type, new_address)
}

// All integers are boxed as an i64, so this is a no-op in this backend
fn sign_extend(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let int = context.current_function_parameters[0];
    let start_type = builder.func.dfg.value_type(int);
    assert!(start_type.bytes() <= target_type.bytes());

    if start_type.bytes() < target_type.bytes() {
        builder.ins().sextend(target_type, int)
    } else {
        int
    }
}

// All integers are boxed as an i64, so this is a no-op in this backend
fn zero_extend(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let int = context.current_function_parameters[0];
    let start_type = builder.func.dfg.value_type(int);
    assert!(start_type.bytes() <= target_type.bytes());

    if start_type.bytes() < target_type.bytes() {
        builder.ins().uextend(target_type, int)
    } else {
        int
    }
}

fn signed_to_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let int = context.current_function_parameters[0];
    builder.ins().fcvt_from_sint(target_type, int)
}

fn unsigned_to_float(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let int = context.current_function_parameters[0];
    builder.ins().fcvt_from_uint(target_type, int)
}

fn float_to_signed(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let flt = context.current_function_parameters[0];
    builder.ins().fcvt_to_sint(target_type, flt)
}

fn float_to_unsigned(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let flt = context.current_function_parameters[0];
    builder.ins().fcvt_to_uint(target_type, flt)
}

fn truncate(context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let int = context.current_function_parameters[0];
    let start_type = builder.func.dfg.value_type(int);
    assert!(start_type.bytes() >= target_type.bytes());

    if start_type.bytes() > target_type.bytes() {
        builder.ins().ireduce(target_type, int)
    } else {
        int
    }
}
