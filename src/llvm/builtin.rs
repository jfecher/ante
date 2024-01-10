//! llvm/builtin.rs - Defines the `builtin` function used in the
//! prelude to implement builtin operators such as addition for integers,
//! multiplication for floats, etc.
//!
//! Note that the builtin function only
//! takes a string as its argument which is matched on in `call_builtin`
//! to get the corresponding builtin operation. Since these operations
//! expect the llvm::Function to have a certain signature, the `builtin`
//! function is prevented from being used outside the prelude.
use crate::mir::{Builtin, IntegerKind, PrimitiveType, Type, Atom};
use crate::llvm::{CodeGen, Generator};

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::types::BasicType;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

pub fn call_builtin<'g>(builtin: &Builtin, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let always_inline = Attribute::get_named_enum_kind_id("alwaysinline");
    assert_ne!(always_inline, 0);
    let attribute = generator.context.create_enum_attribute(always_inline, 1);
    current_function.add_attribute(AttributeLoc::Function, attribute);

    let mut int = |ast: &Atom| ast.codegen(generator).into_int_value();

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
        Builtin::FloatPromote(a, _typ) => float_to_float_cast(a, generator),
        Builtin::FloatDemote(a, _typ) => float_to_float_cast(a, generator),

        Builtin::BitwiseAnd(a, b) => bitwise_and(int(a), int(b), generator),
        Builtin::BitwiseOr(a, b) => bitwise_or(int(a), int(b), generator),
        Builtin::BitwiseXor(a, b) => bitwise_xor(int(a), int(b), generator),
        Builtin::BitwiseNot(a) => bitwise_not(int(a), generator),

        Builtin::Truncate(a, typ) => truncate(int(a), typ, generator),

        Builtin::Deref(a, typ) => deref_ptr(a, typ, generator),
        Builtin::Offset(a, b, typ) => offset(a, int(b), typ, generator),
        Builtin::Transmute(a, _typ) => transmute_value(a, generator),
        Builtin::StackAlloc(a) => stack_alloc(a, generator),
    }
}

fn add_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_add(a, b, "add").unwrap().into()
}

fn add_float<'g>(a: &Atom, b: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_add(a, b, "add").unwrap().into()
}

fn sub_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_sub(a, b, "sub").unwrap().into()
}

fn sub_float<'g>(a: &Atom, b: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_sub(a, b, "sub").unwrap().into()
}

fn mul_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_mul(a, b, "mul").unwrap().into()
}

fn mul_float<'g>(a: &Atom, b: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
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

fn div_float<'g>(a: &Atom, b: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
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
fn mod_float<'g>(a: &Atom, b: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
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

fn less_float<'g>(a: &Atom, b: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_compare(FloatPredicate::OLT, a, b, "less").unwrap().into()
}

fn eq_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").unwrap().into()
}

fn eq_float<'g>(a: &Atom, b: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
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

fn deref_ptr<'g>(ptr: &Atom, typ: &Type, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let element_type = generator.convert_type(typ);
    let ret = element_type.ptr_type(AddressSpace::default());

    let ptr = ptr.codegen(generator).into_pointer_value();
    let ptr = generator.builder.build_pointer_cast(ptr, ret, "bitcast").unwrap();
    generator.builder.build_load(element_type, ptr, "deref").unwrap().into()
}

/// offset (p: Ptr t) (offset: usz) = (p as usize + offset * size_of t) as Ptr t
///
// This builtin is unnecessary once we replace it with size_of
fn offset<'g>(ptr: &Atom, offset: IntValue<'g>, element_type: &Type, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let ptr = ptr.codegen(generator).into_pointer_value();
    // expect ptr to be an i8* so we must multiply offset by type_size manually
    let bits = generator.integer_bit_count(IntegerKind::Usz);

    let type_size = element_type.size_in_bytes();
    let type_size = generator.context.custom_width_int_type(bits).const_int(type_size as u64, true);

    let offset = generator.builder.build_int_mul(offset, type_size, "offset_adjustment").unwrap();
    let element_type = generator.convert_type(element_type);
    unsafe { generator.builder.build_gep(element_type, ptr, &[offset], "offset").unwrap().into() }
}

fn transmute_value<'g>(x: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
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

fn float_to_signed<'g>(x: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    let x = x.codegen(generator).into_float_value();
    generator.builder.build_float_to_signed_int(x, ret, "float_to_signed").unwrap().into()
}

fn float_to_unsigned<'g>(x: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    let x = x.codegen(generator).into_float_value();
    generator.builder.build_float_to_unsigned_int(x, ret, "float_to_unsigned").unwrap().into()
}

fn float_to_float_cast<'g>(x: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
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

fn stack_alloc<'g>(x: &Atom, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let value = x.codegen(generator);
    let alloca = generator.builder.build_alloca(value.get_type(), "alloca").unwrap();
    generator.builder.build_store(alloca, value)
        .expect("Could not build store in stack_alloc");

    let ptr_type = &crate::mir::Type::Primitive(PrimitiveType::Pointer);
    let opaque_ptr_type = generator.convert_type(ptr_type).into_pointer_type();

    generator.builder.build_pointer_cast(alloca, opaque_ptr_type, "bitcast").unwrap().into()
}
