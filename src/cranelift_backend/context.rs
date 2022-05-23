use std::collections::HashMap;
use std::path::Path;

use crate::args::Args;
use crate::hir::{self, Ast, DefinitionId, PrimitiveType, Type};
use crate::util::fmap;

use cranelift::codegen::ir::{types as cranelift_types, FuncRef, Function, StackSlot};
use cranelift::codegen::verify_function;
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift::prelude::isa::CallConv;
use cranelift::prelude::{
    settings, AbiParam, Block, ExtFuncData, ExternalName, InstBuilder, MemFlags, Signature, StackSlotData,
    StackSlotKind, Value as CraneliftValue,
};
use cranelift_module::{DataContext, DataId, FuncId, Linkage, Module};

use super::module::DynModule;
use super::CodeGen;

// TODO: Make this a threadsafe queue so we can compile functions in parallel
type FunctionQueue<'ast> = Vec<(&'ast hir::Lambda, Signature, FuncId)>;

pub struct Context<'ast> {
    pub definitions: HashMap<DefinitionId, Value>,
    module: DynModule,
    data_context: DataContext,
    function_queue: FunctionQueue<'ast>,

    pub current_function_name: Option<DefinitionId>,
    next_func_id: u32,
}

#[derive(Debug)]
pub enum FunctionValue {
    Direct(FuncData),
    Indirect(CraneliftValue), // function pointer
}

/// An almost clone of ExtFuncData which caches the actual function Signature instead
/// of the SigRef value which will be different for each function this is used in.
#[derive(Debug, Clone)]
pub struct FuncData {
    name: ExternalName,
    signature: Signature,
    colocated: bool,
}

impl FuncData {
    pub fn import(self, builder: &mut FunctionBuilder) -> FuncRef {
        let data = ExtFuncData {
            name: self.name,
            colocated: self.colocated,
            signature: builder.import_signature(self.signature),
        };
        builder.import_function(data)
    }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub enum Value {
    Normal(CraneliftValue),
    Function(FuncData),
    Global(DataId),
    Tuple(Vec<Value>),

    /// A loadable is a pointer value that should be loaded before it is used.
    /// Mutable definitions usually translate to these.
    Loadable(CraneliftValue, cranelift_types::Type),

    /// Lazily inserting unit values helps prevent cluttering the IR with too many
    /// unit literals.
    Unit,
}

impl Value {
    pub fn unit() -> Value {
        Value::Unit
    }

    /// Convert the value into a CraneliftValue
    pub fn eval_all(self, context: &mut Context, builder: &mut FunctionBuilder) -> Vec<CraneliftValue> {
        match self {
            Value::Tuple(values) => {
                let mut result = Vec::with_capacity(values.len());
                for value in values {
                    result.append(&mut value.eval_all(context, builder));
                }
                result
            },
            other => vec![other.eval_single(context, builder)],
        }
    }

    /// Convert the value into a single CraneliftValue, panics if this is a tuple.
    pub fn eval_single(self, context: &mut Context, builder: &mut FunctionBuilder) -> CraneliftValue {
        match self {
            Value::Normal(value) => value,
            Value::Loadable(ptr, typ) => builder.ins().load(typ, MemFlags::new(), ptr, 0),
            Value::Unit => {
                let unit_type = cranelift_types::B1;
                builder.ins().bconst(unit_type, false)
            },
            Value::Global(data_id) => {
                let _global = context.module.declare_data_in_func(data_id, builder.func);
                // builder.ins().global_value(BOXED_TYPE, global)
                todo!("globals")
            },
            Value::Function(function) => {
                let function = function.import(builder);
                builder.ins().func_addr(pointer_type(), function)
            },
            Value::Tuple(elems) => panic!("Value::Tuple found in eval_single: {:?}", elems),
        }
    }
}

enum FunctionOrGlobal {
    Function(Signature),
    Global,
}

impl<'local> Context<'local> {
    fn new(output_path: &Path, use_jit: bool) -> (Self, FunctionBuilderContext) {
        let builder_context = FunctionBuilderContext::new();
        let module = DynModule::new(output_path.to_string_lossy().into_owned(), use_jit);

        (
            Context {
                definitions: HashMap::new(),
                module,
                next_func_id: 0,
                data_context: DataContext::new(),
                function_queue: vec![],
                current_function_name: None,
            },
            builder_context,
        )
    }

