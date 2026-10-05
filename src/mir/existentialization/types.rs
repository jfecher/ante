//! This module classifies types as static or dynamic, and lowers static types.
//!
//! A type is dynamic when its layout depends on a generic, and a dynamic value is represented by a
//! pointer to its storage. A static type lowers to the same layout monomorphization gives it, see
//! [Types::lower].

use rustc_hash::FxHashMap;

use crate::{
    iterator_extensions::mapvec,
    lexer::token::IntegerKind,
    mir::{PrimitiveType, Type},
};

pub(super) fn usz() -> Type {
    Type::int(IntegerKind::Usz)
}

pub(super) fn is_aggregate(typ: &Type) -> bool {
    matches!(typ, Type::Tuple(_) | Type::Union(_) | Type::Array { .. } | Type::Function(_))
}

/// True if a value of type `typ` needs to be behind a pointer
pub(super) fn indirect(typ: &Type, ssa_aggregates: bool) -> bool {
    is_dynamic(typ) || (!ssa_aggregates && is_aggregate(typ))
}

/// True if `typ` is a dynamic type whose layout should be precomputed, see
/// [super::convention::TypeInfoTable]
pub(super) fn is_key(typ: &Type) -> bool {
    is_aggregate(typ) && is_dynamic(typ)
}

/// True if the layout of `typ` depends on a generic
pub(super) fn is_dynamic(typ: &Type) -> bool {
    let mut dynamic = false;
    for_each_generic(typ, true, &mut |_| dynamic = true);
    dynamic
}

/// Calls `f` on each generic within `typ` outside of evidence, or only those its layout depends on
pub(super) fn for_each_generic(typ: &Type, layout_only: bool, f: &mut impl FnMut(u32)) {
    match typ {
        Type::Generic(generic) => f(generic.0),
        Type::Primitive(_) | Type::U32(_) | Type::Evidence(_) => (),
        Type::Tuple(fields) | Type::Union(fields) => {
            fields.iter().for_each(|field| for_each_generic(field, layout_only, f))
        },
        Type::Array { length, element } => {
            for_each_generic(length, layout_only, f);
            for_each_generic(element, layout_only, f);
        },
        Type::Function(function) if layout_only => {
            if let Some(environment) = function.environment() {
                for_each_generic(environment, layout_only, f);
            }
        },
        Type::Function(function) => {
            function.parameters.iter().for_each(|parameter| for_each_generic(parameter, layout_only, f));
            for_each_generic(&function.environment, layout_only, f);
            for_each_generic(&function.return_type, layout_only, f);
        },
    }
}

pub(super) struct Types {
    pub(super) ptr_size: u32,

    /// How many type infos a function value of each arity has room for
    reserved: FxHashMap<usize, usize>,
}

impl Types {
    pub(super) fn new(ptr_size: u32, reserved: FxHashMap<usize, usize>) -> Self {
        Self { ptr_size, reserved }
    }

    pub(super) fn reserved(&self, arity: usize) -> usize {
        self.reserved.get(&arity).copied().unwrap_or(0)
    }

    pub(super) fn lower(&self, typ: &Type) -> Type {
        if !needs_lowering(typ) {
            return typ.clone();
        }
        match typ {
            Type::Primitive(PrimitiveType::NoClosureEnv) => Type::UNIT,
            Type::Primitive(_) | Type::U32(_) => typ.clone(),
            Type::Tuple(fields) => Type::tuple(mapvec(fields.iter(), |field| self.lower(field))),
            Type::Union(variants) => {
                Type::find_largest_variant(&mapvec(variants.iter(), |variant| self.lower(variant)), self.ptr_size)
            },
            Type::Array { length, element } => Type::array_with_length(self.lower(length), self.lower(element)),
            Type::Function(function) => {
                let mut fields = self.function_value_prefix(function);
                fields.push(self.lower(&function.environment));
                Type::tuple(fields)
            },
            Type::Evidence(_) => Type::POINTER,
            Type::Generic(_) => panic!("existentialization: cannot lower the dynamic type `{typ}`"),
        }
    }

    /// Lower a type as C sees it, where a function is a plain function pointer
    pub(super) fn lower_c(&self, typ: &Type) -> Type {
        match typ {
            Type::Function(function) => Type::function(
                mapvec(&function.parameters, |parameter| self.lower_c(parameter)),
                self.lower_c(&function.return_type),
            ),
            other => self.lower(other),
        }
    }

    pub(super) fn static_layout(&self, typ: &Type) -> (u32, u32) {
        let lowered = self.lower(typ);
        (lowered.size_in_bytes(self.ptr_size), lowered.align_in_bytes(self.ptr_size))
    }

    /// True if `typ` holds no data, so values of it are never passed or stored
    pub(super) fn is_unit(&self, typ: &Type) -> bool {
        match typ {
            Type::Primitive(PrimitiveType::Unit | PrimitiveType::NoClosureEnv) => true,
            Type::Union(_) => !is_dynamic(typ) && self.lower(typ) == Type::UNIT,
            _ => false,
        }
    }
}

/// False if lowering `typ` would leave it unchanged
fn needs_lowering(typ: &Type) -> bool {
    match typ {
        Type::Primitive(PrimitiveType::NoClosureEnv) => true,
        Type::Primitive(_) | Type::U32(_) => false,
        Type::Tuple(fields) => fields.iter().any(needs_lowering),
        Type::Array { length, element } => needs_lowering(length) || needs_lowering(element),
        Type::Union(_) | Type::Function(_) | Type::Evidence(_) | Type::Generic(_) => true,
    }
}
