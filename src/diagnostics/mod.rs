use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fmt::{Display, Formatter},
    path::PathBuf,
    sync::Arc,
};

use colored::{Color, ColoredString, Colorize};
use serde::{Deserialize, Serialize};

use crate::{
    incremental::{
        AllDefinitions, CheckAll, Db, DbHandle, GetCrateGraph, Parse, SourceFile, TypeCheck, ValidateExports,
    },
    iterator_extensions::mapvec,
    lexer::{
        Lexer,
        token::{Integer, IntegerKind, Token, lookup_keyword},
    },
    name_resolution::namespace::CrateId,
    parser::cst::Name,
    type_inference::{errors::TypeErrorKind, kinds::Kind},
};

mod location;
mod unimplemented_item;

pub use location::*;
pub use unimplemented_item::*;

/// Any diagnostic that the compiler can issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Diagnostic {
    // TODO: `message` could be an enum to save allocation costs
    ParserExpected {
        message: String,
        actual: Token,
        location: Location,
        hint: Option<Hint>,
    },
    ExpectedPathForImport {
        location: Arc<LocationData>,
    },
    NameAlreadyInScope {
        name: Name,
        first_location: Location,
        second_location: Location,
    },
    ImportedNameAlreadyInScope {
        name: Name,
        first_location: Location,
        second_location: Location,
    },
    UnusedName {
        name: Name,
        location: Location,
    },
    UnknownImportFile {
        crate_name: String,
        module_name: Arc<PathBuf>,
        location: Location,
    },
    UnknownImportItem {
        name: Name,
        module: Arc<PathBuf>,
        location: Location,
    },
    ItemNotExported {
        name: Name,
        module: Arc<PathBuf>,
        location: Location,
    },
    ExportedItemNotFound {
        name: Name,
        location: Location,
    },
    NameNotInScope {
        name: Name,
        location: Location,
    },
    ExpectedType {
        actual: String,
        expected: String,
        location: Location,
    },
    ExpectedTypeKind {
        actual: Kind,
        location: Location,
    },
    ExpectedKind {
        actual: Kind,
        expected: Kind,
        location: Location,
    },
    RecursiveType {
        typ: String,
        location: Location,
    },
    RecursiveTypeAlias {
        typ: String,
        location: Location,
    },
    NamespaceNotFound {
        name: String,
        location: Location,
    },
    MethodDeclaredOnUnknownType {
        name: Name,
        location: Location,
    },
    LiteralUsedAsName {
        location: Location,
    },
    ValueExpected {
        location: Location,
        typ: Name,
    },
    TypeExpected {
        name: Name,
        location: Location,
    },
    TypeError {
        actual: String,
        expected: String,
        kind: TypeErrorKind,
        /// True when `actual` and `expected` are equal except for their function environments.
        function_environments_differ: bool,
        location: Location,
    },
    FunctionArgCountMismatch {
        actual: usize,
        expected: usize,
        location: Location,
    },
    ConstructorFieldDuplicate {
        name: Name,
        first_location: Location,
        second_location: Location,
    },
    ConstructorMissingFields {
        missing_fields: Vec<String>,
        location: Location,
    },
    ConstructorNotAStruct {
        typ: String,
        location: Location,
    },
    NoSuchFieldForType {
        name: Name,
        typ: String,
        location: Location,
    },
    ParserComplexImplItemName {
        location: Location,
    },
    TypeMustBeKnownMemberAccess {
        location: Location,
    },
    CannotMatchOnType {
        typ: String,
        location: Location,
    },
    UnreachableCase {
        location: Location,
    },
    MissingCases {
        cases: BTreeSet<Name>,
        location: Location,
    },
    MissingManyCases {
        typ: String,
        location: Location,
    },
    InvalidRangeInPattern {
        start: u64,
        end: u64,
        location: Location,
    },
    InvalidPattern {
        location: Location,
    },
    /// A variable is bound by some but not all alternatives of an OR-pattern.
    OrPatternBindingMismatch {
        name: Name,
        location: Location,
    },
    Unimplemented {
        item: UnimplementedItem,
        location: Location,
    },
    /// `constructor_names` here is limited to 2 for brevity
    ConstructorExpectedFoundType {
        type_name: Name,
        constructor_names: Vec<Name>,
        location: Location,
    },
    NoImplicitFound {
        type_string: String,
        function_name: Option<String>,
        parameter_index: usize,
        location: Location,
        suggestions: Vec<ImportSuggestion>,
    },
    ImplicitNotAVariable {
        location: Location,
    },
    MultipleImplicitsFound {
        matches: Vec<Name>,
        type_string: String,
        function_name: Option<String>,
        parameter_index: usize,
        location: Location,
    },
    AmbiguousImplicit {
        type_string: String,
        function_name: Option<String>,
        parameter_index: usize,
        location: Location,
    },
    TopLevelImplicitTypeAnnotationRequired {
        location: Location,
    },
    ReturnNotInFunction {
        location: Location,
    },
    BreakNotInLoop {
        location: Location,
    },
    ContinueNotInLoop {
        location: Location,
    },
    IntegerTooLarge {
        value: Integer,
        kind: IntegerKind,
        location: Location,
    },
    NoMainFunction {
        location: Location,
    },
    TypeAnnotationNeeded {
        location: Location,
    },
    NotAType {
        name: String,
        location: Location,
    },
    UseOfMovedValue {
        name: String,
        location: Location,
        moved_in: Location,
    },
    MoveInRepeatedContext {
        name: String,
        context: RepeatedContext,
        location: Location,
    },
    AbilityTypeCantBeUsed {
        location: Location,
    },
    HoleCantBeUsed {
        location: Location,
    },
    /// A reference type appeared in a type-constructor position (e.g. inside a
    /// `type ... = ...` body) without an explicit `'name` lifetime.
    MissingExplicitLifetime {
        location: Location,
    },
    FreeVarsInTypeConstructor {
        location: Location,
    },
    HandlerMissingMethods {
        effect_name: Name,
        missing_methods: Vec<String>,
        location: Location,
    },
    HandlerDuplicateMethod {
        name: Name,
        first_location: Location,
        second_location: Location,
    },
    HandlerCrossEffect {
        first_effect: Name,
        second_effect: Name,
        location: Location,
    },
    AssignToImmutable {
        name: Option<Name>,
        location: Location,
    },
    ConfusingOperatorAfterBody {
        body_kind: ConfusingBodyKind,
        operator_location: Location,
        body_location: Location,
    },
}

