//! This module resolves what a value refers to when it is called, see [resolve] and
//! [known_closure]. Both the analysis and the rewrite rely on it.

use std::sync::Arc;

use crate::mir::{
    ConstantEnvironment, Definition, DefinitionId, Instruction, Mir, Type, Value, existentialization::analysis::Kind,
};

/// The type arguments a definition is referenced with
#[derive(Debug, Clone)]
pub(super) enum Bindings {
    /// The referencing definition's own generics, for a reference without an `Instantiate`
    Identity,
    Explicit(Arc<Vec<Type>>),
}

impl Bindings {
    pub(super) fn get(&self, generic: u32) -> Type {
        match self {
            Bindings::Identity => Type::generic(generic),
            Bindings::Explicit(bindings) => bindings[generic as usize].clone(),
        }
    }

    /// `typ` with each generic replaced by its binding
    pub(super) fn substitute(&self, typ: &Type) -> Type {
        match self {
            Bindings::Identity => typ.clone(),
            Bindings::Explicit(bindings) => typ.substitute(bindings),
        }
    }
}

/// What a value refers to when it is called
pub(super) enum Target {
    /// A function definition, called with the direct calling convention
    Function(DefinitionId, Bindings),
    /// A C function
    Extern,
    /// A function value
    Value,
}

/// Resolve what `value` refers to within `definition`
pub(super) fn resolve(value: &Value, definition: &Definition, kind: &impl Fn(DefinitionId) -> Kind) -> Target {
    let (id, bindings) = match definition.follow_ids(*value) {
        Value::Definition(id) => (id, Bindings::Identity),
        Value::InstructionResult(result) => match &definition.instructions[result] {
            Instruction::Instantiate(id, bindings) => (*id, Bindings::Explicit(bindings.clone())),
            Instruction::Extern(_) => return Target::Extern,
            _ => return Target::Value,
        },
        _ => return Target::Value,
    };
    match kind(id) {
        Kind::Function => Target::Function(id, bindings),
        Kind::Extern => Target::Extern,
        Kind::Global | Kind::ComputedGlobal => Target::Value,
    }
}

/// A closure read out of a constant global. This is usually a method of a trait impl
pub(super) struct KnownClosure {
    pub(super) function: DefinitionId,
    pub(super) bindings: Bindings,
    pub(super) environment: ConstantEnvironment,
}

/// Resolve a value of the form `global.index` where `global` is a constant tuple holding a closure
/// of a known function, so the call can skip the uniform calling convention.
///
/// This helps speed up existentialized code noticeably.
pub(super) fn known_closure(
    value: &Value, definition: &Definition, mir: &Mir, kind: &impl Fn(DefinitionId) -> Kind,
) -> Option<KnownClosure> {
    let Value::InstructionResult(id) = definition.follow_ids(*value) else { return None };
    let Instruction::IndexTuple { tuple, index } = &definition.instructions[id] else { return None };
    let Value::Definition(global_id) = definition.follow_ids(*tuple) else { return None };
    if kind(global_id) != Kind::Global {
        return None;
    }
    let global = &mir.definitions[&global_id];
    if global.generic_count > 0 {
        return None;
    }
    let (function, environment) = global.constant_closure_field(*index)?;
    let Target::Function(function, bindings) = resolve(&function, global, kind) else { return None };
    Some(KnownClosure { function, bindings, environment })
}
