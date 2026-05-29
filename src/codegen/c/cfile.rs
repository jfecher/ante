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

    /// Extend `self` with the contents of `other`
    pub(super) fn extend(mut self, other: CFile) -> CFile {
        self.includes += &other.includes;
        self.type_declarations += &other.type_declarations;
        self.type_definitions += &other.type_definitions;
        self.function_declarations += &other.function_declarations;
        self.global_declarations += &other.global_declarations;
        self.global_definitions += &other.global_definitions;
        self.function_definitions += &other.function_definitions;
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

        self.type_declarations += "typedef struct {} Unit;\n";
        self.function_declarations += "void* malloc(size_t);\n";
        self.function_declarations += "void* memcpy(void*, void*, size_t);\n";
        self.function_declarations += "double fmod(double, double);\n";
        self
    }
}