/// Which construct's body was just parsed when a confusing trailing operator
/// was detected. Used to phrase the diagnostic.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ConfusingBodyKind {
    Lambda,
    Return,
}

impl ConfusingBodyKind {
    fn description(&self) -> &'static str {
        match self {
            ConfusingBodyKind::Lambda => "lambda body",
            ConfusingBodyKind::Return => "return expression",
        }
    }
}

/// A hint the compiler can add to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Hint {
    FieldlessTypesNeedConstructors,
}

impl Display for Hint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Hint::FieldlessTypesNeedConstructors => {
                write!(f, "for a fieldless type, define a constructor with no arguments")
            },
        }
    }
}

/// A suggestion to import an out-of-scope item, attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ImportSuggestion {
    /// Fully-qualified import path, e.g. `Std.HashMap.empty`
    pub qualified_path: String,
    pub location: Location,
}

/// Identifies a syntactic context whose body may execute more than once.
/// Used by the affine checker to describe where an outer non-Copy value
/// was moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum RepeatedContext {
    HandlerBranch,
    ForLoop,
    WhileLoop,
}

impl RepeatedContext {
    fn description(&self) -> &'static str {
        match self {
            RepeatedContext::HandlerBranch => "a handler branch",
            RepeatedContext::ForLoop => "a for loop",
            RepeatedContext::WhileLoop => "a while loop",
        }
    }
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
    pub fn kind(&self) -> DiagnosticKind {
        use Diagnostic::*;
        match self {
            NameAlreadyInScope { .. }
            | ImportedNameAlreadyInScope { .. }
            | UnusedName { .. }
            | UnreachableCase { .. }
            | InvalidRangeInPattern { .. }
            | ConfusingOperatorAfterBody { .. } => DiagnosticKind::Warning,
            _ => DiagnosticKind::Error,
        }
    }

    /// Sets the hint on this error, if this kind of error can have a hint.
    ///
    /// If the error already has a hint, it is replaced.
    pub fn with_hint(self, hint: Hint) -> Self {
        match self {
            Diagnostic::ParserExpected { message, actual, location, hint: _ } => {
                Diagnostic::ParserExpected { message, actual, location, hint: Some(hint) }
            },
            other => other,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Diagnostic::ParserExpected { message, actual, .. } => {
                if actual.to_string().contains(" ") {
                    format!("Expected {message} but found {actual}")
                } else {
                    format!("Expected {message} but found `{actual}`")
                }
            },
            Diagnostic::ParserComplexImplItemName { location: _ } => {
                format!("Impl item names should only be a single identifier")
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
            Diagnostic::UnusedName { name, location: _ } => {
                format!("`{name}` is never used")
            },
            Diagnostic::UnknownImportFile { crate_name, module_name, location: _ } => {
                if module_name.display().to_string().is_empty() {
                    format!("Could not find crate `{crate_name}`")
                } else {
                    format!("Could not find module `{}` in crate `{crate_name}`", module_name.display())
                }
            },
            Diagnostic::UnknownImportItem { name, module, location: _ } => {
                format!("`{name}` not found in module `{}`", module.display())
            },
            Diagnostic::ItemNotExported { name, module, location: _ } => {
                format!("`{name}` is not exported from module `{}`", module.display())
            },
            Diagnostic::ExportedItemNotFound { name, location: _ } => {
                format!("`{name}` is not defined")
            },
            Diagnostic::NameNotInScope { name, location: _ } => {
                format!("`{name}` not found in scope")
            },
            Diagnostic::ExpectedType { actual, expected, location: _ } => {
                format!("Expected type `{expected}` but found `{actual}`")
            },
            Diagnostic::RecursiveType { typ, location: _ } => {
                format!("`{}` is infinitely recursive", color_type(typ))
            },
            Diagnostic::RecursiveTypeAlias { typ, location: _ } => {
                format!("`{}` is infinitely recursive", color_type(typ))
            },
            Diagnostic::NamespaceNotFound { name, location: _ } => {
                format!("Namespace `{name}` not found in path")
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
            Diagnostic::TypeExpected { name, location: _ } => {
                format!("Expected a type but `{name}` is a value")
            },
            Diagnostic::TypeError { actual, expected, kind, function_environments_differ: _, location: _ } => {
                kind.message(actual, expected)
            },
            Diagnostic::FunctionArgCountMismatch { actual, expected, location: _ } => {
                let s = if *expected == 1 { "" } else { "s" };
                format!("Expected {expected} argument{s} but found {actual}")
            },
            Diagnostic::NoSuchFieldForType { name, typ, location: _ } => {
                format!("{} has no field named `{name}`", color_type(typ))
            },
            Diagnostic::ConstructorFieldDuplicate { name, first_location: _, second_location: _ } => {
                // TODO: Show both locations in same error
                format!("Duplicate field `{name}`")
            },
            Diagnostic::ConstructorMissingFields { missing_fields, location: _ } => {
                let s = if missing_fields.len() == 1 { "" } else { "s" };
                let fields = missing_fields.join(", ");
                format!("Missing field{s}: {fields}")
            },
            Diagnostic::ConstructorNotAStruct { typ, location: _ } => {
                format!("{} is not a struct", color_type(typ))
            },
            Diagnostic::TypeMustBeKnownMemberAccess { location: _ } => {
                format!("Object type must be known by this point to access its field")
            },
            Diagnostic::CannotMatchOnType { typ, location: _ } => {
                format!("Cannot match on an object of type {}", color_type(typ))
            },
            Diagnostic::UnreachableCase { location: _ } => {
                format!("This case is already matched by prior patterns")
            },
            Diagnostic::MissingCases { cases, location: _ } => {
                let cases_string = mapvec(cases.iter().take(5), |case| color_type(case).to_string()).join(", ");

                if cases.len() == 1 {
                    format!("Missing case: {cases_string}")
                } else if cases.len() <= 5 {
                    format!("Missing cases: {cases_string}")
                } else {
                    format!("Missing cases: {cases_string}, ...")
                }
            },
            Diagnostic::MissingManyCases { typ, location: _ } => {
                format!(
                    "Missing cases for type {}, values of this type require a catch-all pattern like `_`",
                    color_type(typ)
                )
            },
            Diagnostic::InvalidRangeInPattern { start, end, location: _ } => {
                if start == end {
                    format!(
                        "Ranges in Ante are end-exclusive so a range from {} to {} will not match anything",
                        color_constant(&start.to_string()),
                        color_constant(&end.to_string())
                    )
                } else {
                    assert!(start > end);
                    format!(
                        "Range from {} to {} is backwards and will not match anything",
                        color_constant(&start.to_string()),
                        color_constant(&end.to_string())
                    )
                }
            },
            Diagnostic::InvalidPattern { location: _ } => {
                format!("Invalid pattern syntax, expected a variable, constructor, or integer")
            },
            Diagnostic::OrPatternBindingMismatch { name, location: _ } => {
                format!("Variable {} is not bound by every alternative of this OR-pattern", color_constant(name))
            },
            Diagnostic::Unimplemented { item, location: _ } => {
                format!("{item} are currently unimplemented")
            },
            Diagnostic::ConstructorExpectedFoundType { type_name, constructor_names, location: _ } => {
                if constructor_names.is_empty() {
                    format!("The type {} has no variants and thus cannot be matched on", color_type(type_name))
                } else if constructor_names.len() == 1 {
                    let constructor = &constructor_names[0];
                    format!(
                        "{} is a type name, not a constructor. Try {}.{} instead",
                        color_type(type_name),
                        color_type(type_name),
                        color_type(constructor)
                    )
                } else {
                    let first = &constructor_names[0];
                    let second = &constructor_names[1];
                    format!(
                        "{} is a type name, not a constructor. Try a constructor such as {}.{} or {}.{} instead",
                        color_type(type_name),
                        color_type(type_name),
                        color_type(first),
                        color_type(type_name),
                        color_type(second)
                    )
                }
            },
            Diagnostic::NoImplicitFound {
                type_string,
                function_name,
                parameter_index: _,
                location: _,
                suggestions: _,
            } => {
                let function = function_name.as_ref().map(|s| format!("{}", color_name(s)));
                let of_function = function.as_ref().map(String::as_str).unwrap_or("call");
                format!("No implicit found for type {} required by {of_function}", color_type(type_string),)
            },
            Diagnostic::ImplicitNotAVariable { location: _ } => {
                format!("Implicits must be a simple variable, more complex patterns are not supported")
            },
            Diagnostic::MultipleImplicitsFound {
                matches,
                type_string,
                function_name,
                parameter_index: _,
                location: _,
            } => {
                let function = function_name.as_ref().map(|s| format!("{}", color_name(s)));
                let of_function = function.as_ref().map(String::as_str).unwrap_or("call");
                let matches = crate::iterator_extensions::join_arc_str(matches, ", ");
                format!(
                    "Multiple matching implicits found for type {} required by {of_function}: {matches}",
                    color_type(type_string),
                )
            },
            Diagnostic::AmbiguousImplicit { type_string, function_name, parameter_index: _, location: _ } => {
                let function = function_name.as_ref().map(|s| format!("{}", color_name(s)));
                let of_function = function.as_ref().map(String::as_str).unwrap_or("call");
                format!(
                    "Ambiguous implicit of type {} required by {of_function}, type annotation required",
                    color_type(type_string),
                )
            },
            Diagnostic::TopLevelImplicitTypeAnnotationRequired { location: _ } => {
                "Type annotations are required on top-level implicits".to_string()
            },
            Diagnostic::ExpectedTypeKind { actual, location: _ } => {
                let n = actual.required_argument_count();
                let s = if n == 1 { "" } else { "s" };
                format!("Expected a type here, this type constructor is missing {n} argument{s}")
            },
            Diagnostic::ExpectedKind { actual, expected, location } => {
                if *expected == Kind::Type {
                    Diagnostic::ExpectedTypeKind { actual: actual.clone(), location: location.clone() }.message()
                } else {
                    let expected = color_type(&expected.to_string());
                    let actual = color_type(&actual.to_string());
                    format!("Expected a type constructor of kind {expected}, but found one of kind {actual}")
                }
            },
            Diagnostic::ReturnNotInFunction { location: _ } => "`return` can only be used in a function".to_string(),
            Diagnostic::BreakNotInLoop { location: _ } => "`break` can only be used inside a loop".to_string(),
            Diagnostic::ContinueNotInLoop { location: _ } => "`continue` can only be used inside a loop".to_string(),
            Diagnostic::IntegerTooLarge { value, kind, location: _ } => {
                format!("{} is too large for type {}", color_name(&value.to_string()), color_type(&kind.to_string()))
            },
            Diagnostic::NoMainFunction { location: _ } => "This program has no `main` function".to_string(),
            Diagnostic::TypeAnnotationNeeded { location: _ } => "Type annotation needed".to_string(),
            Diagnostic::NotAType { name, location: _ } => {
                format!("{} is not a type", color_name(name))
            },
            Diagnostic::UseOfMovedValue { name, location: _, moved_in: _ } => {
                format!("Use of moved value {}", color_name(name))
            },
            Diagnostic::MoveInRepeatedContext { name, context, location: _ } => {
                format!(
                    "Cannot move {} in {} because it may be executed multiple times",
                    color_name(name),
                    context.description()
                )
            },
            Diagnostic::AbilityTypeCantBeUsed { location: _ } => {
                "Ability types can't be used in this position".to_string()
            },
            Diagnostic::HoleCantBeUsed { location: _ } => {
                format!("A type hole can't be used in this position")
            },
            Diagnostic::MissingExplicitLifetime { location: _ } => {
                format!("A reference in this position requires an explicit `'a` lifetime")
            },
            Diagnostic::FreeVarsInTypeConstructor { location: _ } => {
                format!("Internal compiler error: there are free variables in this type constructor")
            },
            Diagnostic::HandlerMissingMethods { effect_name, missing_methods, location: _ } => {
                let s = if missing_methods.len() == 1 { "" } else { "s" };
                let methods = missing_methods.join(", ");
                format!("Handler for effect {} is missing method{s}: {methods}", color_type(effect_name))
            },
            Diagnostic::HandlerDuplicateMethod { name, first_location: _, second_location: _ } => {
                format!("Effect method {} is handled more than once", color_type(name))
            },
            Diagnostic::HandlerCrossEffect { first_effect, second_effect, location: _ } => {
                let first = color_type(first_effect);
                let second = color_type(second_effect);
                format!("Handler mixes methods from effects {first} and {second}",)
            },
            Diagnostic::AssignToImmutable { name, location: _ } => {
                let var = color_keyword("var");
                if let Some(name) = name {
                    let name = color_name(name);
                    format!("Cannot assign to {name}, declare it with {var} to make it mutable")
                } else {
                    format!("Cannot assign to lvalue, declare it as a mutable variable first with {var}")
                }
            },
            Diagnostic::ConfusingOperatorAfterBody { body_kind, .. } => {
                let kind = body_kind.description();
                format!(
                    "This operator looks like it is in the {kind}, but it is actually part of the outer expression. Use parentheses to avoid confusion"
                )
            },
        }
    }

    /// The primary source location of this diagnostic
    pub fn location(&self) -> &Location {
        match self {
            Diagnostic::ParserExpected { location, .. }
            | Diagnostic::ParserComplexImplItemName { location, .. }
            | Diagnostic::ExpectedPathForImport { location }
            | Diagnostic::NameAlreadyInScope { second_location: location, .. }
            | Diagnostic::ImportedNameAlreadyInScope { second_location: location, .. }
            | Diagnostic::UnusedName { location, .. }
            | Diagnostic::UnknownImportFile { location, .. }
            | Diagnostic::UnknownImportItem { location, .. }
            | Diagnostic::ItemNotExported { location, .. }
            | Diagnostic::ExportedItemNotFound { location, .. }
            | Diagnostic::NameNotInScope { location, .. }
            | Diagnostic::ExpectedType { location, .. }
            | Diagnostic::RecursiveType { location, .. }
            | Diagnostic::RecursiveTypeAlias { location, .. }
            | Diagnostic::NamespaceNotFound { location, .. }
            | Diagnostic::MethodDeclaredOnUnknownType { location, .. }
            | Diagnostic::LiteralUsedAsName { location }
            | Diagnostic::ValueExpected { location, .. }
            | Diagnostic::TypeExpected { location, .. }
            | Diagnostic::TypeError { location, .. }
            | Diagnostic::FunctionArgCountMismatch { location, .. }
            | Diagnostic::NoSuchFieldForType { location, .. }
            | Diagnostic::ConstructorMissingFields { location, .. }
            | Diagnostic::ConstructorNotAStruct { location, .. }
            | Diagnostic::ConstructorFieldDuplicate { second_location: location, .. }
            | Diagnostic::CannotMatchOnType { location, .. }
            | Diagnostic::UnreachableCase { location, .. }
            | Diagnostic::MissingCases { location, .. }
            | Diagnostic::MissingManyCases { location, .. }
            | Diagnostic::InvalidRangeInPattern { location, .. }
            | Diagnostic::InvalidPattern { location }
            | Diagnostic::OrPatternBindingMismatch { location, .. }
            | Diagnostic::TypeMustBeKnownMemberAccess { location }
            | Diagnostic::ConstructorExpectedFoundType { location, .. }
            | Diagnostic::ImplicitNotAVariable { location }
            | Diagnostic::NoImplicitFound { location, .. }
            | Diagnostic::MultipleImplicitsFound { location, .. }
            | Diagnostic::AmbiguousImplicit { location, .. }
            | Diagnostic::TopLevelImplicitTypeAnnotationRequired { location }
            | Diagnostic::ExpectedTypeKind { location, .. }
            | Diagnostic::ExpectedKind { location, .. }
            | Diagnostic::ReturnNotInFunction { location }
            | Diagnostic::BreakNotInLoop { location }
            | Diagnostic::ContinueNotInLoop { location }
            | Diagnostic::IntegerTooLarge { location, .. }
            | Diagnostic::Unimplemented { location, .. }
            | Diagnostic::TypeAnnotationNeeded { location, .. }
            | Diagnostic::NotAType { location, .. }
            | Diagnostic::UseOfMovedValue { location, .. }
            | Diagnostic::MoveInRepeatedContext { location, .. }
            | Diagnostic::AbilityTypeCantBeUsed { location, .. }
            | Diagnostic::HoleCantBeUsed { location, .. }
            | Diagnostic::MissingExplicitLifetime { location, .. }
            | Diagnostic::FreeVarsInTypeConstructor { location, .. }
            | Diagnostic::HandlerMissingMethods { location, .. }
            | Diagnostic::HandlerDuplicateMethod { second_location: location, .. }
            | Diagnostic::HandlerCrossEffect { location, .. }
            | Diagnostic::AssignToImmutable { location, .. }
            | Diagnostic::ConfusingOperatorAfterBody { operator_location: location, .. }
            | Diagnostic::NoMainFunction { location } => location,
        }
    }

    /// An optional secondary message for additional information
    fn note(&self) -> Option<(&Location, String)> {
        match self {
            Diagnostic::ParserExpected { location, hint: Some(hint), .. } => Some((location, hint.to_string())),
            Diagnostic::UseOfMovedValue { name, location: _, moved_in } => {
                let message = format!("{} was previously moved here", color_name(name));
                Some((moved_in, message))
            },
            Diagnostic::NoImplicitFound { suggestions, .. } if !suggestions.is_empty() => {
                let first = color_name(&suggestions[0].qualified_path);
                let message = match suggestions.len() {
                    1 => format!("did you mean to import {first}?"),
                    2 => {
                        let second = color_name(&suggestions[1].qualified_path);
                        format!("did you mean to import {first} or {second}?")
                    },
                    n => {
                        let second = color_name(&suggestions[1].qualified_path);
                        format!("did you mean to import {first} or {second}? (or {} more)", n - 2)
                    },
                };
                Some((&suggestions[0].location, message))
            },
            Diagnostic::HandlerDuplicateMethod { name, first_location, .. } => {
                let message = format!("{} was previously handled here", color_name(name));
                Some((first_location, message))
            },
            Diagnostic::ConfusingOperatorAfterBody { body_kind, body_location, .. } => {
                let kind = body_kind.description();
                Some((body_location, format!("this is where the {kind} body ends")))
            },
            Diagnostic::TypeError { function_environments_differ: true, location, .. } => {
                Some((location, "Separate closures are not equal because they may capture different data".to_string()))
            },
            Diagnostic::RecursiveType { typ: _, location } => Some((
                location,
                format!("Declare the type as `shared` or wrap each instance in a pointer type like `Rc`"),
            )),
            Diagnostic::UnusedName { name: _, location } => {
                 Some((location, format!("Prefix the name with `_` to silence this warning")))
            }
            _ => None,
        }
    }

    pub fn display<'a>(&'a self, show_color: bool, compiler: &'a Db) -> DiagnosticDisplay {
        DiagnosticDisplay { diagnostic: self, compiler, show_color }
    }

    fn format(&self, f: &mut Formatter, show_color: bool, compiler: &Db) -> std::fmt::Result {
        let location = self.location();
        let message = self.message();
        let kind = self.kind();
        let note = self.note();

        let start = location.span.start;
        let file = location.file_id.get(compiler);
        writeln!(
            f,
            "{}:{}:{}",
            os_agnostic_display_path(&file.path, show_color),
            start.line_number,
            start.column_number
        )?;

        // When the note is in another file, render the main block normally and emit
        // the note as a trailing standalone line.
        let inline_note = note.as_ref().filter(|(loc, _)| loc.file_id == location.file_id);
        let blocks = line_blocks(location, inline_note.map(|(loc, _)| *loc));
        let digit_len = blocks.iter().map(|b| b.end).max().unwrap_or(1).ilog10() as usize + 1;

        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                // Pads '...' with spaces if needed, or truncates a dot or two
                writeln!(f, "{:digit_len$} | ", &"..."[..digit_len.min(3)])?;
            }
            for line_no in block.clone() {
                let main = (start.line_number == line_no).then(|| (location.span, &message));
                let note_span =
                    inline_note.and_then(|(loc, msg)| (loc.span.start.line_number == line_no).then(|| (loc.span, msg)));
                output_source_line(&file, line_no as usize, digit_len, main, note_span, show_color, kind, f)?;
            }
        }

        if let Some((_, msg)) = note.as_ref().filter(|(loc, _)| loc.file_id != location.file_id) {
            write_trailing_note(f, digit_len, msg, show_color)?;
        }
        Ok(())
    }
}

