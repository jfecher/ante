//! The structure of evidence and how its slots are found.
//!
//! The handler an effect reaches is decided by where the effect is written, never by what a type or
//! row generic is later bound to. A function's own handlers never catch the effects of one of its
//! row generics, and two effects written separately stay separate even if their type arguments
//! become equal. To get this, evidence holds a slot for every effect of a row in the same order as
//! the row. When a row generic is bound, the effects it is bound to take the generic's place.
//!
//! A slot is found by its position among the slots for the same effect, i.e. how many of them are
//! ahead of it. This changes depending on what the row generics ahead of it are bound to. So an
//! instruction naming a slot also carries the evidence type of everything ahead of the slot (its
//! `prefix`). The prefix is substituted like any other type, which keeps the count correct without
//! anything else in the instruction needing to change.
use crate::{
    iterator_extensions::{mapvec, sort_runs},
    mir::{EffectKey, EvidenceEntry, Generic, Type},
    parser::ids::TopLevelName,
    type_inference::row::{RowRelation, resolve_row},
};

/// Converts evidence to its canonical form by sorting the capabilities between each pair of row
/// generics by effect. Capabilities for the same effect are kept in their original order. Nothing
/// is moved past a row generic since the generic may later be bound to evidence holding the same
/// effects.
pub(crate) fn normalize(mut entries: Vec<EvidenceEntry>) -> Vec<EvidenceEntry> {
    sort_by_effect(&mut entries, |entry| match entry {
        EvidenceEntry::Capability { key, .. } => Some(key.effect),
        EvidenceEntry::Rest { .. } => None,
    });
    entries
}

/// Sort each group of `items` between two row generics by effect. Row generics are the items for
/// which `effect` returns `None`.
pub(crate) fn sort_by_effect<T>(items: &mut [T], effect: impl Fn(&T) -> Option<TopLevelName>) {
    // This needs to be a stable sort since the capabilities for the same effect are distinguished
    // by their order
    sort_runs(items, |item| effect(item).is_none(), |a, b| effect(a).cmp(&effect(b)));
}

/// Applies `substitute` to each entry, replacing each row generic with the entries of the evidence
/// it is bound to. Returns `None` if `substitute` didn't change anything.
pub(crate) fn substitute(
    entries: &[EvidenceEntry], substitute: impl Fn(&Type) -> Option<Type>,
) -> Option<Vec<EvidenceEntry>> {
    let mut changed = false;
    let mut apply = |typ: &Type| {
        let result = substitute(typ);
        changed |= result.is_some();
        result.unwrap_or_else(|| typ.clone())
    };
    let mut out = Vec::with_capacity(entries.len());
    for entry in entries {
        match entry {
            EvidenceEntry::Capability { key, capability } => {
                let key = EffectKey { effect: key.effect, args: mapvec(&key.args, &mut apply) };
                out.push(EvidenceEntry::Capability { key, capability: apply(capability) });
            },
            EvidenceEntry::Rest { generic } => match apply(&Type::Generic(*generic)) {
                // A generic bound to another generic is a row holding only that generic
                Type::Generic(generic) => out.push(EvidenceEntry::Rest { generic }),
                bound => out.extend(self::entries(&bound).iter().cloned()),
            },
        }
    }
    changed.then(|| normalize(out))
}

/// Returns the entries of the evidence type `typ`
pub(crate) fn entries(typ: &Type) -> &[EvidenceEntry] {
    match typ {
        Type::Evidence(entries) => entries,
        // A residual row variable converts to `()`, see `convert_type`
        Type::Tuple(fields) if fields.is_empty() => &[],
        other => panic!("`{other}` is not an evidence type"),
    }
}

/// Returns the evidence type of everything ahead of the slot at `index`. See the module docs.
pub(crate) fn prefix(entries: &[EvidenceEntry], index: usize) -> Type {
    Type::Evidence(std::sync::Arc::new(entries[..index].to_vec()))
}

/// Returns the number of capabilities for `effect` in `entries`, or `None` if this depends on what
/// a row generic among them is bound to
pub(crate) fn count(entries: &[EvidenceEntry], effect: TopLevelName) -> Option<usize> {
    let mut count = 0;
    for entry in entries {
        match entry {
            EvidenceEntry::Capability { key, .. } => count += (key.effect == effect) as usize,
            EvidenceEntry::Rest { .. } => return None,
        }
    }
    Some(count)
}

/// Returns the index of the capability for `effect` in `entries`, skipping the first `ahead`
/// capabilities for that effect
pub(crate) fn slot_index(entries: &[EvidenceEntry], effect: TopLevelName, ahead: usize) -> Option<usize> {
    let slots = entries.iter().enumerate().filter(|(_, entry)| match entry {
        EvidenceEntry::Capability { key, .. } => key.effect == effect,
        EvidenceEntry::Rest { .. } => false,
    });
    slots.map(|(index, _)| index).nth(ahead)
}

/// Returns the index of the row generic in `entries`, skipping the first `ahead` row generics
pub(crate) fn rest_index(entries: &[EvidenceEntry], ahead: usize) -> Option<usize> {
    let rests = entries.iter().enumerate().filter(|(_, entry)| entry.rest_generic().is_some());
    rests.map(|(index, _)| index).nth(ahead)
}

/// Returns the number of row generics in `entries`
pub(crate) fn rest_count(entries: &[EvidenceEntry]) -> usize {
    entries.iter().filter(|entry| entry.rest_generic().is_some()).count()
}

impl EvidenceEntry {
    pub(crate) fn rest_generic(&self) -> Option<Generic> {
        match self {
            EvidenceEntry::Rest { generic } => Some(*generic),
            EvidenceEntry::Capability { .. } => None,
        }
    }
}

/// Resolves each entry of the evidence `needed` to the slot of the evidence `provided` it should be
/// taken from, or `None` if `provided` holds no such entry.
///
/// This uses the same rules type inference uses to check a row against a wider one, see
/// [resolve_row]. Each repeat of an effect or row generic takes the matching repeat in `provided`.
/// If `needed` has more repeats than `provided` does, the extras all share the last one.
pub(crate) fn resolve(needed: &[EvidenceEntry], provided: &[EvidenceEntry]) -> Vec<Option<usize>> {
    #[derive(PartialEq)]
    enum Head {
        Effect(TopLevelName),
        Generic(Generic),
    }
    let head = |entry: &EvidenceEntry| {
        Some(match entry {
            EvidenceEntry::Capability { key, .. } => Head::Effect(key.effect),
            EvidenceEntry::Rest { generic } => Head::Generic(*generic),
        })
    };
    let same = |a: &EvidenceEntry, b: &EvidenceEntry| match (a, b) {
        (EvidenceEntry::Capability { key: a, .. }, EvidenceEntry::Capability { key: b, .. }) => a == b,
        (a, b) => a.rest_generic().is_some() && a.rest_generic() == b.rest_generic(),
    };
    resolve_row(needed, provided, RowRelation::Within, head, |_| true, same)
        .expect("an entry resolved within a row never rules the row out")
}
