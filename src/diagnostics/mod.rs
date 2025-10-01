use std::{cmp::Ordering, path::PathBuf, sync::Arc};

use colored::{ColoredString, Colorize};
use serde::{Deserialize, Serialize};

use crate::{incremental::Db, lexer::token::Token, type_inference::errors::TypeErrorKind};

mod location;

pub use location::*;

pub type Errors = Vec<Diagnostic>;

/// Any diagnostic that the compiler can issue
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Diagnostic {
    // TODO: `message` could be an enum to save allocation costs
    ParserExpected { message: String, actual: Token, location: Location },
    ExpectedPathForImport { location: Arc<LocationData> },
    NameAlreadyInScope { name: Arc<String>, first_location: Location, second_location: Location },
    ImportedNameAlreadyInScope { name: Arc<String>, first_location: Location, second_location: Location },
    UnknownImportFile { crate_name: String, module_name: Arc<PathBuf>, location: Location },
    NameNotInScope { name: Arc<String>, location: Location },
    ExpectedType { actual: String, expected: String, location: Location },
    RecursiveType { typ: String, location: Location },
    NamespaceNotFound { name: String, location: Location },
    NameNotFound { name: Arc<String>, location: Location },
    MethodDeclaredOnUnknownType { name: Arc<String>, location: Location },
    LiteralUsedAsName { location: Location },
    ValueExpected { location: Location, typ: Arc<String> },
    TypeError { actual: String, expected: String, kind: TypeErrorKind, location: Location },
    FunctionArgCountMismatch { actual: usize, expected: usize, location: Location },
}

impl Ord for Diagnostic {
    fn cmp(&self, other: &Self) -> Ordering {
        let order = self.location().cmp(other.location());
        if order != Ordering::Equal {
            return order;
        }
        self.message().cmp(&other.message())
    }
}

impl PartialOrd for Diagnostic {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Diagnostic {
    fn kind(&self) -> DiagnosticKind {
        use Diagnostic::*;
        match self {
            NameAlreadyInScope { .. } | ImportedNameAlreadyInScope { .. } => DiagnosticKind::Warning,
            _ => DiagnosticKind::Error,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Diagnostic::ParserExpected { message, actual, location: _ } => {
                if actual.to_string().contains(" ") {
                    format!("Expected {message} but found {actual}")
                } else {
                    format!("Expected {message} but found `{actual}`")
                }
            },
            Diagnostic::ExpectedPathForImport { .. } => {
                "Imports paths should have at least 2 components (e.g. `Foo.Bar`), otherwise nothing gets imported"
                    .to_string()
            },
            Diagnostic::NameAlreadyInScope { name, first_location: _, second_location: _ } => {
                format!("`{name}` was already defined")
            },
            Diagnostic::ImportedNameAlreadyInScope { name, first_location: _, second_location: _ } => {
                format!("This imports `{name}`, which has already been defined")
            },
            Diagnostic::UnknownImportFile { crate_name, module_name, location: _ } => {
                if module_name.display().to_string().is_empty() {
                    format!("Could not find crate `{crate_name}`")
                } else {
                    format!("Could not find module `{}` in crate `{crate_name}`", module_name.display())
                }
            },
            Diagnostic::NameNotInScope { name, location: _ } => {
                format!("`{name}` is not defined, was it a typo?")
            },
            Diagnostic::ExpectedType { actual, expected, location: _ } => {
                format!("Expected type `{expected}` but found `{actual}`")
            },
            Diagnostic::RecursiveType { typ, location: _ } => {
                format!("Binding here would create an infinitely recursive type with `{typ}`")
            },
            Diagnostic::NamespaceNotFound { name, location: _ } => {
                format!("Namespace `{name}` not found in path")
            },
            Diagnostic::NameNotFound { name, location: _ } => {
                format!("`{name}` not found in scope")
            },
            Diagnostic::MethodDeclaredOnUnknownType { name, location: _ } => {
                format!("Methods can only be defined on types declared within the same file, which `{name}` was not")
            },
            Diagnostic::LiteralUsedAsName { location: _ } => {
                "Expected a definition name but found a literal".to_string()
            },
            Diagnostic::ValueExpected { location: _, typ } => {
                format!("Expected a value but `{}` is a type", typ)
            },
            Diagnostic::TypeError { actual, expected, kind, location: _ } => kind.message(actual, expected),
            Diagnostic::FunctionArgCountMismatch { actual, expected, location: _ } => {
                let s = if *actual == 1 { "" } else { "s" };
                let was = if *expected == 1 { "was" } else { "were" };
                format!("Function accepts {actual} parameter{s} but {expected} {was} expected")
            },
        }
    }