fn color_type(s: &str) -> ColoredString {
    s.color(TYPE_COLOR)
}

fn color_name(s: &str) -> ColoredString {
    s.color(NAME_COLOR)
}

fn color_constant(s: &str) -> ColoredString {
    s.color(CONSTANT_COLOR)
}

fn color_keyword(s: &str) -> ColoredString {
    s.color(KEYWORD_COLOR)
}

/// Emit a `= note: <msg>` line for a note whose location is in a different file
/// than the primary diagnostic.
fn write_trailing_note(f: &mut Formatter, digit_len: usize, msg: &str, show_color: bool) -> std::fmt::Result {
    writeln!(f, "{:digit_len$} = {} {}", "", DiagnosticKind::Note.marker(show_color), msg)
}

/// Given a location, return the line numbers to display the code for
fn lines_to_display(location: &Location) -> std::ops::Range<u32> {
    location.span.start.line_number.saturating_sub(1).max(1)..location.span.start.line_number + 2
}

/// Compute the line blocks to display. Returns 1 merged block if ranges
/// intersect or there's no note, or 2 blocks in source order if disjoint.
fn line_blocks(main_loc: &Location, note_loc: Option<&Location>) -> Vec<std::ops::Range<u32>> {
    let main = lines_to_display(main_loc);
    let Some(note_loc) = note_loc else {
        return vec![main];
    };
    let note = lines_to_display(note_loc);

    if main.start <= note.end && note.start <= main.end {
        vec![main.start.min(note.start)..main.end.max(note.end)]
    } else if note.start < main.start {
        vec![note, main]
    } else {
        vec![main, note]
    }
}

