//! This module rewrites one polymorphic definition into its single existentialized form, see
//! [rewrite_definition]. Its submodules each rewrite one kind of instruction:
//! - `entry`: the definition's own parameters and block parameters
//! - `calls`: calls, function values, and calls to and from C
//! - `memory`: aggregates, loads, stores, allocations, and atomics
//! - `evidence`: building and searching evidence
//! - `destinations`: choosing where dynamic values are built to avoid copying them

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    iterator_extensions::mapvec,
    mir::{
        BlockId, Definition, Instruction, InstructionId, JmpTarget, TerminatorInstruction, Type, Value,
        existentialization::{
            AtomicOperation, Shared,
            analysis::Kind,
            builder::{Arg, FunctionBuilder, usz_value},
            resolve::{self, Bindings, Target},
            types::is_dynamic,
        },
    },
};

mod calls;
mod destinations;
mod entry;
mod evidence;
mod memory;

use destinations::Hint;

pub(super) fn rewrite_definition(shared: &Shared, old: &Definition) -> Definition {
    let mut rewriter = Rewriter::new(shared, old);
    for block in old.topological_sort() {
        rewriter.rewrite_block(block);
    }
    rewriter.builder.finish()
}

struct Rewriter<'a> {
    builder: FunctionBuilder<'a>,
    old: &'a Definition,

    /// Each old value's replacement
    values: FxHashMap<Value, Value>,

    /// The old values which must exist at runtime
    materialized: &'a FxHashSet<Value>,

    /// The storage of each block parameter with a dynamic type
    block_slots: FxHashMap<BlockId, Value>,

    /// Where a function with a dynamic result writes it
    result_slot: Option<Value>,

    /// Where each dynamic value used only once should be built, see [destinations::destination_hints]
    hints: FxHashMap<InstructionId, Hint>,

    /// The storage chosen for each dynamic value with a hint or which is the target of one
    destinations: FxHashMap<InstructionId, Value>,

    /// Whether this remains a global rather than becoming a function
    stays_global: bool,
}

impl<'a> Rewriter<'a> {
    fn rewrite_block(&mut self, block: BlockId) {
        self.builder.block = block;
        for instruction in &self.old.blocks[block].instructions {
            self.rewrite_instruction(*instruction);
        }
        let terminator = self.old.blocks[block].terminator.as_ref().expect("block without a terminator");
        let terminator = self.rewrite_terminator(terminator);
        self.builder.function.blocks[block].terminator = Some(terminator);
    }

    // Values

    fn shared(&self) -> &'a Shared<'a> {
        self.builder.shared
    }

    fn old_type(&self, value: &Value) -> Type {
        self.old.type_of_value(value, &self.shared().mir.externals, &self.shared().mir.definitions)
    }

    fn resolve(&self, value: &Value) -> Target {
        resolve::resolve(value, self.old, &|id| self.shared().kind(id))
    }

    /// The replacement of an old value
    fn value(&mut self, value: &Value) -> Value {
        match value {
            Value::Definition(id) if self.shared().kind(*id) == Kind::Extern => *value,
            Value::Definition(id) => {
                let typ = self.old_type(value);
                self.builder.definition_value(*id, &Bindings::Identity, &typ)
            },
            Value::InstructionResult(_) | Value::Parameter(..) => *self
                .values
                .get(value)
                .unwrap_or_else(|| panic!("existentialization: `{value}` in `{}` has no replacement", self.old.name)),
            Value::Error => panic!("existentialization: error value"),
            constant => *constant,
        }
    }

    fn arg(&mut self, value: &Value) -> (Arg, Type) {
        (Arg::Value(self.value(value)), self.old_type(value))
    }

    /// The replacement of `value`, which must have a static type
    fn direct(&mut self, value: &Value) -> Value {
        let (arg, typ) = self.arg(value);
        self.builder.direct(arg, &typ)
    }

    fn address(&mut self, value: &Value) -> Value {
        let (arg, typ) = self.arg(value);
        self.builder.address_of(arg, &typ)
    }

    fn lower(&self, typ: &Type) -> Type {
        self.shared().types.lower(typ)
    }

    // Instructions

