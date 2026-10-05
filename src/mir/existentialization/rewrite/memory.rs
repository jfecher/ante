//! Rewriting aggregates, loads, stores, allocations, and atomics. A static value keeps its
//! instruction, and a dynamic one is built in or copied through its storage.

use crate::{
    iterator_extensions::mapvec,
    mir::{
        Instruction, InstructionId, Type, Value,
        existentialization::{
            atomics::AtomicOperation,
            builder::{Op, usz_value},
            rewrite::{Hint, Rewriter},
            types::{is_dynamic, usz},
        },
    },
};

impl Rewriter<'_> {
    pub(super) fn index_tuple(&mut self, tuple: &Value, index: u32, typ: &Type) -> Value {
        let tuple_type = self.old_type(tuple);
        if !self.builder.indirect(&tuple_type) {
            let tuple = self.direct(tuple);
            let lowered = self.lower(typ);
            // A static tuple's fields are static, and a tuple kept in SSA keeps its static fields there too
            debug_assert!(!self.builder.indirect(typ));
            return self.builder.emit(Instruction::IndexTuple { tuple, index }, lowered);
        }
        let tuple = self.value(tuple);
        let offset = self.builder.offsets(&tuple_type)[index as usize];
        let field = self.builder.offset(tuple, offset);
        if self.builder.indirect(typ) { field } else { self.builder.load_immutable(field, typ) }
    }

    pub(super) fn make_tuple(&mut self, id: InstructionId, fields: &[Value], typ: &Type) -> Value {
        if !self.builder.indirect(typ) {
            let fields = mapvec(fields, |field| self.direct(field));
            let typ = self.lower(typ);
            return self.builder.emit(Instruction::MakeTuple(fields), typ);
        }
        let slot = self.storage(id, typ);
        let offsets = self.builder.offsets(typ);
        for (index, (field, offset)) in fields.iter().zip(offsets.iter()).enumerate() {
            let (arg, field_type) = self.arg(field);

            // A field built in place already has its storage within this tuple
            let in_place = match field {
                Value::InstructionResult(field) if self.hints.get(field) == Some(&Hint::Field(id, index)) => {
                    self.destinations.get(field).copied()
                },
                _ => None,
            };
            let pointer = in_place.unwrap_or_else(|| self.builder.offset(slot, *offset));
            self.builder.store(pointer, arg, &field_type);
        }
        slot
    }

    pub(super) fn make_array(&mut self, id: InstructionId, elements: &[Value], typ: &Type) -> Value {
        if !self.builder.indirect(typ) {
            let elements = mapvec(elements, |element| self.direct(element));
            let typ = self.lower(typ);
            return self.builder.emit(Instruction::MakeArray(elements), typ);
        }
        let Type::Array { element: element_type, .. } = typ else { panic!("MakeArray of non-array `{typ}`") };
        let slot = self.storage(id, typ);
        let stride = self.builder.stride(element_type);
        let mut offset = Op::Const(0);
        for element in elements {
            let (arg, element_type) = self.arg(element);
            let pointer = self.builder.offset(slot, offset);
            self.builder.store(pointer, arg, &element_type);
            offset = self.builder.add(offset, stride);
        }
        slot
    }

    pub(super) fn get_field_ptr(&mut self, struct_ptr: &Value, struct_type: &Type, index: u32) -> Value {
        let struct_ptr = self.direct(struct_ptr);
        if is_dynamic(struct_type) {
            let offset = self.builder.offsets(struct_type)[index as usize];
            return self.builder.offset(struct_ptr, offset);
        }
        let struct_type = self.lower(struct_type);
        self.builder.emit(Instruction::GetFieldPtr { struct_ptr, struct_type, index }, Type::POINTER)
    }

    pub(super) fn deref(&mut self, id: InstructionId, pointer: &Value, typ: &Type) -> Value {
        let pointer = self.direct(pointer);
        if !self.builder.indirect(typ) {
            return self.builder.load(pointer, typ);
        }
        let slot = self.storage(id, typ);
        self.builder.copy(slot, pointer, typ);
        slot
    }

    pub(super) fn store(&mut self, pointer: &Value, value: &Value) -> Value {
        let pointer = self.direct(pointer);
        let (arg, typ) = self.arg(value);
        self.builder.store(pointer, arg, &typ);
        Value::Unit
    }

    pub(super) fn stack_alloc(&mut self, value: &Value) -> Value {
        let (arg, typ) = self.arg(value);
        if self.shared().types.is_unit(&typ) {
            // Unit holds no data, and LLVM's FastISel rejects storing it
            // TODO: Remove this when all unit values are filtered out as 0-sized
            let typ = self.lower(&typ);
            return self.builder.emit(Instruction::StackAllocUninit(typ), Type::POINTER);
        }
        if self.builder.indirect(&typ) {
            let slot = self.builder.slot(&typ);
            self.builder.store(slot, arg, &typ);
            slot
        } else {
            let value = self.builder.direct(arg, &typ);
            self.builder.emit(Instruction::StackAlloc(value), Type::POINTER)
        }
    }

    pub(super) fn stack_alloc_uninit(&mut self, element: &Type) -> Value {
        if is_dynamic(element) {
            return self.builder.slot(element);
        }
        let element = self.lower(element);
        self.builder.emit(Instruction::StackAllocUninit(element), Type::POINTER)
    }

    pub(super) fn alloc_shared(&mut self, value: &Value) -> Value {
        let (arg, typ) = self.arg(value);
        if !self.builder.indirect(&typ) {
            let value = self.builder.direct(arg, &typ);
            return self.builder.emit(Instruction::AllocShared(value), Type::POINTER);
        }
        let malloc_type = Type::function(vec![usz()], Type::POINTER);
        let malloc = self.builder.emit(Instruction::Extern("malloc".to_string()), malloc_type);
        let size = self.builder.layout(&typ).size;
        let size = usz_value(size);
        let pointer = self.builder.emit(Instruction::Call { function: malloc, arguments: vec![size] }, Type::POINTER);
        self.builder.store(pointer, arg, &typ);
        pointer
    }

    pub(super) fn transmute(&mut self, id: InstructionId, value: &Value, typ: &Type) -> Value {
        let source_type = self.old_type(value);

        // Turning a function into data is how a function pointer is handed to C
        if matches!(source_type, Type::Function(_)) && !matches!(typ, Type::Function(_)) {
            let function = self.native_function(value);
            let typ = self.lower(typ);
            return self.builder.emit(Instruction::Transmute(function), typ);
        }

        // A closure environment nothing reads is a unit pointer
        if self.shared().types.is_unit(&source_type) && *typ == Type::POINTER {
            return self.builder.null();
        }

        if !self.builder.indirect(&source_type) && !self.builder.indirect(typ) {
            let value = self.direct(value);
            let typ = self.lower(typ);
            return self.builder.emit(Instruction::Transmute(value), typ);
        }

        let source = self.address(value);
        let slot = if self.builder.indirect(typ) { self.storage(id, typ) } else { self.builder.slot(typ) };
        if source == slot {
            return slot;
        }
        let source_size = self.builder.layout(&source_type).size;
        let size = self.builder.layout(typ).size;
        let size = self.builder.min(source_size, size);
        self.builder.copy_bytes(slot, source, size);
        if self.builder.indirect(typ) { slot } else { self.builder.load(slot, typ) }
    }

    /// An atomic operation on a value of a dynamic type
    pub(super) fn dynamic_atomic(
        &mut self, operation: AtomicOperation, pointer: &Value, operands: &[Value], result_type: &Type,
    ) -> Value {
        let pointer = self.direct(pointer);
        let size = match operands.first() {
            Some(operand) => self.old_type(operand),
            None => result_type.clone(),
        };
        let size = self.builder.layout(&size).size;
        let operands = mapvec(operands, |operand| self.address(operand));
        self.builder.call_atomic(operation, pointer, size, operands, result_type)
    }
}