const MAX_LINE_WIDTH: usize = 100;

/// "\t// error:" roughly 9 chars minimum (1-space tab + "note" kind)
/// to 19 char maximum (8-space tab + "warning" kind)
const APPROX_COMMENT_OVERHEAD: usize = 15;

/// Convert a span to a `(start_column, end_column)` range clamped to the line length
fn span_to_columns(span: Span, line_len: usize) -> (usize, usize) {
    let start = span.start.column_number.max(1) as usize - 1;
    let len = (span.end.byte_index - span.start.byte_index).min(line_len - start);
    (start, start + len)
}

const KEYWORD_COLOR: Color = Color::Cyan;
const TYPE_COLOR: Color = Color::Blue;
const CONSTANT_COLOR: Color = Color::BrightMagenta;
const NAME_COLOR: Color = Color::Magenta;

/// Map a lexer token to a syntax highlight color.
fn syntax_color(token: &Token, snippet: &str) -> Option<Color> {
    match token {
        Token::TypeName(_) | Token::IntegerType(_) | Token::FloatType(_) => Some(TYPE_COLOR),

        Token::StringLiteral(_)
        | Token::CharLiteral(_)
        | Token::IntegerLiteral(..)
        | Token::FloatLiteral(..)
        | Token::BooleanLiteral(_)
        | Token::UnitLiteral => Some(CONSTANT_COLOR),

        _ if lookup_keyword(snippet).is_some() => Some(KEYWORD_COLOR),
        _ => None,
    }
}

