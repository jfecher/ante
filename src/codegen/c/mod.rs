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
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use dashmap::{DashMap, mapref::entry::Entry};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    cli::OptLevel,
    lexer::token::{FloatKind, IntegerKind},
    mir::{self, DefinitionId, FloatConstant, InstructionId, IntConstant},
    parser::ids::TopLevelName,
};

mod cfile;
use cfile::CFile;

use super::{
    OverflowingIntOp,
    constant::{self, ConstantValue},
};

/// Codegen the given Mir into a single C file, then invoke cc to create
/// a object file and a binary. On success, the object file is removed, but
/// the .c file is kept.
pub fn codegen_c_for_mir(
    mir: &mir::Mir, binary_name: &str, opt_level: OptLevel, selected_main: Option<TopLevelName>,
    link_options: &super::LinkOptions,
) {
    // Create the C file
    let c_file = build_c_file(mir, selected_main);
    let c_file_name = format!("{binary_name}.c");
    std::fs::write(&c_file_name, c_file).unwrap();

    // Create the .o file
    let o_file_name = format!("{binary_name}.o");
    let mut child = Command::new("cc")
        .arg(&c_file_name)
        .arg(format!("-o{o_file_name}"))
        .arg(opt_level.as_cc_opt_string())
        .arg("-c")
        .arg("-w")
        .spawn()
        .unwrap();

    // And link it into a binary
    let status = child.wait().unwrap();
    if status.success() {
        super::link_with_cc(&o_file_name, binary_name, link_options);
        std::fs::remove_file(&c_file_name).unwrap();
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

/// A concurrent cache mapping each distinct tuple type to a stable id and its generated C
/// `struct` definition. Cloning shares the same underlying maps, so every [Builder] working in
/// parallel resolves a given tuple structure to the same `TupleN` name and emits its definition
/// exactly once.
#[derive(Default, Clone)]
struct TupleCache {
    types: Arc<DashMap<Arc<Vec<mir::Type>>, (u32, String)>>,
    next_id: Arc<AtomicU32>,
}

/// Builds a C File for the given [mir::Mir] in-memory. Returns the file contents
pub(crate) fn build_c_file(mir: &mir::Mir, selected_main: Option<TopLevelName>) -> String {
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

    // Globals with non-constant initializers are assigned at startup. Order them so each is
    // assigned after every other deferred global it reads, then wrap them in a startup function.
    let initializers = order_global_initializers(file.take_global_initializers());
    let has_initializers = !initializers.is_empty();
    if has_initializers {
        let mut init = "static void __ante_init_globals(void) {".to_string();
        for global in &initializers {
            init += &global.statement;
        }
        init += "}";
        file.add_function_definition(&init);
    }

    // Emit `main` that captures argc/argv, runs the startup initializer, then calls the program's
    // `main` function which will be mangled, and returns 0. Skipped for libraries.
    if let Some(id) = super::resolve_main_id(selected_main) {
        let init_call = if has_initializers { "__ante_init_globals();" } else { "" };
        let accessors = "static int32_t ante_argc = 0;\nstatic void* ante_argv = 0;\n\
            int32_t ante_get_argc(Unit _0) { return ante_argc; }\n\
            void* ante_get_argv(Unit _0) { return ante_argv; }\n";
        // `main`'s trailing evidence parameter is the empty tuple, registered in the cache
        // when main's own signature was emitted.
        let empty_evidence = Arc::new(Vec::new());
        let evidence_name =
            tuples.types.get(&empty_evidence).map_or("Unit".to_string(), |entry| format!("Tuple{}", entry.value().0));
        let wrapper = format!(
            "int main(int argc, char** argv) {{ ante_argc = argc; ante_argv = (void*)argv; {init_call} main_{}((Unit){{0}}, ({evidence_name}){{0}}); return 0; }}",
            id.0
        );
        file.add_function_definition(accessors);
        file.add_function_definition(&wrapper);
    }

    file.add_starter_items().into_contents()
}

/// Topologically order deferred global initializers so each global is assigned after every other
/// deferred global it reads by value. By-value references between globals cannot form a cycle (the
/// value would be infinite), so the dependency graph (restricted to the deferred set) is a DAG.
/// Ties are broken by definition id for deterministic output.
fn order_global_initializers(mut initializers: Vec<cfile::GlobalInitializer>) -> Vec<cfile::GlobalInitializer> {
    use std::collections::BTreeMap;

    initializers.sort_by_key(|init| init.id.0);
    let deferred: rustc_hash::FxHashSet<_> = initializers.iter().map(|init| init.id).collect();

    // Map each deferred id to its initializer, visiting in id order for determinism.
    let mut by_id: BTreeMap<u32, cfile::GlobalInitializer> =
        initializers.into_iter().map(|init| (init.id.0, init)).collect();

    let mut ordered = Vec::with_capacity(by_id.len());
    let mut visited = rustc_hash::FxHashSet::default();
    let ids: Vec<u32> = by_id.keys().copied().collect();
    for id in ids {
        visit_initializer(id, &deferred, &mut by_id, &mut visited, &mut ordered);
    }
    ordered
}

/// Depth-first post-order visit emitting each global's deferred dependencies before itself.
fn visit_initializer(
    id: u32, deferred: &rustc_hash::FxHashSet<DefinitionId>,
    by_id: &mut std::collections::BTreeMap<u32, cfile::GlobalInitializer>, visited: &mut rustc_hash::FxHashSet<u32>,
    ordered: &mut Vec<cfile::GlobalInitializer>,
) {
    if !visited.insert(id) {
        return;
    }
    // Take the initializer out so its `deps` can be borrowed while recursing on others.
    let Some(init) = by_id.remove(&id) else {
        return;
    };
    for dep in &init.deps {
        if deferred.contains(dep) {
            visit_initializer(dep.0, deferred, by_id, visited, ordered);
        }
    }
    ordered.push(init);
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

    /// Run `f` with an empty `current_item`, returning what it emitted and restoring the prior
    /// `current_item`. Used to build a self-contained fragment (a global, a typedef, an extern
    /// declaration) destined for a different output section without disturbing the function body
    /// currently being assembled in `current_item`.
    fn capture(&mut self, f: impl FnOnce(&mut Self)) -> String {
        let saved = std::mem::take(&mut self.current_item);
        f(self);
        std::mem::replace(&mut self.current_item, saved)
    }

    fn write_byte_array(&mut self, name: &str, bytes: &[u8]) {
        let _ = write!(self.current_item, "static uint8_t {name}[] = {{");
        for (i, byte) in bytes.iter().enumerate() {
            if i != 0 {
                self.write(",");
            }
            let _ = write!(self.current_item, "{byte}");
        }
        self.write("};");
    }

    /// Build the given definition, adding it as a translated C function when finished. Globals
    /// (single block, `Result` terminator) become file-scope variables instead.
    fn build_definition(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        if definition.is_global() {
            return self.build_global(definition, mir);
        }

        self.build_fn_signature(definition, mir);

        self.write("{");
        self.write_fn_body(definition, mir);
        self.write("}");

        self.file.add_function_definition(&self.current_item);
        self.current_item.clear();
    }

    /// Build a global definition as a file-scope C variable `T name_id = <initializer>;`. The
    /// initializer is folded by the shared [constant] evaluator and rendered by [Self::write_constant].
    fn build_global(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        let value = constant::evaluate_global(mir, definition);

        // Globals are emitted in arbitrary order (by worker, then id, with no dependency sort), so
        // forward-declare every global. This lets one global's initializer reference another (e.g.
        // take its address via a `Shared` value) regardless of which is defined first textually.
        // `current_item` is empty here, so write the declaration directly then route it.
        self.write("extern ");
        self.write_declarator(&definition.typ, &|this| this.write_mangled_name(&definition.name, definition.id));
        self.write(";");
        self.file.add_global_declaration(&self.current_item);
        self.current_item.clear();

        let mut aux_index = 0;

        // C requires a file-scope initializer to be a constant expression. A global that reads
        // another global's value is not constant, so emit it zero-initialized and assign its real
        // value at startup (see `build_c_file`) instead.
        if constant::is_c_constant(&value, mir) {
            self.write_declarator(&definition.typ, &|this| this.write_mangled_name(&definition.name, definition.id));
            self.write(" = ");
            self.write_constant(&value, definition.id, &mut aux_index, mir);
            self.write(";");
            self.file.add_global_definition(&self.current_item);
            self.current_item.clear();
            return;
        }

        // `T name_id;` (file-scope, so zero-initialized until the startup assignment runs).
        self.write_declarator(&definition.typ, &|this| this.write_mangled_name(&definition.name, definition.id));
        self.write(";");
        self.file.add_global_definition(&self.current_item);
        self.current_item.clear();

        // `name_id = <value>;`, deferred to the startup initializer.
        self.write_mangled_name(&definition.name, definition.id);
        self.write(" = ");
        self.write_constant_rvalue(&value, &definition.typ, definition.id, &mut aux_index, mir);
        self.write(";");
        let statement = std::mem::take(&mut self.current_item);

        let mut deps = Vec::new();
        constant::referenced_globals(&value, mir, &mut deps);
        self.file.add_global_initializer(cfile::GlobalInitializer { id: definition.id, deps, statement });
    }

    /// Render a [ConstantValue] as a C *rvalue* for an assignment statement (as opposed to a
    /// file-scope initializer). Aggregates need a compound-literal cast since `name = {...}` is not
    /// valid statement syntax; scalars and name references are written as-is.
    fn write_constant_rvalue(
        &mut self, value: &ConstantValue, typ: &mir::Type, global_id: DefinitionId, aux_index: &mut u32, mir: &mir::Mir,
    ) {
        if matches!(value, ConstantValue::Tuple(_) | ConstantValue::Array { .. }) {
            self.write("(");
            self.write_type(typ, "");
            self.write(")");
        }
        self.write_constant(value, global_id, aux_index, mir);
    }

    /// Render a folded [ConstantValue] as a C initializer expression into `current_item`.
    /// `Shared` values are backed by uniquely-named file-scope statics (`aux_index` keeps the
    /// names distinct within this global; the global's id keeps them distinct across globals).
    fn write_constant(&mut self, value: &ConstantValue, global_id: DefinitionId, aux_index: &mut u32, mir: &mir::Mir) {
        match value {
            ConstantValue::Unit => self.write("{0}"),
            ConstantValue::Bool(b) => self.write_value(&mir::Value::Bool(*b), mir),
            ConstantValue::Char(c) => self.write_value(&mir::Value::Char(*c), mir),
            ConstantValue::Int(int) => self.write_integer_constant(*int),
            ConstantValue::Float(float) => self.write_float_constant(*float),
            ConstantValue::Definition(id) => self.write_value(&mir::Value::Definition(*id), mir),
            ConstantValue::Tuple(values) => self.write_brace_list(values, global_id, aux_index, mir),
            ConstantValue::Array { elements, .. } => self.write_brace_list(elements, global_id, aux_index, mir),
            ConstantValue::Bytes(bytes) => {
                // Back the blob with a uniquely-named static array and decay it to a pointer.
                let name = format!("__bytes_{}_{}", global_id.0, *aux_index);
                *aux_index += 1;

                let backing = self.capture(|this| this.write_byte_array(&name, bytes));
                self.file.add_global_definition(&backing);

                self.write(&name);
            },
            ConstantValue::Extern { name, typ } => {
                self.emit_extern_declaration(name, typ);
                self.write(name);
            },
            ConstantValue::Shared { value, typ } => {
                let name = format!("__shared_{}_{}", global_id.0, *aux_index);
                *aux_index += 1;

                // Emit `static T name = <inner>;` into the globals section, then take its address.
                // `write_constant` runs inside `capture`, so any nested statics it emits are routed
                // to their own sections rather than into this fragment.
                let backing = self.capture(|this| {
                    this.write("static ");
                    this.write_declarator(typ, &|this| this.write(&name));
                    this.write(" = ");
                    this.write_constant(value, global_id, aux_index, mir);
                    this.write(";");
                });
                self.file.add_global_definition(&backing);

                self.write("&");
                self.write(&name);
            },
            ConstantValue::Transmute { typ } => {
                // Zero-sized source: emit a zero-initializer of the destination type.
                self.write("(");
                self.write_type(typ, "");
                self.write("){0}");
            },
        }
    }

    fn write_brace_list(
        &mut self, values: &[ConstantValue], global_id: DefinitionId, aux_index: &mut u32, mir: &mir::Mir,
    ) {
        if values.is_empty() {
            self.write("{0}");
            return;
        }
        self.write("{");
        for (i, value) in values.iter().enumerate() {
            if i != 0 {
                self.write(", ");
            }
            self.write_constant(value, global_id, aux_index, mir);
        }
        self.write("}");
    }

    /// Declare the given item
    fn build_external(&mut self, external: &mir::Extern, id: DefinitionId) {
        self.write_declarator(&external.typ, &|this| this.write_mangled_name(&external.name, id));

        self.file.add_function_declaration(&self.current_item);
        self.current_item.clear();
    }

    /// Write a mangled name `name_id` directly to `current_item`. Non-identifier characters in
    /// `name` (Ante allows operator names like `>=`) are replaced with `_`; the unique `id`
    /// suffix keeps the result distinct regardless.
    fn write_mangled_name(&mut self, name: &str, id: DefinitionId) {
        for c in name.chars() {
            self.current_item.push(if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' });
        }
        let _ = write!(self.current_item, "_{}", id.0);
    }

    fn write_value(&mut self, value: &mir::Value, mir: &mir::Mir) {
        let s = match value {
            mir::Value::Error => unreachable!("Error value found in C codegen"),
            mir::Value::Unit => Cow::Borrowed("(Unit){0}"),
            mir::Value::Bool(true) => Cow::Borrowed("true"),
            mir::Value::Bool(false) => Cow::Borrowed("false"),
            mir::Value::Char(c) if c.is_ascii_alphanumeric() || *c == '_' => Cow::Owned(format!("'{c}'")),
            mir::Value::Char(c) => Cow::Owned(format!("(char){}", *c as u32)),
            mir::Value::Integer(int) => return self.write_integer_constant(*int),
            mir::Value::Float(float) => return self.write_float_constant(*float),
            mir::Value::InstructionResult(id) => Cow::Owned(id.to_string()),
            mir::Value::Parameter(block, i) => Cow::Owned(format!("{block}_{i}")),
            mir::Value::Definition(id) => {
                let name = mir.get_name(*id).unwrap().clone();
                return self.write_mangled_name(&name, *id);
            },
        };
        self.write(&s);
    }

    /// Write an integer literal as a width-correct C constant, e.g. `(int32_t)5`. 64-bit
    /// kinds get an `ll`/`ull` suffix so the literal token isn't truncated to `int`.
    fn write_integer_constant(&mut self, int: IntConstant) {
        let c_type = int_kind_c_name(int.kind());
        let _ = match int {
            IntConstant::U8(x) => write!(self.current_item, "({c_type}){x}"),
            IntConstant::U16(x) => write!(self.current_item, "({c_type}){x}"),
            IntConstant::U32(x) => write!(self.current_item, "({c_type}){x}"),
            IntConstant::U64(x) => write!(self.current_item, "({c_type}){x}ull"),
            IntConstant::Usz(x) => write!(self.current_item, "({c_type}){x}ull"),
            IntConstant::I8(x) => write!(self.current_item, "({c_type}){x}"),
            IntConstant::I16(x) => write!(self.current_item, "({c_type}){x}"),
            IntConstant::I32(x) => write!(self.current_item, "({c_type}){x}"),
            // `(int64_t)-9223372036854775808ll` would negate a literal whose magnitude overflows
            // `ll`, so the minimum is written with the same idiom `stdint.h` uses for `INT64_MIN`.
            IntConstant::I64(i64::MIN) | IntConstant::Isz(isize::MIN) => {
                write!(self.current_item, "({c_type})(-9223372036854775807ll - 1)")
            },
            IntConstant::I64(x) => write!(self.current_item, "({c_type}){x}ll"),
            IntConstant::Isz(x) => write!(self.current_item, "({c_type}){x}ll"),
        };
    }

    /// Write a float literal as a cast C constant, e.g. `(ante_f64)1.5`.
    fn write_float_constant(&mut self, float: FloatConstant) {
        let (c_type, value) = match float {
            FloatConstant::F32(v) => (float_kind_c_name(FloatKind::F32), v.0),
            FloatConstant::F64(v) => (float_kind_c_name(FloatKind::F64), v.0),
        };
        // `inf`/`-inf`/`NaN` (how Rust formats non-finite floats) are not C tokens, we need to use
        // `ANTE_INF`/`ANTE_NAN` from [CFile::add_starter_items]
        let _ = if value.is_finite() {
            write!(self.current_item, "({c_type}){value}")
        } else if value.is_nan() {
            write!(self.current_item, "({c_type})ANTE_NAN()")
        } else if value < 0.0 {
            write!(self.current_item, "({c_type})(-ANTE_INF())")
        } else {
            write!(self.current_item, "({c_type})ANTE_INF()")
        };
    }

    /// Build the function's signature in `self.current_item` and also push it as a
    /// function declaration.
    fn build_fn_signature(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        // write `ret_t foo(t0 arg0, ..., tN argN);`
        let mir::Type::Function(function_type) = &definition.typ else { panic!("Definition is not a function") };

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
                mir::PrimitiveType::Int(kind) => int_kind_c_name(*kind),
                mir::PrimitiveType::Float(kind) => float_kind_c_name(*kind),
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
        let body = self.capture(|this| {
            this.write("struct { ");
            if elements.is_empty() {
                this.write("char _unused; ");
            }
            for (i, element) in elements.iter().enumerate() {
                this.write_type(element, &format!("_{i}"));
                this.write("; ");
            }
            this.write("}");
        });

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

    /// Write the function body: hoisted block-parameter declarations followed by each
    /// block as a `goto` label. The body is flat (no nested scopes) so every SSA temporary
    /// lives at function scope and remains visible across the `goto`s that wire blocks together.
    fn write_fn_body(&mut self, definition: &mir::Definition, mir: &mir::Mir) {
        // C has no phi nodes, so non-entry block parameters become ordinary variables a
        // predecessor assigns before jumping. The entry block's parameters are the C function
        // parameters and are already declared by the signature, so they are skipped here.
        for (block_id, block) in definition.blocks.iter() {
            if block_id == mir::BlockId::ENTRY_BLOCK {
                continue;
            }
            for (parameter, typ) in block.parameters(block_id) {
                self.write_declarator(&typ, &|this| this.write_value(&parameter, mir));
                self.write("; ");
            }
        }

        for block_id in definition.topological_sort() {
            let block = &definition.blocks[block_id];

            // The empty statement lets a declaration follow the label (illegal pre-C23 otherwise).
            let _ = write!(self.current_item, "{block_id}:;");

            for instruction_id in &block.instructions {
                let instruction = &definition.instructions[*instruction_id];
                self.write_instruction(*instruction_id, instruction, definition, mir);
            }
            self.write_terminator(block, mir);
        }
    }

    /// Write a block's terminator. Every block ends in a `goto`/`return`/trap so control never
    /// falls through into the textually-following block's label.
    fn write_terminator(&mut self, block: &mir::Block, mir: &mir::Mir) {
        match block.terminator.as_ref().expect("block has no terminator") {
            // `Result` only occurs in globals, handled separately, but treat it as a return defensively.
            mir::TerminatorInstruction::Return(value) | mir::TerminatorInstruction::Result(value) => {
                self.write("return ");
                self.write_value(value, mir);
                self.write(";");
            },
            mir::TerminatorInstruction::Jmp((target, argument)) => {
                self.write_jmp(*target, argument, mir);
            },
            mir::TerminatorInstruction::If { condition, then, else_, end: _ } => {
                self.write("if (");
                self.write_value(condition, mir);
                self.write(") { ");
                self.write_jmp(then.0, &then.1, mir);
                self.write(" } else { ");
                self.write_jmp(else_.0, &else_.1, mir);
                self.write(" }");
            },
            mir::TerminatorInstruction::Switch { int_value, cases, else_, end: _ } => {
                self.write("switch (");
                self.write_value(int_value, mir);
                self.write(") { ");
                for (tag, target) in cases {
                    let _ = write!(self.current_item, "case {tag}: {{ ");
                    self.write_jmp(target.0, &target.1, mir);
                    self.write(" } ");
                }
                self.write("default: { ");
                self.write_jmp(else_.0, &else_.1, mir);
                self.write(" } }");
            },
            mir::TerminatorInstruction::Unreachable => {
                self.write("ANTE_UNREACHABLE();");
            },
        }
    }

    /// Emit a jump to `target`, first assigning the optional branch argument into the target
    /// block's parameter-0 variable (MIR's equivalent of populating a phi).
    fn write_jmp(&mut self, target: mir::BlockId, argument: &Option<mir::Value>, mir: &mir::Mir) {
        if let Some(argument) = argument {
            self.write_value(&mir::Value::Parameter(target, 0), mir);
            self.write(" = ");
            self.write_value(argument, mir);
            self.write("; ");
        }
        let _ = write!(self.current_item, "goto {target};");
    }

    /// Write `<result_type> vN = ` for the given instruction so the caller can append its
    /// right-hand-side expression. Uses the declarator form so array/function-pointer result
    /// types stay well-formed.
    fn write_result_binding(&mut self, id: InstructionId, definition: &mir::Definition) {
        let typ = definition.instruction_result_type(id);
        self.write_declarator(typ, &|this| {
            let _ = write!(this.current_item, "{id}");
        });
        self.write(" = ");
    }

    /// `<result> vN = <a> <op> <b>;`
    fn write_binary(
        &mut self, id: InstructionId, definition: &mir::Definition, mir: &mir::Mir, a: &mir::Value, op: &str,
        b: &mir::Value,
    ) {
        self.write_result_binding(id, definition);
        self.write_value(a, mir);
        self.write(op);
        self.write_value(b, mir);
        self.write(";");
    }

    /// `<result> vN; vN._1 = __builtin_<op>_overflow(a, b, &vN._0);`
    fn write_overflowing_binary(
        &mut self, id: InstructionId, definition: &mir::Definition, mir: &mir::Mir, op: OverflowingIntOp,
        a: &mir::Value, b: &mir::Value,
    ) {
        let result = definition.instruction_result_type(id);
        self.write_declarator(result, &|this| {
            let _ = write!(this.current_item, "{id}");
        });
        let builtin_name = op.c_builtin_name();
        let _ = write!(self.current_item, "; {id}._1 = {builtin_name}(");
        self.write_value(a, mir);
        self.write(", ");
        self.write_value(b, mir);
        let _ = write!(self.current_item, ", &{id}._0);");
    }

    /// Like [Self::write_binary] but casts each operand to its width's signed or unsigned C
    /// type first. C's integer promotions would otherwise force a signedness that's wrong for
    /// `DivUnsigned`/`LessSigned`/etc on sub-`int` widths.
    fn write_binary_signed(
        &mut self, id: InstructionId, definition: &mir::Definition, mir: &mir::Mir, a: &mir::Value, op: &str,
        b: &mir::Value, signed: bool,
    ) {
        self.write_result_binding(id, definition);
        self.write_int_operand(a, signed, definition, mir);
        self.write(op);
        self.write_int_operand(b, signed, definition, mir);
        self.write(";");
    }

    /// Write `(intN_t)value` / `(uintN_t)value`, choosing the same-width type of the requested
    /// signedness so the following operation interprets the bits correctly. Char operands (a
    /// signed/unsigned compare on a `char`) are cast to the matching 1-byte type; anything else
    /// is written as-is.
    fn write_int_operand(&mut self, value: &mir::Value, signed: bool, definition: &mir::Definition, mir: &mir::Mir) {
        match mir.type_of_value(value, definition) {
            mir::Type::Primitive(mir::PrimitiveType::Int(kind)) => {
                let _ = write!(self.current_item, "({})", int_kind_with_sign(kind, signed));
            },
            mir::Type::Primitive(mir::PrimitiveType::Char) => {
                self.write(if signed { "(int8_t)" } else { "(uint8_t)" });
            },
            _ => {},
        }
        self.write_value(value, mir);
    }

    fn write_instruction(
        &mut self, id: InstructionId, instruction: &mir::Instruction, definition: &mir::Definition, mir: &mir::Mir,
    ) {
        match instruction {
            mir::Instruction::Call { function, arguments } => {
                self.write_result_binding(id, definition);
                // Emit a direct call when the function position resolves to a known definition: a
                // free function, or an operator/method stored in a constant ability-dictionary
                // global. cc can inline a direct call but not an opaque call through a function
                // pointer loaded from a mutable global, so this is where most of the C backend's
                // per-operation overhead is removed.
                match resolve_function_id(*function, definition, mir, RESOLVE_FUEL)
                    .filter(|target| mir.get_name(*target).is_some())
                {
                    Some(target) => self.write_value(&mir::Value::Definition(target), mir),
                    None => self.write_value(function, mir),
                }
                self.write("(");
                for (i, argument) in arguments.iter().enumerate() {
                    if i != 0 {
                        self.write(", ");
                    }
                    self.write_value(argument, mir);
                }
                self.write(");");
            },
            mir::Instruction::IndexTuple { tuple, index } => {
                let result = definition.instruction_result_type(id);
                if matches!(result, mir::Type::Array { .. }) {
                    // C can't assign an array, so copy the field out with memcpy instead.
                    self.write_declarator(result, &|this| {
                        let _ = write!(this.current_item, "{id}");
                    });
                    let _ = write!(self.current_item, "; memcpy({id}, ");
                    self.write_value(tuple, mir);
                    let _ = write!(self.current_item, "._{index}, sizeof({id}));");
                } else {
                    self.write_result_binding(id, definition);
                    self.write_value(tuple, mir);
                    let _ = write!(self.current_item, "._{index};");
                }
            },
            mir::Instruction::MakeBytes(bytes) => {
                // Embed the bytes as a static array and return a pointer to it.
                let name = format!("{id}_bytes");
                self.write_byte_array(&name, bytes);
                let _ = write!(self.current_item, " void* {id} = {name};");
            },
            mir::Instruction::MakeTuple(values) => {
                let result = definition.instruction_result_type(id);
                let mir::Type::Tuple(element_types) = result else {
                    panic!("MakeTuple result type is not a tuple: {result}")
                };
                // C can neither brace-initialize nor assign an array member from an array lvalue, so
                // when any element is an array, declare the tuple and fill each field individually:
                // array fields via memcpy, the rest by assignment.
                if element_types.iter().any(|t| matches!(t, mir::Type::Array { .. })) {
                    self.write_declarator(result, &|this| {
                        let _ = write!(this.current_item, "{id}");
                    });
                    self.write(";");
                    for (i, value) in values.iter().enumerate() {
                        if matches!(element_types[i], mir::Type::Array { .. }) {
                            let _ = write!(self.current_item, " memcpy({id}._{i}, ");
                            self.write_value(value, mir);
                            let _ = write!(self.current_item, ", sizeof({id}._{i}));");
                        } else {
                            let _ = write!(self.current_item, " {id}._{i} = ");
                            self.write_value(value, mir);
                            self.write(";");
                        }
                    }
                } else {
                    self.write_result_binding(id, definition);
                    self.write("{");
                    for (i, value) in values.iter().enumerate() {
                        if i != 0 {
                            self.write(", ");
                        }
                        self.write_value(value, mir);
                    }
                    self.write("};");
                }
            },
            mir::Instruction::MakeArray(values) => {
                let result = definition.instruction_result_type(id);
                let mir::Type::Array { element, .. } = result else {
                    panic!("MakeArray result type is not an array: {result}")
                };

                // Nested arrays in C decay into pointers so declare the array and memcpy each element
                if matches!(element.as_ref(), mir::Type::Array { .. }) {
                    self.write_declarator(result, &|this| {
                        let _ = write!(this.current_item, "{id}");
                    });
                    self.write(";");
                    for (i, value) in values.iter().enumerate() {
                        let _ = write!(self.current_item, " memcpy({id}[{i}], ");
                        self.write_value(value, mir);
                        let _ = write!(self.current_item, ", sizeof({id}[{i}]));");
                    }
                } else {
                    self.write_result_binding(id, definition);
                    self.write("{");
                    for (i, value) in values.iter().enumerate() {
                        if i != 0 {
                            self.write(", ");
                        }
                        self.write_value(value, mir);
                    }
                    self.write("};");
                }
            },
            mir::Instruction::StackAlloc(value) => {
                let typ = mir.type_of_value(value, definition);
                self.write_declarator(&typ, &|this| {
                    let _ = write!(this.current_item, "{id}_slot");
                });
                if matches!(typ, mir::Type::Array { .. }) {
                    // Arrays must be memcpy'd in C to be moved
                    let _ = write!(self.current_item, "; memcpy({id}_slot, ");
                    self.write_value(value, mir);
                    let _ = write!(self.current_item, ", sizeof({id}_slot));");
                } else {
                    self.write(" = ");
                    self.write_value(value, mir);
                    self.write(";");
                }
                let _ = write!(self.current_item, " void* {id} = &{id}_slot;");
            },
            mir::Instruction::StackAllocUninit(typ) => {
                self.write_declarator(typ, &|this| {
                    let _ = write!(this.current_item, "{id}_slot");
                });
                let _ = write!(self.current_item, "; void* {id} = &{id}_slot;");
            },
            mir::Instruction::AllocShared(value) => {
                let typ = mir.type_of_value(value, definition);
                let _ = write!(self.current_item, "void* {id} = malloc(sizeof(");
                self.write_type(&typ, "");
                self.write(")); *(");
                self.write_type(&typ, "");
                let _ = write!(self.current_item, "*){id} = ");
                self.write_value(value, mir);
                self.write(";");
            },
            mir::Instruction::Store { pointer, value } => {
                let typ = mir.type_of_value(value, definition);
                self.write("*(");
                self.write_type(&typ, "");
                self.write("*)");
                self.write_value(pointer, mir);
                self.write(" = ");
                self.write_value(value, mir);
                let _ = write!(self.current_item, "; Unit {id} = (Unit){{0}};");
            },
            mir::Instruction::GetFieldPtr { struct_ptr, struct_type, index } => {
                let _ = write!(self.current_item, "void* {id} = (void*)&((");
                self.write_type(struct_type, "");
                self.write("*)");
                self.write_value(struct_ptr, mir);
                let _ = write!(self.current_item, ")->_{index};");
            },
            mir::Instruction::Transmute(value) => {
                // C can't reinterpret an rvalue's bits directly, so round-trip through memcpy.
                let source = mir.type_of_value(value, definition);
                self.write_declarator(&source, &|this| {
                    let _ = write!(this.current_item, "{id}_src");
                });
                self.write(" = ");
                self.write_value(value, mir);
                self.write("; ");
                let result = definition.instruction_result_type(id);
                self.write_declarator(result, &|this| {
                    let _ = write!(this.current_item, "{id}");
                });
                let _ = write!(self.current_item, "; memcpy(&{id}, &{id}_src, sizeof(");
                self.write_type(result, "");
                self.write("));");
            },
            mir::Instruction::Id(value) => {
                self.write_result_binding(id, definition);
                self.write_value(value, mir);
                self.write(";");
            },
            mir::Instruction::Extern(name) => {
                let typ = definition.instruction_result_type(id).clone();
                self.emit_extern_declaration(name, &typ);
                self.write_result_binding(id, definition);
                self.write(name);
                self.write(";");
            },
            mir::Instruction::AtomicLoad { pointer, ordering } => {
                let result = definition.instruction_result_type(id).clone();
                self.write_result_binding(id, definition);
                self.write("__atomic_load_n((");
                self.write_type(&result, "");
                self.write("*)");
                self.write_value(pointer, mir);
                let _ = write!(self.current_item, ", {});", c_atomic_order(*ordering));
            },
            mir::Instruction::AtomicStore { pointer, value, ordering } => {
                let typ = mir.type_of_value(value, definition);
                self.write("__atomic_store_n((");
                self.write_type(&typ, "");
                self.write("*)");
                self.write_value(pointer, mir);
                self.write(", ");
                self.write_value(value, mir);
                let _ = write!(self.current_item, ", {}); Unit {id} = (Unit){{0}};", c_atomic_order(*ordering));
            },
            mir::Instruction::AtomicRmw { op, pointer, value, ordering } => {
                let typ = mir.type_of_value(value, definition);
                let func = match op {
                    mir::AtomicRmwOp::Xchg => "__atomic_exchange_n",
                    mir::AtomicRmwOp::Add => "__atomic_fetch_add",
                    mir::AtomicRmwOp::Sub => "__atomic_fetch_sub",
                    mir::AtomicRmwOp::And => "__atomic_fetch_and",
                    mir::AtomicRmwOp::Or => "__atomic_fetch_or",
                    mir::AtomicRmwOp::Xor => "__atomic_fetch_xor",
                };
                self.write_result_binding(id, definition);
                let _ = write!(self.current_item, "{func}((");
                self.write_type(&typ, "");
                self.write("*)");
                self.write_value(pointer, mir);
                self.write(", ");
                self.write_value(value, mir);
                let _ = write!(self.current_item, ", {});", c_atomic_order(*ordering));
            },
            mir::Instruction::AtomicCmpxchg { pointer, expected, desired, success, failure } => {
                let typ = mir.type_of_value(expected, definition);
                self.write_type(&typ, "");
                let _ = write!(self.current_item, " {id}_exp = ");
                self.write_value(expected, mir);
                self.write("; __atomic_compare_exchange_n((");
                self.write_type(&typ, "");
                self.write("*)");
                self.write_value(pointer, mir);
                let _ = write!(self.current_item, ", &{id}_exp, ");
                self.write_value(desired, mir);
                let _ = write!(
                    self.current_item,
                    ", 0, {}, {}); ",
                    c_atomic_order(*success),
                    c_atomic_order(*failure)
                );
                self.write_type(&typ, "");
                let _ = write!(self.current_item, " {id} = {id}_exp;");
            },
            mir::Instruction::Deref(value) => {
                let result = definition.instruction_result_type(id);
                if matches!(result, mir::Type::Array { .. }) {
                    // Arrays degrade into pointers in C, so copy out of it with memcpy.
                    self.write_declarator(result, &|this| {
                        let _ = write!(this.current_item, "{id}");
                    });
                    let _ = write!(self.current_item, "; memcpy({id}, ");
                    self.write_value(value, mir);
                    let _ = write!(self.current_item, ", sizeof({id}));");
                } else if matches!(result, mir::Type::Function(_)) {
                    // A function pointer needs its extra `*` inside the declarator: `T (**)(args)`.
                    self.write_result_binding(id, definition);
                    self.write("*(");
                    self.write_declarator(result, &|this| this.write("*"));
                    self.write(")");
                    self.write_value(value, mir);
                    self.write(";");
                } else {
                    self.write_result_binding(id, definition);
                    self.write("*(");
                    self.write_type(result, "");
                    self.write("*)");
                    self.write_value(value, mir);
                    self.write(";");
                }
            },

            mir::Instruction::AddInt(a, b) => self.write_binary(id, definition, mir, a, " + ", b),
            mir::Instruction::OverflowingAddInt(a, b) => {
                self.write_overflowing_binary(id, definition, mir, OverflowingIntOp::Add, a, b)
            },
            mir::Instruction::AddFloat(a, b) => self.write_binary(id, definition, mir, a, " + ", b),
            mir::Instruction::SubInt(a, b) => self.write_binary(id, definition, mir, a, " - ", b),
            mir::Instruction::OverflowingSubInt(a, b) => {
                self.write_overflowing_binary(id, definition, mir, OverflowingIntOp::Sub, a, b)
            },
            mir::Instruction::SubFloat(a, b) => self.write_binary(id, definition, mir, a, " - ", b),
            mir::Instruction::MulInt(a, b) => self.write_binary(id, definition, mir, a, " * ", b),
            mir::Instruction::OverflowingMulInt(a, b) => {
                self.write_overflowing_binary(id, definition, mir, OverflowingIntOp::Mul, a, b)
            },
            mir::Instruction::MulFloat(a, b) => self.write_binary(id, definition, mir, a, " * ", b),
            mir::Instruction::DivSigned(a, b) => self.write_binary_signed(id, definition, mir, a, " / ", b, true),
            mir::Instruction::DivUnsigned(a, b) => self.write_binary_signed(id, definition, mir, a, " / ", b, false),
            mir::Instruction::DivFloat(a, b) => self.write_binary(id, definition, mir, a, " / ", b),
            mir::Instruction::ModSigned(a, b) => self.write_binary_signed(id, definition, mir, a, " % ", b, true),
            mir::Instruction::ModUnsigned(a, b) => self.write_binary_signed(id, definition, mir, a, " % ", b, false),
            mir::Instruction::ModFloat(a, b) => {
                self.write_result_binding(id, definition);
                self.write("fmod(");
                self.write_value(a, mir);
                self.write(", ");
                self.write_value(b, mir);
                self.write(");");
            },
            mir::Instruction::LessSigned(a, b) => self.write_binary_signed(id, definition, mir, a, " < ", b, true),
            mir::Instruction::LessUnsigned(a, b) => self.write_binary_signed(id, definition, mir, a, " < ", b, false),
            mir::Instruction::LessFloat(a, b) => self.write_binary(id, definition, mir, a, " < ", b),
            mir::Instruction::EqInt(a, b) => self.write_binary(id, definition, mir, a, " == ", b),
            mir::Instruction::EqFloat(a, b) => self.write_binary(id, definition, mir, a, " == ", b),
            mir::Instruction::BitwiseAnd(a, b) => self.write_binary(id, definition, mir, a, " & ", b),
            mir::Instruction::BitwiseOr(a, b) => self.write_binary(id, definition, mir, a, " | ", b),
            mir::Instruction::BitwiseXor(a, b) => self.write_binary(id, definition, mir, a, " ^ ", b),
            mir::Instruction::BitwiseNot(value) => {
                self.write_result_binding(id, definition);
                self.write("~");
                self.write_value(value, mir);
                self.write(";");
            },

            // Sign/zero extension force the source's signedness; the assignment to the (wider)
            // result type then sign- or zero-extends as C's conversion rules dictate.
            mir::Instruction::SignExtend(value) => {
                self.write_result_binding(id, definition);
                self.write_int_operand(value, true, definition, mir);
                self.write(";");
            },
            mir::Instruction::ZeroExtend(value) => {
                self.write_result_binding(id, definition);
                self.write_int_operand(value, false, definition, mir);
                self.write(";");
            },

            // The remaining conversions are plain casts: assigning to the result-typed variable
            // performs the int<->float / narrowing / float-width conversion directly.
            mir::Instruction::SignedToFloat(value)
            | mir::Instruction::UnsignedToFloat(value)
            | mir::Instruction::FloatToSigned(value)
            | mir::Instruction::FloatToUnsigned(value)
            | mir::Instruction::FloatPromote(value)
            | mir::Instruction::FloatDemote(value)
            | mir::Instruction::Truncate(value) => {
                let result = definition.instruction_result_type(id);
                self.write_result_binding(id, definition);
                self.write("(");
                self.write_type(result, "");
                self.write(")");
                self.write_value(value, mir);
                self.write(";");
            },

            // The following are removed by earlier passes before codegen, mirroring the LLVM backend.
            mir::Instruction::CallClosure { .. } => unreachable!("Instruction::CallClosure remaining in C codegen"),
            mir::Instruction::Perform { .. } => unreachable!("Instruction::Perform remaining in C codegen"),
            mir::Instruction::Handle { .. } => unreachable!("Instruction::Handle remaining in C codegen"),
            mir::Instruction::Capability => unreachable!("Instruction::Capability remaining in C codegen"),
            mir::Instruction::PackClosure { .. } => unreachable!("Instruction::PackClosure remaining in C codegen"),
            mir::Instruction::Instantiate(..) => unreachable!("Instruction::Instantiate remaining in C codegen"),
            mir::Instruction::SizeOf(_) => todo!("SizeOf should be removed by monomorphization"),
            mir::Instruction::ArrayLen(_) => todo!("ArrayLen should be removed by monomorphization"),
        }
    }

    /// Forward-declare an external symbol referenced by an [mir::Instruction::Extern]. Function
    /// types become prototypes; other types become `extern` variable declarations.
    fn emit_extern_declaration(&mut self, name: &str, typ: &mir::Type) {
        // These are already declared in [CFile::add_starter_items], redeclaring would conflict.
        if matches!(name, "malloc" | "memcpy" | "fmod") {
            return;
        }
        let declaration = self.capture(|this| {
            match typ {
                mir::Type::Function(function) => {
                    this.write_declarator(&function.return_type, &|this| {
                        this.write(name);
                        this.write("(");
                        for (i, parameter) in function.parameters.iter().enumerate() {
                            if i != 0 {
                                this.write(", ");
                            }
                            this.write_type(parameter, "");
                        }
                        this.write(")");
                    });
                },
                _ => {
                    this.write("extern ");
                    this.write_type(typ, name);
                },
            }
            this.write(";");
        });
        self.file.add_function_declaration(&declaration);
    }
}

/// Maximum depth when tracing a call's function-position value (or a tuple field) back to a
/// definition. A safety bound against cyclic global references; real chains are only a few deep.
const RESOLVE_FUEL: u32 = 64;

/// The value a global definition holds: the operand of its `Result`/`Return` terminator.
fn global_result_value(definition: &mir::Definition) -> Option<mir::Value> {
    match definition.entry_block().terminator.as_ref()? {
        mir::TerminatorInstruction::Result(value) | mir::TerminatorInstruction::Return(value) => Some(*value),
        _ => None,
    }
}

/// Trace `value`, used in function position, back to the [DefinitionId] of the concrete function it
/// always refers to, if that is statically known. Looks through `Id`/`Instantiate`, through
/// `IndexTuple` projections of constant tuples (`MakeTuple`s and tuple-typed globals such as ability
/// dictionaries), and through global *value* definitions. Returns `None` when the target is
/// genuinely dynamic, e.g. a closure passed in as a parameter.
fn resolve_function_id<'mir>(
    value: mir::Value, definition: &'mir mir::Definition, mir: &'mir mir::Mir, fuel: u32,
) -> Option<DefinitionId> {
    if fuel == 0 {
        return None;
    }
    match value {
        mir::Value::Definition(id) => match mir.definitions.get(&id) {
            // A global *value* (e.g. an ability dictionary): resolve what it holds.
            Some(global) if global.is_global() => {
                resolve_function_id(global_result_value(global)?, global, mir, fuel - 1)
            },
            // A function definition (or an extern, absent from `definitions`): the target itself.
            _ => Some(id),
        },
        mir::Value::InstructionResult(instruction) => match &definition.instructions[instruction] {
            mir::Instruction::Id(inner) => resolve_function_id(*inner, definition, mir, fuel - 1),
            mir::Instruction::Instantiate(id, _) => Some(*id),
            mir::Instruction::IndexTuple { tuple, index } => {
                let (field_def, field_value) = resolve_tuple_field(*tuple, *index, definition, mir, fuel - 1)?;
                resolve_function_id(field_value, field_def, mir, fuel - 1)
            },
            _ => None,
        },
        _ => None,
    }
}

/// Resolve field `index` of `tuple` to the [mir::Value] stored there, paired with the definition
/// that value lives in (which differs from `definition` when `tuple` is a global). Handles
/// `MakeTuple`, `Id`-chains, nested `IndexTuple`s, and tuple-typed globals.
fn resolve_tuple_field<'mir>(
    tuple: mir::Value, index: u32, definition: &'mir mir::Definition, mir: &'mir mir::Mir, fuel: u32,
) -> Option<(&'mir mir::Definition, mir::Value)> {
    if fuel == 0 {
        return None;
    }
    match tuple {
        mir::Value::Definition(id) => {
            let global = mir.definitions.get(&id)?;
            if !global.is_global() {
                return None;
            }
            resolve_tuple_field(global_result_value(global)?, index, global, mir, fuel - 1)
        },
        mir::Value::InstructionResult(instruction) => match &definition.instructions[instruction] {
            mir::Instruction::MakeTuple(values) => values.get(index as usize).map(|value| (definition, *value)),
            mir::Instruction::Id(inner) => resolve_tuple_field(*inner, index, definition, mir, fuel - 1),
            mir::Instruction::IndexTuple { tuple: inner, index: inner_index } => {
                let (inner_def, inner_value) = resolve_tuple_field(*inner, *inner_index, definition, mir, fuel - 1)?;
                resolve_tuple_field(inner_value, index, inner_def, mir, fuel - 1)
            },
            _ => None,
        },
        _ => None,
    }
}

/// The GCC/Clang `__ATOMIC_*` memory-order constant for a given ordering.
/// TODO: Support more C compilers
fn c_atomic_order(ordering: mir::AtomicOrdering) -> &'static str {
    match ordering {
        mir::AtomicOrdering::Relaxed => "__ATOMIC_RELAXED",
        mir::AtomicOrdering::Acquire => "__ATOMIC_ACQUIRE",
        mir::AtomicOrdering::Release => "__ATOMIC_RELEASE",
        mir::AtomicOrdering::AcqRel => "__ATOMIC_ACQ_REL",
        mir::AtomicOrdering::SeqCst => "__ATOMIC_SEQ_CST",
    }
}

/// The C spelling of an integer kind, used both for type declarations and casts.
fn int_kind_c_name(kind: IntegerKind) -> &'static str {
    match kind {
        IntegerKind::I8 => "int8_t",
        IntegerKind::I16 => "int16_t",
        IntegerKind::I32 => "int32_t",
        IntegerKind::I64 => "int64_t",
        IntegerKind::Isz => "ptrdiff_t",
        IntegerKind::U8 => "uint8_t",
        IntegerKind::U16 => "uint16_t",
        IntegerKind::U32 => "uint32_t",
        IntegerKind::U64 => "uint64_t",
        IntegerKind::Usz => "size_t",
    }
}

