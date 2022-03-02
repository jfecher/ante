use std::collections::HashMap;

use crate::cache::{DefinitionInfoId, DefinitionKind, ModuleCache};
use crate::lexer::token::IntegerKind;
use crate::parser::ast::{self, Ast};
use crate::types::typed::Typed;
use crate::types::{Type, FunctionType, TypeBinding, TypeInfoBody, PrimitiveType, TypeConstructor};
use crate::util::{fmap, trustme};
use cranelift::codegen::ir::immediates::Offset32;
use cranelift::codegen::verify_function;
use cranelift::frontend::{FunctionBuilderContext, FunctionBuilder, Variable};
use cranelift::prelude::isa::{TargetFrontendConfig, CallConv};
use cranelift::prelude::{ExtFuncData, Value as CraneliftValue, MemFlags, Signature, InstBuilder, AbiParam, ExternalName, EntityRef, settings};
use cranelift_module::{Linkage, FuncId};
use cranelift::codegen::ir::{types as cranelift_types, Function};

use super::Codegen;

#[allow(unused)]
pub struct Context<'local, 'ast, 'c> {
    cache: &'local mut ModuleCache<'c>,
    pub definitions: HashMap<DefinitionInfoId, Value>,
    builder_context: FunctionBuilderContext,
    pub builder: FunctionBuilder<'local>,
    module: &'local mut dyn cranelift_module::Module,
    module_context: cranelift::codegen::Context,
    unique_id: u32,

    pub current_definition_name: String,
    pub current_function_parameters: Vec<CraneliftValue>,

    alloc_fn: ExtFuncData,
    pub frontend_config: TargetFrontendConfig,

    // TODO: Make this a threadsafe queue so we can compile functions in parallel
    function_queue: Vec<(&'ast ast::Lambda<'c>, Signature, FuncId)>,
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Value {
    Normal(CraneliftValue),
    Function(ExtFuncData),
    Variable(Variable),
}

pub enum FunctionValue {
    Direct(ExtFuncData),
    Indirect(CraneliftValue), // function pointer
}

impl Value {
    /// Convert the value into a CraneliftValue
    pub fn eval<'local, 'ast, 'c>(self, context: &mut Context<'local, 'ast, 'c>) -> CraneliftValue {
        match self {
            Value::Normal(value) => value,
            Value::Variable(variable) => context.builder.use_var(variable),
            Value::Function(data) => {
                let function_ref = context.builder.import_function(data);
                let ptr_type = cranelift_types::I64;
                context.builder.ins().func_addr(ptr_type, function_ref)
            }
        }
    }

    pub fn eval_function<'local, 'ast, 'c>(self) -> FunctionValue {
        match self {
            Value::Function(data) => FunctionValue::Direct(data),
            Value::Normal(value) => FunctionValue::Indirect(value),
            other => unreachable!("Expected a function value, got: {:?}", other),
        }
    }
}

impl<'local, 'ast, 'c> Context<'local, 'ast, 'c> {
    #[allow(unused)]
    fn codegen_main(&mut self, ast: &'ast Ast<'c>) {
        ast.codegen(self);

        while let Some((function, signature, id)) = self.function_queue.pop() {
            self.codegen_function(function, signature, id);
        }
    }

    fn next_unique_id(&mut self) -> u32 {
        self.unique_id += 1;
        self.unique_id
    }

    fn resolve_type(&mut self, typ: &Type) -> Type {
        match typ {
            Type::Primitive(p) => Type::Primitive(*p),
            Type::Function(f) => {
                let f = FunctionType {
                    parameters: fmap(&f.parameters, |parameter| self.resolve_type(parameter)),
                    return_type: Box::new(self.resolve_type(f.return_type.as_ref())),
                    environment: Box::new(self.resolve_type(f.environment.as_ref())),
                    is_varargs: f.is_varargs,
                };
                Type::Function(f)
            },
            Type::TypeVariable(id) => match &self.cache.type_bindings[id.0] {
                TypeBinding::Bound(t) => {
                    let t = t.clone();
                    self.resolve_type(&t)
                }
                // Default to unit
                TypeBinding::Unbound(_, _) => Type::Primitive(PrimitiveType::UnitType),
            },
            Type::UserDefinedType(id) => Type::UserDefinedType(*id),
            Type::TypeApplication(c, args) => Type::TypeApplication(Box::new(self.resolve_type(c)), fmap(args, |arg| self.resolve_type(arg))),
            Type::Ref(id) => Type::Ref(*id),
            Type::ForAll(_vars, typ) => self.resolve_type(typ.as_ref()),
        }
    }

