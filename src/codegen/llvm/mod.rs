use std::sync::Arc;

use inkwell::{
    AddressSpace, FloatPredicate, IntPredicate,
    basic_block::BasicBlock,
    builder::Builder,
    memory_buffer::MemoryBuffer,
    module::Module,
    passes::PassBuilderOptions,
    targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
    types::{BasicType, BasicTypeEnum, IntType},
    values::{AggregateValue, BasicValue, BasicValueEnum, FunctionValue, PhiValue},
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    cli::OptLevel,
    incremental::Db,
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    mir::{self, BlockId, DefinitionId, FloatConstant, InstructionId, PrimitiveType, TerminatorInstruction},
    vecmap::VecMap,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodegenLlvmResult {
    pub module_bitcode: Arc<Vec<u8>>,
}

pub fn initialize_native_target() {
    let config = InitializationConfig::default();
    Target::initialize_native(&config).unwrap();
}

pub fn codegen_llvm(compiler: &Db, show_time: bool, opt_level: OptLevel) -> Option<CodegenLlvmResult> {
    // Whole-program for now; ideally `CodegenLlvmResult` could be split per item.
    let mir =
        crate::timings::time_phase("Monomorphization", show_time, || mir::monomorphization::monomorphize(compiler));
    crate::timings::time_phase("LLVM codegen", show_time, || codegen_llvm_for_mir(&mir, opt_level))
}

/// LLVM IR generation on an already-monomorphized `Mir`
pub(crate) fn codegen_llvm_for_mir(mir: &mir::Mir, opt_level: OptLevel) -> Option<CodegenLlvmResult> {
    let name = &mir.definitions.iter().next().map_or("_", |(_, function)| &function.name);

    initialize_native_target();
    let llvm = inkwell::context::Context::create();
    let mut module = ModuleContext::new(&llvm, mir, name);

    for (id, function) in &mir.definitions {
        module.codegen_function(function, *id);
    }

    module.codegen_main_wrapper();

    assert!(mir.externals.is_empty(), "All Mir compilation units should be linked");

    if let Err(error) = module.module.verify() {
        module.module.print_to_stderr();
        eprintln!("llvm module failed to verify: {error}");
    }

    if opt_level != OptLevel::O0 {
        let target_machine = native_target_machine(opt_level);
        module
            .module
            .run_passes(opt_level.as_passes_string(), &target_machine, PassBuilderOptions::create())
            .expect("LLVM pass pipeline failed");
    }

    // TODO: This is inefficient
    let bitcode = module.module.write_bitcode_to_memory();
    let bitcode = bitcode.as_slice().to_vec();
    let module_bitcode = Arc::new(bitcode);

    // Per-function codegen + later link is likely slower than all-at-once, but easier to relax
    // back into the whole-program path than the reverse.
    Some(CodegenLlvmResult { module_bitcode })
}

/// Link the given list of llvm bitcode modules into an executable.
/// Returns `true` if linking succeeded, `false` otherwise.
pub fn link(modules: Vec<Arc<Vec<u8>>>, binary_name: &str, show_time: bool, opt_level: OptLevel) -> bool {
    let llvm = inkwell::context::Context::create();
    let module = llvm.create_module(binary_name);

    // O(program-size) even for a whole-program single module, so it gets its own bucket.
    crate::timings::time_phase("Bitcode assembly", show_time, || {
        for bitcode in modules {
            let buffer = MemoryBuffer::create_from_memory_range(&bitcode, "buffer");
            let new_module =
                Module::parse_bitcode_from_buffer(&buffer, &llvm).expect("Failed to parse llvm module bitcode");
            module.link_in_module(new_module).expect("Failed to link in llvm module");
        }
    });

    let path = std::path::Path::new(binary_name).with_extension("o");
    let target_machine = native_target_machine(opt_level);

    // Typically the most expensive LLVM step (IR -> machine code).
    crate::timings::time_phase("Object emission", show_time, || {
        target_machine.write_to_file(&module, FileType::Object, &path).unwrap();
    });

    crate::timings::time_phase("Linking", show_time, || {
        super::link_with_gcc(path.to_string_lossy().as_ref(), binary_name)
    })
}

fn native_target_machine(opt_level: OptLevel) -> TargetMachine {
    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).unwrap();
    target.create_target_machine(&triple, "", "", opt_level.inkwell(), RelocMode::PIC, CodeModel::Default).unwrap()
}

