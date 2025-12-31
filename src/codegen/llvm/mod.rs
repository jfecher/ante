use std::sync::Arc;

use inkwell::{
    AddressSpace,
    basic_block::BasicBlock,
    builder::Builder,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple},
    types::{BasicType, BasicTypeEnum, IntType},
    values::{AnyValue, BasicValueEnum, FunctionValue},
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    incremental::{CodegenLlvm, DbHandle},
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    mir::{self, BlockId, FloatConstant, FunctionId, PrimitiveType, TerminatorInstruction},
    vecmap::VecMap,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodegenLlvmResult {
    pub module_string: Option<Arc<String>>,
}

#[allow(unused)]
pub fn initialize_native_target() {
    let config = InitializationConfig::default();
    Target::initialize_native(&config).unwrap();
}

pub fn codegen_llvm_impl(context: &CodegenLlvm, compiler: &DbHandle) -> CodegenLlvmResult {
    let module_string = mir::builder::build_initial_mir(compiler, context.0).map(|mir| {
        let name = &mir.functions.iter().next().unwrap().1.name;
        let llvm = inkwell::context::Context::create();
        let mut module = ModuleContext::new(&llvm, &mir, name);

        for (id, function) in &mir.functions {
            module.codegen_function(function, *id);
        }

        Arc::new(module.module.to_string())
    });

    // Compiling each function separately and linking them together later is probably slower than
    // doing them all together to begin with but oh well. It is easier to start more incremental
    // and be less incremental later than the reverse.
    CodegenLlvmResult { module_string }
}

struct ModuleContext<'ctx> {
    llvm: &'ctx inkwell::context::Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,

    mir: &'ctx mir::Mir,

    target: TargetTriple,

    blocks: VecMap<BlockId, BasicBlock<'ctx>>,

    current_function: Option<FunctionId>,
    current_function_value: Option<FunctionValue<'ctx>>,

    values: FxHashMap<mir::Value, BasicValueEnum<'ctx>>,

    /// Block arguments are added here to later insert them as PHI values.
    ///
    /// Maps merge_block to a vec of each incoming block along with the arguments it branches with.
    incoming: FxHashMap<BlockId, Vec<(BasicBlock<'ctx>, BasicValueEnum<'ctx>)>>,
}

impl<'ctx> ModuleContext<'ctx> {
    fn new(llvm: &'ctx inkwell::context::Context, mir: &'ctx mir::Mir, name: &str) -> Self {
        let module = llvm.create_module(name);
        let target = TargetMachine::get_default_triple();
        module.set_triple(&target);
        Self {
            llvm,
            module,
            target,
            mir,
            current_function: None,
            current_function_value: None,
            values: Default::default(),
            builder: llvm.create_builder(),
            blocks: Default::default(),
            incoming: Default::default(),
        }
    }

    fn codegen_function(&mut self, function: &mir::Function, id: mir::FunctionId) {
        println!("Working on function {}", function.name);

        let function_value = match self.values.get(&mir::Value::Function(id)) {
            Some(existing) => existing.as_any_value_enum().into_function_value(),
            None => {
                let parameter_types =
                    mapvec(&function.blocks[BlockId::ENTRY_BLOCK].parameter_types, |typ| self.convert_type(typ).into());

                let return_type = self.convert_type(&self.find_return_type(function));
                let function_type = return_type.fn_type(&parameter_types, false);
                let function_value = self.module.add_function(&function.name, function_type, None);
                self.values
                    .insert(mir::Value::Function(id), function_value.as_global_value().as_pointer_value().into());
                function_value
            },
        };

        self.current_function = Some(id);
        self.current_function_value = Some(function_value);
        self.create_blocks(function, function_value);

        for i in 0..function.blocks[BlockId::ENTRY_BLOCK].parameter_types.len() as u32 {
            let value = mir::Value::Parameter(BlockId::ENTRY_BLOCK, i);
            let llvm_value = function_value.get_nth_param(i).unwrap();
            self.values.insert(value, llvm_value);
        }

        for block in function.topological_sort() {
            self.codegen_block(block, function);
        }

        self.values.clear();
        self.blocks.clear();
    }

    /// Create an empty block for each block in the given function
    fn create_blocks(&mut self, function: &mir::Function, function_value: FunctionValue<'ctx>) {
        for (i, (block_id, _)) in function.blocks.iter().enumerate() {
            let block = self.llvm.append_basic_block(function_value, "");
            self.blocks.push_existing(block_id, block);
        }
    }

    fn codegen_block(&mut self, block_id: BlockId, function: &mir::Function) {
        let llvm_block = self.blocks[block_id];
        self.builder.position_at_end(llvm_block);
        let block = &function.blocks[block_id];

        // Translate the block parameters into phi instructions
        if block_id != BlockId::ENTRY_BLOCK {
            for (parameter, parameter_type) in block.parameters(block_id) {
                let parameter_type = self.convert_type(&parameter_type);
                let phi = self.builder.build_phi(parameter_type, "").unwrap();

                let incoming = self
                    .incoming
                    .remove(&block_id)
                    .unwrap_or_else(|| panic!("llvm codegen: No incoming for block {block_id}"));

                for (block, block_args) in incoming {
                    phi.add_incoming(&[(&block_args, block)]);
                }
                self.values.insert(parameter, phi.as_basic_value());
            }
        }

        for instruction_id in block.instructions.iter().copied() {
            self.codegen_instruction(function, instruction_id);
        }

        let terminator = block.terminator.as_ref().expect("Incomplete MIR: missing block terminator");
        self.codegen_terminator(terminator);
    }

