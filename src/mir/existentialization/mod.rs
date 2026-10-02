//! This module contains existentialization, the alternative to monomorphization used for
//! unoptimized builds. Where monomorphization compiles a copy of each generic definition for every
//! set of type arguments it is used with, existentialization keeps a single copy of each and passes
//! the type information it needs at runtime. Every type is still laid out exactly as
//! monomorphization would lay it out.
//!
//! The entry point is [existentialize], which works in three steps:
//! - `analysis` decides what each definition needs: the generics it takes a type info for, whether
//!   a global must be computed at runtime, and how many type infos each function value reserves.
//! - `rewrite` rewrites each definition in parallel, using a `FunctionBuilder` from `builder` to
//!   emit instructions and compute layouts at runtime.
//! - `thunks`, `evidence`, and `atomics` generate the definitions the rewritten code calls into:
//!   value thunks and helpers such as the evidence lookup.
//!
//! The rewritten program represents generics as follows:
//! - A definition takes a pointer to a type info table holding a type info `(size, align, id)`
//!   for each generic whose layout it needs, then the precomputed layouts of the dynamic types it
//!   uses. A caller with every type known passes a constant type info table.
//! - A value whose layout depends on a generic is dynamic: a pointer to its storage, passed by
//!   reference and returned through a result pointer. Its storage lives in the entry block of
//!   the function creating it, so no heap allocation is introduced.
//! - Every function value is `(code, type infos.., environment)` with the uniform calling
//!   convention, whatever its type, so function values never need adapting. Each arity reserves
//!   room for the most type infos any function value of that arity needs.
//! - A definition used as a first-class function value gets a value thunk from the uniform calling
//!   convention to its direct calling convention, unless it is never used in a call position, in
//!   which case it takes the uniform calling convention itself.
//! - Evidence is a chain of nodes searched at runtime by each effect's key.
//!
//! Both calling conventions are described in `convention`.

use std::sync::Arc;

use dashmap::DashMap;
use inc_complete::DbGet;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{
    cli::GenericsStrategy,
    incremental::{GetCrateGraph, GetItem, GetItemRaw, GetTypeBody, Parse, TargetPointerSize, TypeCheck},
    iterator_extensions::mapvec,
    mir::{
        BlockId, Definition, DefinitionId, Definitions, Extern, Instruction, Mir, TerminatorInstruction, Type, Value,
        build_effect_lowered_mir, next_definition_id,
    },
};

mod analysis;
mod atomics;
mod builder;
mod convention;
mod evidence;
mod resolve;
mod rewrite;
#[cfg(test)]
mod tests;
mod thunks;
mod types;

use analysis::{Analysis, Kind};
use atomics::AtomicOperation;
use types::Types;

/// Existentialize the whole program, which like monomorphization needs every item at once
pub(crate) fn existentialize<Db>(compiler: &Db) -> Mir
where
    Db: DbGet<TypeCheck>
        + DbGet<GetItem>
        + DbGet<GetItemRaw>
        + DbGet<GetTypeBody>
        + DbGet<GetCrateGraph>
        + DbGet<Parse>
        + DbGet<TargetPointerSize>
        + Sync,
{
    let mir = build_effect_lowered_mir(compiler, GenericsStrategy::Existential);
    existentialize_mir(&mir, TargetPointerSize.get(compiler))
}

fn existentialize_mir(mir: &Mir, ptr_size: u32) -> Mir {
    let Analysis { info, reserved } = analysis::analyze(mir);
    let shared = Shared::new(mir, info, Types::new(ptr_size, reserved));

    let mut definitions: Definitions = mir
        .definitions
        .par_iter()
        .map(|(id, definition)| (*id, rewrite::rewrite_definition(&shared, definition)))
        .collect();

    let build_thunks = |thunks: &DashMap<DefinitionId, DefinitionId>,
                        build: fn(&Shared, DefinitionId, DefinitionId) -> Definition| {
        let thunks = mapvec(thunks, |entry| (*entry.key(), *entry.value()));
        thunks.into_par_iter().map(|(target, id)| (id, build(&shared, target, id))).collect::<Vec<_>>()
    };
    definitions.extend(build_thunks(&shared.thunks, thunks::value_thunk));
    definitions.extend(build_thunks(&shared.native_thunks, thunks::native_thunk));

    for entry in shared.helpers.iter() {
        definitions.insert(*entry.value(), helper(*entry.key(), *entry.value(), shared.types.ptr_size));
    }

    for entry in shared.static_tables.iter() {
        let ((typ, fields), id) = (entry.key(), *entry.value());
        let mut table = Definition::new(Arc::new("type_info_table".to_string()), id, 0, typ.clone());
        let tuple = emit(&mut table, BlockId::ENTRY_BLOCK, Instruction::MakeTuple(fields.clone()), typ.clone());
        table.blocks[BlockId::ENTRY_BLOCK].terminator = Some(TerminatorInstruction::Result(tuple));
        definitions.insert(id, table);
    }

    let externals = mir
        .externals
        .iter()
        .map(|(id, external)| (*id, Extern { name: external.name.clone(), typ: shared.types.lower_c(&external.typ) }))
        .collect();

    let mir = Mir { definitions, externals, preserved_op_indices: Default::default() };

    #[cfg(debug_assertions)]
    let mir = mir.assert_type_checks().assert_no_unions_or_generics().assert_no_closure_types();

    mir
}

