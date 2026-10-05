use std::sync::Arc;

use inkwell::{
    AddressSpace, FloatPredicate, IntPredicate,
    basic_block::BasicBlock,
    builder::Builder,
    module::{Linkage, Module},
    passes::PassBuilderOptions,
    targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine},
    types::{BasicType, BasicTypeEnum, IntType, StructType},
    values::{AggregateValue, BasicValue, BasicValueEnum, FunctionValue, GlobalValue, PhiValue, PointerValue},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    cli::{GenericsStrategy, OptLevel},
    codegen::{
        OverflowingIntOp,
        constant::{self, ConstantValue},
    },
    incremental::Db,
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    mir::{self, BlockId, DefinitionId, FloatConstant, InstructionId, PrimitiveType, TerminatorInstruction},
    parser::ids::TopLevelName,
    vecmap::VecMap,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodegenLlvmResult {
    pub objects: Vec<Arc<Vec<u8>>>,
}

pub fn initialize_native_target() {
    let config = InitializationConfig::default();
    Target::initialize_native(&config).unwrap();
}

pub fn codegen_llvm(
    compiler: &Db, show_time: bool, opt_level: OptLevel, generics: GenericsStrategy, emit_ir: bool,
    selected_main: Option<TopLevelName>,
) -> Option<CodegenLlvmResult> {
    // Whole-program for now; ideally `CodegenLlvmResult` could be split per item.
    // NOTE: Monomorphization being whole-program essentially prevents the above comment
    let mir = crate::timings::time_phase("Lowering generics", show_time, || mir::lower_generics(compiler, generics));
    crate::timings::time_phase("LLVM codegen", show_time, || {
        codegen_llvm_for_mir(&mir, opt_level, emit_ir, show_time, selected_main)
    })
}

/// LLVM IR generation & object emission on a monomorphized [mir::Mir].
/// If `emit_ir` is set, the textual IR is printed to stdout.
pub(crate) fn codegen_llvm_for_mir(
    mir: &mir::Mir, opt_level: OptLevel, emit_ir: bool, show_time: bool, selected_main: Option<TopLevelName>,
) -> Option<CodegenLlvmResult> {
    let main_id = super::resolve_main_id(selected_main);
    initialize_native_target();
    assert!(mir.externals.is_empty(), "All Mir compilation units should be linked");

    // Unoptimized builds are split into parallel modules, optimized builds are a single module
    // so LLVM can do whole-program optimizations.
    let partitions = if opt_level == OptLevel::O0 && !emit_ir { partition_count() } else { 1 };
    if partitions > 1 {
        let parts = partition(mir, partitions);
        let objects = crate::timings::time_phase("Object emission", show_time, || {
            parts.par_iter().map(|part| codegen_partition(mir, part, main_id)).collect()
        });
        return Some(CodegenLlvmResult { objects });
    }

    let name = &mir.definitions.iter().next().map_or("_", |(_, function)| &function.name);
    let llvm = inkwell::context::Context::create();
    let target_machine = native_target_machine(opt_level);
    let module = build_module(&llvm, mir, &target_machine, name, main_id, mir.definitions.keys().copied());

    if opt_level != OptLevel::O0 {
        module
            .module
            .run_passes(opt_level.as_passes_string(), &target_machine, PassBuilderOptions::create())
            .expect("LLVM pass pipeline failed");
    }

    if emit_ir {
        println!("{}", module.module.print_to_string().to_string_lossy());
    }

    let object = crate::timings::time_phase("Object emission", show_time, || emit_object(&module, &target_machine));
    Some(CodegenLlvmResult { objects: vec![object] })
}

/// Build a module defining only `ids`
fn build_module<'ctx>(
    llvm: &'ctx inkwell::context::Context, mir: &'ctx mir::Mir, target_machine: &TargetMachine, name: &str,
    main_id: Option<DefinitionId>, ids: impl IntoIterator<Item = DefinitionId>,
) -> ModuleContext<'ctx> {
    let mut module = ModuleContext::new(llvm, mir, target_machine, name, main_id);
    for id in ids {
        module.codegen_function(&mir.definitions[&id], id);
    }
    module.codegen_main_wrapper();

    if let Err(error) = module.module.verify() {
        module.module.print_to_stderr();
        eprintln!("llvm module failed to verify: {error}");
    }
    module
}

fn emit_object(module: &ModuleContext, target_machine: &TargetMachine) -> Arc<Vec<u8>> {
    let object =
        target_machine.write_to_memory_buffer(&module.module, FileType::Object).expect("Failed to emit object code");
    Arc::new(object.as_slice().to_vec())
}

fn partition_count() -> usize {
    rayon::current_num_threads().min(16)
}

/// Split the definitions into `count` groups of about the same number of instructions
fn partition(mir: &mir::Mir, count: usize) -> Vec<Vec<DefinitionId>> {
    let mut definitions = mapvec(&mir.definitions, |(id, definition)| (*id, definition.instructions.len() + 1));

    // Largest first so it balances well
    definitions.sort_unstable_by_key(|(id, size)| (std::cmp::Reverse(*size), id.0));
    let mut parts = vec![(0usize, Vec::new()); count];
    for (id, size) in definitions {
        let smallest = parts.iter_mut().min_by_key(|(total, _)| *total).unwrap();
        smallest.0 += size;
        smallest.1.push(id);
    }
    parts.into_iter().map(|(_, ids)| ids).filter(|ids| !ids.is_empty()).collect()
}

