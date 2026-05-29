//! This file contains the bulk of the logic for translating Ante's [mir::Mir] IR
//! into C code which can then be compiled & executed by `cc`. For this to be valid,
//! the input [mir::Mir] must be at the end of its pipeline: generics must be removed
//! via either monomorphization or existentialization, largest union variants must be
//! selected, effects must be lowered, etc. See the various [mir::Mir] passes for details.
//!
//! Creating C output is fairly straightforward:
//! - [Builder::build_definition] is called on each definition in the mir to translate
//!   it into a single c function.
//! - The resulting [CFile] artifact is separated into sections so functions and types
//!   can be declared before their first use.
//! - This pass is parallelized by [build_c_file] very simply: for N workers, each worker
//!   compiles definitions with ids given by `id % n = i` where `i` is the worker index.
//! - The entry function [codegen_c_for_mir] will compile & link the resulting C file.
use std::{
    borrow::Cow,
    fmt::Write as _,
    process::Command,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use dashmap::{mapref::entry::Entry, DashMap};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{cli::OptLevel, mir::{self, DefinitionId, InstructionId}};

mod cfile;
use cfile::CFile;

/// A concurrent cache mapping each distinct tuple type to a stable id and its generated C
/// `struct` definition. Cloning shares the same underlying maps, so every [Builder] working in
/// parallel resolves a given tuple structure to the same `TupleN` name and emits its definition
/// exactly once.
#[derive(Default, Clone)]
struct TupleCache {
    types: Arc<DashMap<Arc<Vec<mir::Type>>, (u32, String)>>,
    next_id: Arc<AtomicU32>,
}

/// Codegen the given Mir into a single C file, then invoke cc to create
/// a object file and a binary. On success, the object file is removed, but
/// the .c file is kept.
pub(crate) fn codegen_c_for_mir(mir: &mir::Mir, binary_name: &str, opt_level: OptLevel) {
    // Create the C file
    let c_file = build_c_file(mir);
    let c_file_name = format!("{binary_name}.c");
    std::fs::write(&c_file_name, c_file).unwrap();

    // Create the .o file
    let o_file_name = format!("{binary_name}.o");
    let mut child = Command::new("cc")
        .arg(&c_file_name)
        .arg(&format!("-o{o_file_name}"))
        .arg(opt_level.as_cc_opt_string())
        .arg("-c")
        .arg("-w")
        .spawn()
        .unwrap();

    // And link it into a binary
    let status = child.wait().unwrap();
    if status.success() {
        super::link_with_cc(&o_file_name, binary_name);
    }
}

/// The main context struct to build a [CFile] from [mir::Mir]
#[derive(Default)]
struct Builder {
    file: CFile,

    /// The current item being worked on - either a type or a function.
    /// This will be appended to the appropriate position in `file` when finished.
    current_item: String,

    /// Shared across all builders so the same tuple type resolves to the same C type.
    tuples: TupleCache,
}

/// Builds a C File for the given [mir::Mir] in-memory. Returns the file contents
fn build_c_file(mir: &mir::Mir) -> String {
    // Split Mir definitions into N groups and compile in parallel.
    // Each worker `i` compiles definitions with id `Id % N = i`
    let n = rayon::current_num_threads() as u32;

    // One cache shared by every worker so a tuple type from any of them is named consistently.
    let tuples = TupleCache::default();

    let mut file = (0..n)
        .into_par_iter()
        .map(|i| c_file_with_definitions_subset(mir, n, i, tuples.clone()))
        .reduce(CFile::default, CFile::extend);

    // Emit tuple structs in id order. Inner tuples are registered while generating the body of
    // the tuples that embed them, so they receive smaller ids and are defined first as C requires.
    let mut definitions: Vec<_> = tuples.types.iter().map(|entry| entry.value().clone()).collect();
    definitions.sort_by_key(|(id, _)| *id);
    for (_, definition) in definitions {
        file.add_type_definition(&definition);
    }

    file.add_starter_items().into_contents()
}

/// Create a C file with only definitions of the mir with ids such that `id % n = i`.
/// This is meant to distribute work over `n` workers evenly.
fn c_file_with_definitions_subset(mir: &mir::Mir, n: u32, i: u32, tuples: TupleCache) -> CFile {
    let mut builder = Builder { tuples, ..Default::default() };

    mir.definitions
        .iter()
        .filter(|(id, _)| id.0 % n == i)
        .for_each(|(_id, definition)| builder.build_definition(definition, mir));

    mir.externals
        .iter()
        .filter(|(id, _)| id.0 % n == i)
        .for_each(|(id, external)| builder.build_external(external, *id));

    builder.file
}

impl Builder {
    /// Push the given string to `self.current_item`
    fn write(&mut self, s: &str) {
        self.current_item += s;
    }

    /// Build the given definition, adding it as a translating C function when finished
    fn build_definition(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        self.build_fn_signature(definition, mir);

        self.write("{");
        self.write_fn_body(definition, mir);
        self.write("}");

        self.file.add_function_definition(&self.current_item);
        self.current_item.clear();
    }

    /// Declare the given item
    fn build_external(&mut self, external: &mir::Extern, id: DefinitionId) {
        self.write_declarator(&external.typ, &|this| this.write_mangled_name(&external.name, id));

        self.file.add_function_declaration(&self.current_item);
        self.current_item.clear();
    }

    /// Write a mangled name `name_id` directly to `current_item`
    fn write_mangled_name(&mut self, name: &str, id: DefinitionId) {
        let _ = write!(self.current_item, "{name}_{}", id.0);
    }

    fn write_value(&mut self, value: &mir::Value, mir: &mir::Mir) {
        let s = match value {
            mir::Value::Error => unreachable!("Error value found in C codegen"),
            mir::Value::Unit => Cow::Borrowed("(Unit){}"),
            mir::Value::Bool(true) => Cow::Borrowed("true"),
            mir::Value::Bool(false) => Cow::Borrowed("false"),
            mir::Value::Char(c) if c.is_ascii_alphanumeric() || *c == '_' => Cow::Owned(format!("'{c}'")),
            mir::Value::Char(c) => Cow::Owned(format!("(char){}", *c as u32)),
            mir::Value::Integer(int) => Cow::Owned(int.to_string()), // TODO: Incorrect suffixes. Should be e.g. `(int16_t) x`
            mir::Value::Float(float) => Cow::Owned(float.to_string()),
            mir::Value::InstructionResult(id) => Cow::Owned(id.to_string()),
            mir::Value::Parameter(block, i) => Cow::Owned(format!("{block}_{i}")),
            mir::Value::Definition(id) => {
                let name = mir.get_name(*id).unwrap();
                // This must match the mangling in [Self::write_mangled_name]
                Cow::Owned(format!("{name}_{}", id.0))
            },
        };
        self.write(&s);
    }

    /// Build the function's signature in `self.current_item` and also push it as a
    /// function declaration.
    fn build_fn_signature(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        // write `ret_t foo(t0 arg0, ..., tN argN);`
        let mir::Type::Function(function_type) = &definition.typ else {
            panic!("Definition is not a function")
        };

        // The declared name is `foo(t0 arg0, ..., tN argN)`; weaving the return type
        // around it keeps even array or function-pointer return types correct.
        self.write_declarator(&function_type.return_type, &|this| {
            this.write_mangled_name(&definition.name, definition.id);
            this.write("(");
            for (i, (parameter, typ)) in definition.parameters().enumerate() {
                if i != 0 {
                    this.write(", ");
                }
                this.write_declarator(typ, &|t| t.write_value(&parameter, mir));
            }
            this.write(")");
        });
        self.write(";");

        // `ret_t foo(t0 arg0, ..., tN argN);` written. Forward-declare it then pop the `;`
        self.file.add_function_declaration(&self.current_item);
        self.current_item.pop(); // ;
        self.write(" ");
    }

    /// Write `typ` as a C declaration of `name` into `self.current_item`. Pass an
    /// empty `name` to write the bare type (e.g. a cast or an unnamed parameter type).
    fn write_type(&mut self, typ: &mir::Type, name: &str) {
        self.write_declarator(typ, &|this| this.write(name));
    }

    /// Write `typ` as a C declaration whose name is produced by `write_name`, streaming
    /// directly to `current_item` with no intermediate allocations. C's declarator
    /// syntax weaves the name into the middle of the type for arrays and function
    /// pointers (`int8_t name[5]`, `int32_t (*name)(bool)`), so those arms recurse into
    /// the inner type carrying an extended `write_name`. Pointers here are opaque
    /// (`void*`), so there is no pointer-vs-array/function precedence to juggle.
    fn write_declarator(&mut self, typ: &mir::Type, write_name: &dyn Fn(&mut Self)) {
        match typ {
            mir::Type::Function(function) => {
                // A function value is a function pointer in C: `ret (*<name>)(params)`.
                self.write_declarator(&function.return_type, &|this| {
                    this.write("(*");
                    write_name(this);
                    this.write(")(");
                    for (i, parameter) in function.parameters.iter().enumerate() {
                        if i != 0 {
                            this.write(", ");
                        }
                        this.write_type(parameter, "");
                    }
                    this.write(")");
                });
            },
            mir::Type::Array { length, element } => {
                // The name binds tighter than the brackets: `<element> <name>[length]`.
                self.write_declarator(element, &|this| {
                    write_name(this);
                    this.write("[");
                    this.write_type(length, "");
                    this.write("]");
                });
            },
            _ => {
                self.write_base_type(typ);
                // Separate the base type from the name with a space, then drop it if
                // `write_name` turned out to be empty (a bare, unnamed type).
                let mark = self.current_item.len();
                self.write(" ");
                write_name(self);
                if self.current_item.len() == mark + 1 {
                    self.current_item.truncate(mark);
                }
            },
        }
    }

    /// Write the base (non-weaving) spelling of `typ`: primitives, the opaque pointer,
    /// tuples (by cached struct name), and type-level integers. Arrays and function
    /// pointers are handled by [Self::write_declarator] since they weave in the name.
    fn write_base_type(&mut self, typ: &mir::Type) {
        let s = match typ {
            mir::Type::Primitive(primitive) => match primitive {
                mir::PrimitiveType::Error => unreachable!("Found Error type in C codegen"),
                mir::PrimitiveType::Unit => "Unit",
                mir::PrimitiveType::Bool => "bool",
                mir::PrimitiveType::Pointer => "void*",
                mir::PrimitiveType::Char => "char",
                mir::PrimitiveType::Int(kind) => match kind {
                    crate::lexer::token::IntegerKind::I8 => "int8_t",
                    crate::lexer::token::IntegerKind::I16 => "int16_t",
                    crate::lexer::token::IntegerKind::I32 => "int32_t",
                    crate::lexer::token::IntegerKind::I64 => "int64_t",
                    crate::lexer::token::IntegerKind::Isz => "ptrdiff_t",
                    crate::lexer::token::IntegerKind::U8 => "uint8_t",
                    crate::lexer::token::IntegerKind::U16 => "uint16_t",
                    crate::lexer::token::IntegerKind::U32 => "uint32_t",
                    crate::lexer::token::IntegerKind::U64 => "uint64_t",
                    crate::lexer::token::IntegerKind::Usz => "size_t",
                },
                mir::PrimitiveType::Float(kind) => match kind {
                    crate::lexer::token::FloatKind::F32 => "_Float32",
                    crate::lexer::token::FloatKind::F64 => "_Float64",
                },
                mir::PrimitiveType::NoClosureEnv => unreachable!("NoClosureEnv found in C codegen"),
            },
            mir::Type::Tuple(elements) => return self.write_cached_tuple_type(elements),
            mir::Type::U32(n) => {
                let _ = write!(self.current_item, "{n}");
                return;
            },
            mir::Type::Union(_) => unreachable!("Union types should be removed by the select_largest_variant mir pass"),
            mir::Type::Function(_) | mir::Type::Array { .. } => {
                unreachable!("Function and Array types are handled by write_declarator")
            },
            mir::Type::Generic(_) => unreachable!("Generic found in C codegen"),
        };
        self.write(s);
    }

    /// Retrieve the given tuple type from the cache if there is one and write it,
    /// otherwise cache it and write the newly generated name.
    fn write_cached_tuple_type(&mut self, elements: &Arc<Vec<mir::Type>>) {
        // Fast path: the tuple was already named by this or another worker.
        if let Some(entry) = self.tuples.types.get(elements) {
            let _ = write!(self.current_item, "Tuple{}", entry.value().0);
            return;
        }

        // Render the struct body first. This recurses through `write_type` into any nested
        // tuples, registering them now so they get smaller ids and are emitted before us. No
        // DashMap guard is held across the recursion, so re-entry can't deadlock.
        let saved = std::mem::take(&mut self.current_item);
        self.write("struct { ");
        for (i, element) in elements.iter().enumerate() {
            self.write_type(element, &format!("_{i}"));
            self.write("; ");
        }
        self.write("}");
        let body = std::mem::replace(&mut self.current_item, saved);

        // Re-check on insert in case another worker raced us to the same tuple.
        let id = match self.tuples.types.entry(elements.clone()) {
            Entry::Occupied(entry) => entry.get().0,
            Entry::Vacant(entry) => {
                let id = self.tuples.next_id.fetch_add(1, Ordering::Relaxed);
                entry.insert((id, format!("typedef {body} Tuple{id};")));
                id
            },
        };
        let _ = write!(self.current_item, "Tuple{id}");
    }

    /// Iterate over each block and each instruction, inserting them into the function.
    fn write_fn_body(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        for block_id in definition.topological_sort() {
            let block = &definition.blocks[block_id];

            for instruction_id in &block.instructions {
                let instruction = &definition.instructions[*instruction_id];
                self.write_instruction(*instruction_id, instruction, definition, mir);
            }
        }
    }

    fn write_instruction(&mut self, instruction_id: InstructionId, instruction: &mir::Instruction, definition: &mir::Definition, mir: &mir::Mir) {
        match instruction {
            mir::Instruction::Call { function, arguments } => todo!(),
            mir::Instruction::CallClosure { closure, arguments } => todo!(),
            mir::Instruction::Perform { effect_op, arguments } => todo!(),
            mir::Instruction::Handle { body, cases } => todo!(),
            mir::Instruction::Capability => todo!(),
            mir::Instruction::PackClosure { function, environment } => todo!(),
            mir::Instruction::IndexTuple { tuple, index } => todo!(),
            mir::Instruction::MakeBytes(items) => todo!(),
            mir::Instruction::MakeTuple(values) => todo!(),
            mir::Instruction::MakeArray(values) => todo!(),
            mir::Instruction::StackAlloc(value) => todo!(),
            mir::Instruction::StackAllocUninit(_) => todo!(),
            mir::Instruction::AllocShared(value) => todo!(),
            mir::Instruction::Store { pointer, value } => todo!(),
            mir::Instruction::GetFieldPtr { struct_ptr, struct_type, index } => todo!(),
            mir::Instruction::Transmute(value) => todo!(),
            mir::Instruction::Instantiate(definition_id, items) => todo!(),
            mir::Instruction::Id(value) => todo!(),
            mir::Instruction::Extern(_) => todo!(),
            mir::Instruction::AddInt(value, value1) => todo!(),
            mir::Instruction::AddFloat(value, value1) => todo!(),
            mir::Instruction::SubInt(value, value1) => todo!(),
            mir::Instruction::SubFloat(value, value1) => todo!(),
            mir::Instruction::MulInt(value, value1) => todo!(),
            mir::Instruction::MulFloat(value, value1) => todo!(),
            mir::Instruction::DivSigned(value, value1) => todo!(),
            mir::Instruction::DivUnsigned(value, value1) => todo!(),
            mir::Instruction::DivFloat(value, value1) => todo!(),
            mir::Instruction::ModSigned(value, value1) => todo!(),
            mir::Instruction::ModUnsigned(value, value1) => todo!(),
            mir::Instruction::ModFloat(value, value1) => todo!(),
            mir::Instruction::LessSigned(value, value1) => todo!(),
            mir::Instruction::LessUnsigned(value, value1) => todo!(),
            mir::Instruction::LessFloat(value, value1) => todo!(),
            mir::Instruction::EqInt(value, value1) => todo!(),
            mir::Instruction::EqFloat(value, value1) => todo!(),
            mir::Instruction::BitwiseAnd(value, value1) => todo!(),
            mir::Instruction::BitwiseOr(value, value1) => todo!(),
            mir::Instruction::BitwiseXor(value, value1) => todo!(),
            mir::Instruction::BitwiseNot(value) => todo!(),
            mir::Instruction::SignExtend(value) => todo!(),
            mir::Instruction::ZeroExtend(value) => todo!(),
            mir::Instruction::SignedToFloat(value) => todo!(),
            mir::Instruction::UnsignedToFloat(value) => todo!(),
            mir::Instruction::FloatToSigned(value) => todo!(),
            mir::Instruction::FloatToUnsigned(value) => todo!(),
            mir::Instruction::FloatPromote(value) => todo!(),
            mir::Instruction::FloatDemote(value) => todo!(),
            mir::Instruction::Truncate(value) => todo!(),
            mir::Instruction::Deref(value) => todo!(),
            mir::Instruction::SizeOf(_) => todo!(),
            mir::Instruction::ArrayLen(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::Builder;
    use crate::lexer::token::IntegerKind;
    use crate::mir::{self, Type};

    fn func(params: Vec<Type>, ret: Type) -> Type {
        Type::Function(Arc::new(mir::FunctionType {
            parameters: params,
            environment: Type::NO_CLOSURE_ENV,
            return_type: ret,
        }))
    }

    fn check(typ: &Type, name: &str, expected: &str) {
        let mut builder = Builder::default();
        builder.write_type(typ, name);
        assert_eq!(builder.current_item, expected);
    }

    #[test]
    fn primitive() {
        check(&Type::int(IntegerKind::I8), "x", "int8_t x");
    }

    #[test]
    fn pointer() {
        check(&Type::POINTER, "p", "void* p");
    }

    #[test]
    fn array() {
        check(&Type::array_with_length(Type::U32(5), Type::int(IntegerKind::I8)), "arr", "int8_t arr[5]");
    }

    #[test]
    fn nested_array() {
        let inner = Type::array_with_length(Type::U32(5), Type::int(IntegerKind::I8));
        check(&Type::array_with_length(Type::U32(3), inner), "m", "int8_t m[3][5]");
    }

    #[test]
    fn function_pointer() {
        check(&func(vec![Type::BOOL], Type::int(IntegerKind::I32)), "f", "int32_t (*f)(bool)");
    }

    #[test]
    fn function_pointer_multiple_params() {
        check(&func(vec![Type::BOOL, Type::CHAR], Type::int(IntegerKind::I32)), "f", "int32_t (*f)(bool, char)");
    }

    #[test]
    fn unnamed_function_pointer() {
        check(&func(vec![Type::BOOL], Type::int(IntegerKind::I32)), "", "int32_t (*)(bool)");
    }

    #[test]
    fn function_returning_function() {
        let inner = func(vec![Type::CHAR], Type::int(IntegerKind::I32));
        check(&func(vec![Type::BOOL], inner), "f", "int32_t (*(*f)(bool))(char)");
    }

    #[test]
    fn array_of_function_pointers() {
        let element = func(vec![Type::BOOL], Type::int(IntegerKind::I32));
        check(&Type::array_with_length(Type::U32(3), element), "arr", "int32_t (*arr[3])(bool)");
    }

    #[test]
    fn array_of_function_pointers_returning_function_pointers() {
        let inner = func(vec![Type::BOOL], Type::int(IntegerKind::I8));
        let element = func(vec![Type::CHAR], inner);
        check(&Type::array_with_length(Type::U32(2), element), "arr", "int8_t (*(*arr[2])(char))(bool)");
    }

    #[test]
    fn tuple_writes_name_and_caches_definition() {
        let tuple = Type::tuple(vec![Type::int(IntegerKind::I8), Type::POINTER]);
        let mut builder = Builder::default();
        builder.write_type(&tuple, "t");

        assert_eq!(builder.current_item, "Tuple0 t");
        let Type::Tuple(key) = &tuple else { unreachable!() };
        let definition = builder.tuples.types.get(key).unwrap();
        assert_eq!(definition.value().1, "typedef struct { int8_t _0; void* _1; } Tuple0;");
    }

    #[test]
    fn identical_tuples_share_one_name() {
        let tuple = Type::tuple(vec![Type::int(IntegerKind::I8), Type::POINTER]);
        let mut builder = Builder::default();
        builder.write_type(&tuple, "a");
        builder.write(", ");
        builder.write_type(&tuple, "b");

        assert_eq!(builder.current_item, "Tuple0 a, Tuple0 b");
        assert_eq!(builder.tuples.types.len(), 1);
    }

    #[test]
    fn nested_tuple_inner_gets_lower_id() {
        // The inner tuple is registered while generating the outer's body, so it gets id 0.
        let inner = Type::tuple(vec![Type::int(IntegerKind::I8)]);
        let outer = Type::tuple(vec![inner.clone(), Type::int(IntegerKind::I8)]);
        let mut builder = Builder::default();
        builder.write_type(&outer, "t");

        assert_eq!(builder.current_item, "Tuple1 t");
        let Type::Tuple(inner_key) = &inner else { unreachable!() };
        assert_eq!(builder.tuples.types.get(inner_key).unwrap().value().0, 0);
    }
}
