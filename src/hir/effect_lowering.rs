//! This pass on the HIR occurs after monomorphisation and is meant to convert
//! each usage of effects and effect handlers to coroutines.
//! This entails:
//! - Each effect in a function signature translates to a coroutine parameter
//! - Each handle expression is lowered to a switch on the effect
//! - Each effect call is a suspension of the corresponding coroutine
//! - Each resume call resumes the relevant coroutine
use crate::hir::Ast;

/// Run the effect lowering pass on the given Hir
pub fn convert_effects_to_coroutines(hir: Ast) {
    test
}
