//! This module defines the two calling conventions of existentialized code:
//! - The direct calling convention passes a pointer to the [TypeInfoTable] its callee needs, then
//!   its arguments, then a result pointer if the result is dynamic. See `FunctionBuilder::call_direct`.
//! - The uniform calling convention, used by every function value, passes each argument but
//!   evidence by address, then the result pointer, then the function value's own address. See
//!   `FunctionBuilder::call_value`. A function value is `(code, type infos.., environment)`, see
//!   [Types::function_value_fields].

use crate::{
    lexer::token::IntegerKind,
    mir::{
        DefinitionId, FunctionType, Type,
        existentialization::{
            Shared,
            analysis::Kind,
            types::{Types, indirect, usz},
        },
    },
};

// The direct calling convention

/// A parameter or result of type `typ` is passed by address when it is indirect
pub(super) fn by_address(typ: &Type) -> bool {
    indirect(typ, false)
}

impl Shared<'_> {
    /// Whether a parameter of type `typ` is passed at all
    pub(super) fn passed(&self, typ: &Type) -> bool {
        !self.types.is_unit(typ)
    }

    /// The type a passed parameter of type `typ` takes
    pub(super) fn parameter_type(&self, typ: &Type) -> Type {
        if by_address(typ) { Type::POINTER } else { self.types.lower(typ) }
    }

    /// Whether `id` writes its result through a pointer passed as its last parameter
    pub(super) fn has_result_slot(&self, id: DefinitionId) -> bool {
        matches!(self.kind(id), Kind::Function | Kind::ComputedGlobal) && by_address(&self.return_type(id))
    }

    /// Whether the direct calling convention of the function `target` differs from C's
    pub(super) fn needs_native_thunk(&self, target: DefinitionId) -> bool {
        let parameters = &self.mir.definitions[&target].entry_block().parameter_types;
        parameters.iter().any(|typ| by_address(typ) || !self.passed(typ)) || self.has_result_slot(target)
    }
}

/// The type info table a definition takes as its first parameter. It starts with the
/// `(size, align, id)` of each needed generic. Then for each precomputed layout it holds the
/// type's size, alignment mask, and, for a tuple or function value, the offset of each field.
///
/// Layouts are precomputed for the dynamic aggregates a definition uses, see
/// [super::types::is_key]. A direct caller precomputes them so the callee need not.
pub(super) struct TypeInfoTable {
    pub(super) fields: Vec<Type>,
    /// The byte offset of each field
    pub(super) offsets: Vec<u32>,
    /// The index of the first field of each precomputed layout
    pub(super) key_fields: Vec<usize>,
}

/// The `(size, align, id)` of a type. A type-level `U32` stores its value as its size.
pub(super) fn type_info_type() -> Type {
    Type::tuple(type_info_type_fields())
}

pub(super) fn type_info_type_fields() -> Vec<Type> {
    vec![usz(), usz(), Type::int(IntegerKind::U64)]
}

impl Types {
    pub(super) fn table(&self, needed: usize, keys: &[Type]) -> TypeInfoTable {
        let mut fields = Vec::with_capacity(needed * 3 + keys.len() * 2);
        for _ in 0..needed {
            fields.extend(type_info_type_fields());
        }
        let mut key_fields = Vec::with_capacity(keys.len());
        for key in keys {
            key_fields.push(fields.len());
            fields.extend(std::iter::repeat_n(usz(), 2 + self.key_field_count(key)));
        }
        let offsets = Type::field_offsets(&fields, self.ptr_size);
        TypeInfoTable { fields, offsets, key_fields }
    }

    /// The number of field offsets a type info table holds for the precomputed layout of `key`
    /// after its size and alignment
    pub(super) fn key_field_count(&self, key: &Type) -> usize {
        match key {
            Type::Tuple(elements) => elements.len(),
            Type::Function(function) => self.reserved(function.parameters.len()) + 2,
            _ => 0,
        }
    }
}

// The uniform calling convention

/// The code of a function value takes each argument by address followed by the result pointer
/// and its own address.
pub(super) fn existential_function_type(arity: usize) -> Type {
    Type::function(vec![Type::POINTER; arity + 2], Type::POINTER)
}

pub(super) fn arity(function: &Type) -> usize {
    match function {
        Type::Function(function) => function.parameters.len(),
        other => panic!("existentialization: expected a function type, found `{other}`"),
    }
}

impl Types {
    /// A function value's code as a `Pointer`, reserved type infos, and environment
    pub(super) fn function_value_fields(&self, function: &FunctionType) -> Vec<Type> {
        let arity = function.parameters.len();
        let mut fields = vec![Type::POINTER];
        fields.extend(std::iter::repeat_n(type_info_type(), self.reserved(arity)));
        fields.push(function.environment.clone());
        fields
    }

    /// A function value's code and reserved type infos, already lowered
    pub(super) fn function_value_prefix(&self, function: &FunctionType) -> Vec<Type> {
        let mut fields = self.function_value_fields(function);
        fields.truncate(self.reserved(function.parameters.len()) + 1);
        fields[0] = existential_function_type(function.parameters.len());
        fields
    }
}
