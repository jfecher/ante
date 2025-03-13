use cranelift::frontend::FunctionBuilder;
use cranelift::prelude::types::I8;
use cranelift::prelude::{FloatCC, InstBuilder, IntCC, Value as CraneliftValue};

use crate::hir::{Ast, Builtin};

use super::context::{int_pointer_type, pointer_type};
use super::{CodeGen, Context, Value};

pub fn call_builtin<'ast>(builtin: &'ast Builtin, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
    let mut value = |ast: &'ast Ast| ast.eval_single(context, builder);

    let result = match builtin {
        Builtin::AddInt(a, b) => add_int(value(a), value(b), builder),
        Builtin::AddFloat(a, b) => add_float(value(a), value(b), builder),

        Builtin::SubInt(a, b) => sub_int(value(a), value(b), builder),
        Builtin::SubFloat(a, b) => sub_float(value(a), value(b), builder),

        Builtin::MulInt(a, b) => mul_int(value(a), value(b), builder),
        Builtin::MulFloat(a, b) => mul_float(value(a), value(b), builder),

        Builtin::DivSigned(a, b) => div_signed(value(a), value(b), builder),
        Builtin::DivUnsigned(a, b) => div_unsigned(value(a), value(b), builder),
        Builtin::DivFloat(a, b) => div_float(value(a), value(b), builder),

        Builtin::ModSigned(a, b) => mod_signed(value(a), value(b), builder),
        Builtin::ModUnsigned(a, b) => mod_unsigned(value(a), value(b), builder),
        Builtin::ModFloat(_, _) => unimplemented!("cranelift defines no float remainder function"),

        Builtin::LessSigned(a, b) => less_signed(value(a), value(b), builder),
        Builtin::LessUnsigned(a, b) => less_unsigned(value(a), value(b), builder),
        Builtin::LessFloat(a, b) => less_float(value(a), value(b), builder),

        Builtin::EqInt(a, b) => eq_int(value(a), value(b), builder),
        Builtin::EqFloat(a, b) => eq_float(value(a), value(b), builder),
        Builtin::EqChar(a, b) => eq_char(value(a), value(b), builder),
        Builtin::EqBool(a, b) => eq_bool(value(a), value(b), builder),

        Builtin::SignExtend(a, _typ) => sign_extend(value(a), builder),
        Builtin::ZeroExtend(a, _typ) => zero_extend(value(a), builder),

        Builtin::SignedToFloat(a, _typ) => signed_to_float(value(a), builder),
        Builtin::UnsignedToFloat(a, _typ) => unsigned_to_float(value(a), builder),
        Builtin::FloatToSigned(a, _typ) => float_to_signed(value(a), builder),
        Builtin::FloatToUnsigned(a, _typ) => float_to_unsigned(value(a), builder),
        Builtin::FloatPromote(a) => float_promote(value(a), builder),
        Builtin::FloatDemote(a) => float_demote(value(a), builder),

        Builtin::BitwiseAnd(a, b) => bitwise_and(value(a), value(b), builder),
        Builtin::BitwiseOr(a, b) => bitwise_or(value(a), value(b), builder),
        Builtin::BitwiseXor(a, b) => bitwise_xor(value(a), value(b), builder),
        Builtin::BitwiseNot(a) => bitwise_not(value(a), builder),

        Builtin::Truncate(a, _typ) => truncate(value(a), builder),

        Builtin::Deref(a, typ) => return deref(context, typ, a, builder),
        Builtin::Offset(a, b, elem_size) => return offset(context, a, b, elem_size, builder),
        Builtin::Transmute(a, typ) => return transmute(context, a, typ, builder),
        Builtin::StackAlloc(a) => stack_alloc(a, context, builder),
    };

    Value::Normal(result)
}

fn add_int(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().iadd(param1, param2)
}

fn add_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().fadd(param1, param2)
}

fn sub_int(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().isub(param1, param2)
}

fn sub_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().fsub(param1, param2)
}

fn mul_int(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().imul(param1, param2)
}

fn mul_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().fmul(param1, param2)
}

fn div_signed(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().sdiv(param1, param2)
}

fn div_unsigned(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().udiv(param1, param2)
}

fn div_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().fdiv(param1, param2)
}

fn mod_signed(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().srem(param1, param2)
}

fn mod_unsigned(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().urem(param1, param2)
}

fn b1_to_i8(value: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    // Does this cast preserve the round-trip?
    builder.ins().raw_bitcast(I8, value)
}