/// The codegen-side representation of a [mir::Definition].
///
/// Functions get a real LLVM function definition/declaration; "let"-style globals
/// constant-fold to a value that's inlined at every use site so we don't need to
/// allocate a backing global slot or emit a load before each call.
#[derive(Copy, Clone)]
enum CodegenValue<'ctx> {
    Function(FunctionValue<'ctx>),
    Literal(BasicValueEnum<'ctx>),
}

impl<'ctx> CodegenValue<'ctx> {
    /// Project this codegen value to the [BasicValueEnum] callers see when referencing
    /// the definition: a function pointer for functions, the inlined literal otherwise.
    fn into_basic_value(self) -> BasicValueEnum<'ctx> {
        match self {
            CodegenValue::Function(fv) => fv.as_global_value().as_pointer_value().into(),
            CodegenValue::Literal(v) => v,
        }
    }
}

struct ModuleContext<'ctx> {
    llvm: &'ctx inkwell::context::Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,

    mir: &'ctx mir::Mir,

    blocks: VecMap<BlockId, BasicBlock<'ctx>>,

    current_function: Option<DefinitionId>,
    current_function_value: Option<FunctionValue<'ctx>>,

    /// This is the source-level `main` which we create a wrapper around to return 0 even though
    /// Ante's main has a signature returning a Unit value.
    /// This is `None` for libraries that do not define `main`.
    ante_main: Option<FunctionValue<'ctx>>,

    definitions: FxHashMap<DefinitionId, CodegenValue<'ctx>>,
    values: FxHashMap<mir::Value, BasicValueEnum<'ctx>>,

    /// Block arguments are added here to later insert them as PHI values.
    ///
    /// Maps merge_block to a vec of each incoming block along with the arguments it branches with.
    incoming: FxHashMap<BlockId, Vec<(BasicBlock<'ctx>, BasicValueEnum<'ctx>)>>,

    /// PHI nodes created for each block's parameter. Filled in after all blocks are processed,
    /// so that back edges from later-processed blocks are included.
    phi_nodes: FxHashMap<BlockId, PhiValue<'ctx>>,
}

impl<'ctx> ModuleContext<'ctx> {
    fn new(llvm: &'ctx inkwell::context::Context, mir: &'ctx mir::Mir, name: &str) -> Self {
        let module = llvm.create_module(name);
        let target = TargetMachine::get_default_triple();
        module.set_triple(&target);
        Self {
            llvm,
            module,
            mir,
            current_function: None,
            current_function_value: None,
            ante_main: None,
            definitions: Default::default(),
            values: Default::default(),
            builder: llvm.create_builder(),
            blocks: Default::default(),
            incoming: Default::default(),
            phi_nodes: Default::default(),
        }
    }

    fn codegen_global(&mut self, global: &mir::Definition, id: mir::DefinitionId) {
        // Stash `values` since this may be called mid function-codegen.
        let saved_values = std::mem::take(&mut self.values);

        for instruction in global.entry_block().instructions.iter().copied() {
            let value = self.codegen_constant_instruction(global, instruction);
            self.values.insert(mir::Value::InstructionResult(instruction), value);
        }

        let TerminatorInstruction::Result(result_value) = global.entry_block().terminator.as_ref().unwrap() else {
            panic!("Global definition missing Result terminator");
        };
        let initializer = self.constant_value(*result_value);

        self.values = saved_values;

        // Inlined at every use site rather than emitted as an LLVM global with backing storage.
        // Globals' bodies are already restricted to constant-foldable instructions
        // (see `codegen_constant_instruction`), so a backing slot is pure indirection.
        self.definitions.insert(id, CodegenValue::Literal(initializer));
    }

