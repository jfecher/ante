//! llvm/builtin.rs - Defines the `builtin` function used in the
//! prelude to implement builtin operators such as addition for integers,
//! multiplication for floats, etc.
//!
//! Note that the builtin function only
//! takes a string as its argument which is matched on in `call_builtin`
//! to get the corresponding builtin operation. Since these operations
//! expect the llvm::Function to have a certain signature, the `builtin`
//! function is prevented from being used outside the prelude.
use crate::hir::{Ast, Builtin, PrimitiveType, Type};
use crate::llvm::{CodeGen, Generator};

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

pub fn call_builtin<'g>(builtin: &Builtin, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let always_inline = Attribute::get_named_enum_kind_id("alwaysinline");
    assert_ne!(always_inline, 0);
    let attribute = generator.context.create_enum_attribute(always_inline, 0);
    current_function.add_attribute(AttributeLoc::Function, attribute);

    let mut int = |ast: &Ast| ast.codegen(generator).into_int_value();

    match builtin {
        Builtin::AddInt(a, b) => add_int(int(a), int(b), generator),
        Builtin::AddFloat(a, b) => add_float(a, b, generator),

        Builtin::SubInt(a, b) => sub_int(int(a), int(b), generator),
        Builtin::SubFloat(a, b) => sub_float(a, b, generator),

        Builtin::MulInt(a, b) => mul_int(int(a), int(b), generator),
        Builtin::MulFloat(a, b) => mul_float(a, b, generator),

        Builtin::DivSigned(a, b) => div_signed(int(a), int(b), generator),
        Builtin::DivUnsigned(a, b) => div_unsigned(int(a), int(b), generator),
        Builtin::DivFloat(a, b) => div_float(a, b, generator),

        Builtin::ModSigned(a, b) => mod_signed(int(a), int(b), generator),
        Builtin::ModUnsigned(a, b) => mod_unsigned(int(a), int(b), generator),
        Builtin::ModFloat(a, b) => mod_float(a, b, generator),

        Builtin::LessSigned(a, b) => less_signed(int(a), int(b), generator),
        Builtin::LessUnsigned(a, b) => less_unsigned(int(a), int(b), generator),
        Builtin::LessFloat(a, b) => less_float(a, b, generator),

        Builtin::EqInt(a, b) => eq_int(int(a), int(b), generator),
        Builtin::EqFloat(a, b) => eq_float(a, b, generator),
        Builtin::EqChar(a, b) => eq_char(int(a), int(b), generator),
        Builtin::EqBool(a, b) => eq_bool(int(a), int(b), generator),

        Builtin::SignExtend(a, _typ) => sign_extend(int(a), generator),
        Builtin::ZeroExtend(a, _typ) => zero_extend(int(a), generator),

        Builtin::SignedToFloat(a, _typ) => signed_to_float(int(a), generator),
        Builtin::UnsignedToFloat(a, _typ) => unsigned_to_float(int(a), generator),
        Builtin::FloatToSigned(a, _typ) => float_to_signed(a, generator),
        Builtin::FloatToUnsigned(a, _typ) => float_to_unsigned(a, generator),
        Builtin::FloatPromote(a) => float_to_float_cast(a, generator),
        Builtin::FloatDemote(a) => float_to_float_cast(a, generator),

        Builtin::BitwiseAnd(a, b) => bitwise_and(int(a), int(b), generator),
        Builtin::BitwiseOr(a, b) => bitwise_or(int(a), int(b), generator),
        Builtin::BitwiseXor(a, b) => bitwise_xor(int(a), int(b), generator),
        Builtin::BitwiseNot(a) => bitwise_not(int(a), generator),

        Builtin::Truncate(a, typ) => truncate(int(a), typ, generator),

        Builtin::Deref(a, typ) => deref_ptr(a, typ, generator),
        Builtin::Offset(a, b, typ) => offset(a, int(b), typ, generator),
        Builtin::Transmute(a, _typ) => transmute_value(a, generator),
        Builtin::StackAlloc(a) => stack_alloc(a, generator),

        Builtin::ContinuationInit(f) => continuation_init(f, generator),
        Builtin::ContinuationIsSuspended(k) => continuation_is_suspended(k, generator),
        Builtin::ContinuationArgPush(k, x) => continuation_arg_push(k, x, generator),
        Builtin::ContinuationArgPop(k, typ) => continuation_arg_pop(k, typ, generator),
        Builtin::ContinuationSuspend(k) => continuation_suspend(k, generator),
        Builtin::ContinuationResume(k) => continuation_resume(k, generator),
        Builtin::ContinuationFree(k) => continuation_free(k, generator),
    }
}