/// A definition existentialization generates once for the whole program
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Helper {
    EvidenceLookup,
    /// An atomic operation on a value whose size is only known at runtime
    Atomic(AtomicOperation),
}

fn helper(helper: Helper, id: DefinitionId, ptr_size: u32) -> Definition {
    match helper {
        Helper::EvidenceLookup => evidence::lookup_helper(id, ptr_size),
        Helper::Atomic(operation) => atomics::helper(operation, id, ptr_size),
    }
}

/// Emit an instruction into `block` of a helper, which needs no [builder::FunctionBuilder]
fn emit(definition: &mut Definition, block: BlockId, instruction: Instruction, typ: Type) -> Value {
    let id = definition.instructions.push(instruction);
    definition.instruction_result_types.push_existing(id, typ);
    definition.blocks[block].instructions.push(id);
    Value::InstructionResult(id)
}

/// The whole-program state shared by every definition's rewrite
pub(super) struct Shared<'a> {
    mir: &'a Mir,
    info: rustc_hash::FxHashMap<DefinitionId, analysis::Info>,
    types: Types,

    /// The thunk created for each definition used as a function value
    thunks: DashMap<DefinitionId, DefinitionId>,

    /// The native thunk created for each function handed to C whose direct calling convention C
    /// cannot call
    native_thunks: DashMap<DefinitionId, DefinitionId>,

    helpers: DashMap<Helper, DefinitionId>,

    /// The global holding each constant type info table, deduplicated across the whole program
    static_tables: DashMap<(Type, Vec<Value>), DefinitionId>,
}

impl<'a> Shared<'a> {
    fn new(mir: &'a Mir, info: rustc_hash::FxHashMap<DefinitionId, analysis::Info>, types: Types) -> Self {
        Shared {
            mir,
            info,
            types,
            thunks: Default::default(),
            native_thunks: Default::default(),
            helpers: Default::default(),
            static_tables: Default::default(),
        }
    }

    fn kind(&self, id: DefinitionId) -> Kind {
        self.info.get(&id).map_or(Kind::Extern, |info| info.kind)
    }

    /// The generics `id` needs a type info for
    fn needed(&self, id: DefinitionId) -> &[u32] {
        self.info.get(&id).map_or(&[], |info| &info.needed)
    }

    /// Whether the type ids in the type info table of `id` are ever read, see [analysis::Info::uses_ids]
    fn uses_ids(&self, id: DefinitionId) -> bool {
        self.info.get(&id).is_some_and(|info| info.uses_ids)
    }

    /// The types whose layouts direct callers of `id` precompute, see [convention::TypeInfoTable]
    fn keys(&self, id: DefinitionId) -> &[Type] {
        self.info.get(&id).map_or(&[], |info| &info.keys)
    }

    /// The result type of a function, or the type of a global
    fn return_type(&self, id: DefinitionId) -> Type {
        let definition = &self.mir.definitions[&id];
        match (&definition.typ, self.kind(id)) {
            (Type::Function(function), Kind::Function) => function.return_type.clone(),
            (typ, _) => typ.clone(),
        }
    }

    /// Whether `id` takes the uniform calling convention itself
    fn uniform(&self, id: DefinitionId) -> bool {
        self.info.get(&id).is_some_and(|info| info.uniform)
    }

    fn helper(&self, helper: Helper) -> DefinitionId {
        *self.helpers.entry(helper).or_insert_with(next_definition_id)
    }

    fn static_table(&self, typ: Type, fields: Vec<Value>) -> DefinitionId {
        *self.static_tables.entry((typ, fields)).or_insert_with(next_definition_id)
    }
}