/// The same-width counterpart of `kind` with the given signedness, used to force the
/// correct interpretation of an operand before a sign-sensitive operation (division,
/// remainder, comparison, or a zero/sign extend).
fn int_kind_with_sign(kind: IntegerKind, signed: bool) -> &'static str {
    let signed_kind = match kind {
        IntegerKind::I8 | IntegerKind::U8 => IntegerKind::I8,
        IntegerKind::I16 | IntegerKind::U16 => IntegerKind::I16,
        IntegerKind::I32 | IntegerKind::U32 => IntegerKind::I32,
        IntegerKind::I64 | IntegerKind::U64 => IntegerKind::I64,
        IntegerKind::Isz | IntegerKind::Usz => IntegerKind::Isz,
    };
    let unsigned_kind = match kind {
        IntegerKind::I8 | IntegerKind::U8 => IntegerKind::U8,
        IntegerKind::I16 | IntegerKind::U16 => IntegerKind::U16,
        IntegerKind::I32 | IntegerKind::U32 => IntegerKind::U32,
        IntegerKind::I64 | IntegerKind::U64 => IntegerKind::U64,
        IntegerKind::Isz | IntegerKind::Usz => IntegerKind::Usz,
    };
    int_kind_c_name(if signed { signed_kind } else { unsigned_kind })
}