fn codegen_partition(mir: &mir::Mir, part: &[DefinitionId], main_id: Option<DefinitionId>) -> Arc<Vec<u8>> {
    let llvm = inkwell::context::Context::create();
    let name = format!("part{}", part[0]);
    let target_machine = native_target_machine(OptLevel::O0);
    let module = build_module(&llvm, mir, &target_machine, &name, main_id, part.iter().copied());
    emit_object(&module, &target_machine)
}

/// Link the given list of object code blobs into an executable.
/// Returns `true` if linking succeeded, `false` otherwise.
pub fn link(
    objects: Vec<Arc<Vec<u8>>>, binary_name: &str, show_time: bool, _opt_level: OptLevel,
    link_options: &super::LinkOptions,
) -> bool {
    let paths = crate::timings::time_phase("Object emission", show_time, || {
        assert!(!objects.is_empty(), "Expected at least one object to link");
        mapvec(objects.iter().enumerate(), |(index, object)| {
            let extension = if objects.len() == 1 { "o".to_string() } else { format!("{index}.o") };
            let path = std::path::Path::new(binary_name).with_extension(extension);
            std::fs::write(&path, object.as_slice()).expect("Failed to write object file");
            path.to_string_lossy().into_owned()
        })
    });

    crate::timings::time_phase("Linking", show_time, || super::link_with_cc(&paths, binary_name, link_options))
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

    /// A global holding a [ConstantValue::Reinterpret], which no constant of its type can express.
    /// Its bytes live in immutable static storage and each use loads them as `typ`.
    /// TODO: All globals should be in storage and having them be constants should only be an
    /// optimization
    Stored {
        pointer: PointerValue<'ctx>,
        typ: BasicTypeEnum<'ctx>,
        /// The initial element of `pointer`, may be a different LLVM type than `typ`
        initializer: BasicValueEnum<'ctx>,
    },
}

impl<'ctx> CodegenValue<'ctx> {
    /// This is `None` for [CodegenValue::Stored] which is loaded instead
    fn into_basic_value(self) -> Option<BasicValueEnum<'ctx>> {
        match self {
            CodegenValue::Function(function) => Some(function.as_global_value().as_pointer_value().into()),
            CodegenValue::Literal(value) => Some(value),
            CodegenValue::Stored { .. } => None,
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

    /// The entry-point selected for this binary. `None` for libraries with no `main`.
    main_id: Option<DefinitionId>,

    definitions: FxHashMap<DefinitionId, CodegenValue<'ctx>>,
    values: FxHashMap<mir::Value, BasicValueEnum<'ctx>>,

    /// The storage holding each global whose address was taken with [mir::Instruction::GlobalAddress]
    /// TODO: Should we store all globals and just rely on llvm to inline/remove those that never
    /// have their address taken?
    global_addresses: FxHashMap<DefinitionId, PointerValue<'ctx>>,

    /// See [Self::copy_function]
    copy_function: Option<FunctionValue<'ctx>>,

    /// Block arguments are added here to later insert them as PHI values.
    ///
    /// Maps merge_block to a vec of each incoming block along with the arguments it branches with.
    incoming: FxHashMap<BlockId, Vec<(BasicBlock<'ctx>, BasicValueEnum<'ctx>)>>,

    /// PHI nodes created for each block's parameter. Filled in after all blocks are processed,
    /// so that back edges from later-processed blocks are included.
    phi_nodes: FxHashMap<BlockId, PhiValue<'ctx>>,

    /// The integer type the size of a pointer, which is costly to look up
    pointer_sized_int: IntType<'ctx>,

    /// Positioned by [Self::entry_alloca] at the start of the current function
    alloca_builder: Builder<'ctx>,
}

impl<'ctx> ModuleContext<'ctx> {
    fn new(
        llvm: &'ctx inkwell::context::Context, mir: &'ctx mir::Mir, target_machine: &TargetMachine, name: &str,
        main_id: Option<DefinitionId>,
    ) -> Self {
        let module = llvm.create_module(name);
        module.set_triple(&target_machine.get_triple());
        let target_data = target_machine.get_target_data();
        module.set_data_layout(&target_data.get_data_layout());

        let pointer_sized_int = llvm.ptr_sized_int_type(&target_data, None);
        Self {
            pointer_sized_int,
            llvm,
            module,
            mir,
            current_function: None,
            current_function_value: None,
            ante_main: None,
            main_id,
            definitions: Default::default(),
            values: Default::default(),
            global_addresses: Default::default(),
            copy_function: None,
            builder: llvm.create_builder(),
            alloca_builder: llvm.create_builder(),
            blocks: Default::default(),
            incoming: Default::default(),
            phi_nodes: Default::default(),
        }
    }

    fn ptr_size(&self) -> u32 {
        self.pointer_sized_int.get_bit_width() / 8
    }

    fn codegen_global(&mut self, global: &mir::Definition, id: mir::DefinitionId) {
        let value = constant::evaluate_global(self.mir, global, self.ptr_size());
        let codegen_value = if self.needs_stored_value(&value) {
            let image = self.lower_constant_image(&value, &global.typ);
            let storage = self.private_constant(&image, &format!("{id}_stored"));

            storage.set_alignment(global.typ.align_in_bytes(self.ptr_size()));
            CodegenValue::Stored {
                pointer: storage.as_pointer_value(),
                typ: self.convert_type(&global.typ),
                initializer: image,
            }
        } else {
            CodegenValue::Literal(self.lower_constant(&value))
        };
        self.definitions.insert(id, codegen_value);
    }

