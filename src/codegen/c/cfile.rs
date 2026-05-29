use crate::mir::DefinitionId;

/// A global that cannot be initialized by a constant C initializer (it reads another global's
/// value). Its definition is emitted zero-initialized; `statement` assigns the real value at
/// startup. `deps` names the other deferred globals that must be assigned first.
pub(super) struct GlobalInitializer {
    pub id: DefinitionId,
    pub deps: Vec<DefinitionId>,
    pub statement: String,
}

/// The output .c file.
/// This file is divided into several sections to ensure everything is declared before it is used.
#[derive(Default)]
pub(super) struct CFile {
    includes: String,
    type_declarations: String,
    type_definitions: String,
    function_declarations: String,
    global_declarations: String,
    global_definitions: String,
    function_definitions: String,

    /// Runtime assignments for globals with non-constant initializers, run before `main`.
    global_initializers: Vec<GlobalInitializer>,
}

impl CFile {
    /// Consume the file, concatenating everything into a single contents string
    pub(super) fn into_contents(self) -> String {
        let mut result = self.includes;
        result.reserve_exact(
            self.type_declarations.len()
                + self.type_definitions.len()
                + self.function_declarations.len()
                + self.global_declarations.len()
                + self.global_definitions.len()
                + self.function_definitions.len()
                + 6, // 6 for the newlines separating each.
        );
        let capacity = result.capacity();
        result += "\n";
        result += &self.type_declarations;
        result += "\n";
        result += &self.type_definitions;
        result += "\n";
        result += &self.function_declarations;
        result += "\n";
        result += &self.global_declarations;
        result += "\n";
        result += &self.global_definitions;
        result += "\n";
        result += &self.function_definitions;
        // Ensure the capacity estimate was correct
        assert_eq!(capacity, result.capacity());
        result
    }

    // TODO: Should [super::write_cached_tuple_type] be changed to use this method?
    #[allow(unused)]
    pub(super) fn add_type_declaration(&mut self, decl: &str) {
        self.type_declarations += decl;
        self.type_declarations += "\n";
    }

    pub(super) fn add_type_definition(&mut self, def: &str) {
        self.type_definitions += def;
        self.type_definitions += "\n";
    }

    pub(super) fn add_function_declaration(&mut self, decl: &str) {
        self.function_declarations += decl;
        self.function_declarations += "\n";
    }

    pub(super) fn add_global_declaration(&mut self, decl: &str) {
        self.global_declarations += decl;
        self.global_declarations += "\n";
    }

    pub(super) fn add_global_definition(&mut self, def: &str) {
        self.global_definitions += def;
        self.global_definitions += "\n";
    }

    pub(super) fn add_function_definition(&mut self, def: &str) {
        self.function_definitions += def;
        self.function_definitions += "\n";
    }

    pub(super) fn add_global_initializer(&mut self, init: GlobalInitializer) {
        self.global_initializers.push(init);
    }

    /// Take the collected non-constant global initializers, leaving the list empty. Consumed by
    /// [super::build_c_file] to emit a startup function rather than written into `into_contents`.
    pub(super) fn take_global_initializers(&mut self) -> Vec<GlobalInitializer> {
        std::mem::take(&mut self.global_initializers)
    }

    /// Extend `self` with the contents of `other`
    pub(super) fn extend(mut self, other: CFile) -> CFile {
        self.includes += &other.includes;
        self.type_declarations += &other.type_declarations;
        self.type_definitions += &other.type_definitions;
        self.function_declarations += &other.function_declarations;
        self.global_declarations += &other.global_declarations;
        self.global_definitions += &other.global_definitions;
        self.function_definitions += &other.function_definitions;
        self.global_initializers.extend(other.global_initializers);
        self
    }

    /// Add some necessary items to this CFile that are needed by all Ante programs:
    /// the standard headers and runtime prototypes the generated code references, plus the
    /// `Unit` struct.
    pub(crate) fn add_starter_items(mut self) -> Self {
        // stdlib.h, string.h, math.h would conflict with `extern` statements in source code
        // which declare some of the same functions.
        self.includes += "#include <stdint.h>\n";
        self.includes += "#include <stddef.h>\n";
        self.includes += "#include <stdbool.h>\n";

        self.includes += "\
#if defined(__FLT32_MANT_DIG__) && defined(__FLT64_MANT_DIG__)
typedef _Float32 ante_f32;
typedef _Float64 ante_f64;
#else
typedef float ante_f32;
typedef double ante_f64;
#endif

#if defined(__GNUC__) || defined(__clang__)
#define ANTE_UNREACHABLE() __builtin_unreachable()
#define ANTE_INF() __builtin_inf()
#define ANTE_NAN() __builtin_nan(\"\")
#elif defined(_MSC_VER)
#include <math.h>
#define ANTE_UNREACHABLE() __assume(0)
#define ANTE_INF() INFINITY
#define ANTE_NAN() NAN
#else
#include <math.h>
#define ANTE_UNREACHABLE() ((void)0)
#define ANTE_INF() INFINITY
#define ANTE_NAN() NAN
#endif
";

        self.type_declarations += "typedef struct { char _unused; } Unit;\n";
        self.function_declarations += "void* malloc(size_t);\n";
        self.function_declarations += "void* memcpy(void*, void*, size_t);\n";
        self.function_declarations += "double fmod(double, double);\n";
        self
    }
}
