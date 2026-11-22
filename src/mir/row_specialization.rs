//! Specializes every definition over the effect rows its row generics are bound to. Any other
//! generics are left alone.
//!
//! Evidence holds a slot for each effect of a row, and the position of a slot depends on what the
//! row generics ahead of it are bound to (see [evidence](super::evidence)). So we only ever compile
//! code for rows which are fully known. This is the case regardless of whether type generics are
//! later monomorphized or existentialized. After this pass no evidence type holds a row generic, so
//! [Mir::lower_evidence](super::Mir::lower_evidence) can turn evidence into plain tuples.
//!
//! The row a generic is bound to may still mention type generics from the code binding it, e.g. the
//! `a` in `Emit a`. The specialization takes each of these as an extra generic of its own so that
//! one specialization can be shared by every such row of the same shape.
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    iterator_extensions::mapvec,
    mir::{Definition, DefinitionId, EvidenceEntry, Instruction, Mir, Type, Value, evidence, next_definition_id},
};

/// If a single definition is specialized more times than this, we assume its rows are growing with
/// each recursive call and give up
const MAX_SPECIALIZATIONS: usize = 4096;

impl Mir {
    pub(crate) fn specialize_rows(mut self) -> Mir {
        let row_generics = row_generics(&self);
        if row_generics.values().all(Vec::is_empty) {
            return self;
        }
        let mut templates = FxHashMap::default();
        let mut plain = Vec::new();
        for (id, definition) in std::mem::take(&mut self.definitions) {
            if row_generics[&id].is_empty() {
                plain.push((id, definition));
            } else {
                templates.insert(id, definition);
            }
        }
        let mut context =
            Context { templates: &templates, row_generics, specialized: FxHashMap::default(), queue: Vec::new() };

        for (id, definition) in plain {
            self.definitions.insert(id, context.specialize(definition, None));
        }
        let mut counts: FxHashMap<DefinitionId, usize> = FxHashMap::default();
        while let Some(Specialization { old_id, new_id, mapping, generic_count }) = context.queue.pop() {
            let count = counts.entry(old_id).or_default();
            *count += 1;
            let original = &templates[&old_id];
            assert!(
                *count <= MAX_SPECIALIZATIONS,
                "`{}` is used with an effect row that grows each time it calls itself",
                original.name
            );
            let mut definition = original.clone_with_id(new_id);
            definition.generic_count = generic_count;
            self.definitions.insert(new_id, context.specialize(definition, Some(&mapping)));
        }
        self
    }
}

struct Context<'mir> {
    /// Each definition with row generics. These are never output themselves, only their
    /// specializations are.
    templates: &'mir FxHashMap<DefinitionId, Definition>,

    /// The generics of each definition which evidence is spliced into. These are the generics each
    /// definition is specialized over.
    row_generics: FxHashMap<DefinitionId, Vec<u32>>,

    /// Maps each definition and the rows its row generics are bound to (in order) to its
    /// specialization for those rows. The key also includes the number of generics the
    /// specialization has.
    specialized: FxHashMap<(DefinitionId, Vec<Type>, u32), DefinitionId>,

    queue: Vec<Specialization>,
}

struct Specialization {
    old_id: DefinitionId,
    new_id: DefinitionId,
    /// The type each generic of the original definition is replaced with
    mapping: Vec<Type>,
    generic_count: u32,
}

/// Returns the row generics of each definition. These are the generics which its own types splice
/// evidence into, along with the row generics of any generic definition it refers to without
/// instantiating, since such a definition shares its generics.
fn row_generics(mir: &Mir) -> FxHashMap<DefinitionId, Vec<u32>> {
    let mut generics: FxHashMap<DefinitionId, FxHashSet<u32>> = FxHashMap::default();
    let mut referrers: FxHashMap<DefinitionId, Vec<DefinitionId>> = FxHashMap::default();

    for (id, definition) in &mir.definitions {
        let mut own = FxHashSet::default();
        // Without any generics there is nothing to splice evidence into and nothing to share
        if definition.generic_count == 0 {
            generics.insert(*id, own);
            continue;
        }
        let mut collect = |typ: &Type| for_each_generic(typ, true, &mut |generic| _ = own.insert(generic));
        collect(&definition.typ);
        definition.for_each_type(collect);
        generics.insert(*id, own);

        definition.for_each_used_value(|value| {
            if let Value::Definition(referenced) = value
                && referenced != id
                && mir.definitions.get(referenced).is_some_and(|definition| definition.generic_count > 0)
            {
                referrers.entry(*referenced).or_default().push(*id);
            }
        });
    }

    let mut grown: Vec<DefinitionId> = referrers.keys().copied().collect();
    while let Some(referenced) = grown.pop() {
        for id in referrers.get(&referenced).into_iter().flatten() {
            let [Some(own), Some(inherited)] = generics.get_disjoint_mut([id, &referenced]) else { unreachable!() };
            let count = own.len();
            own.extend(inherited.iter().copied());
            if own.len() != count {
                grown.push(*id);
            }
        }
    }

    generics
        .into_iter()
        .map(|(id, generics)| {
            let mut generics: Vec<u32> = generics.into_iter().collect();
            generics.sort_unstable();
            (id, generics)
        })
        .collect()
}