/// Write `text` with syntax highlighting when `show_color` is true, otherwise plain.
fn write_syntax_highlighted(text: &str, show_color: bool, f: &mut Formatter) -> std::fmt::Result {
    if !show_color {
        return write!(f, "{text}");
    }

    let mut last_end = 0;
    for (token, span) in Lexer::new(text) {
        if matches!(token, Token::EndOfInput | Token::Newline | Token::Indent | Token::Unindent) {
            continue;
        }
        let start = span.start.byte_index;
        let end = span.end.byte_index.min(text.len());
        // Gaps between tokens are whitespace (or comments)
        if start > last_end {
            let text = &text[last_end..start];
            write_whitespace(text, f)?;
        }
        let snippet = &text[start..end];
        // Dim unhighlighted source text so error messages stand out against it
        let snippet =
            syntax_color(&token, snippet).map(|color| snippet.color(color)).unwrap_or_else(|| snippet.bright_black());
        write!(f, "{snippet}")?;
        last_end = end;
    }
    // Print any remaining whitespace
    if last_end < text.len() {
        write_whitespace(&text[last_end..], f)?;
    }
    Ok(())
}

fn write_whitespace(text: &str, f: &mut Formatter) -> std::fmt::Result {
    // Whitespace can contain comments that were ignored by the parser.
    // Highlight comments in bright_black.
    // TODO: Is avoiding unnecessary color codes for just whitespace worth the check here?
    if !text.chars().all(|c| c.is_whitespace()) { write!(f, "{}", text.bright_black()) } else { write!(f, "{text}") }
}

