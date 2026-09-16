//! Row polymorphism helpers shared by effect rows and places rows: sets of entries with an
//! open tail, compared by subtyping or unification.
use std::sync::Arc;

use crate::type_inference::{
    TypeChecker, Variance,
    types::{Type, TypeBindings},
};

pub type Row<T> = Option<Arc<Vec<T>>>;

/// Whether an effect row being compared sits in a covariant or invariant position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowMode {
    /// A position where the mir builder can adapt a wider effect set so subtyping is possible
    Coercible,
    /// Both rows must unify exactly
    Exact,
}

/// Which kind of row a list of entries belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowKind {
    Effects,
    Places,
}

impl RowKind {
    /// The sort-grouping key for an already-followed leaf type, if concrete (else `None`)
    fn head(self, typ: &Type) -> Option<Type> {
        match self {
            RowKind::Effects => {
                let no_bindings = TypeBindings::default();
                typ.effect_head(&no_bindings, &no_bindings).map(Type::UserDefined)
            },
            RowKind::Places => {
                if let Type::Place(_) = typ {
                    Some(typ.clone())
                } else {
                    None
                }
            },
        }
    }

    /// If `typ` is this kind's row type, return its raw entries, `Some(None)` is a closed row
    fn as_row(self, typ: &Type) -> Option<Row<Type>> {
        match (self, typ) {
            (RowKind::Effects, Type::Effects(row)) | (RowKind::Places, Type::Places(row)) => Some(row.clone()),
            _ => None,
        }
    }

    /// Build a row type from an already-canonical (flattened/followed/sorted/deduped) list
    pub(super) fn row_from_canonical(self, list: Vec<Type>) -> Type {
        let row = (!list.is_empty()).then(|| Arc::new(list));
        match self {
            RowKind::Effects => Type::Effects(row),
            RowKind::Places => Type::Places(row),
        }
    }
}

/// The result of matching one row's concrete entries against another's (effects or places).
pub(super) struct RowMatch {
    a_open: Vec<Type>,
    b_open: Vec<Type>,
    /// `a`'s concrete entries with no match in `b`
    a_leftover: Vec<Type>,
    /// `b`'s concrete entries with no match in `a`
    b_leftover: Vec<Type>,
}

/// True if this entry is an unbound type variable acting as the row's open tail.
/// Entries must already be zonked.
fn is_open(typ: &Type) -> bool {
    matches!(typ, Type::Variable(_))
}

/// Flatten every entry reachable from `entries` into `found`
pub(super) fn flatten_row_into(
    kind: RowKind, entries: &[Type], found: &mut Vec<Type>, bindings: &TypeBindings, more_bindings: &TypeBindings,
) {
    for entry in entries {
        let followed = entry.follow_two(bindings, more_bindings);
        match kind.as_row(&followed) {
            Some(Some(row)) => flatten_row_into(kind, &row, found, bindings, more_bindings),
            Some(None) => (),
            None => found.push(followed),
        }
    }
}

/// Zonk each entry in place
pub(super) fn follow_row(entries: &mut [Type], bindings: &TypeBindings, more_bindings: &TypeBindings) {
    for entry in entries.iter_mut() {
        if let Some(typ) = entry.follow_all_opt(bindings, more_bindings) {
            *entry = typ;
        }
    }
}

/// Sort and deduplicate the given row. Entries must already be zonked.
pub(super) fn sort_and_dedup_row(kind: RowKind, entries: &mut Vec<Type>) {
    let sort_key = |entry: &Type| {
        let head = kind.head(entry);
        (is_open(entry), head.is_none(), head)
    };
    entries.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)).then_with(|| a.cmp(b)));
    entries.dedup();
}

/// Flatten, follow, sort, and deduplicate `entries` (dedup is by exact equality, not unification)
pub(super) fn canonicalize_row(
    kind: RowKind, entries: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings,
) -> Vec<Type> {
    let mut list = Vec::with_capacity(entries.len());
    flatten_row_into(kind, entries, &mut list, bindings, more_bindings);
    follow_row(&mut list, bindings, more_bindings);
    sort_and_dedup_row(kind, &mut list);
    list
}

