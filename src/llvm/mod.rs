//! Llvm backend for ante.
//! At the time of writing this is the only backend though in the future there is a cranelift
//! backend planned for faster debug build times and faster build times for the compiler itself
//! so that new users won't have to subject themselves to building llvm.

use crate::cache::{ ModuleCache, DefinitionInfoId, DefinitionNode };
use crate::parser::ast;
use crate::nameresolution::builtin::BUILTIN_ID;
use crate::types::{ self, typechecker, TypeVariableId, TypeBinding, TypeInfoId };
use crate::types::typed::Typed;
use crate::util::{ fmap, trustme };

use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::values::{ BasicValueEnum, BasicValue, FunctionValue };
use inkwell::types::{ BasicTypeEnum, BasicType };
use inkwell::AddressSpace;
use inkwell::targets::{ RelocMode, CodeModel, FileType, TargetTriple };
use inkwell::OptimizationLevel;
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::targets::{InitializationConfig, Target, TargetMachine };

use std::collections::{ HashMap, HashSet };
use std::path::{ Path, PathBuf };
use std::process::Command;

mod builtin;

#[derive(Debug)]
pub struct Generator<'context> {
    context: &'context Context,
    module: Module<'context>,
    builder: Builder<'context>,

    definitions: HashMap<(DefinitionInfoId, types::Type), BasicValueEnum<'context>>,

    types: HashMap<(types::TypeInfoId, Vec<types::Type>), BasicTypeEnum<'context>>,

    /// A stack of the current typevar bindings during monomorphisation. Unlike normal bindings,
    /// these are meant to be easily undone. Since ante doesn't support polymorphic recursion,
    /// we also don't have to worry about encountering the same typevar with a different
    /// monomorphisation binding.
    monomorphisation_bindings: Vec<typechecker::TypeBindings>,

    /// Contains all the definition ids that should be automatically dereferenced because they're
    /// either stored locally in an alloca or in a global.
    auto_derefs: HashSet<DefinitionInfoId>,

    current_function: Option<FunctionValue<'context>>,
    current_function_info: Option<DefinitionInfoId>,
}

pub fn run<'c>(path: &Path, ast: &ast::Ast<'c>, cache: &mut ModuleCache<'c>, show_ir: bool, run_program: bool, delete_binary: bool) {
    let context = Context::create();
    let module_name = path_to_module_name(path);
    let module = context.create_module(&module_name);

    let target_triple = TargetMachine::get_default_triple();
    module.set_triple(&target_triple);
    let mut codegen = Generator {
        context: &context,
        module,
        builder: context.create_builder(),
        definitions: HashMap::new(),
        types: HashMap::new(),
        monomorphisation_bindings: vec![],
        auto_derefs: HashSet::new(),
        current_function: None,
        current_function_info: None,
    };

    codegen.codegen_main(ast, cache);

    codegen.module.verify().map_err(|error| {
        codegen.module.print_to_stderr();
        println!("{}", error);
    }).unwrap();

    codegen.optimize();

    // --show-llvm-ir: Dump the LLVM-IR of the generated module to stderr.
    // Useful to debug codegen
    if show_ir {
        codegen.module.print_to_stderr();
    }

    let binary_name = module_name_to_program_name(&module_name);
    codegen.output(module_name, &binary_name, &target_triple, &codegen.module);

    // --run: compile and run the program
    if run_program {
        let program_command = PathBuf::from("./".to_string() + &binary_name);
        Command::new(&program_command).spawn().unwrap().wait().unwrap();
    }

    // --delete-binary: remove the binary after running the program to
    // avoid littering a testing directory with temporary binaries
    if delete_binary {
        std::fs::remove_file(binary_name).unwrap();
    }
}

fn path_to_module_name(path: &Path) -> String {
    path.with_extension("").to_string_lossy().into()
}