fn less_signed(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    b1_to_i8(builder.ins().icmp(IntCC::SignedLessThan, param1, param2), builder)
}

fn less_unsigned(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    b1_to_i8(builder.ins().icmp(IntCC::UnsignedLessThan, param1, param2), builder)
}

fn less_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    b1_to_i8(builder.ins().fcmp(FloatCC::LessThan, param1, param2), builder)
}

fn eq_int(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    b1_to_i8(builder.ins().icmp(IntCC::Equal, param1, param2), builder)
}

fn eq_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    b1_to_i8(builder.ins().fcmp(FloatCC::Equal, param1, param2), builder)
}

fn eq_char(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    b1_to_i8(builder.ins().icmp(IntCC::Equal, param1, param2), builder)
}

fn eq_bool(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    b1_to_i8(builder.ins().icmp(IntCC::Equal, param1, param2), builder)
}

fn transmute<'a>(
    context: &mut Context<'a>, param: &'a Ast, typ: &crate::hir::Type, builder: &mut FunctionBuilder,
) -> Value {
    let value = param.codegen(context, builder);
    context.transmute(value, typ, builder)
}

fn offset<'a>(
    context: &mut Context<'a>, address: &'a Ast, offset: &'a Ast, elem_type: &crate::hir::Type,
    builder: &mut FunctionBuilder,
) -> Value {
    // The `offset` builtin is used to compile field offsets like `foo.&field` which shouldn't
    // implicitly dereference the stack-allocated value `foo` if it was declared as one.
    // Hence we use `eval_address` here instead of `eval_single`.
    let address = address.eval_address(context, builder);
    let offset = offset.eval_single(context, builder);

    let usize_type = int_pointer_type();
    let pointer_type = pointer_type();

    let elem_size = elem_type.size_in_bytes();
    let size = builder.ins().iconst(usize_type, elem_size as i64);
    let offset = builder.ins().imul(offset, size);

    Value::Normal(if usize_type != pointer_type {
        let address = builder.ins().bitcast(usize_type, address);
        let new_address = builder.ins().iadd(address, offset);
        builder.ins().bitcast(pointer_type, new_address)
    } else {
        builder.ins().iadd(address, offset)
    })
}

// All integers are boxed as an i64, so this is a no-op in this backend
fn sign_extend(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let start_type = builder.func.dfg.value_type(param1);
    assert!(start_type.bytes() <= target_type.bytes());

    if start_type.bytes() < target_type.bytes() {
        builder.ins().sextend(target_type, param1)
    } else {
        param1
    }
}

// All integers are boxed as an i64, so this is a no-op in this backend
fn zero_extend(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let start_type = builder.func.dfg.value_type(param1);
    assert!(start_type.bytes() <= target_type.bytes());

    if start_type.bytes() < target_type.bytes() {
        builder.ins().uextend(target_type, param1)
    } else {
        param1
    }
}

fn signed_to_float(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().fcvt_from_sint(target_type, param1)
}

fn unsigned_to_float(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().fcvt_from_uint(target_type, param1)
}

fn float_to_signed(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().fcvt_to_sint(target_type, param1)
}

fn float_to_unsigned(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().fcvt_to_uint(target_type, param1)
}

fn float_promote(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().fpromote(target_type, param1)
}

fn float_demote(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    builder.ins().fdemote(target_type, param1)
}

fn bitwise_and(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().band(param1, param2)
}

fn bitwise_or(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().bor(param1, param2)
}

fn bitwise_xor(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().bxor(param1, param2)
}

fn bitwise_not(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().bnot(param1)
}

fn truncate(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    let target_type = builder.func.signature.returns[0].value_type;
    let start_type = builder.func.dfg.value_type(param1);
    assert!(start_type.bytes() >= target_type.bytes());

    if start_type.bytes() > target_type.bytes() {
        builder.ins().ireduce(target_type, param1)
    } else {
        param1
    }
}

fn deref<'a>(context: &mut Context<'a>, typ: &crate::hir::Type, addr: &'a Ast, builder: &mut FunctionBuilder) -> Value {
    let addr = addr.eval_single(context, builder);
    context.load_value(typ, addr, &mut 0, builder)
}

fn stack_alloc<'a>(param1: &'a Ast, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> CraneliftValue {
    let value = param1.codegen(context, builder);
    context.stack_alloc(value, builder)
}
