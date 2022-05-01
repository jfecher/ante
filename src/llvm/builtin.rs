//! llvm/builtin.rs - Defines the `builtin` function used in the
//! prelude to implement builtin operators such as addition for integers,
//! multiplication for floats, etc.
//!
//! Note that the builtin function only
//! takes a string as its argument which is matched on in `call_builtin`
//! to get the corresponding builtin operation. Since these operations
//! expect the llvm::Function to have a certain signature, the `builtin`
//! function is prevented from being used outside the prelude.
use crate::hir::Builtin;
use crate::llvm::Generator;

use inkwell::values::{ BasicValue, BasicValueEnum, IntValue, FloatValue };
use inkwell::{ IntPredicate, FloatPredicate };
use inkwell::attributes::{ Attribute, AttributeLoc };

pub fn call_builtin<'g, 'c>(builtin: &Builtin, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let always_inline = Attribute::get_named_enum_kind_id("alwaysinline");
    assert_ne!(always_inline, 0);
    let attribute = generator.context.create_enum_attribute(always_inline, 1);
    current_function.add_attribute(AttributeLoc::Function, attribute);

    match builtin {
        Builtin::AddInt => add_int(generator),
        Builtin::AddFloat => add_float(generator),

        Builtin::SubInt => sub_int(generator),
        Builtin::SubFloat => sub_float(generator),

        Builtin::MulInt => mul_int(generator),
        Builtin::MulFloat => mul_float(generator),

        Builtin::DivInt => div_int(generator),
        Builtin::DivFloat => div_float(generator),

        Builtin::ModInt => mod_int(generator),
        Builtin::ModFloat => mod_float(generator),

        Builtin::LessInt => less_int(generator),
        Builtin::LessFloat => less_float(generator),

        Builtin::GreaterInt => greater_int(generator),
        Builtin::GreaterFloat => greater_float(generator),

        Builtin::EqInt => eq_int(generator),
        Builtin::EqFloat => eq_float(generator),
        Builtin::EqChar => eq_char(generator),
        Builtin::EqBool => eq_bool(generator),

        Builtin::SignExtend => sign_extend(generator),
        Builtin::ZeroExtend => zero_extend(generator),

        Builtin::Truncate => truncate(generator),

        Builtin::Deref => deref_ptr(generator),
        Builtin::Offset => offset(generator),
        Builtin::Transmute => transmute_value(generator),
    }
}

fn two_int_parameters<'g>(generator: &Generator<'g>) -> (IntValue<'g>, IntValue<'g>) {
    let current_function = generator.current_function();
    let a = current_function.get_nth_param(0).unwrap().into_int_value();
    let b = current_function.get_nth_param(1).unwrap().into_int_value();
    (a, b)
}

fn two_float_parameters<'g>(generator: &Generator<'g>) -> (FloatValue<'g>, FloatValue<'g>) {
    let current_function = generator.current_function();
    let a = current_function.get_nth_param(0).unwrap().into_float_value();
    let b = current_function.get_nth_param(1).unwrap().into_float_value();
    (a, b)
}

fn add_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    generator.builder.build_int_add(a, b, "add").as_basic_value_enum()
}

fn add_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_add(a, b, "add").as_basic_value_enum()
}

fn sub_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    generator.builder.build_int_sub(a, b, "sub").as_basic_value_enum()
}

fn sub_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_sub(a, b, "sub").as_basic_value_enum()
}

fn mul_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    generator.builder.build_int_mul(a, b, "mul").as_basic_value_enum()
}

fn mul_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_mul(a, b, "mul").as_basic_value_enum()
}

fn div_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    // TODO: unsigned
    generator.builder.build_int_signed_div(a, b, "div").as_basic_value_enum()
}

fn div_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_div(a, b, "div").as_basic_value_enum()
}

fn mod_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    generator.builder.build_int_signed_rem(a, b, "mod").as_basic_value_enum()
}

fn mod_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_rem(a, b, "mod").as_basic_value_enum()
}

fn less_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    generator.builder.build_int_compare(IntPredicate::SLT, a, b, "less").as_basic_value_enum()
}

fn less_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_compare(FloatPredicate::OLT, a, b, "less").as_basic_value_enum()
}

fn greater_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    generator.builder.build_int_compare(IntPredicate::SGT, a, b, "greater").as_basic_value_enum()
}

fn greater_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_compare(FloatPredicate::OGT, a, b, "greater").as_basic_value_enum()
}

fn eq_int<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_int_parameters(generator);
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").as_basic_value_enum()
}

fn eq_float<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let (a, b) = two_float_parameters(generator);
    generator.builder.build_float_compare(FloatPredicate::OEQ, a, b, "eq").as_basic_value_enum()
}

fn eq_char<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let a = current_function.get_nth_param(0).unwrap().into_int_value();
    let b = current_function.get_nth_param(1).unwrap().into_int_value();
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").as_basic_value_enum()
}

fn eq_bool<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let a = current_function.get_nth_param(0).unwrap().into_int_value();
    let b = current_function.get_nth_param(1).unwrap().into_int_value();
    generator.builder.build_int_compare(IntPredicate::EQ, a, b, "eq").as_basic_value_enum()
}

fn deref_ptr<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ptr = current_function.get_nth_param(0).unwrap().into_pointer_value();
    generator.builder.build_load(ptr, "deref").as_basic_value_enum()
}

/// offset (p: Ptr t) (offset: usz) = (p as usize + offset * size_of t) as Ptr t
///
// This builtin is unnecessary once we replace it with size_of
fn offset<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let ptr = current_function.get_nth_param(0).unwrap().into_pointer_value();
    let offset = current_function.get_nth_param(1).unwrap().into_int_value();

    unsafe { generator.builder.build_gep(ptr, &[offset], "offset").as_basic_value_enum() }
}

fn transmute_value<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let x = current_function.get_nth_param(0).unwrap();
    let ret = current_function.get_type().get_return_type().unwrap();
    generator.builder.build_bitcast(x, ret, "transmute")
}

fn sign_extend<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let x = current_function.get_nth_param(0).unwrap().as_basic_value_enum().into_int_value();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_s_extend(x, ret, "sign_extend").as_basic_value_enum()
}

fn zero_extend<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let x = current_function.get_nth_param(0).unwrap().as_basic_value_enum().into_int_value();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_z_extend(x, ret, "zero_extend").as_basic_value_enum()
}

fn truncate<'g>(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
    let current_function = generator.current_function();
    let x = current_function.get_nth_param(0).unwrap().as_basic_value_enum().into_int_value();
    let ret = current_function.get_type().get_return_type().unwrap().into_int_type();
    generator.builder.build_int_truncate(x, ret, "sign_extend").as_basic_value_enum()
}