    pub fn codegen_all(path: &Path, hir: &'local Ast, args: &Args) {
        let output_path = path.with_extension("");
        let (mut context, mut builder_context) = Context::new(&output_path, !args.build);
        let mut module_context = context.module.make_context();

        let main = context.codegen_main(hir, &mut builder_context, &mut module_context, args);

        // Then codegen any functions used by main and so forth
        while let Some((function, signature, id)) = context.function_queue.pop() {
            context.codegen_function_body(function, &mut builder_context, &mut module_context, signature, id, args);
        }

        context.module.finish(main, &output_path);
    }

    /// Codegens an entire function. Cranelift enforces we must finish compiling the
    /// current function before we move onto the next so we can assume there are no
    /// other partially compiled functions.
    ///
    /// Should this be renamed since it delegates to codegen_function_inner to
    /// compile the actual body of the function?
    fn codegen_function_body(
        &mut self, function: &'local hir::Lambda, context: &mut FunctionBuilderContext,
        module_context: &mut cranelift::codegen::Context, signature: Signature, function_id: FuncId, args: &Args,
    ) {
        module_context.func = Function::with_name_signature(ExternalName::user(0, function_id.as_u32()), signature);
        let mut builder = FunctionBuilder::new(&mut module_context.func, context);

        let entry = builder.create_block();
        builder.switch_to_block(entry);
        builder.seal_block(entry);
        builder.append_block_params_for_function_params(entry);

        let body = self.codegen_lambda(function, &mut builder);
        self.create_return(body, &mut builder);

        builder.finalize();

        if args.show_ir {
            let name = match module_context.func.name {
                ExternalName::User { index, .. } => index,
                ExternalName::TestCase { .. } => unreachable!(),
                ExternalName::LibCall(_) => unreachable!(),
            };

            let func = &self.module.declarations().get_function_decl(FuncId::from_u32(name));
            println!("{} =\n{}", func.name, module_context.func.display());
        }

        let flags = settings::Flags::new(settings::builder());
        if let Err(errors) = verify_function(&module_context.func, &flags) {
            panic!("{}", errors);
        }

        self.module.define_function(function_id, module_context).unwrap();
        module_context.clear();
    }

    pub fn next_unique_id(&mut self) -> u32 {
        self.next_func_id += 1;
        self.next_func_id
    }

    fn codegen_main(
        &mut self, ast: &'local Ast, builder_context: &mut FunctionBuilderContext,
        module_context: &mut cranelift::codegen::Context, args: &Args,
    ) -> FuncId {
        let func = &mut module_context.func;
        func.signature.returns.push(AbiParam::new(cranelift_types::I32));

        let main_id = self.module.declare_function("main", Linkage::Export, &func.signature).unwrap();

        let mut builder = FunctionBuilder::new(func, builder_context);
        let entry = builder.create_block();

        builder.switch_to_block(entry);
        builder.seal_block(entry);

        ast.codegen(self, &mut builder);

        let zero = builder.ins().iconst(cranelift_types::I32, 0);
        self.create_return(Value::Normal(zero), &mut builder);

        builder.finalize();

        let flags = settings::Flags::new(settings::builder());
        let func = &module_context.func;
        let res = verify_function(func, &flags);

        if args.show_ir {
            println!("main =\n{}", func.display());
        }

        if let Err(errors) = res {
            panic!("{}", errors);
        }

        self.module.define_function(main_id, module_context).unwrap();

        module_context.clear();
        main_id
    }