    /// True if `value` holds bytes no constant of its own type can express
    fn needs_stored_value(&mut self, value: &ConstantValue) -> bool {
        match value {
            ConstantValue::Reinterpret { .. } => true,
            ConstantValue::Tuple(values) => values.iter().any(|value| self.needs_stored_value(value)),
            ConstantValue::Array { elements, .. } => elements.iter().any(|value| self.needs_stored_value(value)),
            ConstantValue::Definition(id) => matches!(self.codegen_value_for(*id), CodegenValue::Stored { .. }),
            _ => false,
        }
    }

    /// Render `value` of type `typ` into a constant with the same bytes as `typ`'s layout, whose
    /// LLVM type differs from `typ`'s if [Self::needs_stored_value]
    fn lower_constant_image(&mut self, value: &ConstantValue, typ: &mir::Type) -> BasicValueEnum<'ctx> {
        if !self.needs_stored_value(value) {
            return self.lower_constant(value);
        }
        let ptr_size = self.ptr_size();
        let size = typ.size_in_bytes(ptr_size);
        match (value, typ) {
            (ConstantValue::Reinterpret { value, from, .. }, _) => {
                let image = self.lower_constant_image(value, from);
                let from_size = from.size_in_bytes(ptr_size);
                self.packed_image(vec![(0, from_size, image)], size.max(from_size))
            },
            (ConstantValue::Tuple(values), mir::Type::Tuple(fields)) => {
                let offsets = mir::Type::field_offsets(fields, ptr_size);
                let parts = values.iter().zip(fields.iter()).zip(offsets);
                let parts = mapvec(parts, |((value, field), offset)| {
                    (offset, field.size_in_bytes(ptr_size), self.lower_constant_image(value, field))
                });
                self.packed_image(parts, size)
            },
            (ConstantValue::Array { elements, .. }, mir::Type::Array { element, .. }) => {
                let stride = element.stride(ptr_size);
                let element_size = element.size_in_bytes(ptr_size);
                let parts = mapvec(elements.iter().enumerate(), |(i, value)| {
                    (i as u32 * stride, element_size, self.lower_constant_image(value, element))
                });
                self.packed_image(parts, size)
            },
            (ConstantValue::Definition(id), _) => match self.codegen_value_for(*id) {
                CodegenValue::Stored { initializer: image, .. } => image,
                _ => unreachable!("needs_image only holds for a stored global"),
            },
            (value, typ) => unreachable!("needs_image holds for {value:?} of type `{typ}`"),
        }
    }

    /// A packed struct placing each `(offset, size, part)` at its offset, zero-padded to `size`
    fn packed_image(&self, parts: Vec<(u32, u32, BasicValueEnum<'ctx>)>, size: u32) -> BasicValueEnum<'ctx> {
        let mut fields = Vec::new();
        let mut end = 0;
        for (offset, part_size, part) in parts {
            if offset > end {
                fields.push(self.llvm.i8_type().array_type(offset - end).const_zero().into());
            }
            fields.push(part);
            end = offset + part_size;
        }
        if size > end {
            fields.push(self.llvm.i8_type().array_type(size - end).const_zero().into());
        }
        self.llvm.const_struct(&fields, true).into()
    }

    /// The value of the definition `id` where it is used within a function
    fn definition_value(&mut self, id: DefinitionId) -> BasicValueEnum<'ctx> {
        match self.codegen_value_for(id) {
            CodegenValue::Stored { pointer, typ, .. } => self.builder.build_load(typ, pointer, "").unwrap(),
            value => value.into_basic_value().unwrap(),
        }
    }

    /// Render a folded [ConstantValue] into an inkwell constant
    fn lower_constant(&mut self, value: &ConstantValue) -> BasicValueEnum<'ctx> {
        match value {
            ConstantValue::Unit => self.unit_value(),
            ConstantValue::Bool(b) => self.llvm.bool_type().const_int(*b as u64, false).into(),
            ConstantValue::Char(c) => self.llvm.i8_type().const_int(*c as u64, false).into(),
            ConstantValue::Int(constant) => {
                let kind = constant.kind();
                self.convert_integer_kind(kind).const_int(constant.as_u64(), kind.is_signed()).into()
            },
            ConstantValue::Float(FloatConstant::F32(v)) => self.llvm.f32_type().const_float(v.0).into(),
            ConstantValue::Float(FloatConstant::F64(v)) => self.llvm.f64_type().const_float(v.0).into(),
            ConstantValue::Tuple(values) if values.is_empty() => self.unit_value(),
            ConstantValue::Tuple(values) => {
                let fields = mapvec(values, |v| self.lower_constant(v));
                self.llvm.const_struct(&fields, false).into()
            },
            ConstantValue::Array { elements, element_type } => {
                let values = mapvec(elements, |v| self.lower_constant(v));
                let array_type = self.convert_type(element_type).array_type(elements.len() as u32);
                Self::const_array_of(array_type, &values).into()
            },
            ConstantValue::Bytes(bytes) => {
                let byte_values = mapvec(bytes, |b| self.llvm.i8_type().const_int(*b as u64, false));
                let array = self.llvm.i8_type().const_array(&byte_values);
                self.private_constant(&array, "__bytes").as_pointer_value().into()
            },
            ConstantValue::Definition(id) => self
                .codegen_value_for(*id)
                .into_basic_value()
                .expect("a stored global is lowered with lower_constant_image"),
            ConstantValue::Extern { name, typ } => match self.convert_function_type(typ) {
                Some(fn_type) => {
                    let fn_val =
                        self.module.get_function(name).unwrap_or_else(|| self.module.add_function(name, fn_type, None));
                    fn_val.as_global_value().as_pointer_value().into()
                },
                None => {
                    let global = self
                        .module
                        .get_global(name)
                        .unwrap_or_else(|| self.module.add_global(self.convert_type(typ), None, name));
                    global.as_pointer_value().into()
                },
            },
            ConstantValue::Shared { value, typ, cell } => {
                // No malloc in a constant initializer, so back the value with a global instead.
                // Every module referencing the cell builds this backing under the same name, so
                // the linker merges them into one value.
                let name = format!("{}_shared_{}", cell.global, cell.index);
                if let Some(backing) = self.module.get_global(&name) {
                    return backing.as_pointer_value().into();
                }
                let init_value = self.lower_constant_image(value, typ);
                let backing = self.module.add_global(init_value.get_type(), None, &name);
                backing.set_alignment(typ.align_in_bytes(self.ptr_size()));
                backing.set_linkage(Linkage::WeakAny);
                backing.set_initializer(&init_value);
                backing.as_pointer_value().into()
            },
            ConstantValue::Zeroed { typ } => self.convert_type(typ).const_zero(),
            ConstantValue::IntToPtr { bits, typ } => {
                let pointer_type = self.convert_type(typ).into_pointer_type();
                self.pointer_sized_int.const_int(*bits, false).const_to_pointer(pointer_type).into()
            },
            ConstantValue::PtrToInt { pointer, kind } => {
                let pointer = self.lower_constant(pointer).into_pointer_value();
                pointer.const_to_int(self.convert_integer_kind(*kind)).into()
            },
            ConstantValue::Reinterpret { .. } => {
                unreachable!("a global holding a Reinterpret is lowered with lower_constant_image")
            },
        }
    }