    fn convert_type(&mut self, _typ: &Type) -> cranelift_types::Type {
        cranelift_types::R64 // TODO - is this right if we want to box everything?
    }

    pub fn convert_signature(&mut self, typ: &Type) -> Signature {
        let typ = self.resolve_type(typ);
        let mut sig = Signature::new(CallConv::Fast);

        match typ {
            Type::Function(f) => {
                for parameter in &f.parameters {
                    let cranelift_type = self.convert_type(parameter);
                    sig.params.push(AbiParam::new(cranelift_type));
                }

                let cranelift_type = self.convert_type(f.return_type.as_ref());
                sig.returns.push(AbiParam::new(cranelift_type));
                sig
            },
            _ => unreachable!("called convert_signature with type {}", typ.display(self.cache)),
        }
    }

    pub fn integer_kind_type(&mut self, kind: &IntegerKind) -> cranelift_types::Type {
        match kind {
            IntegerKind::Unknown => unreachable!("Unknown IntegerKind encountered during codegen"),
            IntegerKind::Inferred(id) => {
                self.convert_type(&Type::TypeVariable(*id))
            },
            IntegerKind::I8 | IntegerKind::U8 => cranelift_types::I8,
            IntegerKind::I16 | IntegerKind::U16 => cranelift_types::I16,
            IntegerKind::I32 | IntegerKind::U32 => cranelift_types::I32,
            IntegerKind::I64 | IntegerKind::Isz | IntegerKind::U64 | IntegerKind::Usz => cranelift_types::I64,
        }
    }

    #[allow(unused)]
    pub fn codegen_definition(&mut self, id: DefinitionInfoId) -> Value {
        let definition = &mut self.cache.definition_infos[id.0];
        let definition = trustme::extend_lifetime(definition);

        let value = match &definition.definition {
            Some(DefinitionKind::Definition(definition)) => definition.codegen(self),
            Some(DefinitionKind::Extern(annotation)) => self.codegen_extern(*annotation),
            Some(DefinitionKind::TypeConstructor { name, tag }) => todo!(),
            Some(DefinitionKind::TraitDefinition(definition)) => unreachable!("No trait impl for trait {}", definition),
            Some(DefinitionKind::Parameter) => unreachable!("Parameter definitions should already be codegen'd"),
            Some(DefinitionKind::MatchPattern) => unreachable!("Pattern definitions should already be codegen'd"),
            None => unreachable!("Variable {} has no definition", id.0),
        };

        self.definitions.insert(id, value.clone());
        value
    }

    pub fn create_return(&mut self, value: Value) {
        // TODO: Check for pre-existing branch instruction
        let value = value.eval(self);
        self.builder.ins().return_(&[value]);
    }

    fn codegen_function(&mut self, function: &'ast ast::Lambda<'c>, signature: Signature, function_id: FuncId) {
        let mut func = Function::with_name_signature(ExternalName::user(0, 0), signature);

        let mut builder = FunctionBuilder::new(&mut func, &mut self.builder_context);
        let entry = builder.create_block();

        // TODO Parameter binding
        for _parameter in &function.args {
            let x = Variable::new(0);
            builder.declare_var(x, cranelift_types::I32);
        }

        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let body = function.body.codegen(self);
        self.create_return(body);
        self.builder.finalize();

        self
            .module
            .define_function(function_id, &mut self.module_context)
            .unwrap();

        let flags = settings::Flags::new(settings::builder());
        let res = verify_function(&func, &flags);
        println!("{}", func.display());
        if let Err(errors) = res {
            panic!("{}", errors);
        }
    }

    pub fn add_function_to_queue(&mut self, function: &'ast ast::Lambda<'c>, name: &'ast str) -> Value {
        let signature = self.convert_signature(function.get_type().unwrap());
        let function_id = self.module.declare_function(name, Linkage::Export, &signature).unwrap();
        self.function_queue.push((function, signature.clone(), function_id));

        let signature = self.builder.import_signature(signature);

        Value::Function(ExtFuncData {
            name: ExternalName::user(0, self.next_unique_id()),
            signature,
            colocated: true,
        })
    }

