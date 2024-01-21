//! error/mod.rs - Defines the error, warning, and note macros
//! used to issue compiler errors. There is also an ErrorMessage type
//! for storing messages that may be issued later. Note that all issuing
//! an error does is print it to stderr and update the global ERROR_COUNT.
//!
//! Compiler passes are expected to continue even after issuing errors so
//! that as many can be issued as possible. A possible future improvement
//! would be to implement poisoning so that repeated errors are hidden.
pub mod location;
use crate::error::location::Location;

use colored::ColoredString;
use colored::*;
use std::cmp::{max, min};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::AtomicBool;

static COLORED_OUTPUT: AtomicBool = AtomicBool::new(true);

/// Every diagnostic that may be emitted by the compiler
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    /// These errors are always indicative of a compiler bug.
    /// They're preferred over panics in cases where we have concrete locations of the error.
    InternalError(/*message*/&'static str),

    // Parsing

    // Name Resolution
    //
    TypeVariableAlreadyInScope(/*type variable name*/String),
    ItemNotRequiredByTrait(/*item name*/String, /*trait name*/String),
    AlreadyInScope(/*item name*/String),
    PreviouslyDefinedHere(/*item name*/String),
    IncorrectConstructorArgCount(/*item name*/String, /*expected count*/usize, /*actual count*/usize),

    // This can be combined with IncorrectArgCount
    IncorrectImplTraitArgCount(/*Trait name*/String, /*expected count*/usize, /*actual count*/usize),
    NonIntegerType(/*type name*/String),
    NonFloatType(/*type name*/String),
    NotInScope(/*item kind*/&'static str, /*item name*/String),
    CouldNotFindModule(/*module path*/String),

    // Should this be combined with `NotInScope`?
    NoDeclarationFoundInScope(/*variable name*/String),
    CouldNotOpenFileForImport(/*file path*/PathBuf),
    MissingImplDefinition(/*definition name*/String),
    EffectsMustBeFunctions,
    InvalidHandlerPattern,
    NotAnEffect(/*item name*/String),
    HandlerMissingCases(/*missing effect cases*/ Vec<String>),

    // Type Checking
    //
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorType {
    Error,
    Warning,
    Note,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            DiagnosticKind::InternalError(message) => {
                write!(f, "Internal compiler error: {}", message)
            },
            DiagnosticKind::TypeVariableAlreadyInScope(name) => {
                write!(f, "Type variable '{}' is already in scope", name)
            },
            DiagnosticKind::ItemNotRequiredByTrait(item, trait_name) => {
                write!(f, "{} is not required by {}", item, trait_name)
            },
            DiagnosticKind::AlreadyInScope(item) => {
                write!(f, "{} is already in scope", item)
            },
            DiagnosticKind::PreviouslyDefinedHere(item) => {
                write!(f, "{} previously defined here", item)
            },
            DiagnosticKind::IncorrectConstructorArgCount(item, expected, actual) => {
                let plural_s = if *expected == 1 { "" } else { "s" };
                let is_are = if *actual == 1 { "is" } else { "are" };
                write!(f, "Type {} expects {} argument{}, but {} {} given here", item, expected, plural_s, actual, is_are)
            },
            DiagnosticKind::IncorrectImplTraitArgCount(trait_name, expected, actual) => {
                let plural_s = if *expected == 1 { "" } else { "s" };
                write!(f, "impl has {} type argument{} but {} requires {}", expected, plural_s, trait_name, actual)
            },
            DiagnosticKind::NonIntegerType(typename) => {
                write!(f, "Type {} is not an integer type", typename)
            },
            DiagnosticKind::NonFloatType(typename) => {
                write!(f, "Type {} is not a float type", typename)
            }
            DiagnosticKind::NotInScope(item_kind, item) => {
                write!(f, "{} {} was not found in scope", item_kind, item)
            },
            DiagnosticKind::CouldNotFindModule(module) => {
                write!(f, "Could not find module `{}`", module)
            },
            DiagnosticKind::NoDeclarationFoundInScope(item) => {
                write!(f, "No declaration for `{}` was found in scope", item)
            },
            DiagnosticKind::CouldNotOpenFileForImport(path) => {
                write!(f, "Couldn't open file for import: {}.an", path.display())
            },
            DiagnosticKind::MissingImplDefinition(definition_name) => {
                write!(f, "impl is missing a definition for {}", definition_name)
            },
            DiagnosticKind::EffectsMustBeFunctions => {
                write!(f, "Only function types are allowed in effect declarations")
            },
            DiagnosticKind::InvalidHandlerPattern => {
                write!(f, "Invalid handle pattern. Handle patterns must be an effect function call or a return expression")
            }
            DiagnosticKind::NotAnEffect(item) => {
                write!(f, "{} is not an effect", item)
            },
            DiagnosticKind::HandlerMissingCases(cases) => {
                let plural_s = if cases.len() == 1 { "" } else { "s" };
                let cases = cases.join(", ");
                write!(f, "Handler is missing {} case{}: {}", cases.len(), plural_s, cases)
            }
        }
    }
}