fn module_name_to_program_name(module: &str) -> String {
    if cfg!(target_os = "windows") {
        PathBuf::from(module).with_extension("exe").to_string_lossy().into()
    } else {
        PathBuf::from(module).with_extension("").to_string_lossy().into()
    }
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

        pass_manager_builder.set_optimization_level(OptimizationLevel::Aggressive);
        let pass_manager = PassManager::create(());
        pass_manager_builder.populate_module_pass_manager(&pass_manager);
        pass_manager.run_on(&self.module);
    }

    fn output(&self, module_name: String, binary_name: &str, target_triple: &TargetTriple, module: &Module) {
        // generate the bitcode to a .bc file
        let path = Path::new(&module_name).with_extension("o");
        let target = Target::from_triple(&target_triple).unwrap();
        let target_machine = target.create_target_machine(&target_triple, "x86-64", "+avx2",
                OptimizationLevel::None, RelocMode::PIC, CodeModel::Default).unwrap();

        target_machine.write_to_file(&module, FileType::Object, &path).unwrap();

        // call gcc to compile the bitcode to a binary
        let output = "-o".to_string() + binary_name;
        let mut child = Command::new("gcc")
            .arg(path.to_string_lossy().as_ref())
            .arg("-Wno-everything")
            .arg("-O0")
            .arg("-lm")
            .arg(output)
            .spawn().unwrap();

        // remove the temporary bitcode file
        child.wait().unwrap();
        std::fs::remove_file(path).unwrap();
    }

    fn lookup<'c>(&mut self, id: DefinitionInfoId, typ: &types::Type, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let typ = self.follow_bindings(typ, cache);
        self.definitions.get(&(id, typ)).map(|value| *value)
    }

    fn monomorphise<'c>(&mut self, id: DefinitionInfoId, typ: &types::Type, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let definition = &mut cache.definition_infos[id.0];
        let definition = trustme::extend_lifetime(definition);
        let definition_type = remove_forall(definition.typ.as_ref().unwrap());

        let mut bindings = HashMap::new();
        typechecker::try_unify(typ, definition_type, &mut bindings, definition.location, cache)
            .map_err(|error| println!("{}", error))
            .expect("Unification error during monomorphisation");

        self.monomorphisation_bindings.push(bindings);
        let value;

        // Compile the definition with the bindings in scope. Each definition is expected to
        // add itself to Generator.definitions
        match &definition.definition {
            Some(DefinitionNode::Definition(definition)) => {
                value = Some(self.codegen_monomorphise(*definition, cache));
            }
            Some(DefinitionNode::Extern(_)) => {
                value = Some(self.codegen_extern(id, typ, cache));
            }
            Some(DefinitionNode::TypeConstructor { name, tag }) => {
                value = Some(self.codegen_type_constructor(name, tag, typ, cache))
            },
            Some(DefinitionNode::TraitDefinition(_)) => {
                unreachable!("There is no code in a trait definition that can be codegen'd.\nNo cached impl for {}: {}", definition.name, typ.display(cache));
            },
            Some(DefinitionNode::Impl) => {
                unreachable!("There is no code in a trait impl that can be codegen'd.\nNo cached impl for {}: {}", definition.name, typ.display(cache));
            },
            Some(DefinitionNode::Parameter) => {
                unreachable!("There is no code to (lazily) codegen for parameters");
            },
            None => unreachable!("No definition for {}", definition.name),
        }

        self.monomorphisation_bindings.pop();
        value
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
                // println!("Unbound type variable found during code generation");
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
                unimplemented!("size_of_type(UserDefinedType) is unimplemented");
            },

            TypeApplication(_typ, _args) => {
                unimplemented!("size_of_type(TypeApplication) is unimplemented");
            },

            Tuple(elements) => {
                elements.iter().map(|element| self.size_of_type(element, cache)).sum()
            }

            ForAll(_, typ) => self.size_of_type(typ, cache),
        }
    }

    fn convert_primitive_type(&self, typ: &types::PrimitiveType) -> BasicTypeEnum<'g> {
        use types::PrimitiveType::*;
        match typ {
            IntegerType => self.context.i32_type().into(),
            FloatType => self.context.f64_type().into(),
            CharType => self.context.i8_type().into(),
            BooleanType => self.context.bool_type().into(),
            UnitType => self.context.bool_type().into(),
            ReferenceType => unreachable!("Kind error during code generation"),
        }
    }

    fn convert_struct_type<'c>(&mut self, info: &types::TypeInfo, args: &Vec<types::Field<'c>>, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
        // TODO: cache the struct type for recursive types.
        let typ = self.context.opaque_struct_type(&info.name);

        let fields = fmap(&args, |x| self.convert_type(&x.field_type, cache));

        typ.set_body(&fields, false);
        typ.into()
    }

    fn convert_union_type<'c>(&mut self, info: &types::TypeInfo, args: &Vec<types::TypeConstructor<'c>>, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
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

    fn convert_user_defined_type<'c>(&mut self, id: TypeInfoId, args: Vec<types::Type>, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
        let info = &cache.type_infos[id.0];
        assert!(info.args.len() == args.len(), "Kind error during llvm code generation");

        if let Some(typ) = self.types.get(&(id, args.clone())) {
            return *typ;
        }

        use types::TypeInfoBody::*;
        let typ = match &info.body {
            Union(args) => self.convert_union_type(info, args, cache),
            Struct(fields) => self.convert_struct_type(info, fields, cache),
            Alias(typ) => self.convert_type(typ, cache),
            Unknown => unreachable!(),
        };

        self.types.insert((id, args), typ);
        typ
    }

    fn convert_type<'c>(&mut self, typ: &types::Type, cache: &ModuleCache<'c>) -> BasicTypeEnum<'g> {
        use types::Type::*;
        use types::PrimitiveType::ReferenceType;
        match typ {
            Primitive(primitive) => self.convert_primitive_type(primitive),

            Function(arg_types, return_type) => {
                let args = fmap(arg_types, |typ| self.convert_type(typ, cache));
                let return_type = self.convert_type(return_type, cache);
                return_type.fn_type(&args, false).ptr_type(AddressSpace::Global).into()
            },

            TypeVariable(id) => self.convert_type(&self.find_binding(*id, cache).clone(), cache),

            UserDefinedType(id) => self.convert_user_defined_type(*id, vec![], cache),

            Tuple(elements) => {
                let element_types = fmap(elements, |element| self.convert_type(element, cache));
                self.context.struct_type(&element_types, false).as_basic_type_enum()
            },

            TypeApplication(typ, args) => {
                let args = fmap(args, |arg| self.follow_bindings(arg, cache));
                let typ = self.follow_bindings(typ, cache);

                match &typ {
                    Primitive(ReferenceType) => {
                        assert!(args.len() == 1);
                        self.convert_type(&args[0], cache).ptr_type(AddressSpace::Global).into()
                    },
                    UserDefinedType(id) => self.convert_user_defined_type(*id, args, cache),
                    _ => {
                        unreachable!("Type {} requires 0 type args but was applied to {:?}", typ.display(cache), args);
                    }
                }
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

            Tuple(elements) => {
                Tuple(fmap(elements, |element| self.follow_bindings(element, cache)))
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

                // If this is an impl, insert the value with the trait's definition id as a key as
                // well since this is what will be looked up at the call site.
                // NOTE: this is done before inserting the value for the normal id so that
                //       we don't have to always clone the typ for the common case
                cache.definition_infos[id.0].trait_definition.map(|id| {
                    self.definitions.insert((id, typ.clone()), value);
                });

                self.definitions.insert((id, typ), value);
            },
            TypeAnnotation(annotation) => {
                self.bind_irrefutable_pattern(annotation.lhs.as_ref(), value, cache);
            },
            Tuple(tuple) => {
                for (i, element) in tuple.elements.iter().enumerate() {
                    let element_value = self.builder.build_extract_value(value.into_struct_value(), i as u32, "extract").unwrap();
                    self.bind_irrefutable_pattern(element, element_value, cache);
                }
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
    fn codegen_monomorphise<'c>(&mut self, definition: &ast::Definition<'c>, cache: &mut ModuleCache<'c>) -> BasicValueEnum<'g> {
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
        value
    }

    // Is this a (possibly generalized) function type?
    // Used when to differentiate extern C functions/values when compiling Extern declarations.
    fn is_function_type<'c>(&self, typ: &types::Type, cache: &ModuleCache<'c>) -> bool {
        use types::Type::*;
        let typ = self.follow_bindings(typ, cache);
        match typ {
            Function(..) => true,
            ForAll(_, typ) => self.is_function_type(typ.as_ref(), cache),
            _ => false,
        }
    }

    fn codegen_extern<'c>(&mut self, id: DefinitionInfoId, typ: &types::Type, cache: &mut ModuleCache<'c>) -> BasicValueEnum<'g> {
        // extern definitions should only be declared once - never duplicated & monomorphised.
        // For this reason their value is always stored with the Unit type in the definitions map.
        if let Some(value) = self.lookup(id, &UNBOUND_TYPE, cache) {
            self.definitions.insert((id, typ.clone()), value);
            return value;
        }

        let llvm_type = self.convert_type(typ, cache);
        let name = &cache.definition_infos[id.0].name;

        let global = if self.is_function_type(typ, cache) {
            let function_type = llvm_type.into_pointer_type().get_element_type().into_function_type();
            self.module.add_function(name, function_type, None).as_global_value().as_basic_value_enum()
        } else {
            self.auto_derefs.insert(id);
            self.module.add_global(llvm_type, None, name).as_basic_value_enum()
        };

        // Insert the global for both the current type and the unit type
        self.definitions.insert((id, typ.clone()), global);
        self.definitions.insert((id, UNBOUND_TYPE.clone()), global);
        global
    }

    fn codegen_type_constructor<'c>(&mut self, name: &str, tag: &Option<u8>, typ: &types::Type, cache: &mut ModuleCache<'c>) -> BasicValueEnum<'g> {
        use types::Type::*;
        let typ = self.follow_bindings(typ, cache);
        match &typ {
            Function(_, return_type) => {
                // TODO: refactor function creation code (here + ast::Lambda)
                let llvm_type = self.convert_type(&typ, cache).into_pointer_type().get_element_type();

                let function = self.module.add_function(name, llvm_type.into_function_type(), None);
                let function_pointer = function.as_global_value().as_pointer_value().into();

                let caller_block = self.builder.get_insert_block().unwrap();
                let basic_block = self.context.append_basic_block(function, "entry");

                self.current_function = Some(function);
                self.builder.position_at_end(basic_block);

                let mut elements = vec![];
                let mut element_types = vec![];

                if let Some(tag) = tag {
                    let tag_value = self.tag_value(*tag);
                    elements.push(tag_value);
                    element_types.push(tag_value.get_type());
                }

                for parameter in function.get_param_iter() {
                    elements.push(parameter);
                    element_types.push(parameter.get_type());
                }

                let tuple = self.tuple(elements, element_types);
                let value = self.reinterpret_cast(tuple, &return_type, cache);

                self.builder.build_return(Some(&value));
                self.builder.position_at_end(caller_block);

                function_pointer
            },
            UserDefinedType(_) => {
                let value = tag.map_or(self.unit_value(), |tag| self.tag_value(tag));
                self.reinterpret_cast(value, &typ, cache)
            },
            ForAll(_, typ) => {
                self.codegen_type_constructor(name, tag, &typ, cache)
            },
            _ => unreachable!("Type constructor's type is neither a Function or a  UserDefinedType, {}: {}", name, typ.display(cache)),
        }
    }

    fn tag_value(&self, tag: u8) -> BasicValueEnum<'g> {
        self.context.i8_type().const_int(tag as u64, false).as_basic_value_enum()
    }

    fn reinterpret_cast<'c>(&mut self, value: BasicValueEnum<'g>, target_type: &types::Type, cache: &mut ModuleCache<'c>) -> BasicValueEnum<'g> {
        let source_type = value.get_type();
        let alloca = self.builder.build_alloca(source_type, "alloca");
        self.builder.build_store(alloca, value);

        let target_type = self.convert_type(target_type, cache).ptr_type(AddressSpace::Global);
        let cast = self.builder.build_pointer_cast(alloca, target_type, "cast");
        self.builder.build_load(cast, "union_cast")
    }

    fn tuple<'c>(&mut self, elements: Vec<BasicValueEnum<'g>>, element_types: Vec<BasicTypeEnum<'g>>) -> BasicValueEnum<'g> {
        let tuple_type = self.context.struct_type(&element_types, false);
        let mut tuple = tuple_type.const_zero().into();

        for (i, element) in elements.into_iter().enumerate() {
            tuple = self.builder.build_insert_value(tuple, element, i as u32, "insert").unwrap();
        }

        tuple.as_basic_value_enum()
    }

    fn get_field_index<'c>(&self, field_name: &str, typ: &types::Type, cache: &ModuleCache<'c>) -> u32 {
        use types::Type::*;
        match self.follow_bindings(typ, cache) {
            UserDefinedType(id) => {
                cache.type_infos[id.0].find_field(field_name).map(|(i, _)| i).unwrap()
            },
            TypeVariable(id) => {
                match &cache.type_bindings[id.0] {
                    TypeBinding::Bound(_) => unreachable!("Type variable {} is bound but its binding wasn't found by follow_bindings", id.0),
                    TypeBinding::Unbound(..) => unreachable!("Type variable {} is unbound", id.0),
                }
            },
            _ => {
                unreachable!("get_field_index called with a type that clearly doesn't have a {} field: {}", field_name, typ.display(cache));
            }
        }
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
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
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
                let literal = generator.context.const_string(s.as_bytes(), true);
                let global = generator.module.add_global(literal.get_type(), None, "string_literal");
                global.set_initializer(&literal);
                let value = global.as_pointer_value();
                let cstring_type = generator.context.i8_type().ptr_type(AddressSpace::Global);
                let cast = generator.builder.build_pointer_cast(value, cstring_type, "string_cast");

                let string_type = generator.convert_type(self.typ.as_ref().unwrap(), cache).into_struct_type();
                let length = generator.context.i32_type().const_int(s.len() as u64, false);

                Some(string_type.const_named_struct(&[cast.into(), length.into()]).into())
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

        for binding in self.impl_bindings.iter() {
            // TODO: There are never any bindings for member access trait impls. Can we continue
            // this loop for those but otherwise assert the binding is never None?
            cache.impl_bindings[binding.0].map(|impl_id| {
                let trait_impl = &mut cache.impl_infos[impl_id.0].trait_impl;
                let trait_impl = trustme::extend_lifetime(trait_impl);
                trait_impl.codegen(generator, cache);
            });
        }

        let mut value = match generator.lookup(id, self.typ.as_ref().unwrap(), cache) {
            Some(value) => value,
            None => generator.monomorphise(id, self.typ.as_ref().unwrap(), cache).unwrap(),
        };

        if generator.auto_derefs.contains(&id) {
            value = generator.builder.build_load(value.into_pointer_value(), &self.to_string());
        }

        Some(value)
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

        // Bind each parameter node to the nth parameter of `function`
        for (i, parameter) in self.args.iter().enumerate() {
            let value = function.get_nth_param(i as u32).unwrap();
            generator.bind_irrefutable_pattern(parameter, value, cache);
        }

        let return_value = self.body.codegen(generator, cache);

        generator.builder.build_return(return_value.as_ref().map(|x| x as &dyn BasicValue));
        generator.builder.position_at_end(caller_block);

        Some(function_pointer)
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::FunctionCall<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        match self.function.as_ref() {
            ast::Ast::Variable(variable) if variable.definition == Some(BUILTIN_ID) => {
                // Builtin function
                // TODO: improve this control flow so that the fast path of normal function calls
                // doesn't have to check the rare case of a builtin function call.
                builtin::call_builtin(&self.args, generator)
            },
            _ => {
                let function = self.function.codegen(generator, cache).unwrap();
                let args = fmap(&self.args, |arg| arg.codegen(generator, cache).unwrap());
                generator.builder.build_call(function.into_pointer_value(), &args, "").try_as_basic_value().left()
            },
        }
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Definition<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        if !matches!(self.expr.as_ref(), ast::Ast::Lambda(..)) {
            let value = self.expr.codegen(generator, cache).unwrap();
            generator.bind_irrefutable_pattern(self.pattern.as_ref(), value, cache);
            Some(value)
        } else {
            // If the value is a function we can skip it and come back later to only compile it
            // when it is actually used. This saves the optimizer some work since we won't ever
            // have to search for and remove unused functions.
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
            let then_block = generator.builder.get_insert_block().unwrap();
            generator.builder.build_unconditional_branch(end_block);

            generator.builder.position_at_end(else_block);
            let else_value = otherwise.codegen(generator, cache).unwrap();
            let else_block = generator.builder.get_insert_block().unwrap();
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
        unimplemented!("Codegen for match is unimplemented")
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
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        for definition in self.definitions.iter() {
            generator.codegen_monomorphise(definition, cache);
        }
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

impl<'g, 'c> CodeGen<'g, 'c> for ast::MemberAccess<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let lhs = self.lhs.codegen(generator, cache).unwrap();
        let collection = lhs.into_struct_value();

        let index = generator.get_field_index(&self.field, self.lhs.get_type().unwrap(), cache);
        generator.builder.build_extract_value(collection, index, &self.field)
    }
}

impl<'g, 'c> CodeGen<'g, 'c> for ast::Tuple<'c> {
    fn codegen(&self, generator: &mut Generator<'g>, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        let mut elements = vec![];
        let mut element_types = vec![];

        for element in self.elements.iter() {
            let value = element.codegen(generator, cache).unwrap();
            element_types.push(value.get_type());
            elements.push(value);
        }

        Some(generator.tuple(elements, element_types))
    }
}
