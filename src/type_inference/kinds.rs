use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use crate::diagnostics::{Diagnostic, Location};

/// A type's [Kind] is essentially the type of a type.
/// These differentiate whether something in a type position is itself
/// a type, a type constructor, or a type-level integer.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Hash, Clone)]
pub enum Kind {
    /// A type valid in a type position
    Type,

    /// A type constructor expecting to be applied to N arguments,
    /// each of [Kind::Type]. This isn't required but is separated
    /// from [Kind::TypeConstructorComplex] to avoid allocation in the common case.
    ///
    /// `result` is the kind produced once fully applied.
    TypeConstructorSimple { arity: NonZeroUsize, result: Box<Kind> },

    /// A type constructor expecting to be applied to N arbitrary arguments.
    /// It is not an explicit requirement for this type, but at least one
    /// argument is expected to not be a [Kind::Type], since otherwise
    /// [Kind::TypeConstructorSimple] can be used which avoids an allocation.
    ///
    /// Requires the Vec of parameters to be non-empty.
    TypeConstructorComplex { params: Vec<Kind>, result: Box<Kind> },

    /// A type-level `U32` used (for example) as an array length.
    U32,

    /// The lifetime of a temporary reference
    Lifetime,

    /// The kind of a fully-applied, concrete effect like `Fail` or `Throw String`.
    Effect,

    /// An error occurred while resolving the type this kind belongs to
    Error,
}

impl Kind {
    /// Try to accept the given arguments, returning a diagnostic explaining the error if it cannot
    /// be done.
    ///
    /// TODO: Need location for each Kind to improve errors
    pub fn accepts_arguments(self, args: Vec<Kind>, location: Location) -> Result<(), Diagnostic> {
        match self {
            Kind::Type | Kind::Effect => {
                if args.is_empty() {
                    Ok(())
                } else {
                    Err(Diagnostic::ExpectedTypeKind { actual: Kind::from_args(args), location })
                }
            },
            Kind::TypeConstructorSimple { arity: expected, .. } => {
                if args.len() != expected.into() {
                    let actual = args.len();
                    return Err(Diagnostic::FunctionArgCountMismatch { actual, expected: expected.into(), location });
                }

                for arg in args {
                    if !arg.unifies(&Kind::Type) {
                        return Err(Diagnostic::ExpectedTypeKind { actual: arg, location });
                    }
                }
                Ok(())
            },
            Kind::TypeConstructorComplex { params, .. } => {
                if params.len() != args.len() {
                    let actual = args.len();
                    return Err(Diagnostic::FunctionArgCountMismatch { actual, expected: params.len(), location });
                }

                for (expected, actual) in params.into_iter().zip(args) {
                    if !expected.unifies(&actual) {
                        return Err(Diagnostic::ExpectedKind { actual, expected, location });
                    }
                }
                Ok(())
            },
            Kind::U32 | Kind::Lifetime => {
                if args.is_empty() {
                    Ok(())
                } else {
                    let actual = Kind::from_args(args);
                    Err(Diagnostic::ExpectedKind { actual, expected: self, location })
                }
            },
            Kind::Error => Ok(()),
        }
    }

    /// True if both kinds are compatible
    pub fn unifies(&self, other: &Kind) -> bool {
        match (self, other) {
            (Kind::Error, _) | (_, Kind::Error) => true,
            (Kind::Type, Kind::Type) => true,
            (Kind::Effect, Kind::Effect) => true,
            (
                Kind::TypeConstructorSimple { arity: l, result: l_res },
                Kind::TypeConstructorSimple { arity: r, result: r_res },
            ) => l == r && l_res.unifies(r_res),
            (
                Kind::TypeConstructorComplex { params: l_kinds, result: l_res },
                Kind::TypeConstructorComplex { params: r_kinds, result: r_res },
            ) => l_kinds.len() == r_kinds.len() && l_kinds.iter().zip(r_kinds).all(|(l, r)| l.unifies(r)) && l_res.unifies(r_res),
            (Kind::U32, Kind::U32) => true,
            (Kind::Lifetime, Kind::Lifetime) => true,
            _ => false,
        }
    }

