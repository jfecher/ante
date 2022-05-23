//! llvm/builtin.rs - Defines the `builtin` function used in the
//! prelude to implement builtin operators such as addition for integers,
//! multiplication for floats, etc.
//!
//! Note that the builtin function only
//! takes a string as its argument which is matched on in `call_builtin`
//! to get the corresponding builtin operation. Since these operations
//! expect the llvm::Function to have a certain signature, the `builtin`
//! function is prevented from being used outside the prelude.
use crate::hir::{Ast, Builtin, PrimitiveType};
use crate::llvm::{CodeGen, Generator};

use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, BasicValueEnum, IntValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};

pub fn call_builtin<'g>(builtin: &Builtin, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let always_inline = Attribute::get_named_enum_kind_id("alwaysinline");
    assert_ne!(always_inline, 0);
    let attribute = generator.context.create_enum_attribute(always_inline, 1);
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

        Builtin::Truncate(a, _typ) => truncate(int(a), generator),

        Builtin::Deref(a, _typ) => deref_ptr(a, generator),
        Builtin::Offset(a, b, size) => offset(a, int(b), *size, generator),
        Builtin::Transmute(a, _typ) => transmute_value(a, generator),
        Builtin::StackAlloc(a) => stack_alloc(a, generator),
    }
}

fn add_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_add(a, b, "add").as_basic_value_enum()
}

fn add_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_add(a, b, "add").as_basic_value_enum()
}

fn sub_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_sub(a, b, "sub").as_basic_value_enum()
}

fn sub_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_sub(a, b, "sub").as_basic_value_enum()
}

fn mul_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_mul(a, b, "mul").as_basic_value_enum()
}

fn mul_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_mul(a, b, "mul").as_basic_value_enum()
}

fn div_signed<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_signed_div(a, b, "div").as_basic_value_enum()
}

fn div_unsigned<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_unsigned_div(a, b, "div").as_basic_value_enum()
}

fn div_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_div(a, b, "div").as_basic_value_enum()
}

fn mod_signed<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_signed_rem(a, b, "mod").as_basic_value_enum()
}

fn mod_unsigned<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_unsigned_rem(a, b, "mod").as_basic_value_enum()
}

// Cranelift doesn't support this, perhaps we should remove support altogether for float mod
fn mod_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_rem(a, b, "mod").as_basic_value_enum()
}

fn less_signed<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::SLT, a, b, "less").as_basic_value_enum()
}

fn less_unsigned<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::ULT, a, b, "less").as_basic_value_enum()
}

fn less_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_compare(FloatPredicate::OLT, a, b, "less").as_basic_value_enum()
}

fn eq_int<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").as_basic_value_enum()
}

fn eq_float<'g>(a: &Ast, b: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let a = a.codegen(generator).into_float_value();
    let b = b.codegen(generator).into_float_value();
    generator.builder.build_float_compare(FloatPredicate::OEQ, a, b, "eq").as_basic_value_enum()
}

fn eq_char<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").as_basic_value_enum()
}

fn eq_bool<'g>(a: IntValue<'g>, b: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").as_basic_value_enum()
}

fn deref_ptr<'g>(ptr: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().ptr_type(AddressSpace::Generic);
    let ptr = ptr.codegen(generator).into_pointer_value();
    let ptr = generator.builder.build_pointer_cast(ptr, ret, "bitcast");
    generator.builder.build_load(ptr, "deref").as_basic_value_enum()
}

/// offset (p: Ptr t) (offset: usz) = (p as usize + offset * size_of t) as Ptr t
///
// This builtin is unnecessary once we replace it with size_of
fn offset<'g>(ptr: &Ast, offset: IntValue<'g>, _type_size: u32, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let ptr = ptr.codegen(generator).into_pointer_value();
    unsafe { generator.builder.build_gep(ptr, &[offset], "offset").as_basic_value_enum() }
}

fn transmute_value<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let x = x.codegen(generator);
    let ret = current_function.get_type().get_return_type().unwrap();
    let alloca = generator.builder.build_alloca(x.get_type(), "transmute");
    generator.builder.build_store(alloca, x);
    let casted = generator.builder.build_pointer_cast(alloca, ret.ptr_type(AddressSpace::Generic), "bitcast");
    generator.builder.build_load(casted, "transmute_load")
}

fn sign_extend<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_s_extend(x, ret, "sign_extend").as_basic_value_enum()
}

fn zero_extend<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_z_extend(x, ret, "zero_extend").as_basic_value_enum()
}

fn signed_to_float<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_float_type();
    generator.builder.build_signed_int_to_float(x, ret, "signed_to_float").as_basic_value_enum()
}

fn unsigned_to_float<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_float_type();
    generator.builder.build_unsigned_int_to_float(x, ret, "unsigned_to_float").as_basic_value_enum()
}

fn float_to_signed<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    let x = x.codegen(generator).into_float_value();
    generator.builder.build_float_to_signed_int(x, ret, "float_to_signed").as_basic_value_enum()
}

fn float_to_unsigned<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    let x = x.codegen(generator).into_float_value();
    generator.builder.build_float_to_unsigned_int(x, ret, "float_to_unsigned").as_basic_value_enum()
}

fn truncate<'g>(x: IntValue<'g>, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_truncate(x, ret, "sign_extend").as_basic_value_enum()
}

fn stack_alloc<'g>(x: &Ast, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let value = x.codegen(generator);
    let alloca = generator.builder.build_alloca(value.get_type(), "alloca");
    generator.builder.build_store(alloca, value);

    let ptr_type = &crate::hir::Type::Primitive(PrimitiveType::Pointer);
    let opaque_ptr_type = generator.convert_type(ptr_type).into_pointer_type();

    generator.builder.build_pointer_cast(alloca, opaque_ptr_type, "bitcast").as_basic_value_enum()
}
