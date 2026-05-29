/// The output .c file.
/// This file is divided into several sections to ensure everything is declared before it is used.
#[derive(Default)]
pub(super) struct CFile {
    includes: String,
    type_declarations: String,
    type_definitions: String,
    function_declarations: String,
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
                + self.function_definitions.len()
                + 4, // 4 for the newlines separating each.
        );
        let capacity = result.capacity();
        result += "\n";
        result += &self.type_declarations;
        result += "\n";
        result += &self.type_definitions;
        result += "\n";
        result += &self.function_declarations;
        result += "\n";
        result += &self.function_definitions;
        // Ensure the capacity estimate was correct
        assert_eq!(capacity, result.capacity());
        result
    }

    pub(super) fn add_include(&mut self, include: &str) {
        self.includes += include;
        self.includes += "\n";
    }

    pub(super) fn add_type_declaration(&mut self, decl: &str) {
        self.type_declarations+= decl;
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
        self.function_definitions += &other.function_definitions;
        self
    }
}