fn add_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_add(a, b, "add").unwrap().into()
}

fn add_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_add(a, b, "add").unwrap().into()
}

fn sub_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_sub(a, b, "sub").unwrap().into()
}

fn sub_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_sub(a, b, "sub").unwrap().into()
}

fn mul_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_mul(a, b, "mul").unwrap().into()
}

fn mul_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_mul(a, b, "mul").unwrap().into()
}

fn div_signed<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_signed_div(a, b, "div").unwrap().into()
}

fn div_unsigned<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_unsigned_div(a, b, "div").unwrap().into()
}

fn div_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_div(a, b, "div").unwrap().into()
}

fn mod_signed<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_signed_rem(a, b, "mod").unwrap().into()
}

fn mod_unsigned<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_unsigned_rem(a, b, "mod").unwrap().into()
}

// Cranelift doesn't support this, perhaps we should remove support altogether for float mod
fn mod_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_rem(a, b, "mod").unwrap().into()
}

fn less_signed<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::SLT, a, b, "less").unwrap().into()
}

fn less_unsigned<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::ULT, a, b, "less").unwrap().into()
}

fn less_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_compare(FloatPredicate::OLT, a, b, "less").unwrap().into()
}

fn eq_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").unwrap().into()
}

fn eq_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_compare(FloatPredicate::OEQ, a, b, "eq").unwrap().into()
}

fn eq_char<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").unwrap().into()
}

fn eq_bool<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").unwrap().into()
}

fn deref_ptr<'g>(ptr: &Ast, typ: &Type, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let element_type = generator.convert_type(typ);
    let ret = generator.context.ptr_type(AddressSpace::default());

    let ptr = ptr.codegen(generator).into_pointer_value();
    let ptr = generator.builder.build_pointer_cast(ptr, ret, "bitcast").unwrap();
    generator.builder.build_load(element_type, ptr, "deref").unwrap()
}

/// offset (p: Ptr t) (offset: usz) = (p as usize + offset * size_of t) as Ptr t
///
// This builtin is unnecessary once we replace it with size_of
fn offset<'g>(
    ptr: &Ast, offset: IntValue<'g>, element_type: &Type, generator: &mut Generator<'g>,
) -> BasicValueEnum<'g> {
    let ptr = ptr.codegen(generator);
    let ptr = generator.reference_or_assume_ptr(ptr).into_pointer_value();

    let element_type = generator.convert_type(element_type);
    unsafe { generator.builder.build_gep(element_type, ptr, &[offset], "offset").unwrap().into() }
}

fn transmute_value<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let value = x.codegen(generator);
    let ret = current_function.get_type().get_return_type().unwrap();
    generator.reinterpret_cast(value, ret)
}

fn sign_extend<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_s_extend(x, ret, "sign_extend").unwrap().into()
}

fn zero_extend<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_z_extend(x, ret, "zero_extend").unwrap().into()
}

fn signed_to_float<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_float_type();
    generator.builder.build_signed_int_to_float(x, ret, "signed_to_float").unwrap().into()
}

fn unsigned_to_float<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_float_type();
    generator.builder.build_unsigned_int_to_float(x, ret, "unsigned_to_float").unwrap().into()
}

fn float_to_signed<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    let x = x.codegen(generator).into_float_value();
    generator.builder.build_float_to_signed_int(x, ret, "float_to_signed").unwrap().into()
}

fn float_to_unsigned<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    let x = x.codegen(generator).into_float_value();
    generator.builder.build_float_to_unsigned_int(x, ret, "float_to_unsigned").unwrap().into()
}

