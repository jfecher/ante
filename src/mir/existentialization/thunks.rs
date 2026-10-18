//! This module generates thunks, which adapt a function to a calling convention other than its
//! direct calling convention:
//! - A value thunk lets a function be called as a function value, see [value_thunk].
//! - A native thunk lets C call a function, see [native_thunk].

use std::sync::Arc;

use crate::{
    iterator_extensions::mapvec,
    mir::{
        BlockId, Definition, DefinitionId, Instruction, TerminatorInstruction, Type,
        existentialization::{
            Shared,
            builder::{Arg, FunctionBuilder},
            resolve::Bindings,
            types::is_dynamic,
        },
        next_definition_id,
    },
};

impl Shared<'_> {
    /// The code of function values referring to `target`
    pub(super) fn thunk(&self, target: DefinitionId) -> DefinitionId {
        if self.uniform(target) {
            return target;
        }
        *self.thunks.entry(target).or_insert_with(next_definition_id)
    }

    // If `target` needs extra type info args we can't pass to C, return a native thunk that we can
    pub(super) fn native(&self, target: DefinitionId) -> DefinitionId {
        if !self.needs_native_thunk(target) {
            return target;
        }
        *self.native_thunks.entry(target).or_insert_with(next_definition_id)
    }
}

/// The code of function values referring to `target`, calling it with the direct calling convention
pub(super) fn value_thunk(shared: &Shared, target: DefinitionId, id: DefinitionId) -> Definition {
    let definition = &shared.mir.definitions[&target];
    let name = Arc::new(format!("{}_value", definition.name));
    let (mut builder, addresses, result) = FunctionBuilder::new_uniform(shared, name, id, definition);
    let args = mapvec(addresses.into_iter().zip(&definition.entry_block().parameter_types), |(address, typ)| {
        let arg = if matches!(typ, Type::Evidence(_)) { Arg::Value(address) } else { Arg::Address(address) };
        (arg, typ.clone())
    });

    let return_type = shared.return_type(target);
    builder.call_direct(target, &Bindings::Identity, args, &return_type, Some(result));
    builder.function.blocks[BlockId::ENTRY_BLOCK].terminator = Some(TerminatorInstruction::Return(result));
    builder.finish()
}

/// A function C can call in place of `target`, which passes aggregates by value and has
/// no type info table
pub(super) fn native_thunk(shared: &Shared, target: DefinitionId, id: DefinitionId) -> Definition {
    let definition = &shared.mir.definitions[&target];
    let unsupported = |typ: &Type| is_dynamic(typ) || matches!(typ, Type::Function(_));
    let parameters = &definition.entry_block().parameter_types;
    let return_type = shared.return_type(target);
    if parameters.iter().any(unsupported) || unsupported(&return_type) {
        panic!(
            "existentialization: `{}` is passed to C but takes or returns a function or dynamic value",
            definition.name
        )
    }
    let name = Arc::new(format!("{}_native", definition.name));
    let typ = shared.types.lower_c(&definition.typ);
    let mut builder = FunctionBuilder::new(shared, name, id, typ, &[]);

    let args = mapvec(parameters, |typ| {
        let parameter = builder.push_parameter(shared.types.lower_c(typ));
        if !builder.indirect(typ) {
            return (Arg::Value(parameter), typ.clone());
        }
        let slot = builder.slot(typ);
        builder.emit(Instruction::Store { pointer: slot, value: parameter }, Type::UNIT);
        (Arg::Address(slot), typ.clone())
    });
    let result = builder.call_direct(target, &Bindings::Identity, args, &return_type, None);
    let result = builder.direct(Arg::Value(result), &return_type);
    builder.function.blocks[BlockId::ENTRY_BLOCK].terminator = Some(TerminatorInstruction::Return(result));
    builder.finish()
}