    fn rewrite_instruction(&mut self, id: InstructionId) {
        let result = Value::InstructionResult(id);
        let typ = self.old.instruction_result_type(id).clone();
        let replacement = match &self.old.instructions[id] {
            Instruction::Call { function, arguments } | Instruction::CallClosure { closure: function, arguments } => {
                Some(self.call(id, function, arguments, &typ))
            },
            Instruction::PackClosure { function, environment } => Some(self.pack_closure(function, environment, &typ)),
            Instruction::Instantiate(target, bindings) => self.materialized.contains(&result).then(|| {
                let bindings = Bindings::Explicit(bindings.clone());
                self.builder.definition_value(*target, &bindings, &typ)
            }),
            Instruction::Id(value) => self.materialized.contains(&result).then(|| self.value(value)),
            Instruction::IndexTuple { tuple, index } => Some(self.index_tuple(tuple, *index, &typ)),
            Instruction::MakeTuple(fields) => Some(self.make_tuple(id, fields, &typ)),
            Instruction::MakeArray(elements) => Some(self.make_array(id, elements, &typ)),
            Instruction::MakeBytes(bytes) => {
                Some(self.builder.emit(Instruction::MakeBytes(bytes.clone()), Type::POINTER))
            },
            Instruction::StackAlloc(value) => Some(self.stack_alloc(value)),
            Instruction::StackAllocUninit(element) => Some(self.stack_alloc_uninit(element)),
            Instruction::AllocShared(value) => Some(self.alloc_shared(value)),
            Instruction::Store { pointer, value } => Some(self.store(pointer, value)),
            Instruction::GetFieldPtr { struct_ptr, struct_type, index } => {
                Some(self.get_field_ptr(struct_ptr, struct_type, *index))
            },
            Instruction::Transmute(value) => Some(self.transmute(id, value, &typ)),
            Instruction::Deref(pointer) => Some(self.deref(id, pointer, &typ)),
            Instruction::SizeOf(sized) => Some(usz_value(self.builder.layout(sized).size)),
            Instruction::ArrayLen(Type::Array { length, .. }) => Some(usz_value(self.builder.array_length(length))),
            Instruction::ArrayLen(array) => panic!("ArrayLen of the non-array `{array}`"),
            Instruction::LookupEvidence { evidence, key } => Some(self.lookup_evidence(evidence, key, &typ)),
            Instruction::MakeEvidence { capabilities, rest } => Some(self.make_evidence(capabilities, rest)),
            Instruction::Extern(name) => Some(self.extern_symbol(name, &typ)),
            Instruction::AtomicLoad { pointer, ordering } if is_dynamic(&typ) => {
                Some(self.dynamic_atomic(AtomicOperation::Load(*ordering), pointer, &[], &typ))
            },
            Instruction::AtomicStore { pointer, value, ordering } if is_dynamic(&self.old_type(value)) => {
                Some(self.dynamic_atomic(AtomicOperation::Store(*ordering), pointer, &[*value], &typ))
            },
            Instruction::AtomicRmw { op, pointer, value, ordering } if is_dynamic(&typ) => {
                Some(self.dynamic_atomic(AtomicOperation::Rmw(*op, *ordering), pointer, &[*value], &typ))
            },
            Instruction::AtomicCmpxchg { pointer, expected, desired, success, failure } if is_dynamic(&typ) => {
                let operation = AtomicOperation::Cmpxchg(*success, *failure);
                Some(self.dynamic_atomic(operation, pointer, &[*expected, *desired], &typ))
            },
            Instruction::Capability | Instruction::Handle { .. } | Instruction::Perform { .. } => {
                unreachable!("effects are lowered before existentialization")
            },
            Instruction::StackAllocBytes(_)
            | Instruction::MemCopy { .. }
            | Instruction::PointerOffset { .. }
            | Instruction::GlobalAddress(_) => {
                unreachable!("existentialization instructions are only created by existentialization")
            },
            other => Some(self.scalar(id, other, &typ)),
        };
        if let Some(replacement) = replacement {
            self.values.insert(result, replacement);
        }
    }

    /// An instruction on static values, which is kept as is
    fn scalar(&mut self, id: InstructionId, instruction: &Instruction, typ: &Type) -> Value {
        assert!(!is_dynamic(typ), "existentialization: `{instruction:?}` in `{}` on a dynamic type", self.old.name);
        let mut instruction = instruction.clone();
        instruction.for_each_value_mut(|value| *value = self.direct(value));
        let lowered = self.lower(typ);
        let result = self.builder.emit(instruction, lowered);
        if !self.builder.indirect(typ) {
            return result;
        }
        // An instruction with an aggregate result, such as overflowing arithmetic
        let slot = self.storage(id, typ);
        self.builder.emit(Instruction::Store { pointer: slot, value: result }, Type::UNIT);
        slot
    }

    // Terminators

    fn rewrite_terminator(&mut self, terminator: &TerminatorInstruction) -> TerminatorInstruction {
        match terminator {
            TerminatorInstruction::Jmp(target) => TerminatorInstruction::Jmp(self.jump_target(target)),
            TerminatorInstruction::If { condition, then, else_, end } => {
                let condition = self.direct(condition);
                let then = self.jump_target(then);
                let else_ = self.jump_target(else_);
                TerminatorInstruction::If { condition, then, else_, end: *end }
            },
            TerminatorInstruction::Switch { int_value, cases, else_, end } => {
                let int_value = self.direct(int_value);
                let cases = mapvec(cases, |(tag, target)| (*tag, self.jump_target(target)));
                let else_ = self.jump_target(else_);
                TerminatorInstruction::Switch { int_value, cases, else_, end: *end }
            },
            TerminatorInstruction::Unreachable => TerminatorInstruction::Unreachable,
            TerminatorInstruction::Return(value) | TerminatorInstruction::Result(value) => {
                if let Some(slot) = self.result_slot {
                    let (arg, typ) = self.arg(value);
                    self.builder.store(slot, arg, &typ);
                    return TerminatorInstruction::Return(slot);
                }
                let value = self.direct(value);
                if self.stays_global {
                    TerminatorInstruction::Result(value)
                } else {
                    TerminatorInstruction::Return(value)
                }
            },
        }
    }

    /// Passes a dynamic block argument through the target block's storage
    fn jump_target(&mut self, (target, argument): &JmpTarget) -> JmpTarget {
        let argument = argument.and_then(|argument| match self.block_slots.get(target).copied() {
            Some(slot) => {
                let (arg, typ) = self.arg(&argument);
                self.builder.store(slot, arg, &typ);
                None
            },
            None => Some(self.direct(&argument)),
        });
        (*target, argument)
    }
}
