//! This module gathers the whole-program facts each definition's rewrite relies on, see [analyze].
//!
//! Each definition's own [facts::Facts] are collected in parallel, then propagated through the
//! references between definitions until nothing changes.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::mir::{
    DefinitionId, Mir, Type, Value,
    existentialization::{convention::arity, types::for_each_generic},
};

mod facts;

use facts::Facts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Function,
    /// A global whose value codegen can fold to a constant
    Global,
    /// A global computed each time it is used, since it needs type infos or runtime work
    ComputedGlobal,
    /// A global holding a C function, or a symbol defined outside of the program
    Extern,
}

pub(super) struct Info {
    pub(super) kind: Kind,

    /// The generics this definition takes a type info for, in ascending order
    pub(super) needed: Vec<u32>,

    /// See [facts::materialized_values]
    pub(super) materialized: FxHashSet<Value>,

    /// Whether this function is only ever used as a function value, so it can take the uniform
    /// calling convention itself rather than needing a value thunk
    pub(super) uniform: bool,

    /// The dynamic tuples, unions, arrays, and function values whose layouts direct callers pass
    /// precomputed in the type info table. See [super::convention::TypeInfoTable]
    pub(super) keys: Vec<Type>,

    /// Whether the type ids in this definition's type info table are ever read, by it or anything it passes
    /// them to. If not, callers can pass zeroed values.
    pub(super) uses_ids: bool,
}

pub(super) struct Analysis {
    pub(super) info: FxHashMap<DefinitionId, Info>,

    /// How many type infos a function value of each arity has room for
    pub(super) reserved: FxHashMap<usize, usize>,
}

pub(super) fn analyze(mir: &Mir) -> Analysis {
    let kinds: FxHashMap<DefinitionId, Kind> = mir
        .definitions
        .iter()
        .map(|(id, definition)| (*id, facts::kind_of(definition)))
        .chain(mir.externals.keys().map(|id| (*id, Kind::Extern)))
        .collect();

    let mut facts = facts::collect(mir, &|id| kinds.get(&id).copied().unwrap_or(Kind::Extern));
    let mut needed = propagate_needed(mir, &facts);
    let computed = computed_globals(&facts, &kinds, &needed);
    let uses_ids = propagate_uses_ids(&facts);

    let values: FxHashSet<DefinitionId> = facts.values().flat_map(|facts| &facts.function_values).copied().collect();
    let direct: FxHashSet<DefinitionId> = facts.values().flat_map(|facts| &facts.direct_uses).copied().collect();

    let info: FxHashMap<DefinitionId, Info> = kinds
        .iter()
        .map(|(id, kind)| {
            let mut needed: Vec<u32> = needed.remove(id).unwrap_or_default().into_iter().collect();
            needed.sort_unstable();
            let kind = if computed.contains(id) { Kind::ComputedGlobal } else { *kind };
            let materialized =
                facts.get_mut(id).map(|facts| std::mem::take(&mut facts.materialized)).unwrap_or_default();

            let uniform = kind == Kind::Function && values.contains(id) && !direct.contains(id);

            // Function values carry only type infos, so a uniform function computes its layouts
            let keys = match facts.get_mut(id) {
                Some(facts) if !uniform && !needed.is_empty() => std::mem::take(&mut facts.keys),
                _ => Vec::new(),
            };
            let uses_ids = uses_ids.contains(id);
            (*id, Info { kind, needed, materialized, uniform, keys, uses_ids })
        })
        .collect();

    let mut reserved = FxHashMap::default();
    for function in facts.values().flat_map(|facts| &facts.function_values) {
        let count = reserved.entry(arity(&mir.definitions[function].typ)).or_insert(0);
        *count = info[function].needed.len().max(*count);
    }

    Analysis { info, reserved }
}

/// The generics each definition needs a type info for: those of its own layouts, and those its
/// references need of the types it instantiates them with
fn propagate_needed(mir: &Mir, facts: &FxHashMap<DefinitionId, Facts>) -> FxHashMap<DefinitionId, FxHashSet<u32>> {
    let mut needed: FxHashMap<DefinitionId, FxHashSet<u32>> =
        facts.iter().map(|(id, facts)| (*id, facts.layout_generics.clone())).collect();
    while {
        let mut changed = false;
        for (id, definition) in &mir.definitions {
            let mut new = Vec::new();
            for (callee, bindings) in &facts[id].references {
                for generic in needed.get(callee).into_iter().flatten() {
                    for_each_generic(&bindings.get(*generic), false, &mut |generic| new.push(generic));
                }
            }
            let own = needed.get_mut(id).unwrap();
            for generic in new {
                changed |= generic < definition.generic_count && own.insert(generic);
            }
        }
        changed
    }{}
    needed
}

/// A global is computed if it is generic, does more than build a constant, or reads a computed global
fn computed_globals(
    facts: &FxHashMap<DefinitionId, Facts>, kinds: &FxHashMap<DefinitionId, Kind>,
    needed: &FxHashMap<DefinitionId, FxHashSet<u32>>,
) -> FxHashSet<DefinitionId> {
    propagate(facts, |id, facts, computed| {
        kinds[id] == Kind::Global
            && (facts.runtime_work || !needed[id].is_empty() || facts.globals.iter().any(|g| computed.contains(g)))
    })
}

/// Ids are read for effect keys and passed on in function values and to callees reading them
fn propagate_uses_ids(facts: &FxHashMap<DefinitionId, Facts>) -> FxHashSet<DefinitionId> {
    propagate(facts, |_, facts, uses_ids| {
        facts.reads_ids || facts.references.iter().any(|(callee, _)| uses_ids.contains(callee))
    })
}

/// The smallest set holding each definition `holds` accepts given the set so far
fn propagate(
    facts: &FxHashMap<DefinitionId, Facts>, holds: impl Fn(&DefinitionId, &Facts, &FxHashSet<DefinitionId>) -> bool,
) -> FxHashSet<DefinitionId> {
    let mut set = FxHashSet::default();
    while {
        let mut changed = false;
        for (id, facts) in facts {
            if !set.contains(id) && holds(id, facts, &set) {
                changed |= set.insert(*id);
            }
        }
        changed
    }{}
    set
}