/// Write the source line with colored segments. Main span color takes priority over note.
fn write_colored_line(
    line: &str, main_range: Option<(usize, usize)>, note_range: Option<(usize, usize)>, show_color: bool,
    kind: DiagnosticKind, f: &mut Formatter,
) -> std::fmt::Result {
    let mut bounds = vec![0, line.len()];
    if let Some((s, e)) = main_range {
        bounds.extend([s, e]);
    }
    if let Some((s, e)) = note_range {
        bounds.extend([s, e]);
    }
    bounds.sort();
    bounds.dedup();

    for pair in bounds.windows(2) {
        let range = pair[0]..pair[1];
        let segment = &line[range.clone()];

        if range_contains(main_range, range.start, range.end) {
            let styled = kind.color(segment, show_color);
            write!(f, "{}", if show_color { styled.underline() } else { styled })?;
        } else if range_contains(note_range, range.start, range.end) {
            let styled = DiagnosticKind::Note.color(segment, show_color);
            write!(f, "{}", if show_color { styled.underline() } else { styled })?;
        } else {
            write_syntax_highlighted(segment, show_color, f)?;
        }
    }
    Ok(())
}

/// Write `^^^` / `---` indicators for no-color mode, aligned under the source line
fn write_no_color_indicator(
    digit_len: usize, main_range: Option<(usize, usize)>, note_range: Option<(usize, usize)>, f: &mut Formatter,
) -> std::fmt::Result {
    let end = main_range.map_or(0, |(_, e)| e).max(note_range.map_or(0, |(_, e)| e));
    let indicator: String = (0..end)
        .map(|i| {
            if range_contains(main_range, i, i + 1) {
                '^'
            } else if range_contains(note_range, i, i + 1) {
                '-'
            } else {
                ' '
            }
        })
        .collect();
    writeln!(f, "{:digit_len$} | {}", "", indicator.trim_end())
}