    fn codegen_lambda(&mut self, lambda: &'local hir::Lambda, builder: &mut FunctionBuilder) -> Value {
        let block = builder.current_block().unwrap();
        let mut i = 0;
        let all_parameters = builder.block_params(block);

        // all_parameters flattens structs into 1 parameter per field, so we must
        // un-flatten them here to bind names to multiple parameters
        for ((parameter, _), param_type) in lambda.args.iter().zip(&lambda.typ.parameters) {
            let arg = self.fmap_type(param_type, &mut |_, _| {
                i += 1;
                all_parameters[i - 1]
            });

            self.definitions.insert(parameter.definition_id, arg);
        }

        lambda.body.codegen(self, builder)
    }

    /// Where `codegen_function_body` creates a new function in the IR and codegens
    /// its body, this function essentially codegens the reference to a function at
    /// the callsite.
    pub fn codegen_function_use(&mut self, ast: &'local hir::Ast, builder: &mut FunctionBuilder) -> FunctionValue {
        let value = ast.codegen(self, builder);

        // If we have a direct call we can return early. Otherwise we need to check the expected
        // type to see if we expect a function pointer or a boxed closure value.
        match value {
            Value::Function(data) => FunctionValue::Direct(data),
            Value::Normal(value) => FunctionValue::Indirect(value),
            Value::Loadable(ptr, typ) => {
                let value = builder.ins().load(typ, MemFlags::new(), ptr, 0);
                FunctionValue::Indirect(value)
            },
            Value::Global(_) => {
                todo!("Is this case reachable? Can we have function-value Value::Globals that are not Value::Function?")
            },
            Value::Tuple(_) => unreachable!(),
            Value::Unit => unreachable!(),
        }
    }

    /// For each cranelift type in this hir::Type, do f.
    /// This will apply f to each element in the case of tuples.
    /// Otherwise, it will only apply it once.
    pub fn for_each_type_in<F>(&mut self, typ: &Type, mut f: F)
    where
        F: FnMut(&mut Self, cranelift_types::Type),
    {
        self.for_each_helper(typ, &mut f)
    }

    // Need to pass around the function by mutable ref to use it in the for loop
    fn for_each_helper<F: FnMut(&mut Self, cranelift_types::Type)>(&mut self, typ: &Type, f: &mut F) {
        match typ {
            Type::Primitive(p) => f(self, convert_primitive_type(p)),
            Type::Function(_) => f(self, function_type()),
            Type::Tuple(elems) => {
                for elem in elems {
                    self.for_each_helper(elem, f);
                }
            },
        }
    }

    pub fn array_to_value(&mut self, values: &[CraneliftValue], expected_type: &Type) -> Value {
        let mut i = 0;
        self.fmap_type(expected_type, &mut |_, _| {
            i += 1;
            values[i - 1]
        })
    }

    pub fn fmap_type(
        &mut self, typ: &Type, f: &mut impl FnMut(&mut Self, cranelift_types::Type) -> CraneliftValue,
    ) -> Value {
        use Value::*;
        match typ {
            Type::Primitive(p) => Normal(f(self, convert_primitive_type(p))),
            Type::Function(_) => Normal(f(self, function_type())),
            Type::Tuple(elems) => Value::Tuple(fmap(elems, |elem| self.fmap_type(elem, f))),
        }
    }

    pub fn new_block_with_arg(&mut self, typ: &Type, builder: &mut FunctionBuilder) -> Block {
        let block = builder.create_block();
        self.for_each_type_in(typ, |_, typ| {
            builder.append_block_param(block, typ);
        });
        block
    }

    pub fn convert_signature(&mut self, f: &hir::FunctionType) -> Signature {
        let mut sig = Signature::new(CallConv::Fast);

        for parameter in &f.parameters {
            self.for_each_type_in(parameter, |_, typ| {
                sig.params.push(AbiParam::new(typ));
            });
        }

        self.for_each_type_in(&f.return_type, |_, typ| {
            sig.returns.push(AbiParam::new(typ));
        });

        sig
    }

