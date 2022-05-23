use cranelift::frontend::FunctionBuilder;
use cranelift::prelude::{FloatCC, InstBuilder, IntCC, StackSlotData, StackSlotKind, Value as CraneliftValue};

use crate::hir::{Ast, Builtin};

use super::context::{int_pointer_type, pointer_type};
use super::{CodeGen, Context, Value};

pub fn call_builtin<'ast>(builtin: &'ast Builtin, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
    let mut value = |ast: &'ast Box<Ast>| ast.eval_single(context, builder);

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

        Builtin::Truncate(a, _typ) => truncate(value(a), builder),

        Builtin::Deref(a, typ) => return deref(context, typ, a, builder),
        Builtin::Offset(a, b, elem_size) => offset(value(a), value(b), *elem_size, builder),
        Builtin::Transmute(a, _typ) => transmute(value(a), builder),
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

fn less_signed(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().icmp(IntCC::SignedLessThan, param1, param2)
}

fn less_unsigned(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().icmp(IntCC::UnsignedLessThan, param1, param2)
}

fn less_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().fcmp(FloatCC::LessThan, param1, param2)
}

fn eq_int(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn eq_float(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().fcmp(FloatCC::Equal, param1, param2)
}

fn eq_char(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn eq_bool(param1: CraneliftValue, param2: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    builder.ins().icmp(IntCC::Equal, param1, param2)
}

fn transmute(param1: CraneliftValue, builder: &mut FunctionBuilder) -> CraneliftValue {
    // TODO: struct types
    let target_type = builder.func.signature.returns[0].value_type;
    let start_type = builder.func.dfg.value_type(param1);

    if start_type != target_type {
        builder.ins().bitcast(target_type, param1)
    } else {
        param1
    }
}

fn offset(
    address: CraneliftValue, offset: CraneliftValue, elem_size: u32, builder: &mut FunctionBuilder,
) -> CraneliftValue {
    let usize_type = int_pointer_type();
    let pointer_type = pointer_type();

    let size = builder.ins().iconst(int_pointer_type(), elem_size as i64);
    let offset = builder.ins().imul(offset, size);

    let address = builder.ins().bitcast(usize_type, address);
    let new_address = builder.ins().iadd(address, offset);
    builder.ins().bitcast(pointer_type, new_address)
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
    let values = param1.eval_all(context, builder);

    let size = values.iter().map(|value| builder.func.dfg.value_type(*value).bytes()).sum();

    let data = StackSlotData::new(StackSlotKind::ExplicitSlot, size);
    let slot = builder.create_stack_slot(data);

    let mut offset: u32 = 0;
    for value in values {
        builder.ins().stack_store(value, slot, offset as i32);
        offset += builder.func.dfg.value_type(value).bytes();
    }

    builder.ins().stack_addr(pointer_type(), slot, 0)
}
