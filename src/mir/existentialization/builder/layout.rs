//! The layouts of types in terms of the generics in scope. A static type's layout is a constant,
//! and a dynamic type's is read from the type info table or computed in the prologue.

use std::sync::Arc;

use crate::{
    iterator_extensions::mapvec,
    mir::{
        Instruction, Type, Value,
        existentialization::{
            builder::{At, FunctionBuilder, Op},
            types::is_dynamic,
        },
    },
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    pub(crate) size: Op,
    /// One less than the alignment, so the larger of two is their bitwise or
    pub(crate) align_mask: Op,
}

impl FunctionBuilder<'_> {
    /// The size and alignment of `typ`, in terms of the generics in scope
    pub(crate) fn layout(&mut self, typ: &Type) -> Layout {
        if !is_dynamic(typ) {
            let (size, align) = self.shared.types.static_layout(typ);
            return Layout { size: Op::Const(size as u64), align_mask: Op::Const(align as u64 - 1) };
        }
        if matches!(typ, Type::Tuple(_) | Type::Function(_)) {
            return self.tuple_layout(typ).0;
        }
        if let Some(layout) = self.cache.layouts.get(typ) {
            return *layout;
        }
        if let Some((layout, _)) = self.key_layout(typ) {
            self.cache.layouts.insert(typ.clone(), layout);
            return layout;
        }
        let layout = match typ {
            Type::Generic(generic) => {
                let (size, align) = self.generic_size_align(generic.0);
                Layout { size, align_mask: self.sub(align, Op::Const(1)) }
            },
            Type::Union(variants) => {
                // Mirrors `find_largest_variant`: the largest size rounded up to the largest alignment
                let mut largest = self.layout(&variants[0]);
                for variant in &variants[1..] {
                    let layout = self.layout(variant);
                    let keep = self.less_mask(layout.size, largest.size);
                    let size = self.select(keep, largest.size, layout.size);
                    let align_mask = self.or(largest.align_mask, layout.align_mask);
                    largest = Layout { size, align_mask };
                }
                Layout { size: self.align_up(largest.size, largest.align_mask), ..largest }
            },
            Type::Array { length, element } => {
                let stride = self.stride(element);
                let align_mask = self.layout(element).align_mask;
                let length = self.array_length(length);
                Layout { size: self.mul(stride, length), align_mask }
            },
            Type::Primitive(_) | Type::U32(_) | Type::Evidence(_) | Type::Tuple(_) | Type::Function(_) => {
                unreachable!("handled above")
            },
        };
        self.cache.layouts.insert(typ.clone(), layout);
        layout
    }

    /// The distance between consecutive array elements of type `element`
    pub(crate) fn stride(&mut self, element: &Type) -> Op {
        let layout = self.layout(element);
        self.align_up(layout.size, layout.align_mask)
    }

    /// The layout and field offsets of a tuple or function value, mirroring [Type::size_in_bytes]
    pub(super) fn tuple_layout(&mut self, typ: &Type) -> (Layout, Arc<Vec<Op>>) {
        if let Some((layout, offsets)) = self.cache.tuples.get(typ) {
            return (*layout, offsets.clone());
        }
        if let Some((layout, offsets)) = self.key_layout(typ) {
            let offsets = Arc::new(offsets);
            self.cache.tuples.insert(typ.clone(), (layout, offsets.clone()));
            return (layout, offsets);
        }
        let function_fields;
        let fields = match typ {
            Type::Tuple(fields) => fields.as_slice(),
            Type::Function(function) => {
                function_fields = self.shared.types.function_value_fields(function);
                &function_fields
            },
            other => panic!("existentialization: `{other}` has no fields"),
        };
        let mut offset = Op::Const(0);
        let mut align_mask = Op::Const(0);
        let mut nonempty = false;
        let mut offsets = Vec::with_capacity(fields.len());
        for field in fields {
            let field = self.layout(field);
            offset = self.align_up(offset, field.align_mask);
            offsets.push(offset);
            offset = self.add(offset, field.size);
            align_mask = self.or(align_mask, field.align_mask);
            nonempty |= matches!(field.size, Op::Const(size) if size > 0);
        }
        let mut size = self.align_up(offset, align_mask);
        // An empty struct still takes up a byte
        if !nonempty {
            let empty = self.less_mask(size, Op::Const(1));
            size = self.select(empty, Op::Const(1), size);
        }
        let layout = Layout { size, align_mask };
        let offsets = Arc::new(offsets);
        self.cache.tuples.insert(typ.clone(), (layout, offsets.clone()));
        (layout, offsets)
    }

    /// The offset of each field of a tuple or function value
    pub(crate) fn offsets(&mut self, typ: &Type) -> Arc<Vec<Op>> {
        self.tuple_layout(typ).1
    }

    pub(crate) fn array_length(&mut self, length: &Type) -> Op {
        match length {
            Type::U32(length) => Op::Const(*length as u64),
            Type::Generic(generic) => self.generic_size_align(generic.0).0,
            other => panic!("existentialization: `{other}` is not an array length"),
        }
    }

    /// The layout of `typ` if this function's type info table holds it, and the offsets of its fields if any
    fn key_layout(&mut self, typ: &Type) -> Option<(Layout, Vec<Op>)> {
        let table = self.table.as_ref()?;
        let first = table.layout.key_fields[*table.key_indices.get(typ)?];
        let size = Op::Value(self.table_field(first));
        let align_mask = Op::Value(self.table_field(first + 1));
        let offsets = mapvec(0..self.shared.types.key_field_count(typ), |field| match field {
            0 => Op::Const(0),
            _ => Op::Value(self.table_field(first + 2 + field)),
        });
        Some((Layout { size, align_mask }, offsets))
    }

    /// Read field `field` of this function's type info table
    pub(super) fn table_field(&mut self, field: usize) -> Value {
        let table = self.table.as_ref().expect("existentialization: no type infos in scope");
        if let Some(value) = table.reads.get(&field) {
            return *value;
        }
        let (pointer, offset, typ) = (table.pointer, table.layout.offsets[field], table.layout.fields[field].clone());
        let pointer = self.offset_at(At::Prologue, pointer, Op::Const(offset.into()));
        let value = self.emit_prologue(Instruction::Deref(pointer), typ);
        self.table.as_mut().unwrap().reads.insert(field, value);
        value
    }
}