impl DiagnosticKind {
    pub fn error_type(&self) -> ErrorType {
        use ErrorType::*;

        match &self {
            DiagnosticKind::InternalError(_) => Error,
            DiagnosticKind::TypeVariableAlreadyInScope(_) => Error,
            DiagnosticKind::ItemNotRequiredByTrait(..) => Error,
            DiagnosticKind::AlreadyInScope(_) => Error,
            DiagnosticKind::PreviouslyDefinedHere(_) => Note,
            DiagnosticKind::IncorrectConstructorArgCount(..) => Error,
            DiagnosticKind::IncorrectImplTraitArgCount(..) => Error,
            DiagnosticKind::NonIntegerType(_) => Error,
            DiagnosticKind::NonFloatType(_) => Error,
            DiagnosticKind::NotInScope(_, _) => Error,
            DiagnosticKind::CouldNotFindModule(_) => Error,
            DiagnosticKind::NoDeclarationFoundInScope(_) => Error,
            DiagnosticKind::CouldNotOpenFileForImport(_) => Error,
            DiagnosticKind::MissingImplDefinition(_) => Error,
            DiagnosticKind::EffectsMustBeFunctions => Error,
            DiagnosticKind::InvalidHandlerPattern => Error,
        }
    }
}

/// An error (or warning/note) message to be printed out on screen.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic<'a> {
    msg: DiagnosticKind,
    location: Location<'a>,
}

impl<'a> Diagnostic<'a> {
    pub fn error_type(&self) -> ErrorType {
        self.msg.error_type()
    }

    fn marker(&self) -> ColoredString {
        match self.error_type() {
            ErrorType::Error => self.color("error:"),
            ErrorType::Warning => self.color("warning:"),
            ErrorType::Note => self.color("note:"),
        }
    }

    /// Color the given string in either the error, warning, or note color
    fn color(&self, msg: &str) -> ColoredString {
        match (COLORED_OUTPUT.load(SeqCst), self.error_type()) {
            (false, _) => msg.normal(),
            (_, ErrorType::Error) => msg.red(),
            (_, ErrorType::Warning) => msg.yellow(),
            (_, ErrorType::Note) => msg.purple(),
        }
    }
}

/// Reads the given file, returning all of its contents
fn read_file_or_panic(path: &Path) -> String {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    contents
}

/// Sets whether error message output should be colored or not
pub fn color_output(should_color: bool) {
    COLORED_OUTPUT.store(should_color, SeqCst);
}

/// Format the path in an OS-agnostic way. By default rust uses "/" on Unix
/// and "\" on windows as the path separator. This makes testing more
/// difficult and isn't needed for error reporting so we implement our own
/// path-Displaying here that is roughly the same as printing Unix paths.
fn os_agnostic_display_path(path: &Path) -> ColoredString {
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

    if COLORED_OUTPUT.load(SeqCst) {
        ret.italic()
    } else {
        ret.normal()
    }
}

impl<'a> Display for Location<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let filename = os_agnostic_display_path(self.filename);
        write!(f, "{}:{}:{}", filename, self.start.line, self.start.column)
    }
}

impl<'a> Display for Diagnostic<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let start = self.location.start;

        writeln!(f, "{}\t{} {}", self.location, self.marker(), self.msg)?;

        let file_contents = read_file_or_panic(self.location.filename);
        let line = file_contents.lines().nth(max(1, start.line) as usize - 1).unwrap_or("");

        let start_column = max(1, start.column) as usize - 1;
        let actual_len = min(self.location.length(), line.len() - start_column);

        // In case we have an odd Location that has start.index = end.index,
        // we show a minimum of one indicator (^) to show where the error is.
        // let adjusted_len = max(1, actual_len);

        // write the first part of the line, then the erroring part in red, then the rest
        write!(f, "{}", &line[0..start_column])?;
        write!(f, "{}", self.color(&line[start_column..start_column + actual_len]))?;
        writeln!(f, "{}", &line[start_column + actual_len..])?;

        if !COLORED_OUTPUT.load(SeqCst) {
            let padding = " ".repeat(start_column);
            let indicator = self.color(&"^".repeat(max(1, actual_len)));
            writeln!(f, "{}{}", padding, indicator)?;
        }
        Ok(())
    }
}
