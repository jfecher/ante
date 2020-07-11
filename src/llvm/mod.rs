//! Llvm backend for ante.
//! At the time of writing this is the only backend though in the future there is a cranelift
//! backend planned for faster debug build times and faster build times for the compiler itself
//! so that new users won't have to subject themselves to building llvm.

use crate::cache::{ ModuleCache, DefinitionInfoId, DefinitionNode };
use crate::parser::ast;
use crate::types::{ self, typechecker, TypeVariableId };
use crate::util::{ fmap, trustme };

use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::values::{ BasicValueEnum, BasicValue, FunctionValue };
use inkwell::types::{ BasicTypeEnum, BasicType };
use inkwell::AddressSpace;
use inkwell::OptimizationLevel::Aggressive;
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::targets::{InitializationConfig, Target, TargetMachine };

use std::path::Path;
use std::ffi::OsStr;
use std::collections::HashMap;

#[derive(Debug)]
struct Generator<'context> {
    context: &'context Context,
    module: Module<'context>,
    builder: Builder<'context>,

    definitions: HashMap<(DefinitionInfoId, types::Type), BasicValueEnum<'context>>,

    types: HashMap<(types::TypeInfoId, Vec<types::Type>), BasicValueEnum<'context>>,

    /// A stack of the current typevar bindings during monomorphisation. Unlike normal bindings,
    /// these are meant to be easily undone. Since ante doesn't support polymorphic recursion,
    /// we also don't have to worry about encountering the same typevar with a different
    /// monomorphisation binding.
    monomorphisation_bindings: Vec<typechecker::TypeBindings>,

    current_function: Option<FunctionValue<'context>>,
    current_function_info: Option<DefinitionInfoId>,
}

pub fn run<'c>(path: &Path, ast: &ast::Ast<'c>, cache: &mut ModuleCache<'c>) {
    let context = Context::create();
    let module_name = path_to_module_name(path);
    let module = context.create_module(module_name);
    module.set_triple(&TargetMachine::get_default_triple());
    let mut codegen = Generator {
        context: &context,
        module,
        builder: context.create_builder(),
        definitions: HashMap::new(),
        types: HashMap::new(),
        monomorphisation_bindings: vec![],
        current_function: None,
        current_function_info: None,
    };

    codegen.codegen_main(ast, cache);
    codegen.optimize();
    codegen.output(module_name);
}

fn path_to_module_name(path: &Path) -> &str {
    path.file_stem().and_then(OsStr::to_str).unwrap_or("foo")
}

fn remove_forall(typ: &types::Type) -> &types::Type {
    match typ {
        types::Type::ForAll(_, t) => t,
        _ => typ,
    }
}

// TODO: remove
const UNBOUND_TYPE: types::Type = types::Type::Primitive(types::PrimitiveType::UnitType);

impl<'g> Generator<'g> {
    fn codegen_main<'c>(&mut self, ast: &ast::Ast<'c>, cache: &mut ModuleCache<'c>) {
        let i32_type = self.context.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let function = self.module.add_function("main", main_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");

        self.current_function = Some(function);
        self.builder.position_at_end(basic_block);

        ast.codegen(self, cache);

        let success = i32_type.const_int(0, true);
        self.builder.build_return(Some(&success));
    }

    fn optimize(&self) {
        let config = InitializationConfig::default();
        Target::initialize_native(&config).unwrap();
        let pass_manager_builder = PassManagerBuilder::create();

        pass_manager_builder.set_optimization_level(Aggressive);
        let pass_manager = PassManager::create(());
        pass_manager_builder.populate_module_pass_manager(&pass_manager);
        pass_manager.run_on(&self.module);
    }

    fn output(&self, module_name: &str) {
        self.module.print_to_stderr();
        let path = Path::new(module_name).with_extension("ll");
        self.module.write_bitcode_to_path(&path);

        let output = "-o".to_string() + module_name;
        std::process::Command::new("clang")
            .arg(path.to_string_lossy().as_ref())
            .arg("-Wno-everything")
            .arg(output)
            .spawn().unwrap();
    }