    /// The primary source location of this diagnostic
    pub fn location(&self) -> &Location {
        match self {
            Diagnostic::ParserExpected { location, .. }
            | Diagnostic::ExpectedPathForImport { location }
            | Diagnostic::NameAlreadyInScope { second_location: location, .. }
            | Diagnostic::ImportedNameAlreadyInScope { second_location: location, .. }
            | Diagnostic::UnknownImportFile { location, .. }
            | Diagnostic::NameNotInScope { location, .. }
            | Diagnostic::ExpectedType { location, .. }
            | Diagnostic::RecursiveType { location, .. }
            | Diagnostic::NamespaceNotFound { location, .. }
            | Diagnostic::MethodDeclaredOnUnknownType { location, .. }
            | Diagnostic::LiteralUsedAsName { location }
            | Diagnostic::ValueExpected { location, .. }
            | Diagnostic::TypeError { location, .. }
            | Diagnostic::FunctionArgCountMismatch { location, .. }
            | Diagnostic::NameNotFound { location, .. } => location,
        }
    }

    fn marker(&self, show_color: bool) -> ColoredString {
        match self.kind() {
            DiagnosticKind::Error => self.color("error:", show_color),
            DiagnosticKind::Warning => self.color("warning:", show_color),
            DiagnosticKind::Note => self.color("note:", show_color),
        }
    }

    /// Color the given string in either the error, warning, or note color
    fn color(&self, msg: &str, show_color: bool) -> ColoredString {
        match (show_color, self.kind()) {
            (false, _) => msg.normal(),
            (_, DiagnosticKind::Error) => msg.red(),
            (_, DiagnosticKind::Warning) => msg.yellow(),
            (_, DiagnosticKind::Note) => msg.purple(),
        }
    }

    pub fn display(self, show_color: bool, compiler: &Db) -> DiagnosticDisplay {
        DiagnosticDisplay { diagnostic: self, compiler, show_color }
    }

    fn format(&self, f: &mut std::fmt::Formatter, show_color: bool, compiler: &Db) -> std::fmt::Result {
        let location = self.location();
        let start = location.span.start;

        let file = location.file_id.get(compiler);
        let relative_path = os_agnostic_display_path(&file.path, show_color);

        writeln!(
            f,
            "{}:{}:{}\t{} {}",
            relative_path,
            start.line_number,
            start.column_number,
            self.marker(show_color),
            self.message()
        )?;

        let line = file.contents.lines().nth(start.line_number.max(1) as usize - 1).unwrap_or("");

        let start_column = start.column_number.max(1) as usize - 1;
        let length = location.span.end.byte_index - start.byte_index;

        // If the length continues to multiple lines, cut it short after the first line
        let length = length.min(line.len() - start_column);

        // write the first part of the line, then the erroring part in red, then the rest
        write!(f, "{}", &line[0..start_column])?;
        write!(f, "{}", self.color(&line[start_column..start_column + length], show_color))?;
        writeln!(f, "{}", &line[start_column + length..])?;

        // If we're not printing in color, print a `^^^` indicator to show where the error is.
        if !show_color {
            let padding = " ".repeat(start_column);
            let indicator = "^".repeat(length.max(1));
            writeln!(f, "{}{}", padding, indicator)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone)]
enum DiagnosticKind {
    Error,
    Warning,
    #[allow(unused)]
    Note,
}

/// Format the path in an OS-agnostic way. By default rust uses "/" on Unix
/// and "\" on windows as the path separator. This makes testing more
/// difficult and isn't needed for error reporting so we implement our own
/// path-Displaying here that is roughly the same as printing Unix paths.
fn os_agnostic_display_path(path: &std::path::Path, show_color: bool) -> ColoredString {
    let mut ret = String::new();

    for (i, component) in path.components().enumerate() {
        use std::path::Component;

        // Use / as the separator regardless of the host OS so
        // we can use the same tests for Linux/Mac/Windows
        if i != 0 && ret != "/" {
            ret += "/";
        }

        ret += match component {
            Component::CurDir => ".",
            Component::Normal(s) => s.to_str().expect("Path contains invalid utf-8"),
            Component::ParentDir => "..",
            Component::Prefix(_) => "",
            Component::RootDir => "/",
        }
    }

    if show_color {
        ret.italic()
    } else {
        ret.normal()
    }
}

pub struct DiagnosticDisplay<'a> {
    diagnostic: Diagnostic,
    compiler: &'a Db,
    show_color: bool,
}

impl std::fmt::Display for DiagnosticDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.diagnostic.format(f, self.show_color, self.compiler)
    }
}