/// Construct a canonicalized row by flattening, following & deduplicating entries.
pub(super) fn construct_row(
    kind: RowKind, list: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings,
) -> Type {
    kind.row_from_canonical(canonicalize_row(kind, list, bindings, more_bindings))
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Partition `a_list`/`b_list` into open vs. concrete entries, then pair up
    /// concrete entries between the two sides using `try_match`. Entries must already
    /// be flattened, zonked, and deduplicated.
    pub(super) fn match_row_entries(
        a_list: Vec<Type>, b_list: Vec<Type>, mut try_match: impl FnMut(&Type, &Type) -> bool,
    ) -> RowMatch {
        let (a_open, a_concrete): (Vec<Type>, Vec<Type>) = a_list.into_iter().partition(is_open);
        let (b_open, b_concrete): (Vec<Type>, Vec<Type>) = b_list.into_iter().partition(is_open);

        let mut b_matched = vec![false; b_concrete.len()];
        let mut a_leftover = Vec::new();
        for a_item in a_concrete {
            let matched =
                b_concrete.iter().enumerate().find(|(i, b_item)| !b_matched[*i] && try_match(&a_item, b_item));
            match matched {
                Some((i, _)) => b_matched[i] = true,
                None => a_leftover.push(a_item),
            }
        }

        let b_leftover =
            b_concrete.into_iter().zip(b_matched).filter_map(|(item, matched)| (!matched).then_some(item)).collect();
        RowMatch { a_open, b_open, a_leftover, b_leftover }
    }

    /// Flattens two effect rows and matches `a`'s concrete effects against `b`'s.
    /// Returns `None` if either row contains an error type
    fn match_rows(&self, a: &Type, b: &Type, variance: Variance, new_bindings: &mut TypeBindings) -> Option<RowMatch> {
        let a_list = self.collect_and_merge_effects(a, new_bindings);
        let b_list = self.collect_and_merge_effects(b, new_bindings);

        if a_list.iter().chain(b_list.iter()).any(|effect| effect.is_error()) {
            return None;
        }

        Some(Self::match_row_entries(a_list, b_list, |a_effect, b_effect| {
            self.subtype_matching_effect(std::slice::from_ref(b_effect), |_| false, a_effect, variance, new_bindings)
                .is_some()
        }))
    }

    /// Row-subtype two rows: is `a`'s actual set of entries permitted by `b`'s expected set?
    pub(super) fn row_subtype_generic(
        &self, kind: RowKind, m: RowMatch, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let RowMatch { a_open, b_open, mut a_leftover, mut b_leftover } = m;

        // What is left of `b`'s row end after it absorbs the entries `a` has that `b` didn't list
        let b_residual = match (b_open.first(), a_leftover.is_empty()) {
            (open, true) => open.cloned(),
            (Some(open), false) => {
                let fresh = self.next_type_variable();
                a_leftover.push(fresh.clone());
                let binding = construct_row(kind, &a_leftover, &self.bindings, new_bindings);
                self.subtype(open, &binding, Variance::Invariant, RowMode::Exact, new_bindings)?;
                Some(fresh)
            },
            (None, false) => return Err(()),
        };

        let Some(a_open_first) = a_open.first() else { return Ok(()) };

        match b_residual {
            // Binding `a_open_first` to a row containing itself would create an infinitely recursive type
            Some(residual) if self.identical_entries(&residual, a_open_first, new_bindings) => return Ok(()),
            Some(residual) => b_leftover.push(residual),
            None => (),
        }

        let binding = construct_row(kind, &b_leftover, &self.bindings, new_bindings);
        self.subtype(a_open_first, &binding, Variance::Invariant, RowMode::Exact, new_bindings)
    }

    /// Row-subtype two effect rows: is `a`'s actual set of effects permitted by `b`'s expected set?
    pub(super) fn row_subtype(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_rows(a, b, Variance::Contravariant, new_bindings) else { return Ok(()) };
        self.row_subtype_generic(RowKind::Effects, m, new_bindings)
    }

    /// Unify two rows, both must end up with the same set of entries.
    pub(super) fn row_unify_generic(
        &self, kind: RowKind, m: RowMatch, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let RowMatch { a_open, b_open, mut a_leftover, mut b_leftover } = m;

        let both_closed = |a_leftover: &[Type], b_leftover: &[Type]| {
            (a_leftover.is_empty() && b_leftover.is_empty()).then_some(()).ok_or(())
        };
        match (a_open.first(), b_open.first()) {
            (None, None) => both_closed(&a_leftover, &b_leftover),
            (Some(a_end), None) if a_leftover.is_empty() => self.bind_row_end(kind, a_end, &b_leftover, new_bindings),
            (None, Some(b_end)) if b_leftover.is_empty() => self.bind_row_end(kind, b_end, &a_leftover, new_bindings),
            (Some(_), None) | (None, Some(_)) => Err(()),
            (Some(a_end), Some(b_end)) if self.identical_entries(a_end, b_end, new_bindings) => {
                both_closed(&a_leftover, &b_leftover)
            },
            (Some(a_end), Some(b_end)) => {
                // Each end absorbs what the other side has that it lacks, sharing a fresh end
                let fresh = self.next_type_variable();
                a_leftover.push(fresh.clone());
                b_leftover.push(fresh);
                self.bind_row_end(kind, a_end, &b_leftover, new_bindings)?;
                self.bind_row_end(kind, b_end, &a_leftover, new_bindings)
            },
        }
    }

    /// Unify two effect rows
    pub(super) fn row_unify(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_rows(a, b, Variance::Invariant, new_bindings) else { return Ok(()) };
        self.row_unify_generic(RowKind::Effects, m, new_bindings)
    }

    /// Bind a row's open end to the row of `entries`
    fn bind_row_end(
        &self, kind: RowKind, end: &Type, entries: &[Type], new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let binding = construct_row(kind, entries, &self.bindings, new_bindings);
        self.subtype(end, &binding, Variance::Invariant, RowMode::Exact, new_bindings)
    }

    /// True if both entries are exactly equal after following type variables
    fn identical_entries(&self, a: &Type, b: &Type, new_bindings: &TypeBindings) -> bool {
        a.follow_two(&self.bindings, new_bindings) == b.follow_two(&self.bindings, new_bindings)
    }
}
