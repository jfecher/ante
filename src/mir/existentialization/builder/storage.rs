//! Stack slots, loads, and stores, and converting between a value and its address. A dynamic
//! value is always represented by its address, and a static one is spilled when an address is needed.

use crate::mir::{
    BlockId, Instruction, IntConstant, Type, Value,
    existentialization::{
        builder::{At, FunctionBuilder, Op, arithmetic::usz_value},
        types::is_dynamic,
    },
};

/// An argument given to a call
#[derive(Debug, Clone, Copy)]
pub(crate) enum Arg {
    /// A value represented as its type dictates: directly if static, by address if dynamic
    Value(Value),
    /// The address of a value, whatever its type
    Address(Value),
}

impl FunctionBuilder<'_> {
    /// Uninitialized storage for a value of `typ`
    pub(crate) fn slot(&mut self, typ: &Type) -> Value {
        if is_dynamic(typ) {
            let size = self.layout(typ).size;
            let size = usz_value(size);
            self.emit_prologue(Instruction::StackAllocBytes(size), Type::POINTER)
        } else {
            let lowered = self.shared.types.lower(typ);
            self.emit_prologue(Instruction::StackAllocUninit(lowered), Type::POINTER)
        }
    }

    /// Copy the value of type `typ` at `source` to `destination`
    pub(crate) fn copy(&mut self, destination: Value, source: Value, typ: &Type) {
        let size = self.layout(typ).size;
        self.copy_bytes(destination, source, size);
    }

    pub(crate) fn copy_bytes(&mut self, destination: Value, source: Value, size: Op) {
        // A value built in place is already where it is copied to
        if size != Op::Const(0) && destination != source {
            let size = usz_value(size);
            self.emit(Instruction::MemCopy { destination, source, size }, Type::UNIT);
        }
    }

    pub(crate) fn offset(&mut self, pointer: Value, offset: Op) -> Value {
        self.offset_at(At::Block, pointer, offset)
    }

    pub(super) fn offset_at(&mut self, at: At, pointer: Value, offset: Op) -> Value {
        match offset {
            Op::Const(0) => pointer,
            offset => {
                let offset = usz_value(offset);
                self.emit_at(at, Instruction::PointerOffset { pointer, offset }, Type::POINTER)
            },
        }
    }

    /// Store the scalar `value` `offset` bytes into `pointer`
    pub(crate) fn store_word(&mut self, pointer: Value, offset: Op, value: Value) {
        self.store_word_at(At::Block, pointer, offset, value);
    }

    pub(super) fn store_word_at(&mut self, at: At, pointer: Value, offset: Op, value: Value) {
        let pointer = self.offset_at(at, pointer, offset);
        self.emit_at(at, Instruction::Store { pointer, value }, Type::UNIT);
    }

    /// Load a value of the static type `typ` from `pointer`
    pub(crate) fn load(&mut self, pointer: Value, typ: &Type) -> Value {
        let lowered = self.shared.types.lower(typ);
        self.emit(Instruction::Deref(pointer), lowered)
    }

    /// Load from storage which is never written again, which can then stand in for the value's address
    pub(crate) fn load_immutable(&mut self, pointer: Value, typ: &Type) -> Value {
        let value = self.load(pointer, typ);
        self.cache.addresses.insert(value, pointer);
        value
    }

    pub(crate) fn null(&mut self) -> Value {
        if let Some(null) = self.cache.null {
            return null;
        }
        let null = self.emit_prologue(Instruction::Transmute(Value::Integer(IntConstant::Usz(0))), Type::POINTER);
        self.cache.null = Some(null);
        null
    }

    /// The address of `arg`, a value of type `typ`
    pub(crate) fn address_of(&mut self, arg: Arg, typ: &Type) -> Value {
        match arg {
            Arg::Address(address) => address,
            Arg::Value(value) if self.indirect(typ) => value,
            Arg::Value(value) if self.cache.addresses.contains_key(&value) => self.cache.addresses[&value],
            Arg::Value(value) => self.spill(value, typ),
        }
    }

    /// Store the static `value` in a new slot
    fn spill(&mut self, value: Value, typ: &Type) -> Value {
        // Parameters and constants never change, so each is spilled once in the prologue
        let at = match value {
            Value::InstructionResult(_) => At::Block,
            Value::Parameter(block, _) if block != BlockId::ENTRY_BLOCK => At::Block,
            _ => At::Prologue,
        };
        let block = match at {
            At::Block => self.block,
            At::Prologue => BlockId::ENTRY_BLOCK,
        };
        if let Some(slot) = self.cache.spills.get(&(block, value)) {
            return *slot;
        }
        let slot = self.slot(typ);
        if !self.shared.types.is_unit(typ) {
            self.emit_at(at, Instruction::Store { pointer: slot, value }, Type::UNIT);
        }
        self.cache.spills.insert((block, value), slot);
        slot
    }

    /// The value of `arg`, a value of the static type `typ`, loaded if it is indirect
    pub(crate) fn direct(&mut self, arg: Arg, typ: &Type) -> Value {
        assert!(!is_dynamic(typ), "existentialization: `{typ}` is dynamic");
        if self.indirect(typ) {
            let address = self.address_of(arg, typ);
            return self.load(address, typ);
        }
        match arg {
            Arg::Value(value) => value,
            Arg::Address(address) => self.load_immutable(address, typ),
        }
    }

    /// Store `value` of type `typ` at `pointer`
    pub(crate) fn store(&mut self, pointer: Value, arg: Arg, typ: &Type) {
        // Unit holds no data, and LLVM's FastISel rejects storing it
        if self.shared.types.is_unit(typ) {
            return;
        }
        if self.indirect(typ) {
            let source = self.address_of(arg, typ);
            self.copy(pointer, source, typ);
        } else {
            let value = self.direct(arg, typ);
            self.emit(Instruction::Store { pointer, value }, Type::UNIT);
        }
    }

    /// The result of a call which wrote its result to `slot`
    pub(super) fn read_result(&mut self, slot: Value, typ: &Type) -> Value {
        match typ {
            _ if self.indirect(typ) => slot,
            &Type::UNIT => Value::Unit,
            _ => self.load_immutable(slot, typ),
        }
    }
}