/// These type aliases are defined in [CFile::add_starter_items]
fn float_kind_c_name(kind: FloatKind) -> &'static str {
    match kind {
        FloatKind::F32 => "ante_f32",
        FloatKind::F64 => "ante_f64",
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

    #[test]
    fn integer_literals_are_valid_c() {
        use crate::mir::IntConstant;
        let cases = [
            (IntConstant::I32(5), "(int32_t)5"),
            (IntConstant::I8(-3), "(int8_t)-3"),
            (IntConstant::U8(200), "(uint8_t)200"),
            (IntConstant::I64(-3), "(int64_t)-3ll"),
            (IntConstant::U64(9), "(uint64_t)9ull"),
            (IntConstant::Usz(9), "(size_t)9ull"),
            (IntConstant::Isz(-1), "(ptrdiff_t)-1ll"),
            // The minimum can't be written as a negated literal without overflowing `ll`.
            (IntConstant::I64(i64::MIN), "(int64_t)(-9223372036854775807ll - 1)"),
            (IntConstant::Isz(isize::MIN), "(ptrdiff_t)(-9223372036854775807ll - 1)"),
        ];
        for (constant, expected) in cases {
            let mut builder = Builder::default();
            builder.write_integer_constant(constant);
            assert_eq!(builder.current_item, expected);
        }
    }

    #[test]
    fn mangled_names_are_valid_identifiers() {
        use crate::mir::DefinitionId;
        // Operator names sanitize their non-identifier characters to `_`; the id keeps them unique.
        let mut builder = Builder::default();
        builder.write_mangled_name(">=", DefinitionId(9));
        assert_eq!(builder.current_item, "___9");

        let mut builder = Builder::default();
        builder.write_mangled_name("main", DefinitionId(5));
        assert_eq!(builder.current_item, "main_5");
    }
}