/// Format a message comment: ` // error: <message>`
fn format_message_comment(kind: DiagnosticKind, message: &str, show_color: bool) -> String {
    let comment = if show_color { "//".bright_black() } else { "//".into() };
    format!("\t{comment} {} {message}", kind.marker(show_color))
}

/// Write a message comment on its own line, aligned to the source indentation.
/// Uses blank padding instead of a line number.
fn write_overflow_message(
    digit_len: usize, indent: usize, kind: DiagnosticKind, message: &str, show_color: bool, f: &mut Formatter,
) -> std::fmt::Result {
    let comment = if show_color { "//".bright_black() } else { "//".into() };
    writeln!(f, "{:digit_len$} | {:indent$}{comment} {} {message}", "", "", kind.marker(show_color))
}

/// Outputs a formatted source line to the formatter (`line_no` is 1-indexed).
///
/// If `main`/`note` are set, their spans are highlighted and their messages
/// shown as comments. These comments are inline if they fit within [MAX_LINE_WIDTH],
/// otherwise they're on the line above.
fn output_source_line(
    file: &SourceFile, line_no: usize, digit_len: usize, main: Option<(Span, &String)>, note: Option<(Span, &String)>,
    show_color: bool, kind: DiagnosticKind, f: &mut Formatter,
) -> std::fmt::Result {
    let line = file.contents.lines().nth(line_no.saturating_sub(1)).unwrap_or("");
    if line.is_empty() && main.is_none() && note.is_none() {
        return Ok(());
    }

    let main_range = main.map(|(span, _)| span_to_columns(span, line.len()));
    let note_range = note.map(|(span, _)| span_to_columns(span, line.len()));

    if main_range.is_none() && note_range.is_none() {
        write!(f, "{line_no:digit_len$} | ")?;
        write_syntax_highlighted(line, show_color, f)?;
        return writeln!(f);
    }

    let messages = collect_messages(main, note, kind);
    let inline = messages_fit_inline(digit_len, line, &messages);

    if !inline {
        let indent = line.len() - line.trim_start().len();
        for (msg_kind, text) in &messages {
            write_overflow_message(digit_len, indent, *msg_kind, text, show_color, f)?;
        }
    }

    write!(f, "{line_no:digit_len$} | ")?;
    write_colored_line(line, main_range, note_range, show_color, kind, f)?;

    if inline {
        for (msg_kind, text) in &messages {
            write!(f, "{}", format_message_comment(*msg_kind, text, show_color))?;
        }
    }
    writeln!(f)?;

    if !show_color {
        write_no_color_indicator(digit_len, main_range, note_range, f)?;
    }
    Ok(())
}