    pub fn unit_value(&mut self) -> Value {
        Value::Normal(self.builder.ins().bconst(cranelift_types::B64, false))
    }

    /// Boxes a value at runtime.
    ///
    /// This will be called very often as the cranelift backend will perform
    /// boxing instead of monomorphisation to handle generics.
    #[allow(unused)]
    fn alloc(&mut self, value: Value) -> CraneliftValue {
        let function_ref = self.builder.import_function(self.alloc_fn.clone());
        let arg = value.eval(self);
        let call = self.builder.ins().call(function_ref, &[arg]);
        let results = self.builder.inst_results(call);
        assert_eq!(results.len(), 1);
        results[0]
    }

    /// Binds the given pattern to the given value, recursively filling in
    /// any definitions in the pattern to the corresponding value.
    ///
    /// Like all values in this IR, `value` is expected to be boxed, so
    /// we must unbox the value and cast it at each step as we unwrap it.
    pub fn bind_pattern(&mut self, pattern: &Ast, value: CraneliftValue) {
        match pattern {
            Ast::Literal(_) => (), // Nothing to do
            Ast::Variable(variable) => {
                let id = variable.definition.unwrap();

                // Unlike monomorphisation in the llvm pass, we should never expect to
                // invalidate previous work by binding the same definition to a new value.
                if let Some(old_value) = self.definitions.insert(id, Value::Normal(value)) {
                    unreachable!("bind_pattern tried to bind to {}, but it was already bound to {:?}", pattern, old_value);
                }
            },
            // This should be an irrefutable pattern (struct/tuple), arbitrary patterns
            // are handled only when compiling decision trees.
            Ast::FunctionCall(call) => {
                let offsets = self.field_offsets(call.typ.as_ref().unwrap());
                assert_eq!(offsets.len(), call.args.len());

                for (arg_pattern, arg_offset) in call.args.iter().zip(offsets) {
                    let typ = cranelift_types::R64;
                    let flags = MemFlags::new();
                    let arg_value = self.builder.ins().load(typ, flags, value, arg_offset);
                    self.bind_pattern(arg_pattern, arg_value);
                }
            },
            Ast::TypeAnnotation(annotation) => self.bind_pattern(&annotation.lhs, value),
            _ => unreachable!("Invalid pattern given to bind_pattern: {}", pattern),
        }
    }

    /// Returns a Vec of byte offsets of each field of this type.
    fn field_offsets(&self, struct_type: &Type) -> Vec<Offset32> {
        match struct_type {
            Type::Primitive(_) => unreachable!(),
            Type::Function(_) => unreachable!(),
            Type::TypeVariable(id) => {
                match &self.cache.type_bindings[id.0] {
                    TypeBinding::Bound(binding) => self.field_offsets(binding),
                    TypeBinding::Unbound(..) => unreachable!(),
                }
            },
            Type::Ref(_) => unreachable!(),
            Type::ForAll(_, _) => unreachable!(),
            Type::UserDefinedType(id) => {
                let type_info = &self.cache.type_infos[id.0];
                match &type_info.body {
                    TypeInfoBody::Union(_) => unreachable!(),
                    TypeInfoBody::Unknown => unreachable!(),
                    TypeInfoBody::Alias(alias) => self.field_offsets(alias),
                    TypeInfoBody::Struct(fields) => {
                        let mut offset = 0;
                        fmap(fields, |field| {
                            let field_offset = offset;
                            offset += self.size_of_unboxed_type(&field.field_type);
                            Offset32::new(field_offset)
                        })
                    },
                }
            },

            // This is much simpler than the equivalent monomorphised version
            // since we do not have to keep track of type arguments thanks to
            // uniform representation.
            Type::TypeApplication(base_type, _) => self.field_offsets(base_type),
        }
    }

