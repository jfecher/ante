//! This module represents evidence at runtime: a chain of nodes, each holding `(key, capability)`
//! entries, searched by each effect's key. See [evidence_node_fields] for a node's layout.

use std::sync::Arc;

use crate::{
    lexer::token::IntegerKind,
    mir::{
        Block, BlockId, Definition, DefinitionId, EffectKey, Instruction, IntConstant, TerminatorInstruction, Type,
        Value,
        existentialization::{Helper, builder::FunctionBuilder, builder::Op, emit, types::usz},
    },
};

/// The fields of one `(key, capability)` entry of an evidence node
fn evidence_entry_fields() -> Vec<Type> {
    vec![Type::int(IntegerKind::U64), Type::POINTER]
}

/// The fields of an evidence node: `(rest, count, entries..)`
fn evidence_node_fields(entries: usize) -> Vec<Type> {
    let mut fields = vec![Type::POINTER, usz()];
    fields.extend(std::iter::repeat_n(Type::tuple(evidence_entry_fields()), entries));
    fields
}

// Evidence nodes are written a word at a time, since LLVM's FastISel rejects aggregate stores
impl FunctionBuilder<'_> {
    /// A new evidence node with room for `count` entries, followed by `rest`
    pub(crate) fn evidence_node(&mut self, rest: Value, count: usize) -> Value {
        let node_fields = evidence_node_fields(count);
        let node_offsets = Type::field_offsets(&node_fields, self.shared.types.ptr_size);
        let node = self.emit_prologue(Instruction::StackAllocUninit(Type::tuple(node_fields)), Type::POINTER);
        self.store_word(node, Op::Const(node_offsets[0] as u64), rest);
        let count = Value::Integer(IntConstant::Usz(count));
        self.store_word(node, Op::Const(node_offsets[1] as u64), count);
        node
    }

    /// Fill in entry `index` of `node`, a node from [Self::evidence_node] with room for `count` entries
    pub(crate) fn store_evidence_entry(
        &mut self, node: Value, count: usize, index: usize, key: Value, capability: Value,
    ) {
        let ptr_size = self.shared.types.ptr_size;
        let node_offsets = Type::field_offsets(&evidence_node_fields(count), ptr_size);
        let entry_offsets = Type::field_offsets(&evidence_entry_fields(), ptr_size);
        let entry = node_offsets[2 + index] as u64;
        self.store_word(node, Op::Const(entry + entry_offsets[0] as u64), key);
        self.store_word(node, Op::Const(entry + entry_offsets[1] as u64), capability);
    }

    /// The capability for `key` within `evidence`, as a value of type `typ`
    pub(crate) fn lookup_evidence(&mut self, evidence: Value, key: &EffectKey, typ: &Type) -> Value {
        let key = self.effect_key(key);
        let lookup = Value::Definition(self.shared.helper(Helper::EvidenceLookup));
        let call = Instruction::Call { function: lookup, arguments: vec![evidence, key] };
        let address = self.emit(call, Type::POINTER);
        if self.indirect(typ) { address } else { self.load_immutable(address, typ) }
    }
}

/// `fn (evidence: Pointer) (key: U64) -> Pointer`, searching each [evidence_node_fields] node for `key`
pub(super) fn lookup_helper(id: DefinitionId, ptr_size: u32) -> Definition {
    let u64_type = Type::int(IntegerKind::U64);
    let typ = Type::function(vec![Type::POINTER, u64_type.clone()], Type::POINTER);
    let mut definition = Definition::new(Arc::new("evidence_lookup".to_string()), id, 0, typ);
    let entry = BlockId::ENTRY_BLOCK;
    definition.blocks[entry].parameter_types = vec![Type::POINTER, u64_type.clone()];
    let (evidence, key) = (Value::Parameter(entry, 0), Value::Parameter(entry, 1));

    let state_type = Type::tuple(vec![Type::POINTER, usz()]);
    let head = definition.blocks.push(Block::new(vec![state_type.clone()]));
    let check = definition.blocks.push(Block::new(Vec::new()));
    let found = definition.blocks.push(Block::new(Vec::new()));
    let next_entry = definition.blocks.push(Block::new(Vec::new()));
    let next_node = definition.blocks.push(Block::new(Vec::new()));

    let usz_const = |n: u32| Value::Integer(IntConstant::Usz(n as usize));
    let node_offsets = Type::field_offsets(&evidence_node_fields(1), ptr_size);
    let (count_offset, entries_offset) = (node_offsets[1], node_offsets[2]);
    let capability_offset = Type::field_offsets(&evidence_entry_fields(), ptr_size)[1];
    let entry_stride = Type::tuple(evidence_entry_fields()).stride(ptr_size);

    let start = emit(&mut definition, entry, Instruction::MakeTuple(vec![evidence, usz_const(0)]), state_type.clone());
    definition.blocks[entry].terminator = Some(TerminatorInstruction::jmp(head, start));

    // head(node, index): if index < node.count then check else next_node
    let state = Value::Parameter(head, 0);
    let node = emit(&mut definition, head, Instruction::IndexTuple { tuple: state, index: 0 }, Type::POINTER);
    let index = emit(&mut definition, head, Instruction::IndexTuple { tuple: state, index: 1 }, usz());
    let count_pointer = emit(
        &mut definition,
        head,
        Instruction::PointerOffset { pointer: node, offset: usz_const(count_offset) },
        Type::POINTER,
    );
    let count = emit(&mut definition, head, Instruction::Deref(count_pointer), usz());
    let more = emit(&mut definition, head, Instruction::LessUnsigned(index, count), Type::BOOL);
    definition.blocks[head].terminator = Some(TerminatorInstruction::if_(more, check, next_node, next_node));

    // check: if entries[index].key == key then found else next_entry
    let offset = emit(&mut definition, check, Instruction::MulInt(index, usz_const(entry_stride)), usz());
    let offset = emit(&mut definition, check, Instruction::AddInt(offset, usz_const(entries_offset)), usz());
    let entry_pointer =
        emit(&mut definition, check, Instruction::PointerOffset { pointer: node, offset }, Type::POINTER);
    let entry_key = emit(&mut definition, check, Instruction::Deref(entry_pointer), u64_type);
    let equal = emit(&mut definition, check, Instruction::EqInt(entry_key, key), Type::BOOL);
    definition.blocks[check].terminator = Some(TerminatorInstruction::if_(equal, found, next_entry, next_entry));

    // found: return entries[index].capability
    let capability_pointer = emit(
        &mut definition,
        found,
        Instruction::PointerOffset { pointer: entry_pointer, offset: usz_const(capability_offset) },
        Type::POINTER,
    );
    let capability = emit(&mut definition, found, Instruction::Deref(capability_pointer), Type::POINTER);
    definition.blocks[found].terminator = Some(TerminatorInstruction::Return(capability));

    // next_entry: head(node, index + 1)
    let next = emit(&mut definition, next_entry, Instruction::AddInt(index, usz_const(1)), usz());
    let state = emit(&mut definition, next_entry, Instruction::MakeTuple(vec![node, next]), state_type.clone());
    definition.blocks[next_entry].terminator = Some(TerminatorInstruction::jmp(head, state));

    // next_node: head(node.rest, 0)
    let rest = emit(&mut definition, next_node, Instruction::Deref(node), Type::POINTER);
    let state = emit(&mut definition, next_node, Instruction::MakeTuple(vec![rest, usz_const(0)]), state_type);
    definition.blocks[next_node].terminator = Some(TerminatorInstruction::jmp(head, state));

    definition
}
