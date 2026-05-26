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
    TypeConstructorSimple(NonZeroUsize),

    /// A type constructor expecting to be applied to N arbitrary arguments.
    /// It is not an explicit requirement for this type, but at least one
    /// argument is expected to not be a [Kind::Type], since otherwise
    /// [Kind::TypeConstructorSimple] can be used which avoids an allocation.
    ///
    /// Requires the Vec of parameters to be non-empty.
    TypeConstructorComplex(Vec<Kind>),

    /// A type-level `U32` used (for example) as an array length.
    U32,

    /// The lifetime of a temporary reference
    Lifetime,

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
            Kind::Type => {
                if args.is_empty() {
                    Ok(())
                } else {
                    Err(Diagnostic::ExpectedTypeKind { actual: Kind::from_args(args), location })
                }
            },
            Kind::TypeConstructorSimple(expected) => {
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
            Kind::TypeConstructorComplex(kinds) => {
                if kinds.len() != args.len() {
                    let actual = args.len();
                    return Err(Diagnostic::FunctionArgCountMismatch { actual, expected: kinds.len(), location });
                }

                for (expected, actual) in kinds.into_iter().zip(args) {
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
            (Kind::TypeConstructorSimple(l), Kind::TypeConstructorSimple(r)) => l == r,
            (Kind::TypeConstructorComplex(l_kinds), Kind::TypeConstructorComplex(r_kinds)) => {
                l_kinds.len() == r_kinds.len() && l_kinds.iter().zip(r_kinds).all(|(l, r)| l.unifies(r))
            },
            (Kind::U32, Kind::U32) => true,
            (Kind::Lifetime, Kind::Lifetime) => true,
            _ => false,
        }
    }

    /// Create a `Kind` that accepts the given arguments
    pub fn from_args(args: Vec<Kind>) -> Kind {
        if args.is_empty() {
            Kind::Type
        } else if args.iter().all(|arg| matches!(arg, Kind::Type)) {
            Kind::TypeConstructorSimple(NonZeroUsize::new(args.len()).unwrap())
        } else {
            Kind::TypeConstructorComplex(args)
        }
    }

    pub fn required_argument_count(&self) -> usize {
        use Kind::*;
        match self {
            Type | U32 | Lifetime | Error => 0,
            Kind::TypeConstructorSimple(n) => (*n).into(),
            Kind::TypeConstructorComplex(kinds) => kinds.len(),
        }
    }

    /// True if this Kind accepts `n` arguments (partial application is disallowed).
    pub fn accepts_n_arguments(&self, n: usize) -> bool {
        use Kind::*;
        match self {
            Type | U32 | Lifetime => n == 0,
            Kind::TypeConstructorSimple(count) => n == usize::from(*count),
            Kind::TypeConstructorComplex(kinds) => n == kinds.len(),
            Kind::Error => true,
        }
    }

    /// Returns the `n`th parameter's kind, zero-indexed.
    /// Panics if this kind does not support at least `n+1` parameters.
    pub fn get_nth_parameter_kind(&self, n: usize) -> Kind {
        match self {
            Kind::Type => panic!("Kind::Type has no parameters"),
            Kind::TypeConstructorSimple(count) => {
                assert!(n < usize::from(*count));
                Kind::Type
            },
            Kind::TypeConstructorComplex(kinds) => kinds[n].clone(),
            Kind::U32 => panic!("Kind::U32 has no parameters"),
            Kind::Lifetime => panic!("Kind::Lifetime has no parameters"),
            Kind::Error => Kind::Error, // Try to avoid further errors
        }
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let should_parenthesize = |kind: &Kind| match kind {
            Kind::Type | Kind::U32 | Kind::Lifetime | Kind::Error => false,
            Kind::TypeConstructorSimple(_) => true,
            Kind::TypeConstructorComplex(_) => true,
        };

        match self {
            Kind::Type => write!(f, "type"),
            Kind::TypeConstructorSimple(n) => {
                for _ in 0..usize::from(*n) {
                    write!(f, "type -> ")?;
                }
                write!(f, "type")
            },
            Kind::TypeConstructorComplex(kinds) => {
                for kind in kinds {
                    if should_parenthesize(kind) {
                        write!(f, "({kind}) -> ")?;
                    } else {
                        write!(f, "{kind} -> ")?;
                    }
                }
                write!(f, "type")
            },
            Kind::U32 => write!(f, "U32"),
            Kind::Lifetime => write!(f, "lifetime"),
            Kind::Error => write!(f, "<Error>"),
        }
    }
}
