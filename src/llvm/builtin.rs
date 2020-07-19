use crate::llvm::{ Generator, LazyValue };
use crate::cache::ModuleCache;

use inkwell::values::{ BasicValue, BasicValueEnum, IntValue, FloatValue };

#[derive(Debug, Copy, Clone)]
pub enum BuiltinFunction {
    AddInt,
    AddFloat,

    SubInt,
    SubFloat,

    MulInt,
    MulFloat,

    DivInt,
    DivFloat,

    ModInt,
    ModFloat,

    LessThanInt,
    LessThanFloat,

    GreaterThanInt,
    GreaterThanFloat,

    LessThanEqualInt,
    LessThanEqualFloat,

    GreaterThanEqualInt,
    GreaterThanEqualFloat,

    EqualInt,
    EqualFloat,
    EqualChar,
    EqualBoolean,
}

pub fn declare_builtin_functions<'g, 'c>(generator: &mut Generator<'g>, cache: &ModuleCache<'c>) {
    use crate::types::Type::*;
    use crate::types::PrimitiveType::*;
    use BuiltinFunction::*;

    let int_function = Function(vec![Primitive(IntegerType), Primitive(IntegerType)], Box::new(Primitive(IntegerType)));
    let float_function = Function(vec![Primitive(FloatType), Primitive(FloatType)], Box::new(Primitive(FloatType)));

    let id = cache.builtins.definitions[0].1;
    generator.definitions.insert((id, int_function.clone()), LazyValue::Thunk(AddInt));
    generator.definitions.insert((id, float_function.clone()), LazyValue::Thunk(AddFloat));

    let id = cache.builtins.definitions[1].1;
    generator.definitions.insert((id, int_function.clone()), LazyValue::Thunk(SubInt));
    generator.definitions.insert((id, float_function.clone()), LazyValue::Thunk(SubFloat));

    let id = cache.builtins.definitions[2].1;
    generator.definitions.insert((id, int_function.clone()), LazyValue::Thunk(MulInt));
    generator.definitions.insert((id, float_function.clone()), LazyValue::Thunk(MulFloat));

    let id = cache.builtins.definitions[3].1;
    generator.definitions.insert((id, int_function.clone()), LazyValue::Thunk(DivInt));
    generator.definitions.insert((id, float_function.clone()), LazyValue::Thunk(DivFloat));

    let id = cache.builtins.definitions[4].1;
    generator.definitions.insert((id, int_function.clone()), LazyValue::Thunk(ModInt));
    generator.definitions.insert((id, float_function.clone()), LazyValue::Thunk(ModFloat));
}

impl<'g, 'c> BuiltinFunction {
    pub fn eval(&self, generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        use BuiltinFunction::*;
        match self {
            AddInt => BuiltinFunction::add_int(generator),
            AddFloat => BuiltinFunction::add_float(generator),

            SubInt => BuiltinFunction::sub_int(generator),
            SubFloat => BuiltinFunction::sub_float(generator),

            MulInt => BuiltinFunction::mul_int(generator),
            MulFloat => BuiltinFunction::mul_float(generator),

            DivInt => BuiltinFunction::div_int(generator),
            DivFloat => BuiltinFunction::div_float(generator),

            ModInt => BuiltinFunction::mod_int(generator),
            ModFloat => BuiltinFunction::mod_float(generator),

            LessThanInt => unimplemented!(),
            LessThanFloat => unimplemented!(),

            GreaterThanInt => unimplemented!(),
            GreaterThanFloat => unimplemented!(),

            LessThanEqualInt => unimplemented!(),
            LessThanEqualFloat => unimplemented!(),

            GreaterThanEqualInt => unimplemented!(),
            GreaterThanEqualFloat => unimplemented!(),

            EqualInt => unimplemented!(),
            EqualFloat => unimplemented!(),
            EqualChar => unimplemented!(),
            EqualBoolean => unimplemented!(),
        }
    }

    fn int_function<F>(name: &str, generator: &mut Generator<'g>, operation: F) -> BasicValueEnum<'g>
        where F: FnOnce(&mut Generator<'g>, IntValue<'g>, IntValue<'g>) -> BasicValueEnum<'g>
    {
        let i32_type = generator.context.i32_type();
        let function_type = i32_type.fn_type(&[i32_type.into(), i32_type.into()], false);
        let function = generator.module.add_function(name, function_type, None);
        let caller_block = generator.builder.get_insert_block().unwrap();
        let basic_block = generator.context.append_basic_block(function, "entry");

        generator.builder.position_at_end(basic_block);

        let arg1 = function.get_nth_param(0).unwrap().into_int_value();
        let arg2 = function.get_nth_param(1).unwrap().into_int_value();
        let value = operation(generator, arg1, arg2);
        generator.builder.build_return(Some(&value));

        generator.builder.position_at_end(caller_block);
        function.as_global_value().as_basic_value_enum()
    }

    fn float_function<F>(name: &str, generator: &mut Generator<'g>, operation: F) -> BasicValueEnum<'g>
        where F: FnOnce(&mut Generator<'g>, FloatValue<'g>, FloatValue<'g>) -> BasicValueEnum<'g>
    {
        let f64_type = generator.context.f64_type();
        let function_type = f64_type.fn_type(&[f64_type.into(), f64_type.into()], false);
        let function = generator.module.add_function(name, function_type, None);
        let caller_block = generator.builder.get_insert_block().unwrap();
        let basic_block = generator.context.append_basic_block(function, "entry");

        generator.builder.position_at_end(basic_block);

        let arg1 = function.get_nth_param(0).unwrap().into_float_value();
        let arg2 = function.get_nth_param(1).unwrap().into_float_value();
        let value = operation(generator, arg1, arg2);
        generator.builder.build_return(Some(&value));

        generator.builder.position_at_end(caller_block);
        function.as_global_value().as_basic_value_enum()
    }

    fn add_int(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::int_function("+", generator, |generator, a, b|
            generator.builder.build_int_add(a, b, "add").as_basic_value_enum()
        )
    }

    fn add_float(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::float_function("+", generator, |generator, a, b|
            generator.builder.build_float_add(a, b, "add").as_basic_value_enum()
        )
    }

    fn sub_int(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::int_function("-", generator, |generator, a, b|
            generator.builder.build_int_sub(a, b, "sub").as_basic_value_enum()
        )
    }

    fn sub_float(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::float_function("-", generator, |generator, a, b|
            generator.builder.build_float_sub(a, b, "sub").as_basic_value_enum()
        )
    }

    fn mul_int(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::int_function("*", generator, |generator, a, b|
            generator.builder.build_int_mul(a, b, "mul").as_basic_value_enum()
        )
    }

    fn mul_float(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::float_function("*", generator, |generator, a, b|
            generator.builder.build_float_mul(a, b, "mul").as_basic_value_enum()
        )
    }

    fn div_int(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::int_function("/", generator, |generator, a, b|
            generator.builder.build_int_signed_div(a, b, "div").as_basic_value_enum()
        )
    }

    fn div_float(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::float_function("/", generator, |generator, a, b|
            generator.builder.build_float_div(a, b, "div").as_basic_value_enum()
        )
    }

    fn mod_int(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::int_function("%", generator, |generator, a, b|
            generator.builder.build_int_signed_rem(a, b, "mod").as_basic_value_enum()
        )
    }

    fn mod_float(generator: &mut Generator<'g>) -> BasicValueEnum<'g> {
        BuiltinFunction::float_function("%", generator, |generator, a, b|
            generator.builder.build_float_rem(a, b, "mod").as_basic_value_enum()
        )
    }
}