    fn get_string_type(&self) -> inkwell::types::StructType<'g> {
        let c8_pointer = self.context.i8_type().ptr_type(AddressSpace::Global).into();
        let usz = self.context.i64_type().into();
        self.context.struct_type(&[c8_pointer, usz], false)
    }

    fn lookup<'c>(&self, id: DefinitionInfoId, typ: &types::Type, cache: &ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let typ = self.follow_bindings(typ, cache);
        let value = self.definitions.get(&(id, typ));
        value.map(|x| *x)
    }

    fn monomorphise<'c>(&mut self, id: DefinitionInfoId, typ: &types::Type, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let definition = &mut cache.definition_infos[id.0];
        let definition = trustme::extend_lifetime_mut(definition);
        let definition_type = remove_forall(definition.typ.as_ref().unwrap());

        let mut bindings = HashMap::new();
        typechecker::try_unify(typ, definition_type, &mut bindings, definition.location, cache)
            .expect("Unification error during monomorphisation");

        self.monomorphisation_bindings.push(bindings);

        // Compile the definition with the bindings in scope. Each definition is expected to
        // add itself to Generator.definitions
        match &definition.definition {
            Some(DefinitionNode::Definition(definition)) => {
                self.codegen_monomorphise(*definition, cache);
            }
            _ => unimplemented!(),
        }

        self.monomorphisation_bindings.pop();
        self.lookup(id, typ, cache)
    }

    fn find_binding<'c, 'b>(&'b self, id: TypeVariableId, cache: &'b ModuleCache<'c>) -> &types::Type {
        use types::TypeBinding::*;
        use types::Type::TypeVariable;

        match &cache.type_bindings[id.0] {
            Bound(TypeVariable(id)) => self.find_binding(*id, cache),
            Bound(binding) => binding,
            Unbound(..) => {
                for bindings in self.monomorphisation_bindings.iter().rev() {
                    if let Some(binding) = bindings.get(&id) {
                        return binding;
                    }
                }
                println!("Unbound type variable found during code generation");
                &UNBOUND_TYPE
            },
        }
    }

    fn size_of_type<'c>(&self, typ: &types::Type, cache: &ModuleCache<'c>) -> usize {
        use types::Type::*;
        use types::PrimitiveType::*;
        match typ {
            Primitive(IntegerType) => 4,
            Primitive(FloatType) => 8,
            Primitive(CharType) => 1,
            Primitive(StringType) => 8 + 8,
            Primitive(BooleanType) => 1,
            Primitive(UnitType) => 1,
            Primitive(ReferenceType) => 8,

            Function(..) => 8,

            TypeVariable(id) => {
                let binding = self.find_binding(*id, cache);
                self.size_of_type(binding, cache)
            },

            UserDefinedType(id) => {
                let _info = &cache.type_infos[id.0];
                unimplemented!();
            },

            TypeApplication(_typ, _args) => {
                unimplemented!();
            },

            ForAll(_, typ) => self.size_of_type(typ, cache),
        }
    }

    fn convert_primitive_type(&self, typ: &types::PrimitiveType) -> BasicTypeEnum<'g> {
        use types::PrimitiveType::*;
        match typ {
            IntegerType => self.context.i32_type().into(),
            FloatType => self.context.f64_type().into(),
            CharType => self.context.i8_type().into(),
            StringType => self.get_string_type().into(),
            BooleanType => self.context.bool_type().into(),
            UnitType => self.context.bool_type().into(),
            ReferenceType => unreachable!("Kind error during code generation"),
        }
    }

    fn convert_struct_type<'c>(&self, info: &types::TypeInfo, args: &Vec<types::Field<'c>>, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
        // TODO: cache the struct type for recursive types.
        let typ = self.context.opaque_struct_type(&info.name);

        let fields = fmap(&args, |x| self.convert_type(&x.field_type, cache));

        typ.set_body(&fields, false);
        typ.into()
    }

    fn convert_union_type<'c>(&self, info: &types::TypeInfo, args: &Vec<types::TypeConstructor<'c>>, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
        // TODO: cache the struct type for recursive types. - possibly cache in convert_type
        // TypeApplication case
        let typ = self.context.opaque_struct_type(&info.name);

        let max_size = 0;
        let mut largest_variant = None;
        for variant in args.iter() {
            let size: usize = variant.args.iter().map(|arg| self.size_of_type(arg, cache)).sum();
            if size > max_size {
                largest_variant = Some(variant);
            }
        }

        if let Some(variant) = largest_variant {
            let fields = fmap(&variant.args, |typ| self.convert_type(typ, cache));
            typ.set_body(&fields, false);
        }
        typ.into()
    }

    fn convert_type_constructor<'c>(&self, typ: &types::Type, args: Vec<BasicTypeEnum<'g>>, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
        use types::Type::*;
        match typ {
            Primitive(primitive) => {
                // ref is the only primitive type constructor
                assert!(*primitive == types::PrimitiveType::ReferenceType && args.len() == 1);
                args[0].ptr_type(AddressSpace::Global).into()
            },

            Function(_arg_types, _return_type) => {
                unimplemented!("function types cannot yet be used in a type constructor position")
            },

            TypeVariable(id) => {
                let binding = self.find_binding(*id, cache);
                self.convert_type_constructor(binding, args, cache)
            },

            UserDefinedType(id) => {
                let _info = &cache.type_infos[id.0];
                unimplemented!();
            },

            TypeApplication(_typ, _args) => {
                unimplemented!();
            },

            ForAll(_, typ) => self.convert_type(typ, cache),
        }
    }

    fn convert_type<'c>(&self, typ: &types::Type, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
        use types::Type::*;
        match typ {
            Primitive(primitive) => self.convert_primitive_type(primitive),

            Function(arg_types, return_type) => {
                let args = fmap(arg_types, |typ| self.convert_type(typ, cache));
                let return_type = self.convert_type(return_type, cache);
                return_type.fn_type(&args, false).ptr_type(AddressSpace::Global).into()
            },

            TypeVariable(id) => self.convert_type(self.find_binding(*id, cache), cache),

            UserDefinedType(id) => {
                let info = &cache.type_infos[id.0];
                assert!(info.args.is_empty(), "Kind error during llvm code generation");
                
                use types::TypeInfoBody::*;
                match &info.body {
                    Union(args) => self.convert_union_type(info, args, cache),
                    Struct(fields) => self.convert_struct_type(info, fields, cache),
                    Alias(typ) => self.convert_type(typ, cache),
                    Unknown => unreachable!(),
                }
            },

            TypeApplication(typ, args) => {
                let args = fmap(args, |arg| self.convert_type(arg, cache));
                self.convert_type_constructor(typ, args, cache)
            },

            ForAll(_, typ) => self.convert_type(typ, cache),
        }
    }

    fn unit_value(&self) -> BasicValueEnum<'g> {
        // TODO: compile () to void, mainly higher-order functions and struct/tuple
        // indexing need to be addressed for this.
        let i1 = self.context.bool_type();
        i1.const_int(0, false).into()
    }

    fn follow_bindings<'c>(&self, typ: &types::Type, cache: &ModuleCache<'c>) -> types::Type {
        use types::Type::*;
        match typ {
            Primitive(primitive) => Primitive(*primitive),

            Function(arg_types, return_type) => {
                let args = fmap(arg_types, |typ| self.follow_bindings(typ, cache));
                let return_type = self.follow_bindings(return_type, cache);
                Function(args, Box::new(return_type))
            },

            TypeVariable(id) => self.follow_bindings(self.find_binding(*id, cache), cache),

            UserDefinedType(id) => UserDefinedType(*id),

            TypeApplication(typ, args) => {
                let typ = self.follow_bindings(typ, cache);
                let args = fmap(args, |arg| self.follow_bindings(arg, cache));
                TypeApplication(Box::new(typ), args)
            },

            // unwrap foralls
            ForAll(_, typ) => self.follow_bindings(typ, cache),
        }
    }

    fn bind_irrefutable_pattern<'c>(&mut self, ast: &ast::Ast<'c>, value: BasicValueEnum<'g>, cache: &mut ModuleCache<'c>) {
        use ast::Ast::*;
        use ast::LiteralKind;
        match ast {
            Literal(literal) => {
                assert!(literal.kind == LiteralKind::Unit)
                // pass, we don't need to actually do any assignment when ignoring unit values
            },
            Variable(variable) => {
                let id = variable.definition.unwrap();
                let typ = self.follow_bindings(variable.typ.as_ref().unwrap(), cache);
                self.definitions.insert((id, typ), value);
            },
            TypeAnnotation(annotation) => {
                self.bind_irrefutable_pattern(annotation.lhs.as_ref(), value, cache);
            },
            _ => {
                unreachable!();
            }
        }
    }

    // codegen a Definition that should be monomorphised.
    // Really all definitions should be monomorphised, this is just used as a wrapper so
    // we only compilie function definitions when they're used at their call sites so that
    // we have all the monomorphisation bindings in scope.
    fn codegen_monomorphise<'c>(&mut self, definition: &ast::Definition<'c>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        // If we're defining a lambda, give the lambda info on DefinitionInfoId so that it knows
        // what to name itself in the IR and so recursive functions can properly codegen without
        // attempting to re-compile themselves over and over.
        if matches!(definition.expr.as_ref(), ast::Ast::Lambda(..)) {
            match definition.pattern.as_ref() {
                ast::Ast::Variable(variable) => {
                    self.current_function_info = Some(variable.definition.unwrap());
                }
                _ => (),
            }
        }

        let value = definition.expr.codegen(self, cache).unwrap();
        self.bind_irrefutable_pattern(definition.pattern.as_ref(), value, cache);
        Some(value)
    }
}