    pub fn create_return(&mut self, value: Value, builder: &mut FunctionBuilder) {
        let values = value.eval_all(self, builder);
        builder.ins().return_(&values);
    }

    pub fn add_function_to_queue(&mut self, function: &'local hir::Lambda, name: &str) -> Value {
        let signature = self.convert_signature(&function.typ);

        let name = format!("lambda{}", name);
        let function_id = self.module.declare_function(&name, Linkage::Export, &signature).unwrap();

        self.function_queue.push((function, signature.clone(), function_id));

        Value::Function(FuncData {
            name: ExternalName::user(0, function_id.as_u32()),
            signature,
            // Using 'true' here gives an unimplemented error on aarch64
            colocated: false,
        })
    }

    pub fn codegen_extern(&mut self, name: &str, typ: &Type) -> Value {
        match self.convert_extern_signature(typ) {
            FunctionOrGlobal::Global => {
                let data_id = self.module.declare_data(&name, Linkage::Import, true, false).unwrap();

                self.data_context.clear();
                Value::Global(data_id)
            },
            FunctionOrGlobal::Function(signature) => {
                // Don't mangle extern names
                let id = self.module.declare_function(&name, Linkage::Import, &signature).unwrap();

                Value::Function(FuncData { name: ExternalName::user(0, id.as_u32()), signature, colocated: false })
            },
        }
    }

    fn extern_call_conv() -> CallConv {
        // TODO: Change based on target os rather than just host os
        if cfg!(windows) {
            CallConv::WindowsFastcall
        } else {
            CallConv::SystemV
        }
    }

    fn convert_extern_signature(&mut self, typ: &Type) -> FunctionOrGlobal {
        match typ {
            Type::Function(f) => {
                let mut signature = self.convert_signature(f);
                signature.call_conv = Self::extern_call_conv();
                FunctionOrGlobal::Function(signature)
            },
            _ => FunctionOrGlobal::Global,
        }
    }

    /// Declare a string global value and get a reference to it
    pub fn c_string_value(&mut self, value: &str, builder: &mut FunctionBuilder) -> CraneliftValue {
        let mut value = value.to_owned();
        assert!(!value.ends_with('\0'));
        value.push('\0');

        let value = value.into_bytes().into_boxed_slice();
        self.data_context.define(value);

        let name = format!("string{}", self.next_unique_id());
        let data_id = self.module.declare_data(&name, Linkage::Local, true, false).unwrap();

        self.module.define_data(data_id, &self.data_context).unwrap();
        self.data_context.clear();

        let global = self.module.declare_data_in_func(data_id, builder.func);
        builder.ins().symbol_value(pointer_type(), global)
    }

    pub fn reinterpret_cast(&mut self, value: Value, target_type: &Type, builder: &mut FunctionBuilder) -> Value {
        let size = size_of(target_type);
        let data = StackSlotData::new(StackSlotKind::ExplicitSlot, size);
        let slot = builder.create_stack_slot(data);

        self.store_stack_value(value, slot, &mut 0, builder);
        self.load_stack_value(target_type, slot, &mut 0, builder)
    }

    fn store_stack_value(&mut self, value: Value, slot: StackSlot, offset: &mut u32, builder: &mut FunctionBuilder) {
        match value {
            Value::Tuple(elems) => {
                for elem in elems {
                    self.store_stack_value(elem, slot, offset, builder);
                }
            },
            value => {
                let value = value.eval_single(self, builder);
                builder.ins().stack_store(value, slot, *offset as i32);
                *offset += builder.func.dfg.value_type(value).bytes();
            },
        }
    }

    pub fn store_value(&mut self, addr: CraneliftValue, value: Value, offset: &mut u32, builder: &mut FunctionBuilder) {
        match value {
            Value::Tuple(elems) => {
                for elem in elems {
                    self.store_value(addr, elem, offset, builder);
                }
            },
            value => {
                let value = value.eval_single(self, builder);
                builder.ins().store(MemFlags::new(), value, addr, *offset as i32);
                *offset += builder.func.dfg.value_type(value).bytes();
            },
        }
    }

