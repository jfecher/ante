//! This module contains [FunctionBuilder], which builds one existentialized function. Its
//! submodules each extend it with one concern:
//! - `arithmetic`: size arithmetic, folded when both sides are known
//! - `layout`: the layouts of dynamic types, computed from the type infos in scope
//! - `type_info`: type infos and ids of types, and the type info tables passed to callees
//! - `storage`: stack slots, loads, stores, and converting values to and from addresses
//! - `calls`: both calling conventions, see [super::convention]
//!
//! Layout arithmetic and stack slots only depend on the type infos, so they go in the entry
//! block's prologue. Each slot is then allocated once per call rather than once per loop iteration.

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::mir::{
    BlockId, Definition, DefinitionId, Instruction, InstructionId, Name, Type, Value,
    existentialization::{Shared, convention::TypeInfoTable, types},
};

mod arithmetic;
mod calls;
mod layout;
mod storage;
mod type_info;

pub(super) use arithmetic::{Op, usz_value};
pub(super) use storage::Arg;

use layout::Layout;

pub(super) struct FunctionBuilder<'a> {
    pub(super) shared: &'a Shared<'a>,
    pub(super) function: Definition,
    pub(super) block: BlockId,
    prologue: Vec<InstructionId>,

    /// The generics in scope with a type info, in the order they are held by `table`
    needed: &'a [u32],

    /// The type info table this function reads, if it has one
    table: Option<ScopeTable<'a>>,

    /// Whether static aggregates stay SSA values, as a global's constant initializer needs
    ssa_aggregates: bool,

    cache: Caches,
}

/// A pointer to a type info table holding `needed` and the precomputed layouts of `keys`
struct ScopeTable<'a> {
    pointer: Value,

    /// The types whose precomputed layouts the table holds, see [TypeInfoTable]
    keys: &'a [Type],

    layout: TypeInfoTable,

    /// The index of each of `keys`
    key_indices: FxHashMap<Type, usize>,

    /// Each field of the table read so far
    reads: FxHashMap<usize, Value>,
}

/// Values computed once per function and reused
#[derive(Default)]
struct Caches {
    layouts: FxHashMap<Type, Layout>,

    /// The layout and field offsets of each tuple or function value
    tuples: FxHashMap<Type, (Layout, Arc<Vec<Op>>)>,

    type_infos: FxHashMap<Type, Value>,

    type_ids: FxHashMap<Type, Op>,

    /// Each type info table passed to a callee
    metadata_arguments: FxHashMap<Vec<Op>, Value>,

    /// Values loaded from storage which is never written again, mapped to that storage
    addresses: FxHashMap<Value, Value>,

    /// Where each static value was spilled within each block
    spills: FxHashMap<(BlockId, Value), Value>,

    /// The result of each arithmetic operation in the prologue, keyed by its operation and operands
    arithmetic: FxHashMap<(std::mem::Discriminant<Instruction>, Value, Value), Value>,

    null: Option<Value>,
}

/// Where an instruction is emitted
#[derive(Debug, Clone, Copy)]
enum At {
    /// The end of the current block
    Block,
    /// The prologue, which runs once per call
    Prologue,
}

impl<'a> FunctionBuilder<'a> {
    pub(super) fn new(shared: &'a Shared<'a>, name: Name, id: DefinitionId, typ: Type, needed: &'a [u32]) -> Self {
        Self {
            shared,
            function: Definition::new(name, id, 0, typ),
            block: BlockId::ENTRY_BLOCK,
            prologue: Vec::new(),
            needed,
            table: None,
            ssa_aggregates: false,
            cache: Caches::default(),
        }
    }

    /// Read type infos from the type info table `pointer` points to, which holds the precomputed
    /// layouts of `keys` too
    pub(super) fn set_metadata(&mut self, pointer: Value, keys: &'a [Type]) {
        let layout = self.shared.types.table(self.needed.len(), keys);
        let key_indices = keys.iter().enumerate().map(|(index, key)| (key.clone(), index)).collect();
        self.table = Some(ScopeTable { pointer, keys, layout, key_indices, reads: Default::default() });
    }

    /// Keep static aggregates as SSA values, for a global's constant initializer
    pub(super) fn keep_aggregates_in_ssa(&mut self) {
        self.ssa_aggregates = true;
    }

    /// See [types::indirect]
    pub(super) fn indirect(&self, typ: &Type) -> bool {
        types::indirect(typ, self.ssa_aggregates)
    }

    pub(super) fn push_parameter(&mut self, typ: Type) -> Value {
        let parameters = &mut self.function.blocks[BlockId::ENTRY_BLOCK].parameter_types;
        parameters.push(typ);
        Value::Parameter(BlockId::ENTRY_BLOCK, parameters.len() as u32 - 1)
    }

    /// Finish the function, placing the prologue at the start of its entry block
    pub(super) fn finish(mut self) -> Definition {
        self.remove_unused_prologue();
        let entry = &mut self.function.blocks[BlockId::ENTRY_BLOCK].instructions;
        self.prologue.append(entry);
        *entry = self.prologue;
        self.function
    }

    /// Layouts are computed eagerly, so drop the parts of them nothing ended up using
    fn remove_unused_prologue(&mut self) {
        if self.prologue.is_empty() {
            return;
        }
        let mut used = FxHashSet::default();
        for block in self.function.blocks.values() {
            for id in &block.instructions {
                self.function.instructions[*id].for_each_value(|value| _ = used.insert(*value));
            }
            if let Some(terminator) = &block.terminator {
                terminator.for_each_value(|value| _ = used.insert(*value));
            }
        }
        let mut kept = Vec::with_capacity(self.prologue.len());
        for id in self.prologue.iter().rev() {
            let instruction = &self.function.instructions[*id];
            let pure = !matches!(
                instruction,
                Instruction::Store { .. } | Instruction::Call { .. } | Instruction::MemCopy { .. }
            );
            if pure && !used.contains(&Value::InstructionResult(*id)) {
                continue;
            }
            instruction.for_each_value(|value| _ = used.insert(*value));
            kept.push(*id);
        }
        kept.reverse();
        self.prologue = kept;
    }

    /// Emit an instruction into the current block
    pub(super) fn emit(&mut self, instruction: Instruction, typ: Type) -> Value {
        self.emit_at(At::Block, instruction, typ)
    }

    /// Emit an instruction into the prologue, which runs once per call
    pub(super) fn emit_prologue(&mut self, instruction: Instruction, typ: Type) -> Value {
        self.emit_at(At::Prologue, instruction, typ)
    }

    fn emit_at(&mut self, at: At, instruction: Instruction, typ: Type) -> Value {
        let id = self.function.instructions.push(instruction);
        self.function.instruction_result_types.push_existing(id, typ);
        match at {
            At::Block => self.function.blocks[self.block].instructions.push(id),
            At::Prologue => self.prologue.push(id),
        }
        Value::InstructionResult(id)
    }
}
