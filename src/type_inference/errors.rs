use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Location,
    parser::ids::{ExprId, NameId, PathId, PatternId, TopLevelId},
    type_inference::{TypeChecker, types::NO_CLOSURE_ENV_STRING},
};

/// Different kinds of type errors.
/// All of these boil down to "expected {expected}, but found {actual}" but each
/// variant carries more contextual information on the location of this error.
/// E.g. "then-clause of type {expected} does not match the else-clause's type of {actual}"
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum TypeErrorKind {
    /// A general type error with no specific verbage
    General,
    /// `expr : Type` where `Type` does not match the inferred type of `expr`
    TypeAnnotationMismatch,
    /// if's `else` type (actual) does not match its `then` type (expected)
    Else,
    /// match branch type (actual) does not match the type of the first branch (expected)
    MatchBranch,
    /// `if` is used without an `else` so it always returns Unit, but a non-Unit return was expected
    IfStatement,
    /// `actual` Type of a lambda is not a function like expected
    Lambda { expected_parameter_count: usize },
    /// A closure's actual captured variables does not match its expected type
    ClosureEnv,
    /// A function call argument with the given 0-based index
    CallArgument { index: usize },
    /// The return type of a function call (actual) does not match the
    /// type expected by the surrounding context (expected)
    FunctionReturn,
    /// `main` function defined with a type other than `fn Unit -> Unit pure`
    MainFn,
    /// A name is bound (actual) with a type conflicting with a previous binding (expected)
    NameAlreadyBound,
    /// A pattern's type (actual) does not match the type of the value being matched (expected)
    Pattern,
    /// A called expression's type (actual) does not match the function type it is called as (expected)
    Callee,
    /// A function body's type (actual) does not match the function's return type (expected)
    FunctionBody,
    /// A method's expected object type (actual) does not match the object it is called on (expected)
    MethodObject,
    /// An if/while condition (actual) is not a Bool (expected)
    Condition,
    /// A constructor field value (actual) does not match the field's declared type (expected)
    ConstructorField,
    /// An effect operation's type (actual) does not match its handler pattern (expected)
    EffectPattern,
    /// A compound assignment operator's type (actual) does not match its operand types (expected)
    CompoundOperator,
    /// An assigned value (actual) does not match the assignment target's type (expected)
    Assignment,
    /// A returned value (actual) does not match the enclosing function's return type (expected)
    Return,
    /// A loop body (actual) is not Unit-typed (expected)
    LoopBody,
    /// A for-range bound (actual) does not match the range's integer type (expected)
    LoopRange,
    /// An array literal element (actual) does not match the array's element type (expected)
    ArrayElement,
}

impl TypeErrorKind {
    pub fn message(self, actual_type: &str, expected_type: &str) -> String {
        let actual = actual_type.blue();
        let expected = expected_type.blue();
        match self {
            TypeErrorKind::General => format!("Expected {expected} but found {actual}"),
            TypeErrorKind::TypeAnnotationMismatch => {
                format!("Type annotation {expected} does not match the inferred type {actual}")
            },
            TypeErrorKind::Else => {
                format!("Then branch's type of {expected} does not match the else branch's type {actual}")
            },
            TypeErrorKind::MatchBranch => format!(
                "This match branch has type {actual} which does not match the first branch's type of {expected}"
            ),
            TypeErrorKind::IfStatement => {
                format!("This `if` has no `else` so it always returns {actual}, but {expected} was expected instead")
            },
            TypeErrorKind::Lambda { expected_parameter_count } => {
                let s = if expected_parameter_count == 1 { "" } else { "s" };
                format!("Expected a function with {expected_parameter_count} parameter{s}, but found {actual}")
            },
            TypeErrorKind::ClosureEnv => {
                if expected_type == NO_CLOSURE_ENV_STRING {
                    format!(
                        "Expected a free function, but this closure captures variable(s) in the outer scope. The captured environment is of type {actual}"
                    )
                } else if actual_type == NO_CLOSURE_ENV_STRING {
                    format!("Expected the closure environment {expected}, but the actual environment was empty")
                } else {
                    format!(
                        "Expected the closure environment {expected}, but the actual environment was of type {actual}"
                    )
                }
            },
            TypeErrorKind::CallArgument { index } => {
                format!("Argument {} has type {actual} but {expected} was expected", index + 1)
            },
            TypeErrorKind::FunctionReturn => {
                format!("This function call returns {actual} but {expected} was expected")
            },
            TypeErrorKind::MainFn => {
                format!("{} here has type {actual} but it should always have type {expected}", "main".purple())
            },
            TypeErrorKind::NameAlreadyBound => {
                format!("This is bound to type {actual} here but was previously bound to type {expected}")
            },
            TypeErrorKind::Pattern => {
                format!("This pattern has type {actual} but the matched value has type {expected}")
            },
            TypeErrorKind::Callee => {
                format!("This is called as if it were a {expected} but it has type {actual}")
            },
            TypeErrorKind::FunctionBody => {
                format!("The body of this function has type {actual} but it is expected to return {expected}")
            },
            TypeErrorKind::MethodObject => {
                format!("This method expects an object of type {actual} but was called on a value of type {expected}")
            },
            TypeErrorKind::Condition => {
                format!("Conditions must have type {expected} but this condition has type {actual}")
            },
            TypeErrorKind::ConstructorField => {
                format!("This field value has type {actual} but the field is declared with type {expected}")
            },
            TypeErrorKind::EffectPattern => {
                format!("This handler pattern has type {expected} but the effect operation has type {actual}")
            },
            TypeErrorKind::CompoundOperator => {
                format!("This operator has type {actual} but its operands require it to have type {expected}")
            },
            TypeErrorKind::Assignment => {
                format!("Cannot assign a value of type {actual} to a target of type {expected}")
            },
            TypeErrorKind::Return => {
                format!("This returns a value of type {actual} but the enclosing function returns {expected}")
            },
            TypeErrorKind::LoopBody => {
                format!("Loop bodies must have type {expected} but this body has type {actual}")
            },
            TypeErrorKind::LoopRange => {
                format!("This range bound has type {actual} but {expected} was expected")
            },
            TypeErrorKind::ArrayElement => {
                format!("This array element has type {actual} but {expected} was expected")
            },
        }
    }
}

pub(super) trait Locateable {
    fn locate(&self, context: &TypeChecker) -> Location;
}

impl Locateable for Location {
    fn locate(&self, _: &TypeChecker) -> Location {
        self.clone()
    }
}

impl Locateable for ExprId {
    fn locate(&self, context: &TypeChecker) -> Location {
        context.current_extended_context().expr_location(*self)
    }
}

impl Locateable for PatternId {
    fn locate(&self, context: &TypeChecker) -> Location {
        context.current_context().pattern_location(*self).clone()
    }
}

impl Locateable for PathId {
    fn locate(&self, context: &TypeChecker) -> Location {
        context.current_extended_context().path_location(*self)
    }
}

impl Locateable for NameId {
    fn locate(&self, context: &TypeChecker) -> Location {
        context.current_context().name_location(*self).clone()
    }
}

impl Locateable for TopLevelId {
    fn locate(&self, context: &TypeChecker) -> Location {
        let (_, context, _) = &context.item_contexts[self];
        context.location().clone()
    }
}