    fn convert_type(&self, typ: &mir::Type) -> BasicTypeEnum<'ctx> {
        match typ {
            mir::Type::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type),
            mir::Type::Tuple(fields) => {
                let fields = mapvec(fields.iter(), |typ| self.convert_type(typ));
                let struct_type = self.llvm.struct_type(&fields, false);
                BasicTypeEnum::StructType(struct_type)
            },
            mir::Type::Function(_) => self.llvm.ptr_type(AddressSpace::default()).into(),
            mir::Type::Union(_) => self.llvm.ptr_type(AddressSpace::default()).into(),
            mir::Type::Generic(_) => self.llvm.ptr_type(AddressSpace::default()).into(),
        }
    }

    fn convert_primitive_type(&self, primitive_type: PrimitiveType) -> BasicTypeEnum<'ctx> {
        match primitive_type {
            PrimitiveType::Error => self.llvm.struct_type(&[], false).into(),//unreachable!("Cannot codegen llvm with errors"),
            PrimitiveType::Unit => self.llvm.struct_type(&[], false).into(),
            PrimitiveType::Bool => self.llvm.bool_type().into(),
            PrimitiveType::Pointer => self.llvm.ptr_type(AddressSpace::default()).into(),
            PrimitiveType::Char => self.llvm.i32_type().into(),
            PrimitiveType::Int(kind) => self.convert_integer_kind(kind).into(),
            PrimitiveType::Float(FloatKind::F32) => self.llvm.f32_type().into(),
            PrimitiveType::Float(FloatKind::F64) => self.llvm.f64_type().into(),
        }
    }

    fn convert_integer_kind(&self, kind: IntegerKind) -> IntType<'ctx> {
        match kind {
            IntegerKind::I8 | IntegerKind::U8 => self.llvm.i8_type(),
            IntegerKind::I16 | IntegerKind::U16 => self.llvm.i16_type(),
            IntegerKind::I32 | IntegerKind::U32 => self.llvm.i32_type(),
            IntegerKind::I64 | IntegerKind::U64 => self.llvm.i64_type(),
            IntegerKind::Isz | IntegerKind::Usz => {
                let triple = self.target.as_str().to_string_lossy();
                let triple = TargetTriple::create(&triple);
                let target = Target::from_triple(&triple).unwrap();
                let machine = target
                    .create_target_machine(
                        &triple,
                        "",
                        "",
                        inkwell::OptimizationLevel::None,
                        RelocMode::PIC,
                        CodeModel::Default,
                    )
                    .unwrap();
                let target_data = machine.get_target_data();
                self.llvm.ptr_sized_int_type(&target_data, None)
            },
        }
    }

    /// Convert a type into a function type, panics if the given type is not a function.
    /// When passed to [Self::convert_type], function types are translated to pointers by default,
    /// necessitating this function when an actual function type is required.
    fn convert_function_type(&self, typ: &mir::Type) -> inkwell::types::FunctionType<'ctx> {
        let mir::Type::Function(function_type) = typ else {
            panic!("Non-function type `{typ}` passed to `convert_function_type`")
        };

        let return_type = self.convert_type(&function_type.return_type);
        let parameters = mapvec(&function_type.parameters, |parameter| self.convert_type(parameter).into());
        return_type.fn_type(&parameters, false)
    }

    /// TODO: We could store the return type directly in the function to avoid searching for it
    fn find_return_type(&self, function: &mir::Function) -> mir::Type {
        for (_, block) in function.blocks.iter() {
            if let Some(TerminatorInstruction::Return(value)) = &block.terminator {
                return function.type_of_value(*value);
            }
        }
        mir::Type::ERROR
    }

    fn get_function(&self, id: FunctionId) -> &'ctx mir::Function {
        &self.mir.functions[&id]
    }

    fn lookup_value(&mut self, value: mir::Value) -> BasicValueEnum<'ctx> {
        match value {
            mir::Value::Error => unreachable!("Error value encountered during llvm codegen"),
            mir::Value::Unit => self.llvm.const_struct(&[], false).into(),
            mir::Value::Bool(value) => self.llvm.bool_type().const_int(value as u64, false).into(),
            mir::Value::Char(value) => self.llvm.i32_type().const_int(value as u64, false).into(),
            mir::Value::Integer(constant) => {
                let kind = constant.kind();
                let typ = self.convert_integer_kind(kind);
                typ.const_int(constant.as_u64(), kind.is_signed()).into()
            },
            mir::Value::Float(FloatConstant::F32(value)) => self.llvm.f32_type().const_float(value.0).into(),
            mir::Value::Float(FloatConstant::F64(value)) => self.llvm.f64_type().const_float(value.0).into(),
            mir::Value::InstructionResult(_) | mir::Value::Parameter(..) => {
                *self.values.get(&value).unwrap_or_else(|| panic!("llvm codegen: mir value is not cached: {value}"))
            },
            mir::Value::Function(function_id) => {
                if let Some(value) = self.values.get(&value) {
                    return *value;
                }

                let typ = self.get_function(self.current_function.unwrap()).type_of_value(value);
                let typ = self.convert_function_type(&typ);

                let name = &self.get_function(function_id).name;
                let function_value =
                    self.module.add_function(name, typ, None).as_global_value().as_pointer_value().into();
                self.values.insert(value, function_value);
                function_value
            },
            mir::Value::Global(_top_level_name) => todo!("lookup_value for globals"),
        }
    }

    fn codegen_instruction(&mut self, function: &mir::Function, id: mir::InstructionId) {
        let result = match &function.instructions[id] {
            mir::Instruction::Call { function, arguments } => {
                let function = self.lookup_value(*function).as_any_value_enum().into_function_value();
                let arguments = mapvec(arguments, |arg| self.lookup_value(*arg).into());
                self.builder.build_call(function, &arguments, "").unwrap().try_as_basic_value().unwrap_basic()
            },
            mir::Instruction::IndexTuple { tuple, index } => {
                let tuple = self.lookup_value(*tuple).into_struct_value();
                self.builder.build_extract_value(tuple, *index, "").unwrap()
            },
            mir::Instruction::MakeString(string) => {
                let string_data = self.llvm.const_string(string.as_bytes(), false).into();
                let length = self.llvm.i32_type().const_int(string.len() as u64, false).into();
                self.llvm.const_struct(&[string_data, length], false).into()
            },
            mir::Instruction::MakeTuple(fields) => {
                let fields = mapvec(fields, |field| self.lookup_value(*field));
                self.llvm.const_struct(&fields, false).into()
            },
            mir::Instruction::StackAlloc(value) => {
                let value = self.lookup_value(*value);
                let alloca = self.builder.build_alloca(value.get_type(), "").unwrap();
                self.builder.build_store(alloca, value).unwrap();
                alloca.into()
            },
            mir::Instruction::Transmute(value) => {
                // Transmute the value by storing it in an alloca and loading it as a different type
                let result_type = self.convert_type(&function.type_of_value(mir::Value::InstructionResult(id)));
                let value = self.lookup_value(*value);
                let alloca = self.builder.build_alloca(value.get_type(), "").unwrap();
                self.builder.build_store(alloca, value).unwrap();
                self.builder.build_load(result_type, alloca, "").unwrap()
            },
        };
        self.values.insert(mir::Value::InstructionResult(id), result);
    }

    fn remember_incoming(&mut self, target: BlockId, argument: &Option<mir::Value>) {
        if let Some(argument) = argument {
            let current_block = self.builder.get_insert_block().unwrap();
            let argument = self.lookup_value(*argument);
            self.incoming.entry(target).or_default().push((current_block, argument));
        }
    }

    fn codegen_terminator(&mut self, terminator: &TerminatorInstruction) {
        match terminator {
            TerminatorInstruction::Jmp((target_id, argument)) => {
                let target = self.blocks[*target_id];
                self.builder.build_unconditional_branch(target).unwrap();
                self.remember_incoming(*target_id, argument);
            },
            TerminatorInstruction::If { condition, then, else_, end: _ } => {
                let condition = self.lookup_value(*condition).into_int_value();

                let then_target = self.blocks[then.0];
                let else_target = self.blocks[else_.0];
                self.builder.build_conditional_branch(condition, then_target, else_target).unwrap();

                self.remember_incoming(then.0, &then.1);
                self.remember_incoming(else_.0, &else_.1);
            },
            TerminatorInstruction::Switch { int_value, cases, else_, end: _ } => {
                let int_value = self.lookup_value(*int_value).into_int_value();

                let cases = mapvec(cases.iter().enumerate(), |(i, (case_block, case_args))| {
                    self.remember_incoming(*case_block, case_args);
                    let case_block = self.blocks[*case_block];
                    let int_value = int_value.get_type().const_int(i as u64, false);
                    (int_value, case_block)
                });

                let else_block = if let Some((else_block, args)) = else_ {
                    self.remember_incoming(*else_block, args);
                    self.blocks[*else_block]
                } else {
                    // No else block but switch in llvm requires one.
                    // Create an empty block with an `unreachable` terminator.
                    let block = self.llvm.append_basic_block(self.current_function_value.unwrap(), "");
                    let current_block = self.builder.get_insert_block().unwrap();
                    self.builder.position_at_end(block);
                    self.builder.build_unreachable().unwrap();
                    self.builder.position_at_end(current_block);
                    block
                };

                self.builder.build_switch(int_value, else_block, &cases).unwrap();
            },
            TerminatorInstruction::Unreachable => {
                self.builder.build_unreachable().unwrap();
            },
            TerminatorInstruction::Return(value) => {
                let value = self.lookup_value(*value);
                self.builder.build_return(Some(&value)).unwrap();
            },
        }
    }
}
