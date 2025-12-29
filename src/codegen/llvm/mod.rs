use std::sync::Arc;

use inkwell::{
    AddressSpace,
    basic_block::BasicBlock,
    builder::Builder,
    module::Module,
    targets::{TargetData, TargetMachine, TargetTriple},
    types::{BasicType, BasicTypeEnum, IntType},
    values::{BasicValueEnum, FunctionValue},
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    incremental::{CodegenLlvm, DbHandle},
    iterator_extensions::vecmap,
    lexer::token::{FloatKind, IntegerKind},
    mir::{self, BlockId, FloatConstant, PrimitiveType, TerminatorInstruction},
    vecmap::VecMap,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodegenLlvmResult {
    module_string: Option<Arc<String>>,
}

pub fn codegen_llvm_impl(context: &CodegenLlvm, compiler: &DbHandle) -> CodegenLlvmResult {
    let module_string = crate::mir::builder::build_initial_mir(compiler, context.0).map(|mir| {
        let name = &mir.iter().next().unwrap().1.name;
        let llvm = inkwell::context::Context::create();
        let mut module = ModuleContext::new(&llvm, name);

        for (_id, function) in &mir {
            module.codegen_function(function);
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

    target: TargetTriple,

    blocks: VecMap<BlockId, BasicBlock<'ctx>>,

    incoming: FxHashMap<BlockId, Vec<(BlockId, Vec<BasicValueEnum<'ctx>>)>>,
}

impl<'ctx> ModuleContext<'ctx> {
    fn new(llvm: &'ctx inkwell::context::Context, name: &str) -> Self {
        let module = llvm.create_module(name);
        let target = TargetMachine::get_default_triple();
        module.set_triple(&target);
        Self {
            llvm,
            module,
            target,
            builder: llvm.create_builder(),
            blocks: Default::default(),
            incoming: Default::default(),
        }
    }

    fn codegen_function(&mut self, function: &crate::mir::Function) {
        let parameter_types =
            vecmap(&function.blocks[BlockId::ENTRY_BLOCK].parameter_types, |typ| self.convert_type(typ).into());

        let return_type = self.convert_type(&self.find_return_type(function));
        let function_type = return_type.fn_type(&parameter_types, false);
        let function_value = self.module.add_function(&function.name, function_type, None);

        self.create_blocks(function, function_value);

        for block in function.topological_sort() {
            self.codegen_block(block, function);
        }
    }

    /// Create an empty block for each block in the given function
    fn create_blocks(&mut self, function: &crate::mir::Function, function_value: FunctionValue<'ctx>) {
        for (block_id, _) in function.blocks.iter() {
            let block = self.llvm.append_basic_block(function_value, "");
            self.blocks.push_existing(block_id, block);
        }
    }

    fn codegen_block(&self, block_id: BlockId, function: &mir::Function) {
        let llvm_block = self.blocks[block_id];
        self.builder.position_at_end(llvm_block);

        let block = &function.blocks[block_id];
        for instruction_id in block.instructions.iter().copied() {
            let instruction = &function.instructions[instruction_id];
            self.codegen_instruction(instruction);
        }

        let terminator = block.terminator.as_ref().expect("Incomplete MIR: missing block terminator");
        self.codegen_terminator(terminator);
    }

    fn convert_type(&self, typ: &mir::Type) -> BasicTypeEnum<'ctx> {
        match typ {
            mir::Type::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type),
            mir::Type::Tuple(fields) => {
                let fields = vecmap(fields.iter(), |typ| self.convert_type(typ));
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
            PrimitiveType::Error => unreachable!("Cannot codegen llvm with errors"),
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
                let target_data = TargetData::create(&self.target.to_string());
                self.llvm.ptr_sized_int_type(&target_data, None)
            },
        }
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

    fn lookup_value(&self, value: mir::Value) -> BasicValueEnum<'ctx> {
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
            mir::Value::Float(FloatConstant::F32(value)) => self.llvm.f32_type().const_float(value as f64).into(),
            mir::Value::Float(FloatConstant::F64(value)) => self.llvm.f64_type().const_float(value as f64).into(),
            mir::Value::InstructionResult(instruction_id) => todo!(),
            mir::Value::Parameter(block_id, _) => todo!(),
            mir::Value::Function(function_id) => todo!(),
            mir::Value::Global(top_level_name) => todo!(),
        }
    }

    fn codegen_instruction(&self, instruction: &mir::Instruction) {
        match instruction {
            mir::Instruction::Call { function, arguments } => todo!(),
            mir::Instruction::IndexTuple { tuple, index } => todo!(),
            mir::Instruction::MakeString(_) => todo!(),
            mir::Instruction::MakeTuple(values) => todo!(),
            mir::Instruction::StackAlloc(value) => todo!(),
            mir::Instruction::Transmute(value) => todo!(),
        }
    }

    fn codegen_terminator(&self, terminator: &TerminatorInstruction) {
        match terminator {
            TerminatorInstruction::Jmp((target, arguments)) => {
                let target = self.blocks[*target];
                self.builder.build_unconditional_branch(target);
            },
            TerminatorInstruction::If { condition, then, else_, end: _ } => {
                self.lookup_value(*condition);
            },
            TerminatorInstruction::Switch { int_value, cases, else_, end: _ } => todo!(),
            TerminatorInstruction::Unreachable => todo!(),
            TerminatorInstruction::Return(value) => todo!(),
        }
    }
}