    fn constant_value(&mut self, value: mir::Value) -> BasicValueEnum<'ctx> {
        match value {
            mir::Value::Unit => self.unit_value(),
            mir::Value::Bool(b) => self.llvm.bool_type().const_int(b as u64, false).into(),
            mir::Value::Char(c) => self.llvm.i8_type().const_int(c as u64, false).into(),
            mir::Value::Integer(constant) => {
                let kind = constant.kind();
                let typ = self.convert_integer_kind(kind);
                typ.const_int(constant.as_u64(), kind.is_signed()).into()
            },
            mir::Value::Float(FloatConstant::F32(v)) => self.llvm.f32_type().const_float(v.0).into(),
            mir::Value::Float(FloatConstant::F64(v)) => self.llvm.f64_type().const_float(v.0).into(),
            mir::Value::InstructionResult(_) | mir::Value::Parameter(..) => {
                *self.values.get(&value).expect("constant value not cached")
            },
            mir::Value::Definition(id) => self.codegen_value_for(id).into_basic_value(),
            mir::Value::Error => unreachable!("Error value in global initializer"),
        }
    }

    fn codegen_constant_instruction(
        &mut self, global: &mir::Definition, id: mir::InstructionId,
    ) -> BasicValueEnum<'ctx> {
        match &global.instructions[id] {
            mir::Instruction::MakeTuple(fields) => {
                let fields: Vec<mir::Value> = fields.clone();
                let field_values: Vec<BasicValueEnum<'ctx>> = fields.iter().map(|f| self.constant_value(*f)).collect();
                self.llvm.const_struct(&field_values, false).into()
            },
            mir::Instruction::Id(value) => {
                let value = *value;
                self.constant_value(value)
            },
            mir::Instruction::Transmute(value) => {
                // Const context can't alloca/store/load, so only zero-sized sources
                // (e.g. unit `{}` reinterpreted as a None variant's inner field) work
                // here, producing undef of the destination type.
                let source = self.constant_value(*value);
                let is_empty_struct = matches!(
                    source.get_type(),
                    BasicTypeEnum::StructType(s) if s.count_fields() == 0
                );
                assert!(
                    is_empty_struct,
                    "Transmute in global initializer requires a zero-sized source type, got {:?}",
                    source.get_type()
                );
                let result_type = self.convert_type(global.instruction_result_type(id));
                Self::undef_value(result_type)
            },
            mir::Instruction::Extern(name) => {
                let typ = global.instruction_result_type(id);
                match self.convert_function_type(typ) {
                    Some(fn_type) => {
                        let fn_val = self
                            .module
                            .get_function(name)
                            .unwrap_or_else(|| self.module.add_function(name, fn_type, None));
                        fn_val.as_global_value().as_pointer_value().into()
                    },
                    None => {
                        let global = self
                            .module
                            .get_global(name)
                            .unwrap_or_else(|| self.module.add_global(self.convert_type(typ), None, name));
                        global.as_pointer_value().into()
                    },
                }
            },
            other => panic!("Unsupported instruction in global initializer: {other:?}"),
        }
    }

    fn codegen_function(&mut self, function: &mir::Definition, id: mir::DefinitionId) {
        if function.is_global() {
            self.codegen_global(function, id);
            return;
        }

        // A function whose signature exposes `Never` to LLVM type lowering can never
        // actually be called. Emit it with a Never-free placeholder signature and an
        // `unreachable`-only body.
        let body_is_unreachable = Self::function_signature_reaches_never(&function.typ);
        let signature_typ =
            if body_is_unreachable { Self::replace_never_with_unit(&function.typ) } else { function.typ.clone() };

        let is_ante_main = function.name.as_str() == "main";
        let function_value = match self.definitions.get(&id) {
            Some(CodegenValue::Function(fv)) => *fv,
            Some(CodegenValue::Literal(_)) => panic!(
                "codegen_function: definition {id} was already codegen'd as a literal global, but its body is a function"
            ),
            None => {
                let function_type = self.convert_function_type(&signature_typ).unwrap();
                // Rename `main`: see `codegen_main_wrapper`.
                let mangled_name = if is_ante_main {
                    format!("main_{}%", function.id)
                } else {
                    format!("{}_{}", function.name, function.id)
                };
                let function_value = self.module.add_function(&mangled_name, function_type, None);
                self.definitions.insert(id, CodegenValue::Function(function_value));
                function_value
            },
        };

        if is_ante_main {
            self.ante_main = Some(function_value);
        }

        self.current_function = Some(id);
        self.current_function_value = Some(function_value);

        if body_is_unreachable {
            let entry = self.llvm.append_basic_block(function_value, "");
            self.builder.position_at_end(entry);
            self.builder.build_unreachable().unwrap();
            return;
        }

        self.create_blocks(function, function_value);

        for i in 0..function.blocks[BlockId::ENTRY_BLOCK].parameter_types.len() as u32 {
            let value = mir::Value::Parameter(BlockId::ENTRY_BLOCK, i);
            let llvm_value = function_value.get_nth_param(i).unwrap();
            self.values.insert(value, llvm_value);
        }

        for block in function.topological_sort() {
            self.codegen_block(block, function);
        }

        // Done after all blocks are processed so back-edge sources are included.
        for (block_id, phi) in self.phi_nodes.drain() {
            let incoming = self
                .incoming
                .remove(&block_id)
                .unwrap_or_else(|| panic!("llvm codegen: No incoming for block {block_id}"));
            for (pred_block, value) in incoming {
                phi.add_incoming(&[(&value, pred_block)]);
            }
        }

        self.values.clear();
        self.blocks.clear();
        self.incoming.clear();
    }

    /// Emit a `main (): I32` wrapper around the source `main` that returns 0.
    fn codegen_main_wrapper(&mut self) {
        let Some(ante_main) = self.ante_main else { return };

        let i32_type = self.llvm.i32_type();
        let wrapper_type = i32_type.fn_type(&[], false);
        let wrapper = self.module.add_function("main", wrapper_type, None);

        let entry = self.llvm.append_basic_block(wrapper, "");
        self.builder.position_at_end(entry);

        let unit = self.unit_value().into();
        self.builder.build_direct_call(ante_main, &[unit], "").unwrap();
        self.builder.build_return(Some(&i32_type.const_int(0, false))).unwrap();
    }

    fn create_blocks(&mut self, function: &mir::Definition, function_value: FunctionValue<'ctx>) {
        for (block_id, _) in function.blocks.iter() {
            let block = self.llvm.append_basic_block(function_value, "");
            self.blocks.push_existing(block_id, block);
        }
    }

    fn codegen_block(&mut self, block_id: BlockId, function: &mir::Definition) {
        let llvm_block = self.blocks[block_id];
        self.builder.position_at_end(llvm_block);
        let block = &function.blocks[block_id];

        // PHI incomings are filled in by `codegen_function` after every block is processed.
        if block_id != BlockId::ENTRY_BLOCK {
            for (parameter, parameter_type) in block.parameters(block_id) {
                let parameter_type = self.convert_type(&parameter_type);
                let phi = self.builder.build_phi(parameter_type, "").unwrap();
                self.values.insert(parameter, phi.as_basic_value());
                self.phi_nodes.insert(block_id, phi);
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
            mir::Type::Function(f) => {
                if let Some(env) = f.environment() {
                    // A closure value is a {fn_ptr, env} struct, matching what PackClosure produces.
                    let ptr = self.llvm.ptr_type(AddressSpace::default()).into();
                    let env_type = self.convert_type(env);
                    self.llvm.struct_type(&[ptr, env_type], false).into()
                } else {
                    self.llvm.ptr_type(AddressSpace::default()).into()
                }
            },
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
            PrimitiveType::Char => self.llvm.i8_type().into(),
            PrimitiveType::Int(kind) => self.convert_integer_kind(kind).into(),
            PrimitiveType::Float(FloatKind::F32) => self.llvm.f32_type().into(),
            PrimitiveType::Float(FloatKind::F64) => self.llvm.f64_type().into(),
            PrimitiveType::Never => unreachable!("Cannot codegen llvm with a bare Never type"),
            PrimitiveType::NoClosureEnv => unreachable!("Cannot convert NoClosureEnv"),
        }
    }

    fn convert_integer_kind(&self, kind: IntegerKind) -> IntType<'ctx> {
        match kind {
            IntegerKind::I8 | IntegerKind::U8 => self.llvm.i8_type(),
            IntegerKind::I16 | IntegerKind::U16 => self.llvm.i16_type(),
            IntegerKind::I32 | IntegerKind::U32 => self.llvm.i32_type(),
            IntegerKind::I64 | IntegerKind::U64 => self.llvm.i64_type(),
            IntegerKind::Isz | IntegerKind::Usz => {
                // Pointer size is a property of the target triple, not the opt level, so O0 is fine here.
                let machine = native_target_machine(OptLevel::O0);
                let target_data = machine.get_target_data();
                self.llvm.ptr_sized_int_type(&target_data, None)
            },
        }
    }

    /// If `typ` is a function type whose return is `Never`, return a copy with
    /// the return type swapped for `result_type`. Otherwise return `typ`
    /// unchanged.
    fn fixup_never_return(typ: mir::Type, result_type: &mir::Type) -> mir::Type {
        match typ {
            mir::Type::Function(f) if f.return_type == mir::Type::NEVER => {
                mir::Type::Function(Arc::new(mir::FunctionType {
                    parameters: f.parameters.clone(),
                    environment: f.environment.clone(),
                    return_type: result_type.clone(),
                }))
            },
            other => other,
        }
    }

    fn replace_never_with_unit(typ: &mir::Type) -> mir::Type {
        match typ {
            mir::Type::Primitive(mir::PrimitiveType::Never) => mir::Type::UNIT,
            mir::Type::Primitive(_) | mir::Type::Generic(_) => typ.clone(),
            mir::Type::Tuple(fields) => {
                mir::Type::Tuple(Arc::new(fields.iter().map(Self::replace_never_with_unit).collect()))
            },
            mir::Type::Union(fields) => {
                mir::Type::Union(Arc::new(fields.iter().map(Self::replace_never_with_unit).collect()))
            },
            mir::Type::Function(f) => mir::Type::Function(Arc::new(mir::FunctionType {
                parameters: f.parameters.iter().map(Self::replace_never_with_unit).collect(),
                environment: Self::replace_never_with_unit(&f.environment),
                return_type: Self::replace_never_with_unit(&f.return_type),
            })),
        }
    }

    /// True if [`Self::convert_type`] would reach `convert_primitive_type(Never)`
    /// when invoked on this type. Mirrors `convert_type`'s recursion exactly:
    /// primitives are checked directly; tuples and unions recurse into fields;
    /// nested function types only recurse into their environment (their own
    /// parameters / return types become opaque function pointers and are not
    /// followed). Generics never reach a primitive.
    fn convert_reaches_never(typ: &mir::Type) -> bool {
        match typ {
            mir::Type::Primitive(mir::PrimitiveType::Never) => true,
            mir::Type::Primitive(_) | mir::Type::Generic(_) => false,
            mir::Type::Tuple(fields) | mir::Type::Union(fields) => fields.iter().any(Self::convert_reaches_never),
            mir::Type::Function(f) => Self::convert_reaches_never(&f.environment),
        }
    }

    /// True if `convert_function_type(typ)` would reach a `Never` while lowering.
    fn function_signature_reaches_never(typ: &mir::Type) -> bool {
        match typ {
            mir::Type::Function(f) => {
                Self::convert_reaches_never(&f.return_type)
                    || f.parameters.iter().any(Self::convert_reaches_never)
                    || Self::convert_reaches_never(&f.environment)
            },
            _ => false,
        }
    }

    /// Convert a type into a function type, returns None if the given type is not a function.
    /// When passed to [Self::convert_type], function types are translated to pointers by default,
    /// necessitating this function when an actual function type is required.
    fn convert_function_type(&self, typ: &mir::Type) -> Option<inkwell::types::FunctionType<'ctx>> {
        let mir::Type::Function(function_type) = typ else {
            return None;
        };

        let return_type = self.convert_type(&function_type.return_type);
        let mut parameters = mapvec(&function_type.parameters, |parameter| self.convert_type(parameter).into());
        if let Some(env) = function_type.environment() {
            parameters.push(self.convert_type(env).into());
        }
        Some(return_type.fn_type(&parameters, false))
    }

    /// Returns the name of the given [DefinitionId].
    /// As long as the [DefinitionId] is referenced in `self.mir`, this should never panic.
    fn get_name(&self, id: DefinitionId) -> &'ctx str {
        self.mir.get_name(id).unwrap().as_ref()
    }

    /// Resolve a [DefinitionId] to its [CodegenValue], codegen-ing the definition on demand
    /// when this is the first reference to it (e.g. a forward reference from another function,
    /// or a global referenced inside another global initializer).
    fn codegen_value_for(&mut self, id: DefinitionId) -> CodegenValue<'ctx> {
        if let Some(existing) = self.definitions.get(&id) {
            return *existing;
        }

        let def = self.mir.definitions.get(&id).expect("codegen_value_for: definition not found").clone();
        if def.is_global() {
            self.codegen_global(&def, id);
        } else {
            // Forward-declare the function with the mangled name `codegen_function` will use,
            // to avoid colliding with C extern names. If the signature would
            // expose `Never` to LLVM type lowering, swap it for `Unit` here so
            // the LLVM signature is valid; the body emitted later is just
            // `unreachable`.
            let signature_typ = if Self::function_signature_reaches_never(&def.typ) {
                Self::replace_never_with_unit(&def.typ)
            } else {
                def.typ.clone()
            };
            let fn_type = self
                .convert_function_type(&signature_typ)
                .expect("codegen_value_for: non-global definition must have a function type");
            let mangled_name = format!("{}_{}", self.get_name(id), id);
            let fv = self.module.add_function(&mangled_name, fn_type, None);
            self.definitions.insert(id, CodegenValue::Function(fv));
        }
        self.definitions[&id]
    }

    fn lookup_value(&mut self, value: &mir::Value) -> BasicValueEnum<'ctx> {
        match value {
            mir::Value::Error => unreachable!("Error value encountered during llvm codegen"),
            mir::Value::Unit => self.unit_value(),
            mir::Value::Bool(value) => self.llvm.bool_type().const_int(*value as u64, false).into(),
            mir::Value::Char(value) => self.llvm.i8_type().const_int(*value as u64, false).into(),
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
            mir::Value::Definition(function_id) => self.codegen_value_for(*function_id).into_basic_value(),
        }
    }

    fn unit_value(&mut self) -> BasicValueEnum<'ctx> {
        self.llvm.const_struct(&[], false).into()
    }

    fn codegen_instruction(&mut self, function: &mir::Definition, id: mir::InstructionId) {
        let result = match &function.instructions[id] {
            mir::Instruction::Call { function: function_value, arguments } => {
                let raw = self.mir.type_of_value(function_value, function);
                let result_type = function.instruction_result_type(id);
                let fn_type = Self::fixup_never_return(raw, result_type);
                let typ = self.convert_function_type(&fn_type).unwrap();
                let function = self.lookup_value(function_value).into_pointer_value();
                let arguments = mapvec(arguments, |arg| self.lookup_value(arg).into());
                self.builder
                    .build_indirect_call(typ, function, &arguments, "")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
            },
            mir::Instruction::Perform { .. } => {
                unreachable!("Instruction::Perform remaining in LLVM codegen")
            },
            mir::Instruction::Handle { .. } => {
                unreachable!("Instruction::Handle remaining LLVM codegen")
            },
            mir::Instruction::HandlerCap => {
                unreachable!("Instruction::HandlerCap remaining in LLVM codegen")
            },
            mir::Instruction::CallClosure { closure, arguments } => {
                let raw = self.mir.type_of_value(closure, function);
                let result_type = function.instruction_result_type(id);
                let fn_type = Self::fixup_never_return(raw, result_type);
                let typ = self.convert_function_type(&fn_type).unwrap();
                let closure = self.lookup_value(closure).into_struct_value();
                let mut arguments = mapvec(arguments, |arg| self.lookup_value(arg).into());
                let function = self.builder.build_extract_value(closure, 0, "").unwrap().into_pointer_value();
                let environment = self.builder.build_extract_value(closure, 1, "").unwrap();
                arguments.push(environment.into());
                self.builder
                    .build_indirect_call(typ, function, &arguments, "")
                    .unwrap()
                    .try_as_basic_value()
                    .unwrap_basic()
            },
            mir::Instruction::PackClosure { function, environment } => self.make_tuple(&[*function, *environment]),
            mir::Instruction::IndexTuple { tuple, index } => {
                let tuple = self.lookup_value(tuple).into_struct_value();
                self.builder.build_extract_value(tuple, *index, "").unwrap()
            },
            // String: { ptr data, ptr rc, u32 len, u32 offset }
            mir::Instruction::MakeString(string) => {
                let string_data = self.llvm.const_string(string.as_bytes(), true);
                // Need to create a unique name across modules. Llvm will auto-rename within
                // the same module so duplicate names within one function won't matter.
                let name = format!("{}_str", self.current_function.unwrap());
                let global = self.module.add_global(string_data.get_type(), None, &name);
                global.set_initializer(&string_data);
                let c_string = global.as_pointer_value().into();

                let length = self.llvm.i32_type().const_int(string.len() as u64, false).into();
                let offset = self.llvm.i32_type().const_int(0, false).into();
                let null_ptr = self.llvm.ptr_type(AddressSpace::default()).const_null().into();
                self.llvm.const_struct(&[c_string, null_ptr, length, offset], false).into()
            },
            mir::Instruction::MakeTuple(fields) => self.make_tuple(fields),
            mir::Instruction::StackAlloc(value) => {
                let value = self.lookup_value(value);
                let alloca = self.builder.build_alloca(value.get_type(), "").unwrap();
                self.builder.build_store(alloca, value).unwrap();
                alloca.into()
            },
            mir::Instruction::Transmute(value) => self.transmute(value, function, id),
            mir::Instruction::Id(value) => self.lookup_value(value),
            mir::Instruction::Instantiate(..) => {
                unreachable!("Instruction::Instantiate remaining in the code during llvm codegen")
            },
            mir::Instruction::AddInt(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_add(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::AddFloat(a, b) => {
                let a = self.lookup_value(a).into_float_value();
                let b = self.lookup_value(b).into_float_value();
                self.builder.build_float_add(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::SubInt(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_sub(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::SubFloat(a, b) => {
                let a = self.lookup_value(a).into_float_value();
                let b = self.lookup_value(b).into_float_value();
                self.builder.build_float_sub(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::MulInt(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_mul(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::MulFloat(a, b) => {
                let a = self.lookup_value(a).into_float_value();
                let b = self.lookup_value(b).into_float_value();
                self.builder.build_float_mul(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::DivSigned(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_signed_div(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::DivUnsigned(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_unsigned_div(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::DivFloat(a, b) => {
                let a = self.lookup_value(a).into_float_value();
                let b = self.lookup_value(b).into_float_value();
                self.builder.build_float_div(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::ModSigned(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_signed_rem(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::ModUnsigned(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_unsigned_rem(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::ModFloat(a, b) => {
                let a = self.lookup_value(a).into_float_value();
                let b = self.lookup_value(b).into_float_value();
                self.builder.build_float_rem(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::LessSigned(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_compare(IntPredicate::SLT, a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::LessUnsigned(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_compare(IntPredicate::ULT, a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::LessFloat(a, b) => {
                let a = self.lookup_value(a).into_float_value();
                let b = self.lookup_value(b).into_float_value();
                self.builder.build_float_compare(FloatPredicate::OLT, a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::EqInt(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_int_compare(IntPredicate::EQ, a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::EqFloat(a, b) => {
                let a = self.lookup_value(a).into_float_value();
                let b = self.lookup_value(b).into_float_value();
                self.builder.build_float_compare(FloatPredicate::OEQ, a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::BitwiseAnd(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_and(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::BitwiseOr(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_or(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::BitwiseXor(a, b) => {
                let a = self.lookup_value(a).into_int_value();
                let b = self.lookup_value(b).into_int_value();
                self.builder.build_xor(a, b, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::BitwiseNot(value) => {
                let value = self.lookup_value(value).into_int_value();
                self.builder.build_not(value, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::SignExtend(value) => {
                let value = self.lookup_value(value).into_int_value();
                let int_type = self.convert_type(function.instruction_result_type(id)).into_int_type();
                self.builder.build_int_s_extend(value, int_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::ZeroExtend(value) => {
                let value = self.lookup_value(value).into_int_value();
                let int_type = self.convert_type(function.instruction_result_type(id)).into_int_type();
                self.builder.build_int_z_extend(value, int_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::SignedToFloat(value) => {
                let value = self.lookup_value(value).into_int_value();
                let float_type = self.convert_type(function.instruction_result_type(id)).into_float_type();
                self.builder.build_signed_int_to_float(value, float_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::UnsignedToFloat(value) => {
                let value = self.lookup_value(value).into_int_value();
                let float_type = self.convert_type(function.instruction_result_type(id)).into_float_type();
                self.builder.build_unsigned_int_to_float(value, float_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::FloatToSigned(value) => {
                let value = self.lookup_value(value).into_float_value();
                let int_type = self.convert_type(function.instruction_result_type(id)).into_int_type();
                self.builder.build_float_to_signed_int(value, int_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::FloatToUnsigned(value) => {
                let value = self.lookup_value(value).into_float_value();
                let int_type = self.convert_type(function.instruction_result_type(id)).into_int_type();
                self.builder.build_float_to_unsigned_int(value, int_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::FloatPromote(value) => {
                let value = self.lookup_value(value).into_float_value();
                let float_type = self.convert_type(function.instruction_result_type(id)).into_float_type();
                self.builder.build_float_cast(value, float_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::FloatDemote(value) => {
                let value = self.lookup_value(value).into_float_value();
                let float_type = self.convert_type(function.instruction_result_type(id)).into_float_type();
                self.builder.build_float_cast(value, float_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::Truncate(value) => {
                let value = self.lookup_value(value).into_int_value();
                let int_type = self.convert_type(function.instruction_result_type(id)).into_int_type();
                self.builder.build_int_truncate(value, int_type, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::Deref(value) => {
                let value = self.lookup_value(value).into_pointer_value();
                let result_type = self.convert_type(function.instruction_result_type(id));
                self.builder.build_load(result_type, value, "").unwrap().as_basic_value_enum()
            },
            mir::Instruction::Store { pointer, value } => {
                let pointer = self.lookup_value(pointer).into_pointer_value();
                let value = self.lookup_value(value);
                self.builder.build_store(pointer, value).unwrap();
                self.unit_value()
            },
            mir::Instruction::GetFieldPtr { struct_ptr, struct_type, index } => {
                let struct_ptr = self.lookup_value(struct_ptr).into_pointer_value();
                let struct_llvm_type = self.convert_type(struct_type).into_struct_type();
                self.builder.build_struct_gep(struct_llvm_type, struct_ptr, *index, "").unwrap().into()
            },
            mir::Instruction::SizeOf(_) => todo!("SizeOf should be removed by monomorphization"),
            mir::Instruction::Extern(name) => {
                let typ = function.instruction_result_type(id);
                match self.convert_function_type(typ) {
                    Some(fn_type) => {
                        let fn_val = self
                            .module
                            .get_function(name)
                            .unwrap_or_else(|| self.module.add_function(name, fn_type, None));
                        fn_val.as_global_value().as_pointer_value().into()
                    },
                    None => {
                        let global = self
                            .module
                            .get_global(name)
                            .unwrap_or_else(|| self.module.add_global(self.convert_type(typ), None, name));
                        global.as_pointer_value().into()
                    },
                }
            },
        };
        self.values.insert(mir::Value::InstructionResult(id), result);
    }

    fn transmute(&mut self, value: &mir::Value, function: &mir::Definition, id: InstructionId) -> BasicValueEnum<'ctx> {
        let result_type = self.convert_type(function.instruction_result_type(id));
        let value = self.lookup_value(value);
        let alloca = self.builder.build_alloca(value.get_type(), "").unwrap();
        self.builder.build_store(alloca, value).unwrap();
        self.builder.build_load(result_type, alloca, "").unwrap()
    }

    fn make_tuple(&mut self, fields: &[mir::Value]) -> BasicValueEnum<'ctx> {
        let fields = mapvec(fields, |field| self.lookup_value(field));
        let const_fields =
            mapvec(&fields, |field| if field.is_const() { *field } else { Self::undef_value(field.get_type()) });
        let mut tuple = self.llvm.const_struct(&const_fields, false).as_aggregate_value_enum();

        for (i, field) in fields.into_iter().enumerate() {
            if !field.is_const() {
                tuple = self.builder.build_insert_value(tuple, field, i as u32, "").unwrap();
            }
        }
        tuple.as_basic_value_enum()
    }

    fn undef_value(typ: BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
        match typ {
            BasicTypeEnum::ArrayType(array) => array.get_undef().into(),
            BasicTypeEnum::FloatType(float) => float.get_undef().into(),
            BasicTypeEnum::IntType(int) => int.get_undef().into(),
            BasicTypeEnum::PointerType(pointer) => pointer.get_undef().into(),
            BasicTypeEnum::StructType(tuple) => tuple.get_undef().into(),
            BasicTypeEnum::VectorType(vector) => vector.get_undef().into(),
            BasicTypeEnum::ScalableVectorType(vector) => vector.get_undef().into(),
        }
    }

    fn remember_incoming(&mut self, target: BlockId, argument: &Option<mir::Value>) {
        if let Some(argument) = argument {
            let current_block = self.builder.get_insert_block().unwrap();
            let argument = self.lookup_value(argument);
            self.incoming.entry(target).or_default().push((current_block, argument));
        }
    }

    fn codegen_terminator(&mut self, terminator: &TerminatorInstruction) {
        match terminator {
            TerminatorInstruction::Jmp((target_id, argument)) => {
                let target = self.blocks[*target_id];
                // remember_incoming can emit load instructions so it needs to be
                // called before we insert the terminator instruction
                self.remember_incoming(*target_id, argument);
                self.builder.build_unconditional_branch(target).unwrap();
            },
            TerminatorInstruction::If { condition, then, else_, end: _ } => {
                let condition = self.lookup_value(condition).into_int_value();

                let then_target = self.blocks[then.0];
                let else_target = self.blocks[else_.0];

                self.remember_incoming(then.0, &then.1);
                self.remember_incoming(else_.0, &else_.1);

                self.builder.build_conditional_branch(condition, then_target, else_target).unwrap();
            },
            TerminatorInstruction::Switch { int_value, cases, else_, end: _ } => {
                let int_value = self.lookup_value(int_value).into_int_value();

                let cases = mapvec(cases.iter(), |(case_value, target)| {
                    let (case_block, case_args) = target;
                    self.remember_incoming(*case_block, case_args);
                    let case_block = self.blocks[*case_block];
                    let int_value = int_value.get_type().const_int(*case_value as u64, false);
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
                let value = self.lookup_value(value);
                self.builder.build_return(Some(&value)).unwrap();
            },
            TerminatorInstruction::Result(_) => {
                unreachable!("Result terminator encountered during function codegen")
            },
        }
    }
}