    fn codegen_function(&mut self, function: &mir::Definition, id: mir::DefinitionId) {
        if function.is_global() {
            self.codegen_global(function, id);
            return;
        }

        let is_ante_main = self.main_id == Some(id);
        let function_value = match self.definitions.get(&id) {
            Some(CodegenValue::Function(fv)) => *fv,
            Some(CodegenValue::Literal(_) | CodegenValue::Stored { .. }) => panic!(
                "codegen_function: definition {id} was already codegen'd as a literal global, but its body is a function"
            ),
            None => {
                let function_type = self.convert_function_type(&function.typ).unwrap();
                let function_value = self.module.add_function(&self.function_name(id), function_type, None);
                self.definitions.insert(id, CodegenValue::Function(function_value));
                function_value
            },
        };

        if is_ante_main {
            self.ante_main = Some(function_value);
        }

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

    /// Emit a `main (argc, argv): I32` wrapper around the source `main` that
    /// stashes the OS-supplied argc/argv into module-level globals (so
    /// `Std.Env.args` can read them later via the accessor functions defined
    /// below) and then calls the user's main, returning 0.
    fn codegen_main_wrapper(&mut self) {
        let Some(ante_main) = self.ante_main else { return };

        let i32_type = self.llvm.i32_type();
        let ptr_type = self.llvm.ptr_type(AddressSpace::default());
        let unit_type = self.convert_primitive_type(PrimitiveType::Unit).into_struct_type();

        // Module-local globals holding argc/argv for the lifetime of the process.
        let argc_global = self.module.add_global(i32_type, None, "ante_argc");
        argc_global.set_initializer(&i32_type.const_zero());
        argc_global.set_linkage(Linkage::Private);

        let argv_global = self.module.add_global(ptr_type, None, "ante_argv");
        argv_global.set_initializer(&ptr_type.const_null());
        argv_global.set_linkage(Linkage::Private);

        // Accessors used by `Std.Env`. If the program imported `Env.args`,
        // codegen has already declared `ante_get_argc` / `ante_get_argv` as
        // externs (signature `fn (Unit) -> X` ~ `i32 ({})` / `ptr ({})`).
        // Reuse those declarations so the names line up; otherwise add fresh
        // declarations with the matching signature.
        let getc = self.module.get_function("ante_get_argc").unwrap_or_else(|| {
            self.module.add_function("ante_get_argc", i32_type.fn_type(&[unit_type.into()], false), None)
        });
        let bb = self.llvm.append_basic_block(getc, "");
        self.builder.position_at_end(bb);
        let v = self.builder.build_load(i32_type, argc_global.as_pointer_value(), "").unwrap();
        self.builder.build_return(Some(&v)).unwrap();

        let getv = self.module.get_function("ante_get_argv").unwrap_or_else(|| {
            self.module.add_function("ante_get_argv", ptr_type.fn_type(&[unit_type.into()], false), None)
        });
        let bb = self.llvm.append_basic_block(getv, "");
        self.builder.position_at_end(bb);
        let v = self.builder.build_load(ptr_type, argv_global.as_pointer_value(), "").unwrap();
        self.builder.build_return(Some(&v)).unwrap();

        // The C-callable main: (i32, i8**) -> i32.
        let wrapper_type = i32_type.fn_type(&[i32_type.into(), ptr_type.into()], false);
        let wrapper = self.module.add_function("main", wrapper_type, None);

        let entry = self.llvm.append_basic_block(wrapper, "");
        self.builder.position_at_end(entry);

        let argc = wrapper.get_nth_param(0).unwrap();
        let argv = wrapper.get_nth_param(1).unwrap();
        self.builder.build_store(argc_global.as_pointer_value(), argc).unwrap();
        self.builder.build_store(argv_global.as_pointer_value(), argv).unwrap();

        // Pass a zeroed value for each of `main`'s parameters
        let arguments = mapvec(ante_main.get_param_iter(), |parameter| parameter.get_type().const_zero().into());
        self.builder.build_direct_call(ante_main, &arguments, "").unwrap();
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
            mir::Type::Tuple(fields) if fields.is_empty() => self.unit_type().into(),
            mir::Type::Tuple(fields) => {
                let fields = mapvec(fields.iter(), |typ| self.convert_type(typ));
                let struct_type = self.llvm.struct_type(&fields, false);
                BasicTypeEnum::StructType(struct_type)
            },
            // After `lower_closures`, every Function type is a raw fn ptr; closures
            // are explicit Tuples handled by the `Tuple` arm above.
            mir::Type::Function(_) => self.llvm.ptr_type(AddressSpace::default()).into(),
            mir::Type::Union(_) => self.llvm.ptr_type(AddressSpace::default()).into(),
            mir::Type::Array { length, element } => {
                let length = match length.as_ref() {
                    mir::Type::U32(n) => *n,
                    other => panic!("LLVM codegen: Array with non-constant length {other}"),
                };
                self.convert_type(element).array_type(length).into()
            },
            mir::Type::U32(_) => self.llvm.struct_type(&[], false).into(),
            mir::Type::Generic(_) => self.llvm.ptr_type(AddressSpace::default()).into(),
            mir::Type::Evidence(_) => unreachable!("evidence is lowered before codegen"),
        }
    }