    /// Returns the size of the given type in bytes.
    ///
    /// The type is considered to be shallowly-unboxed.
    /// That is, the outermost type will be unboxed but any
    /// fields contained within will still be boxed.
    pub fn size_of_unboxed_type(&self, field_type: &Type) -> i32 {
        match field_type {
            Type::Primitive(primitive) => self.size_of_primitive(primitive),
            Type::Function(_) => self.pointer_size(),
            Type::TypeVariable(id) => {
                match &self.cache.type_bindings[id.0] {
                    TypeBinding::Bound(binding) => self.size_of_unboxed_type(binding),
                    // Default to i32. TODO: Re-evaluate this. We could default to unit instead.
                    TypeBinding::Unbound(..) => std::mem::size_of::<i32>() as i32,
                }
            },
            Type::UserDefinedType(id) => {
                let type_info = &self.cache.type_infos[id.0];
                match &type_info.body {
                    TypeInfoBody::Unknown => unreachable!(),
                    TypeInfoBody::Alias(alias) => self.size_of_unboxed_type(alias),
                    // All fields are boxed
                    TypeInfoBody::Struct(fields) => fields.len() as i32 * self.pointer_size(),
                    TypeInfoBody::Union(variants) => self.size_of_union(variants),
                }
            },
            Type::TypeApplication(base_type, _) => self.size_of_unboxed_type(base_type),
            Type::Ref(_) => self.pointer_size(),
            Type::ForAll(_, typ) => self.size_of_unboxed_type(typ),
        }
    }

    fn size_of_primitive(&self, primitive: &PrimitiveType) -> i32 {
        match primitive {
            PrimitiveType::IntegerType(kind) => {
                match kind {
                    IntegerKind::Unknown => unreachable!(),
                    IntegerKind::Inferred(id) => {
                        match &self.cache.type_bindings[id.0] {
                            TypeBinding::Bound(binding) => self.size_of_unboxed_type(binding),
                            // Default to i32
                            TypeBinding::Unbound(..) => std::mem::size_of::<i32>() as i32,
                        }
                    },
                    IntegerKind::I8
                    | IntegerKind::U8 => 1,
                    IntegerKind::I16
                    | IntegerKind::U16 => 2,
                    IntegerKind::I32
                    | IntegerKind::U32 => 4,
                    IntegerKind::I64
                    | IntegerKind::U64 => 8,
                    IntegerKind::Isz
                    | IntegerKind::Usz => self.pointer_size(),
                }
            },
            PrimitiveType::FloatType => 8,
            PrimitiveType::CharType => 1,
            PrimitiveType::BooleanType => 1,
            PrimitiveType::UnitType => 1,
            PrimitiveType::Ptr => self.pointer_size(),
        }
    }

    /// Returns the size of a sum type in bytes.
    /// This should match the size of its largest variant + an extra byte for the tag
    fn size_of_union(&self, variants: &[TypeConstructor]) -> i32 {
        variants.iter().map(|variant| {
            variant.args.len() as i32 * self.pointer_size() + 1
        }).max().unwrap_or(1)
    }

    /// Returns the size of a pointer in bytes.
    /// TODO: Adjust based on target platform
    fn pointer_size(&self) -> i32 {
        std::mem::size_of::<*const u8>() as i32
    }

    fn codegen_extern(&self, annotation: &ast::TypeAnnotation) -> Value {
        let _typ = self.convert_extern_type(annotation.typ.as_ref().unwrap());
        todo!()
    }

    /// Convert the type of an extern value to a cranelift type.
    ///
    /// Note that this is currently separate from convert_type and convert_signature
    /// because we need to error if any externs are declared that use C structs or
    /// other types that would be incompatible with our "box everything" approach.
    fn convert_extern_type(&self, _typ: &Type) -> cranelift_types::Type {
        todo!()
        // match typ {
        //     Type::Primitive(p) => cranelift_types::R64,
        //     Type::Function(f) => {
        //         let f = FunctionType {
        //             parameters: fmap(&f.parameters, |parameter| self.resolve_type(parameter)),
        //             return_type: Box::new(self.resolve_type(f.return_type.as_ref())),
        //             environment: Box::new(self.resolve_type(f.environment.as_ref())),
        //             is_varargs: f.is_varargs,
        //         };
        //         Type::Function(f)
        //     },
        //     Type::TypeVariable(id) => match &self.cache.type_bindings[id.0] {
        //         TypeBinding::Bound(t) => self.convert_extern_type(t),
        //         TypeBinding::Unbound(_, _) => todo!(),
        //     },
        //     Type::UserDefinedType(id) => Type::UserDefinedType(*id),
        //     Type::TypeApplication(c, args) => Type::TypeApplication(Box::new(self.resolve_type(c)), fmap(args, |arg| self.resolve_type(arg))),
        //     Type::Ref(_) => todo!(),
        //     Type::ForAll(_vars, typ) => self.convert_extern_type(typ.as_ref()),
        // }
    }
}