/// Collect the diagnostic messages to display for a source line
fn collect_messages<'a>(
    main: Option<(Span, &'a String)>, note: Option<(Span, &'a String)>, kind: DiagnosticKind,
) -> Vec<(DiagnosticKind, &'a str)> {
    let mut messages = Vec::new();
    if let Some((_, msg)) = main {
        messages.push((kind, msg.as_str()));
    }
    if let Some((_, msg)) = note {
        messages.push((DiagnosticKind::Note, msg.as_str()));
    }
    messages
}

/// Compute the length of a string, ignoring ANSI escape sequences.
fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            len += 1;
        }
    }
    len
}

/// Check whether all message comments fit inline on the source line.
/// Multiple messages are never inlined together to keep each on its own line.
fn messages_fit_inline(digit_len: usize, line: &str, messages: &[(DiagnosticKind, &str)]) -> bool {
    if messages.len() > 1 {
        return false;
    }
    let prefix_len = digit_len + 3; // "{line_no} | "
    let suffix_len: usize = messages.iter().map(|(_, m)| visible_len(m) + APPROX_COMMENT_OVERHEAD).sum();
    prefix_len + line.len() + suffix_len <= MAX_LINE_WIDTH
}

/// True if the given range is `Some` and fully contains `[start, end)`
fn range_contains(range: Option<(usize, usize)>, start: usize, end: usize) -> bool {
    range.is_some_and(|(s, e)| start >= s && end <= e)
}

#[derive(Copy, Clone)]
pub enum DiagnosticKind {
    Error,
    Warning,
    #[allow(unused)]
    Note,
}

impl DiagnosticKind {
    fn marker(self, show_color: bool) -> ColoredString {
        match self {
            DiagnosticKind::Error => self.color("error:", show_color),
            DiagnosticKind::Warning => self.color("warning:", show_color),
            DiagnosticKind::Note => self.color("note:", show_color),
        }
    }

    /// Color the given string in either the error, warning, or note color
    fn color(self, msg: &str, show_color: bool) -> ColoredString {
        match (show_color, self) {
            (false, _) => msg.normal(),
            (_, DiagnosticKind::Error) => msg.red(),
            (_, DiagnosticKind::Warning) => msg.yellow(),
            (_, DiagnosticKind::Note) => msg.purple(),
        }
    }
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

    if show_color { ret.italic() } else { ret.normal() }
}

pub struct DiagnosticDisplay<'a> {
    diagnostic: &'a Diagnostic,
    compiler: &'a Db,
    show_color: bool,
}

impl std::fmt::Display for DiagnosticDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.diagnostic.format(f, self.show_color, self.compiler)
    }
}

/// Check the entire program, collecting all diagnostics
pub(crate) fn check_all(_: &CheckAll, compiler: &DbHandle) {
    let crates = GetCrateGraph.get(compiler);

    for crate_ in crates.values() {
        for file in crate_.source_files.values() {
            let parse = Parse(*file).get(compiler);

            for item in &parse.cst.top_level_items {
                TypeCheck(item.id).get(compiler);
            }

            ValidateExports(*file).get(compiler);
        }
    }

    let local_crate = &crates[&CrateId::LOCAL];
    let has_main = local_crate
        .source_files
        .values()
        .any(|file| AllDefinitions(*file).get(compiler).definitions.keys().any(|k| k.as_str() == "main"));

    if !has_main {
        if let Some(first_file) = local_crate.source_files.values().next() {
            let position = Position::start();
            let location = Span { start: position, end: position }.in_file(*first_file);
            compiler.accumulate(Diagnostic::NoMainFunction { location });
        }
    }
}

pub fn collect_all_diagnostics(compiler: &mut Db) -> BTreeSet<Diagnostic> {
    compiler.get_accumulated_uncached(CheckAll)
}