fn float_to_float_cast<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_float_type();
    let x = x.codegen(generator).into_float_value();
    generator.builder.build_float_cast(x, ret, "float_cast").unwrap().into()
}

fn bitwise_and<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_and(a, b, "bitwise_and").unwrap().into()
}

fn bitwise_or<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_or(a, b, "bitwise_or").unwrap().into()
}

fn bitwise_xor<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_xor(a, b, "bitwise_xor").unwrap().into()
}

fn bitwise_not<'g>(a: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_not(a, "bitwise_not").unwrap().into()
}

fn truncate<'g>(x: IntValue<'g>, typ: &Type, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let ret = generator.convert_type(typ).into_int_type();
    generator.builder.build_int_truncate(x, ret, "sign_extend").unwrap().into()
}

fn stack_alloc<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let value = x.codegen(generator);
    stack_alloc_basic_value(value, generator)
}

pub fn stack_alloc_basic_value<'g>(value: BasicValueEnum<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let alloca = generator.builder.build_alloca(value.get_type(), "alloca").unwrap();
    generator.builder.build_store(alloca, value).expect("Could not build store in stack_alloc");

    let ptr_type = &crate::hir::Type::Primitive(PrimitiveType::Pointer);
    let opaque_ptr_type = generator.convert_type(ptr_type).into_pointer_type();

    generator.builder.build_pointer_cast(alloca, opaque_ptr_type, "bitcast").unwrap().into()
}

fn continuation_init<'g>(f: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let f = f.codegen(generator);
    let init = generator.continuation_init.unwrap();
    generator
        .builder
        .build_direct_call(init, &[f.into()], "continuation_init")
        .unwrap()
        .try_as_basic_value()
        .unwrap_left()
}

fn continuation_is_suspended<'g>(k: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let is_suspended = generator.continuation_is_suspended.unwrap();
    let k = k.codegen(generator);

    generator
        .builder
        .build_direct_call(is_suspended, &[k.into()], "continuation_is_suspended")
        .unwrap()
        .try_as_basic_value()
        .unwrap_left()
}

// mco_push(k, &x, sizeof(x));
fn continuation_arg_push<'g>(k: &Ast, x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let push = generator.continuation_arg_push.unwrap();

    let k = k.codegen(generator);
    let x = x.codegen(generator);
    let x_size = x.get_type().size_of().unwrap();
    let x_ref = generator.reference_or_alloc(x);

    generator
        .builder
        .build_direct_call(push, &[k.into(), x_ref.into(), x_size.into()], "continuation_arg_push")
        .unwrap();
    generator.unit_value()
}

// result_type ret;
// mco_pop(k, &ret, sizeof(result_type));
// ret
fn continuation_arg_pop<'g>(k: &Ast, typ: &Type, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let pop = generator.continuation_arg_pop.unwrap();

    let k = k.codegen(generator);
    let typ = generator.convert_type(typ);
    let ret = generator.builder.build_alloca(typ, "k_arg").unwrap();
    let size = typ.size_of().unwrap();

    generator.builder.build_direct_call(pop, &[k.into(), ret.into(), size.into()], "continuation_arg_pop").unwrap();
    generator.builder.build_load(typ, ret, "k_arg").unwrap()
}

fn continuation_suspend<'g>(k: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let suspend = generator.continuation_suspend.unwrap();
    let k = k.codegen(generator);
    generator.builder.build_direct_call(suspend, &[k.into()], "continuation_suspend").unwrap();
    generator.unit_value()
}

fn continuation_resume<'g>(k: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let resume = generator.continuation_resume.unwrap();
    let k = k.codegen(generator);
    generator.builder.build_direct_call(resume, &[k.into()], "continuation_resume").unwrap();
    generator.unit_value()
}

fn continuation_free<'g>(k: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let free = generator.continuation_free.unwrap();
    let k = k.codegen(generator);
    generator.builder.build_direct_call(free, &[k.into()], "continuation_free").unwrap();
    generator.unit_value()
}