    fn convert_primitive_type(&self, primitive_type: PrimitiveType) -> BasicTypeEnum<'ctx> {
        match primitive_type {
            PrimitiveType::Error => unreachable!("Cannot codegen llvm with errors"),
            PrimitiveType::Unit => self.unit_type().into(),
            PrimitiveType::Bool => self.llvm.bool_type().into(),
            PrimitiveType::Pointer => self.llvm.ptr_type(AddressSpace::default()).into(),
            PrimitiveType::Char => self.llvm.i8_type().into(),
            PrimitiveType::Int(kind) => self.convert_integer_kind(kind).into(),
            PrimitiveType::Float(FloatKind::F32) => self.llvm.f32_type().into(),
            PrimitiveType::Float(FloatKind::F64) => self.llvm.f64_type().into(),
            PrimitiveType::NoClosureEnv => unreachable!("Cannot convert NoClosureEnv"),
        }
    }

    fn convert_integer_kind(&self, kind: IntegerKind) -> IntType<'ctx> {
        match kind {
            IntegerKind::I8 | IntegerKind::U8 => self.llvm.i8_type(),
            IntegerKind::I16 | IntegerKind::U16 => self.llvm.i16_type(),
            IntegerKind::I32 | IntegerKind::U32 => self.llvm.i32_type(),
            IntegerKind::I64 | IntegerKind::U64 => self.llvm.i64_type(),
            IntegerKind::Isz | IntegerKind::Usz => self.pointer_sized_int,
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
        let parameters = mapvec(&function_type.parameters, |parameter| self.convert_type(parameter).into());
        Some(return_type.fn_type(&parameters, false))
    }

    /// Returns the name of the given [DefinitionId].
    /// As long as the [DefinitionId] is referenced in `self.mir`, this should never panic.
    fn get_name(&self, id: DefinitionId) -> &'ctx str {
        self.mir.get_name(id).unwrap().as_ref()
    }

    fn function_name(&self, id: DefinitionId) -> String {
        // See [Self::codegen_main_wrapper]
        if self.main_id == Some(id) { format!("main_{id}%") } else { format!("{}_{id}", self.get_name(id)) }
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
            // Forward-declare the function with the mangled name `codegen_function`
            // to avoid colliding with C extern names.
            let fn_type = self
                .convert_function_type(&def.typ)
                .expect("codegen_value_for: non-global definition must have a function type");
            let fv = self.module.add_function(&self.function_name(id), fn_type, None);
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
                *self.values.get(value).unwrap_or_else(|| panic!("llvm codegen: mir value is not cached: {value}"))
            },
            mir::Value::Definition(id) => self.definition_value(*id),
        }
    }

    /// A module-local constant holding `value`, whose address is not significant
    fn private_constant(&self, value: &dyn BasicValue<'ctx>, name: &str) -> GlobalValue<'ctx> {
        let value = value.as_basic_value_enum();
        let global = self.module.add_global(value.get_type(), None, name);
        global.set_initializer(&value);
        global.set_constant(true);
        global.set_linkage(Linkage::Private);
        global.set_unnamed_addr(true);
        global
    }

    /// A function copying a runtime number of bytes, which handles the common small sizes itself
    /// rather than calling `memmove`. This is primarily used to speed up compilation of code
    /// lowered via existentialization.
    /// TODO: Refactor, clean up
    fn copy_function(&mut self, size_type: inkwell::types::IntType<'ctx>) -> FunctionValue<'ctx> {
        if let Some(function) = self.copy_function {
            return function;
        }
        let ptr_type = self.llvm.ptr_type(AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), size_type.into()], false);
        let function = self.module.add_function("ante_copy", fn_type, Some(Linkage::Private));
        self.copy_function = Some(function);

        let caller_block = self.builder.get_insert_block();
        let destination = function.get_nth_param(0).unwrap().into_pointer_value();
        let source = function.get_nth_param(1).unwrap().into_pointer_value();
        let size = function.get_nth_param(2).unwrap().into_int_value();
        let entry = self.llvm.append_basic_block(function, "");
        self.builder.position_at_end(entry);

        // Every load comes before any store, so overlapping copies work
        let (i8_type, i64_type) = (self.llvm.i8_type(), self.llvm.i64_type());
        let byte_at = |base, offset| unsafe {
            self.builder.build_in_bounds_gep(i8_type, base, &[i64_type.const_int(offset, false)], "").unwrap()
        };
        for bytes in [8u64, 16, 24, 4, 1, 2] {
            let width = bytes.min(8);
            let word_type = self.llvm.custom_width_int_type(std::num::NonZero::new(width as u32 * 8).unwrap()).unwrap();
            let words = mapvec((0..bytes).step_by(width as usize), |offset| (word_type, offset));
            let matched = self.llvm.append_basic_block(function, "");
            let next = self.llvm.append_basic_block(function, "");
            let expected = size_type.const_int(bytes, false);
            let equal = self.builder.build_int_compare(inkwell::IntPredicate::EQ, size, expected, "").unwrap();
            self.builder.build_conditional_branch(equal, matched, next).unwrap();

            self.builder.position_at_end(matched);
            let loaded = mapvec(&words, |(typ, offset)| {
                let load = self.builder.build_load(*typ, byte_at(source, *offset), "").unwrap();
                load.as_instruction_value().unwrap().set_alignment(1).unwrap();
                load
            });
            for ((_, offset), value) in words.iter().zip(loaded) {
                self.builder.build_store(byte_at(destination, *offset), value).unwrap().set_alignment(1).unwrap();
            }
            self.builder.build_return(Some(&destination)).unwrap();
            self.builder.position_at_end(next);
        }

        // LLVM's FastISel handles a plain call but not the intrinsic with a runtime size
        let memmove =
            self.module.get_function("memmove").unwrap_or_else(|| self.module.add_function("memmove", fn_type, None));
        let arguments = [destination.into(), source.into(), size.into()];
        let function_pointer = memmove.as_global_value().as_pointer_value();
        self.builder.build_indirect_call(fn_type, function_pointer, &arguments, "").unwrap();
        self.builder.build_return(Some(&destination)).unwrap();

        if let Some(block) = caller_block {
            self.builder.position_at_end(block);
        }
        function
    }

    fn unit_type(&self) -> StructType<'ctx> {
        self.llvm.struct_type(&[self.llvm.i8_type().into()], false)
    }

    fn unit_value(&self) -> BasicValueEnum<'ctx> {
        self.unit_type().const_zero().into()
    }

    fn codegen_instruction(&mut self, function: &mir::Definition, id: mir::InstructionId) {
        let result = match &function.instructions[id] {
            mir::Instruction::Call { function: function_value, arguments } => {
                let fn_type = self.mir.type_of_value(function_value, function);
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
            mir::Instruction::Capability => {
                unreachable!("Instruction::Capability remaining in LLVM codegen")
            },
            mir::Instruction::LookupEvidence { .. } | mir::Instruction::MakeEvidence { .. } => {
                unreachable!("evidence instruction remaining in LLVM codegen")
            },
            mir::Instruction::CallClosure { .. } => {
                unreachable!("Instruction::CallClosure remaining in LLVM codegen")
            },
            mir::Instruction::PackClosure { .. } => {
                unreachable!("Instruction::PackClosure remaining in LLVM codegen")
            },
            mir::Instruction::IndexTuple { tuple, index } => {
                let tuple = self.lookup_value(tuple).into_struct_value();
                self.builder.build_extract_value(tuple, *index, "").unwrap()
            },
            mir::Instruction::MakeBytes(bytes) => {
                let bytes_data = self.llvm.const_string(bytes, false);
                // Llvm doesn't rename across modules so we mangle this with the current function id.
                let name = format!("{}_bytes", self.current_function.unwrap());
                let global = self.module.add_global(bytes_data.get_type(), None, &name);
                global.set_initializer(&bytes_data);
                global.as_pointer_value().into()
            },
            mir::Instruction::MakeTuple(fields) => self.make_tuple(fields),
            mir::Instruction::MakeArray(elements) => {
                let result_type = self.convert_type(function.instruction_result_type(id)).into_array_type();
                self.make_array(result_type, elements)
            },
            mir::Instruction::StackAlloc(value) => {
                let value = self.lookup_value(value);
                let alloca = self.entry_alloca(value.get_type());
                self.builder.build_store(alloca, value).unwrap();
                alloca.into()
            },
            mir::Instruction::StackAllocUninit(typ) => {
                let typ = self.convert_type(typ);
                self.entry_alloca(typ).into()
            },
            mir::Instruction::StackAllocBytes(size) => {
                let size = self.lookup_value(size).into_int_value();
                let alloca = self.builder.build_array_alloca(self.llvm.i8_type(), size, "").unwrap();
                alloca.as_instruction().unwrap().set_alignment(mir::MAX_ALIGNMENT).unwrap();
                alloca.into()
            },
            mir::Instruction::GlobalAddress(global) => match self.global_addresses.get(global) {
                Some(address) => (*address).into(),
                None => {
                    let address = match self.codegen_value_for(*global) {
                        CodegenValue::Stored { pointer, .. } => pointer,
                        _ => {
                            let value = self.definition_value(*global);
                            self.private_constant(&value, &format!("{global}_address")).as_pointer_value()
                        },
                    };
                    self.global_addresses.insert(*global, address);
                    address.into()
                },
            },
            mir::Instruction::MemCopy { destination, source, size } => {
                let destination = self.lookup_value(destination).into_pointer_value();
                let source = self.lookup_value(source).into_pointer_value();
                let size = self.lookup_value(size).into_int_value();
                if size.is_const() {
                    // This makes existentialized code a bit faster to compile
                    self.builder.build_memcpy(destination, 1, source, 1, size).unwrap();
                } else {
                    let copy = self.copy_function(size.get_type());
                    let arguments = [destination.into(), source.into(), size.into()];
                    self.builder.build_direct_call(copy, &arguments, "").unwrap();
                }
                self.unit_value()
            },
            mir::Instruction::PointerOffset { pointer, offset } => {
                let pointer = self.lookup_value(pointer).into_pointer_value();
                let offset = self.lookup_value(offset).into_int_value();
                let i8_type = self.llvm.i8_type();
                unsafe { self.builder.build_in_bounds_gep(i8_type, pointer, &[offset], "").unwrap().into() }
            },
            mir::Instruction::AllocShared(value) => {
                let value = self.lookup_value(value);
                let ptr = self.builder.build_malloc(value.get_type(), "").unwrap();
                self.builder.build_store(ptr, value).unwrap();
                ptr.into()
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
            mir::Instruction::OverflowingAddInt(a, b) => self.overflowing_int_op(function, a, b, OverflowingIntOp::Add),
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
            mir::Instruction::OverflowingSubInt(a, b) => self.overflowing_int_op(function, a, b, OverflowingIntOp::Sub),
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
            mir::Instruction::OverflowingMulInt(a, b) => self.overflowing_int_op(function, a, b, OverflowingIntOp::Mul),
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
            mir::Instruction::ArrayLen(_) => todo!("ArrayLen should be removed by monomorphization"),
            mir::Instruction::AtomicLoad { pointer, ordering } => {
                // An atomic load is expressed as `atomicrmw or ptr, 0`, which reads and returns
                // the current value. This avoids the separate-instruction alignment/ordering setup.
                let ptr = self.lookup_value(pointer).into_pointer_value();
                let result_type = self.convert_type(function.instruction_result_type(id)).into_int_type();
                let zero = result_type.const_zero();
                self.builder
                    .build_atomicrmw(inkwell::AtomicRMWBinOp::Or, ptr, zero, to_llvm_ordering(*ordering))
                    .unwrap()
                    .as_basic_value_enum()
            },
            mir::Instruction::AtomicStore { pointer, value, ordering } => {
                // An atomic store is expressed as `atomicrmw xchg`, discarding the previous value.
                let ptr = self.lookup_value(pointer).into_pointer_value();
                let value = self.lookup_value(value).into_int_value();
                self.builder
                    .build_atomicrmw(inkwell::AtomicRMWBinOp::Xchg, ptr, value, to_llvm_ordering(*ordering))
                    .unwrap();
                self.unit_value()
            },
            mir::Instruction::AtomicRmw { op, pointer, value, ordering } => {
                let ptr = self.lookup_value(pointer).into_pointer_value();
                let value = self.lookup_value(value).into_int_value();
                self.builder
                    .build_atomicrmw(to_llvm_rmw(*op), ptr, value, to_llvm_ordering(*ordering))
                    .unwrap()
                    .as_basic_value_enum()
            },
            mir::Instruction::AtomicCmpxchg { pointer, expected, desired, success, failure } => {
                let ptr = self.lookup_value(pointer).into_pointer_value();
                let expected = self.lookup_value(expected).into_int_value();
                let desired = self.lookup_value(desired).into_int_value();
                let result = self
                    .builder
                    .build_cmpxchg(ptr, expected, desired, to_llvm_ordering(*success), to_llvm_ordering(*failure))
                    .unwrap();
                // cmpxchg yields `{ old_value, success_flag }`; return the previous value.
                self.builder.build_extract_value(result, 0, "").unwrap()
            },
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

    fn overflowing_int_op(
        &mut self, function: &mir::Definition, a: &mir::Value, b: &mir::Value, op: OverflowingIntOp,
    ) -> BasicValueEnum<'ctx> {
        let a_type = function.type_of_value(a, &self.mir.externals, &self.mir.definitions);
        let signedness = match a_type {
            mir::Type::Primitive(mir::PrimitiveType::Int(kind)) if kind.is_signed() => "s",
            mir::Type::Primitive(mir::PrimitiveType::Int(_)) => "u",
            other => panic!("overflowing_int_op expected integer operand, got {other}"),
        };

        let a = self.lookup_value(a).into_int_value();
        let b = self.lookup_value(b).into_int_value();
        let int_type = a.get_type();
        let overflow_type = self.llvm.struct_type(&[int_type.into(), self.llvm.bool_type().into()], false);
        let fn_type = overflow_type.fn_type(&[int_type.into(), int_type.into()], false);
        let intrinsic_name =
            format!("llvm.{signedness}{}.with.overflow.i{}", op.llvm_name_part(), int_type.get_bit_width());
        let intrinsic = self
            .module
            .get_function(&intrinsic_name)
            .unwrap_or_else(|| self.module.add_function(&intrinsic_name, fn_type, None));

        self.builder.build_call(intrinsic, &[a.into(), b.into()], "").unwrap().try_as_basic_value().basic().unwrap()
    }

    fn transmute(&mut self, value: &mir::Value, function: &mir::Definition, id: InstructionId) -> BasicValueEnum<'ctx> {
        let result_type = self.convert_type(function.instruction_result_type(id));
        let value = self.lookup_value(value);
        let alloca = self.entry_alloca(value.get_type());
        self.builder.build_store(alloca, value).unwrap();
        self.builder.build_load(result_type, alloca, "").unwrap()
    }

    /// An alloca in the entry block, since one elsewhere allocates more stack each time it runs
    fn entry_alloca(&self, typ: BasicTypeEnum<'ctx>) -> inkwell::values::PointerValue<'ctx> {
        let entry = self.current_function_value.unwrap().get_first_basic_block().unwrap();
        match entry.get_first_instruction() {
            Some(first) => self.alloca_builder.position_before(&first),
            None => self.alloca_builder.position_at_end(entry),
        }
        self.alloca_builder.build_alloca(typ, "").unwrap()
    }

    fn make_tuple(&mut self, fields: &[mir::Value]) -> BasicValueEnum<'ctx> {
        let fields = mapvec(fields, |field| self.lookup_value(field));
        if fields.is_empty() {
            return self.unit_value();
        }
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

    fn make_array(
        &mut self, array_type: inkwell::types::ArrayType<'ctx>, elements: &[mir::Value],
    ) -> BasicValueEnum<'ctx> {
        let element_type = array_type.get_element_type();
        let values = mapvec(elements, |e| self.lookup_value(e));
        let seed = mapvec(&values, |v| if v.is_const() { *v } else { Self::undef_value(element_type) });
        let mut array = Self::const_array_of(array_type, &seed).as_aggregate_value_enum();

        for (i, value) in values.into_iter().enumerate() {
            if !value.is_const() {
                array = self.builder.build_insert_value(array, value, i as u32, "").unwrap();
            }
        }
        array.as_basic_value_enum()
    }

    /// Build an LLVM constant array of `array_type` with the given element values. Inkwell's
    /// `const_array` is type-specific, so we dispatch on the element type.
    fn const_array_of(
        array_type: inkwell::types::ArrayType<'ctx>, elements: &[BasicValueEnum<'ctx>],
    ) -> inkwell::values::ArrayValue<'ctx> {
        let element_type = array_type.get_element_type();
        match element_type {
            BasicTypeEnum::IntType(t) => {
                let vals: Vec<_> = elements.iter().map(|e| e.into_int_value()).collect();
                t.const_array(&vals)
            },
            BasicTypeEnum::FloatType(t) => {
                let vals: Vec<_> = elements.iter().map(|e| e.into_float_value()).collect();
                t.const_array(&vals)
            },
            BasicTypeEnum::PointerType(t) => {
                let vals: Vec<_> = elements.iter().map(|e| e.into_pointer_value()).collect();
                t.const_array(&vals)
            },
            BasicTypeEnum::StructType(t) => {
                let vals: Vec<_> = elements.iter().map(|e| e.into_struct_value()).collect();
                t.const_array(&vals)
            },
            BasicTypeEnum::ArrayType(t) => {
                let vals: Vec<_> = elements.iter().map(|e| e.into_array_value()).collect();
                t.const_array(&vals)
            },
            BasicTypeEnum::VectorType(t) => {
                let vals: Vec<_> = elements.iter().map(|e| e.into_vector_value()).collect();
                t.const_array(&vals)
            },
            BasicTypeEnum::ScalableVectorType(t) => {
                let vals: Vec<_> = elements.iter().map(|e| e.into_scalable_vector_value()).collect();
                t.const_array(&vals)
            },
        }
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

                let (else_block, else_args) = else_;
                self.remember_incoming(*else_block, else_args);
                let else_block = self.blocks[*else_block];

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

fn to_llvm_ordering(ordering: crate::mir::AtomicOrdering) -> inkwell::AtomicOrdering {
    use crate::mir::AtomicOrdering as O;
    match ordering {
        O::Relaxed => inkwell::AtomicOrdering::Monotonic,
        O::Acquire => inkwell::AtomicOrdering::Acquire,
        O::Release => inkwell::AtomicOrdering::Release,
        O::AcqRel => inkwell::AtomicOrdering::AcquireRelease,
        O::SeqCst => inkwell::AtomicOrdering::SequentiallyConsistent,
    }
}

fn to_llvm_rmw(op: crate::mir::AtomicRmwOp) -> inkwell::AtomicRMWBinOp {
    use crate::mir::AtomicRmwOp as Op;
    match op {
        Op::Xchg => inkwell::AtomicRMWBinOp::Xchg,
        Op::Add => inkwell::AtomicRMWBinOp::Add,
        Op::Sub => inkwell::AtomicRMWBinOp::Sub,
        Op::And => inkwell::AtomicRMWBinOp::And,
        Op::Or => inkwell::AtomicRMWBinOp::Or,
        Op::Xor => inkwell::AtomicRMWBinOp::Xor,
    }
}