    fn load_stack_value(
        &mut self, target_type: &Type, slot: StackSlot, offset: &mut u32, builder: &mut FunctionBuilder,
    ) -> Value {
        let mut load_single = |typ| {
            let value = builder.ins().stack_load(typ, slot, *offset as i32);
            *offset += typ.bytes();
            Value::Normal(value)
        };

        match target_type {
            Type::Tuple(elems) => Value::Tuple(fmap(elems, |elem| self.load_stack_value(elem, slot, offset, builder))),
            Type::Primitive(p) => load_single(convert_primitive_type(p)),
            Type::Function(_) => load_single(function_type()),
        }
    }

    pub fn load_value(
        &mut self, target_type: &Type, addr: CraneliftValue, offset: &mut i32, builder: &mut FunctionBuilder,
    ) -> Value {
        let mut load_single = |typ| {
            let value = builder.ins().load(typ, MemFlags::new(), addr, *offset);
            *offset += typ.bytes() as i32;
            Value::Normal(value)
        };

        match target_type {
            Type::Tuple(elems) => Value::Tuple(fmap(elems, |elem| self.load_value(elem, addr, offset, builder))),
            Type::Primitive(p) => load_single(convert_primitive_type(p)),
            Type::Function(_) => load_single(function_type()),
        }
    }
}

/// Returns the size of a pointer in bytes.
/// TODO: Adjust based on target platform
pub fn pointer_size() -> i32 {
    std::mem::size_of::<*const u8>() as i32
}

pub fn pointer_type() -> cranelift_types::Type {
    let size = pointer_size();
    if size == 8 {
        // TODO: Using R64 here breaks global values which seem to always be of type I64?
        cranelift_types::I64
    } else if size == 4 {
        cranelift_types::I32
    } else {
        panic!("Unsupported pointer size: {} bytes", size)
    }
}

pub fn int_pointer_type() -> cranelift_types::Type {
    let size = pointer_size();
    if size == 8 {
        cranelift_types::I64
    } else if size == 4 {
        cranelift_types::I32
    } else {
        panic!("Unsupported pointer size: {} bytes", size)
    }
}

/// Returns the size of the given type in bytes
pub fn size_of(typ: &Type) -> u32 {
    match typ {
        Type::Primitive(p) => convert_primitive_type(p).bytes(),
        Type::Function(_) => function_type().bytes(),
        Type::Tuple(types) => types.iter().map(size_of).sum(),
    }
}

fn function_type() -> cranelift_types::Type {
    pointer_type()
}

fn convert_primitive_type(typ: &PrimitiveType) -> cranelift_types::Type {
    match typ {
        PrimitiveType::Integer(kind) => convert_integer_kind(*kind),
        PrimitiveType::Float => cranelift_types::F64,
        PrimitiveType::Char => cranelift_types::I8,
        PrimitiveType::Boolean => cranelift_types::B1,
        PrimitiveType::Unit => cranelift_types::B1,
        PrimitiveType::Pointer => pointer_type(),
    }
}

pub fn convert_integer_kind(kind: hir::IntegerKind) -> cranelift_types::Type {
    use hir::IntegerKind;
    match kind {
        IntegerKind::I8 => cranelift_types::I8,
        IntegerKind::I16 => cranelift_types::I16,
        IntegerKind::I32 => cranelift_types::I32,
        IntegerKind::I64 => cranelift_types::I64,
        IntegerKind::Isz => int_pointer_type(),
        IntegerKind::U8 => cranelift_types::I8,
        IntegerKind::U16 => cranelift_types::I16,
        IntegerKind::U32 => cranelift_types::I32,
        IntegerKind::U64 => cranelift_types::I64,
        IntegerKind::Usz => int_pointer_type(),
    }
}