trait CodeGen<'g, 'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>>;
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Ast<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        dispatch_on_expr!(self, CodeGen::codegen, generator, cache)
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Literal<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, _cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        match &self.kind {
            ast::LiteralKind::Char(c) => {
                let c8 = generator.context.i8_type();
                Some(c8.const_int(*c as u64, false).into())
            },
            ast::LiteralKind::Bool(b) => {
                let i1 = generator.context.bool_type();
                Some(i1.const_int(*b as u64, false).into())
            },
            ast::LiteralKind::Float(f) => {
                let float = generator.context.f64_type();
                Some(float.const_float(*f).into())
            },
            ast::LiteralKind::Integer(i) => {
                let int = generator.context.i32_type();
                Some(int.const_int(*i, true).into())
            },
            ast::LiteralKind::String(s) => {
                let string = generator.get_string_type();
                let literal = generator.context.const_string(s.as_bytes(), true);
                let length = generator.context.i64_type().const_int(s.len() as u64, false);
                Some(string.const_named_struct(&[literal.into(), length.into()]).into())
            },
            ast::LiteralKind::Unit => {
                Some(generator.unit_value())
            },
        }
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Variable<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let id = self.definition.unwrap();
        match generator.lookup(id, self.typ.as_ref().unwrap(), cache) {
            Some(value) => Some(value),
            None => generator.monomorphise(id, self.typ.as_ref().unwrap(), cache),
        }
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Lambda<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let typ = generator.convert_type(self.typ.as_ref().unwrap(), cache).into_pointer_type().get_element_type();

        let function_name = match &generator.current_function_info {
            Some(id) => &cache.definition_infos[id.0].name,
            None => "lambda",
        };

        let function = generator.module.add_function(&function_name, typ.into_function_type(), None);

        // Cache the function value so recursive functions can call themselves without trying
        // to infinitely re-compile their Definitions
        let function_pointer = function.as_global_value().as_pointer_value().into();
        if let Some(id) = generator.current_function_info {
            let typ = generator.follow_bindings(self.typ.as_ref().unwrap(), cache);
            generator.definitions.insert((id, typ), function_pointer);
            generator.current_function_info = None;
        }

        generator.current_function = Some(function);

        let basic_block = generator.context.append_basic_block(function, "entry");

        let caller_block = generator.builder.get_insert_block().unwrap();
        generator.builder.position_at_end(basic_block);

        let return_value = self.body.codegen(generator, cache);

        generator.builder.build_return(return_value.as_ref().map(|x| x as &dyn BasicValue));
        generator.builder.position_at_end(caller_block);

        Some(function_pointer)
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::FunctionCall<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        // TODO: lookup builtin functions
        let function = self.function.codegen(generator, cache).unwrap();
        let args = fmap(&self.args, |arg| arg.codegen(generator, cache).unwrap());
        generator.builder.build_call(function.into_pointer_value(), &args, "").try_as_basic_value().left()
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Definition<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        // If we're defining a lambda, give the lambda info on DefinitionInfoId so that it knows
        // what to name itself in the IR and so recursive functions can properly codegen without
        // attempting to re-compile themselves over and over.
        if !matches!(self.expr.as_ref(), ast::Ast::Lambda(..)) {
            let value = self.expr.codegen(generator, cache).unwrap();
            generator.bind_irrefutable_pattern(self.pattern.as_ref(), value, cache);
            Some(value)
        } else {
            None
        }
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::If<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let condition = self.condition.codegen(generator, cache).unwrap();

        let then_block = generator.context.append_basic_block(generator.current_function.unwrap(), "then");

        // TODO: Cleanup
        if let Some(otherwise) = &self.otherwise {
            let else_block = generator.context.append_basic_block(generator.current_function.unwrap(), "else");
            generator.builder.build_conditional_branch(condition.into_int_value(), then_block, else_block);
            let end_block = generator.context.append_basic_block(generator.current_function.unwrap(), "end_if");

            generator.builder.position_at_end(then_block);
            let then_value = self.then.codegen(generator, cache).unwrap();
            generator.builder.build_unconditional_branch(end_block);

            generator.builder.position_at_end(else_block);
            let else_value = otherwise.codegen(generator, cache).unwrap();
            generator.builder.build_unconditional_branch(end_block);

            generator.builder.position_at_end(end_block);

            let phi = generator.builder.build_phi(then_value.get_type(), "if_result");
            phi.add_incoming(&[(&then_value, then_block), (&else_value, else_block)]);
            Some(phi.as_basic_value())
        } else {
            let end_block = generator.context.append_basic_block(generator.current_function.unwrap(), "end_if");
            generator.builder.build_conditional_branch(condition.into_int_value(), then_block, end_block);

            generator.builder.position_at_end(then_block);
            self.then.codegen(generator, cache).unwrap();
            generator.builder.build_unconditional_branch(end_block);

            generator.builder.position_at_end(end_block);
            Some(generator.unit_value())
        }
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Match<'c> {
    fn codegen(&self, _generator: &mut Generator<'g>, _cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        unimplemented!()
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::TypeDefinition<'c> {
    fn codegen(&self, _generator: &mut Generator<'g>, _cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        None
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::TypeAnnotation<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        self.lhs.codegen(generator, cache)
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Import<'c> {
    fn codegen(&self, _generator: &mut Generator<'g>, _cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        None
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::TraitDefinition<'c> {
    fn codegen(&self, _generator: &mut Generator<'g>, _cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        None
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::TraitImpl<'c> {
    fn codegen(&self, _generator: &mut Generator<'g>, _cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        None
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Return<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let value = self.expression.codegen(generator, cache).unwrap();
        generator.builder.build_return(Some(&value));
        Some(value)
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Sequence<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let mut last_value = None;
        for statement in self.statements.iter() {
            last_value = statement.codegen(generator, cache);
        }
        last_value
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Extern<'c> {
    fn codegen(&self, _generator: &mut Generator<'g>, _cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        None
    }
}