    /// Create a `Kind` that accepts the given arguments and produces a `Kind::Type` result.
    pub fn from_args(args: Vec<Kind>) -> Kind {
        Kind::from_args_with_result(args, Kind::Type)
    }

    /// Create a `Kind` that accepts the given arguments and produces the given result kind.
    pub fn from_args_with_result(args: Vec<Kind>, result: Kind) -> Kind {
        if args.is_empty() {
            result
        } else if args.iter().all(|arg| matches!(arg, Kind::Type)) {
            Kind::TypeConstructorSimple { arity: NonZeroUsize::new(args.len()).unwrap(), result: Box::new(result) }
        } else {
            Kind::TypeConstructorComplex { params: args, result: Box::new(result) }
        }
    }

    /// The kind produced once this type constructor is fully applied; `self` otherwise.
    pub fn result_kind(&self) -> Kind {
        match self {
            Kind::TypeConstructorSimple { result, .. } => (**result).clone(),
            Kind::TypeConstructorComplex { result, .. } => (**result).clone(),
            other => other.clone(),
        }
    }

    pub fn required_argument_count(&self) -> usize {
        use Kind::*;
        match self {
            Type | U32 | Lifetime | Error | Effect => 0,
            Kind::TypeConstructorSimple { arity, .. } => (*arity).into(),
            Kind::TypeConstructorComplex { params, .. } => params.len(),
        }
    }

    /// True if this Kind accepts `n` arguments (partial application is disallowed).
    pub fn accepts_n_arguments(&self, n: usize) -> bool {
        use Kind::*;
        match self {
            Type | U32 | Lifetime | Effect => n == 0,
            Kind::TypeConstructorSimple { arity, .. } => n == usize::from(*arity),
            Kind::TypeConstructorComplex { params, .. } => n == params.len(),
            Kind::Error => true,
        }
    }

    /// Returns the `n`th parameter's kind, zero-indexed.
    /// Panics if this kind does not support at least `n+1` parameters.
    pub fn get_nth_parameter_kind(&self, n: usize) -> Kind {
        match self {
            Kind::Type => panic!("Kind::Type has no parameters"),
            Kind::Effect => panic!("Kind::Effect has no parameters"),
            Kind::TypeConstructorSimple { arity, .. } => {
                assert!(n < usize::from(*arity));
                Kind::Type
            },
            Kind::TypeConstructorComplex { params, .. } => params[n].clone(),
            Kind::U32 => panic!("Kind::U32 has no parameters"),
            Kind::Lifetime => panic!("Kind::Lifetime has no parameters"),
            Kind::Error => Kind::Error, // Try to avoid further errors
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let should_parenthesize = |kind: &Kind| match kind {
            Kind::Type | Kind::U32 | Kind::Lifetime | Kind::Error | Kind::Effect => false,
            Kind::TypeConstructorSimple { .. } => true,
            Kind::TypeConstructorComplex { .. } => true,
        };

        match self {
            Kind::Type => write!(f, "type"),
            Kind::Effect => write!(f, "effect"),
            Kind::TypeConstructorSimple { arity, result } => {
                for _ in 0..usize::from(*arity) {
                    write!(f, "type -> ")?;
                }
                write!(f, "{result}")
            },
            Kind::TypeConstructorComplex { params, result } => {
                for kind in params {
                    if should_parenthesize(kind) {
                        write!(f, "({kind}) -> ")?;
                    } else {
                        write!(f, "{kind} -> ")?;
                    }
                }
                write!(f, "{result}")
            },
            Kind::U32 => write!(f, "U32"),
            Kind::Lifetime => write!(f, "lifetime"),
            Kind::Error => write!(f, "<Error>"),
        }
    }
}
