//! error/mod.rs - Defines the error, warning, and note macros
//! used to issue compiler errors. There is also an ErrorMessage type
//! for storing messages that may be issued later. Note that all issuing
//! an error does is print it to stderr and update the global ERROR_COUNT.
//!
//! Compiler passes are expected to continue even after issuing errors so
//! that as many can be issued as possible. A possible future improvement
//! would be to implement poisoning so that repeated errors are hidden.
pub mod location;
use crate::cache::{cached_read, ModuleCache};
use crate::error::location::{Locatable, Location};

use colored::ColoredString;
use colored::*;
use std::cmp::{max, min};
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

static COLORED_OUTPUT: AtomicBool = AtomicBool::new(true);

/// Every diagnostic that may be emitted by the compiler
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    /// These errors are always indicative of a compiler bug.
    /// They're preferred over panics in cases where we have concrete locations of the error.
    InternalError(/*message*/ &'static str),

    //
    //                     Parsing
    //
    ParserExpected(/*Expected tokens*/ Vec<String>),
    ParserErrorInRule(/*Failing parse rule*/ &'static str),
    LexerError(String),

    //
    //                 Name Resolution
    //
    TypeVariableAlreadyInScope(/*type variable name*/ String),
    ItemNotRequiredByTrait(/*item name*/ String, /*trait name*/ String),
    AlreadyInScope(/*item name*/ String),
    PreviouslyDefinedHere(/*item name*/ String),
    IncorrectConstructorArgCount(/*item name*/ String, /*actual count*/ usize, /*expected count*/ usize),

    // This can be combined with IncorrectArgCount
    IncorrectImplTraitArgCount(/*Trait name*/ String, /*actual count*/ usize, /*expected count*/ usize),
    NonIntegerType(/*type name*/ String),
    NonFloatType(/*type name*/ String),
    NotInScope(/*item kind*/ &'static str, /*item name*/ String),
    CouldNotFindModule(/*module path*/ String),

    // Should this be combined with `NotInScope`?
    NoDeclarationFoundInScope(/*variable name*/ String),
    CouldNotOpenFileForImport(/*file path*/ PathBuf),
    MissingImplDefinition(/*definition name*/ String),
    EffectsMustBeFunctions,
    InvalidHandlerPattern,
    NotAnEffect(/*item name*/ String),
    HandlerMissingCases(/*missing effect cases*/ Vec<String>),
    ImportShadowsPreviousDefinition(/*item name*/ String),
    Unused(/*item name*/ String),
    NotAStruct(/*struct name*/ String),
    MissingFields(/*missing struct fields*/ Vec<String>),
    NotAStructField(/*field name*/ String),

    //
    //                  Type Checking
    //
    TypeLengthMismatch(Vec<String>, Vec<String>),
    PatternIsNotIrrefutable,
    InvalidSyntaxInPattern,
    InvalidSyntaxInIrrefutablePattern,
    FunctionParameterCountMismatch(/*type*/ String, /*actual*/ usize, /*expected*/ usize),
    MutRefToImmutableVariable(/*name*/ String),
    MutRefToTemporary,

    // Type errors are grouped together here for ease of passing different TypeErrorKinds to
    // `try_unify` while delaying converting the types to strings until the error actually is
    // pushed.
    TypeError(TypeErrorKind, /*actual type*/ String, /*expected type*/ String),
    MultipleMatchingImpls(/*constraint*/ String, /*impl count*/ usize),
    ImplCandidate(/*candidate index*/ usize),
    ImplCandidateWithMoreHidden(/*candidate index*/ usize, /*remaining hidden candidate count*/ usize),
    NoMatchingImpls(/*constraint*/ String),
    UnreachablePattern,
    MissingCase(/*case*/ String),
    UnhandledEffectsInMain(/*effects*/ String),
    FunctionTypeMismatch(/*actual*/String, /*expected*/String),
    FunctionEffectsNotSpecified,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorType {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeErrorKind {
    ExpectedUnitTypeFromPattern,
    ExpectedPairTypeFromPattern,
    VariableDoesNotMatchDeclaredType,
    PatternTypeDoesNotMatchAnnotatedType,
    PatternTypeDoesNotMatchDefinitionType,
    FunctionBodyDoesNotMatchReturnType,
    CalledValueIsNotAFunction,
    ArgumentTypeMismatch,
    NonBoolInCondition,
    IfBranchMismatch,
    MatchPatternTypeDiffers,
    MatchReturnTypeDiffers,
    DoesNotMatchAnnotatedType,
    ExpectedStructReference,

    // This taking a String is the reason we can't have nice things (Copy)
    NoFieldOfType(/*field name*/ String),
    AssignToNonMutRef,
    AssignToWrongType,
    HandleBranchMismatch,
    PatternReturnTypeMismatch,
    MonomorphizationError,

    NeverShown,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            DiagnosticKind::InternalError(message) => {
                write!(f, "Internal compiler error: {}", message)
            },
            DiagnosticKind::ParserExpected(tokens) => {
                if tokens.len() == 1 {
                    write!(f, "Parser expected {} here", tokens[0])
                } else {
                    write!(f, "Parser expected one of {}", tokens.join(", "))
                }
            },
            DiagnosticKind::ParserErrorInRule(rule) => {
                write!(f, "Failed trying to parse a {}", rule)
            },
            DiagnosticKind::LexerError(error) => {
                write!(f, "{}", error)
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
                write!(f, "{} was previously defined here", item)
            },
            DiagnosticKind::IncorrectConstructorArgCount(item, actual, expected) => {
                let plural_s = if *expected == 1 { "" } else { "s" };
                let is_are = if *actual == 1 { "is" } else { "are" };
                write!(
                    f,
                    "Type {} expects {} argument{}, but {} {} given here",
                    item, expected, plural_s, actual, is_are
                )
            },
            DiagnosticKind::IncorrectImplTraitArgCount(trait_name, actual, expected) => {
                let plural_s = if *expected == 1 { "" } else { "s" };
                write!(f, "impl has {} type argument{} but {} requires {}", expected, plural_s, trait_name, actual)
            },
            DiagnosticKind::NonIntegerType(typename) => {
                write!(f, "Type {} is not an integer type", typename)
            },
            DiagnosticKind::NonFloatType(typename) => {
                write!(f, "Type {} is not a float type", typename)
            },
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
                write!(
                    f,
                    "Invalid handle pattern. Handle patterns must be an effect function call or a return expression"
                )
            },
            DiagnosticKind::NotAnEffect(item) => {
                write!(f, "{} is not an effect", item)
            },
            DiagnosticKind::HandlerMissingCases(cases) => {
                let plural_s = if cases.len() == 1 { "" } else { "s" };
                let cases_str = cases.join(", ");
                write!(f, "Handler is missing {} case{}: {}", cases.len(), plural_s, cases_str)
            },
            DiagnosticKind::ImportShadowsPreviousDefinition(item) => {
                write!(f, "import shadows previous definition of {item}")
            },
            DiagnosticKind::Unused(item) => {
                write!(f, "{item} is unused (prefix name with _ to silence this warning)")
            },
            DiagnosticKind::NotAStruct(name) => {
                write!(f, "{} is not a struct", name)
            },
            DiagnosticKind::NotAStructField(name) => {
                write!(f, "{} is not a struct field", name)
            },
            DiagnosticKind::MissingFields(fields) => {
                write!(f, "Missing fields: {}", fields.join(", "))
            },
            DiagnosticKind::TypeLengthMismatch(left, right) => {
                write!(
                    f,
                    "Type-length mismatch: {} versus {} when unifying [{}] and [{}]",
                    left.len(),
                    right.len(),
                    left.join(", "),
                    right.join(", ")
                )
            },
            DiagnosticKind::PatternIsNotIrrefutable => {
                write!(f, "Pattern is not irrefutable")
            },
            DiagnosticKind::InvalidSyntaxInPattern => {
                write!(f, "Invalid syntax in pattern, expected a name, type annotation, or type constructor")
            },
            DiagnosticKind::InvalidSyntaxInIrrefutablePattern => {
                write!(
                    f,
                    "Invalid syntax in irrefutable pattern, expected a name, type annotation, or type constructor"
                )
            },
            DiagnosticKind::FunctionParameterCountMismatch(typ, actual, expected) => {
                let plural_s = if *expected == 1 { "" } else { "s" };
                let was_were = if *actual == 1 { "was" } else { "were" };
                write!(f, "Function of type {typ} declared to take {expected} parameter{plural_s}, but {actual} {was_were} supplied")
            },
            DiagnosticKind::TypeError(TypeErrorKind::ExpectedUnitTypeFromPattern, actual, _expected) => {
                write!(f, "Expected a unit type from this pattern, but the corresponding value has the type {}", actual)
            },
            DiagnosticKind::TypeError(TypeErrorKind::ExpectedPairTypeFromPattern, actual, _expected) => {
                write!(f, "Expected a pair type from this pattern, but found {actual}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::VariableDoesNotMatchDeclaredType, actual, expected) => {
                write!(f, "Variable type {actual} does not match its declared type of {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::PatternTypeDoesNotMatchAnnotatedType, actual, expected) => {
                write!(f, "Pattern type {actual} does not match the annotated type {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::PatternTypeDoesNotMatchDefinitionType, actual, expected) => {
                write!(f, "Pattern type {actual} does not match the definition's type {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::FunctionBodyDoesNotMatchReturnType, actual, expected) => {
                write!(f, "Function body type {actual} does not match declared return type of {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::CalledValueIsNotAFunction, actual, _expected) => {
                write!(f, "Value being called is not a function, it is a {actual}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::ArgumentTypeMismatch, actual, expected) => {
                write!(f, "Expected argument of type {expected}, but found {actual}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::NonBoolInCondition, actual, expected) => {
                write!(f, "{actual} should be a {expected} to be used in an if condition")
            },
            DiagnosticKind::TypeError(TypeErrorKind::IfBranchMismatch, actual, expected) => {
                write!(
                    f,
                    "Expected 'then' and 'else' branch types to match, but found {expected} and {actual} respectively"
                )
            },
            DiagnosticKind::TypeError(TypeErrorKind::MatchPatternTypeDiffers, actual, expected) => {
                write!(f, "This pattern of type {actual} does not match the type {expected} that is being matched on")
            },
            DiagnosticKind::TypeError(TypeErrorKind::MatchReturnTypeDiffers, actual, expected) => {
                write!(
                    f,
                    "This branch's return type {actual} does not match the previous branches which return {expected}"
                )
            },
            DiagnosticKind::TypeError(TypeErrorKind::DoesNotMatchAnnotatedType, actual, expected) => {
                write!(f, "Expression of type {actual} does not match its annotated type {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::ExpectedStructReference, actual, _expected) => {
                write!(f, "Expected a struct reference but found {actual} instead")
            },
            DiagnosticKind::TypeError(TypeErrorKind::NoFieldOfType(field_name), actual, expected) => {
                write!(f, "{actual} has no field '{field_name}' of type {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::AssignToNonMutRef, actual, expected) => {
                write!(f, "Expression of type {actual} must be a mutable reference type ({expected}) to be assigned to")
            },
            DiagnosticKind::TypeError(TypeErrorKind::AssignToWrongType, actual, expected) => {
                write!(f, "Cannot assign expression of type {actual} to a Ref of type {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::HandleBranchMismatch, actual, expected) => {
                write!(f, "The type of this branch ({actual}) should match the type of the expression being handled: {expected}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::PatternReturnTypeMismatch, actual, expected) => {
                write!(f, "Expected type {expected} does not match the pattern's return type {actual}")
            },
            DiagnosticKind::TypeError(TypeErrorKind::NeverShown, actual, expected) => {
                unreachable!("This type error should never be shown. Expected {}, Actual {}", expected, actual)
            },
            DiagnosticKind::TypeError(TypeErrorKind::MonomorphizationError, actual, expected) => {
                unreachable!(
                    "Unification error during monomorphization: Could not unify definition {} with instantiation {}",
                    expected, actual
                )
            },
            DiagnosticKind::MultipleMatchingImpls(constraint, count) => {
                write!(f, "{count} matching impls found for {constraint}")
            },
            DiagnosticKind::ImplCandidate(index) => {
                write!(f, "Candidate {index}")
            },
            DiagnosticKind::ImplCandidateWithMoreHidden(index, hidden_remaining) => {
                write!(f, "Candidate {index} ({hidden_remaining} more hidden)")
            },
            DiagnosticKind::NoMatchingImpls(constraint) => {
                write!(f, "No impl found for {constraint}")
            },
            DiagnosticKind::UnreachablePattern => {
                write!(f, "Unreachable pattern")
            },
            DiagnosticKind::MissingCase(case) => {
                write!(f, "Missing case {case}")
            },
            DiagnosticKind::UnhandledEffectsInMain(effects) => {
                write!(f, "Unhandled effects at top-level: {effects}")
            },
            DiagnosticKind::MutRefToImmutableVariable(name) => {
                write!(f, "Cannot mutably reference `{name}`. It was declared as immutable")
            },
            DiagnosticKind::MutRefToTemporary => {
                write!(f, "Cannot mutably reference a temporary value")
            },
            DiagnosticKind::FunctionTypeMismatch(actual, expected) => {
                write!(f, "Expected function of type {expected}, but found {actual}")
            },
            DiagnosticKind::FunctionEffectsNotSpecified => {
                write!(f, "This function type's effects must be specified. Add `pure` if it shouldn't have any.")
            },
        }
    }
}

impl DiagnosticKind {
    pub fn error_type(&self) -> ErrorType {
        use DiagnosticKind::*;
        use ErrorType::*;

        match &self {
            PreviouslyDefinedHere(_) | ImplCandidate(_) | ImplCandidateWithMoreHidden(_, _) => Note,

            Unused(_) | UnreachablePattern => Warning,

            LexerError(_)
            | ParserExpected(_)
            | ParserErrorInRule(_)
            | TypeVariableAlreadyInScope(_)
            | ItemNotRequiredByTrait(..)
            | AlreadyInScope(_)
            | IncorrectConstructorArgCount(..)
            | IncorrectImplTraitArgCount(..)
            | NonIntegerType(_)
            | NonFloatType(_)
            | NotInScope(_, _)
            | CouldNotFindModule(_)
            | NoDeclarationFoundInScope(_)
            | CouldNotOpenFileForImport(_)
            | MissingImplDefinition(_)
            | EffectsMustBeFunctions
            | InvalidHandlerPattern
            | NotAnEffect(_)
            | HandlerMissingCases(_)
            | ImportShadowsPreviousDefinition(_)
            | TypeLengthMismatch(..)
            | PatternIsNotIrrefutable
            | InvalidSyntaxInIrrefutablePattern
            | InvalidSyntaxInPattern
            | FunctionParameterCountMismatch(..)
            | TypeError(..)
            | MultipleMatchingImpls(_, _)
            | NoMatchingImpls(_)
            | MissingCase(_)
            | InternalError(_)
            | NotAStruct(_)
            | MissingFields(_)
            | UnhandledEffectsInMain(_)
            | MutRefToImmutableVariable(_)
            | MutRefToTemporary
            | FunctionTypeMismatch(..)
            | FunctionEffectsNotSpecified
            | NotAStructField(_) => Error,
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
    pub fn new(location: Location<'a>, msg: DiagnosticKind) -> Self {
        Self { location, msg }
    }

    // This is used by ante-ls to avoid displaying the location and
    // location contents inline when we're already highlighting the
    // same location within a source file.
    #[allow(unused)]
    pub fn msg(&self) -> &DiagnosticKind {
        &self.msg
    }

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

    /// Return a displayable version of this Diagnostic.
    /// Note that before being displayed, Diagnostics should always be pushed
    /// to the ModuleCache first.
    pub fn display<'l>(&'l self, cache: &'l ModuleCache<'a>) -> DisplayDiagnostic<'l, 'a> {
        DisplayDiagnostic(self, cache)
    }

    /// Display isn't implemented directly on Diagnostic to avoid accidentally printing
    /// out errors when they should be pushed to the ModuleCache first.
    fn format(&self, f: &mut Formatter, cache: &'a ModuleCache) -> std::fmt::Result {
        let start = self.location.start;

        let relative_path =
            os_agnostic_display_path(cache.strip_root(self.location.filename).unwrap_or(self.location.filename));

        writeln!(f, "{}:{}:{}\t{} {}", relative_path, start.line, start.column, self.marker(), self.msg)?;

        let file_contents = cached_read(&cache.file_cache, self.location.filename).unwrap();
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

impl<'a> Locatable<'a> for Diagnostic<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
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

pub struct DisplayDiagnostic<'local, 'cache>(&'local Diagnostic<'cache>, &'local ModuleCache<'cache>);

impl<'local, 'cache> Display for DisplayDiagnostic<'local, 'cache> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.format(f, self.1)
    }
}