/// Calls `f` on each generic mentioned in `typ`. If `rows_only` is true, `f` is only called on the
/// generics which `typ` splices evidence into.
fn for_each_generic(typ: &Type, rows_only: bool, f: &mut impl FnMut(u32)) {
    match typ {
        Type::Primitive(_) | Type::U32(_) => (),
        Type::Generic(generic) => {
            if !rows_only {
                f(generic.0);
            }
        },
        Type::Tuple(types) | Type::Union(types) => types.iter().for_each(|typ| for_each_generic(typ, rows_only, f)),
        Type::Array { length, element } => {
            for_each_generic(length, rows_only, f);
            for_each_generic(element, rows_only, f);
        },
        Type::Function(function) => {
            function.parameters.iter().for_each(|typ| for_each_generic(typ, rows_only, f));
            for_each_generic(&function.environment, rows_only, f);
            for_each_generic(&function.return_type, rows_only, f);
        },
        Type::Evidence(entries) => {
            for entry in entries.iter() {
                match entry {
                    EvidenceEntry::Rest { generic } => f(generic.0),
                    EvidenceEntry::Capability { key, capability } => {
                        key.args.iter().for_each(|typ| for_each_generic(typ, rows_only, f));
                        for_each_generic(capability, rows_only, f);
                    },
                }
            }
        },
    }
}

impl Context<'_> {
    /// Applies `mapping` to `definition` and replaces each definition it uses with the
    /// specialization for the rows it is used with
    fn specialize(&mut self, mut definition: Definition, mapping: Option<&Vec<Type>>) -> Definition {
        if let Some(mapping) = mapping {
            let substitute = |typ: &mut Type| {
                if typ.contains_generics() {
                    *typ = typ.substitute(mapping);
                }
            };
            substitute(&mut definition.typ);
            definition.for_each_type_mut(substitute);
        }
        let generic_count = definition.generic_count;

        for instruction in definition.instructions.values_mut() {
            if let Instruction::Instantiate(id, bindings) = instruction
                && let Some((new_id, bindings)) = self.instantiation(*id, bindings)
            {
                *instruction = match bindings.is_empty() {
                    true => Instruction::Id(Value::Definition(new_id)),
                    false => Instruction::Instantiate(new_id, Arc::new(bindings)),
                };
            }
        }

        // A generic definition used without an `Instantiate` shares this definition's generics, so
        // we need to specialize it over the same rows
        definition.for_each_used_value_mut(|value| {
            let Value::Definition(id) = value else { return };
            let Some(rows) = self.row_generics.get(id).filter(|rows| !rows.is_empty()) else { return };
            let mapping = mapping.expect("only a definition with row generics refers to another that shares them");
            let key = rows.iter().map(|generic| mapping[*generic as usize].clone()).collect();
            let own_count = self.templates[id].generic_count as usize;
            *id = self.specialization(*id, key, generic_count, |_| mapping[..own_count.min(mapping.len())].to_vec());
        });
        definition
    }

    /// Returns the specialization of `id` for `bindings` along with the bindings for its remaining
    /// generics. Returns `None` if `id` has no row generics to specialize over.
    fn instantiation(&mut self, id: DefinitionId, bindings: &[Type]) -> Option<(DefinitionId, Vec<Type>)> {
        let rows = &self.row_generics[&id];
        if rows.is_empty() {
            return None;
        }

        // Each generic mentioned by a row becomes a new generic of the specialization, numbered
        // after the definition's own generics
        let own_count = self.templates[&id].generic_count;
        // These are kept in the order they first appear
        let mut mentioned: Vec<u32> = Vec::new();
        for generic in rows {
            for_each_generic(&bindings[*generic as usize], false, &mut |generic| {
                if !mentioned.contains(&generic) {
                    mentioned.push(generic);
                }
            });
        }
        let rename_count = mentioned.iter().max().map_or(0, |max| max + 1);
        let mut rename = mapvec(0..rename_count, Type::generic);
        for (index, generic) in mentioned.iter().enumerate() {
            rename[*generic as usize] = Type::generic(own_count + index as u32);
        }

        let mut new_bindings = bindings.to_vec();
        let mut key = Vec::with_capacity(rows.len());
        for &generic in rows {
            key.push(match &bindings[generic as usize] {
                row @ Type::Evidence(_) if mentioned.is_empty() => row.clone(),
                row @ Type::Evidence(_) => row.substitute(&rename),
                other => Type::Evidence(Arc::new(evidence::entries(other).to_vec())),
            });
            new_bindings[generic as usize] = Type::UNIT;
        }
        new_bindings.extend(mentioned.iter().map(|generic| Type::generic(*generic)));

        let rows = rows.clone();
        let generic_count = own_count + mentioned.len() as u32;
        let new_id = self.specialization(id, key, generic_count, |key| {
            let mut mapping = mapvec(0..own_count, Type::generic);
            for (generic, row) in rows.iter().zip(key) {
                mapping[*generic as usize] = row.clone();
            }
            mapping
        });
        Some((new_id, new_bindings))
    }

    /// Returns the specialization of `id` for the rows in `key`. If this is a new specialization,
    /// it is queued to be created later using the mapping returned by `mapping` for those rows.
    fn specialization(
        &mut self, id: DefinitionId, key: Vec<Type>, generic_count: u32, mapping: impl FnOnce(&[Type]) -> Vec<Type>,
    ) -> DefinitionId {
        let key = (id, key, generic_count);
        if let Some(existing) = self.specialized.get(&key) {
            return *existing;
        }
        let new_id = next_definition_id();
        self.queue.push(Specialization { old_id: id, new_id, mapping: mapping(&key.1), generic_count });
        self.specialized.insert(key, new_id);
        new_id
    }
}
