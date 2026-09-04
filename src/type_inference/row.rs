//! This file implements various helper functions for row polymorphism used by the type checker.
//! Both effects and places are row polymorphic, although they also use subtyping as well.
use std::sync::Arc;

use crate::type_inference::{types::{Effect, Type, TypeBindings}, TypeChecker, Variance};


pub type Row<T> = Option<Arc<Vec<T>>>;

/// Whether an effect row being compared sits in a covariant or invariant position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowMode {
    /// A position where the mir builder can adapt a wider effect set so subtyping is possible
    Coercible,
    /// Both rows must unify exactly
    Exact,
}

/// The result of matching one row's concrete entries against another's (effects or places).
pub(super) struct RowMatch<T> {
    a_open: Vec<T>,
    b_open: Vec<T>,
    /// `a`'s concrete entries with no match in `b`
    a_leftover: Vec<T>,
    /// `b`'s concrete entries with no match in `a`
    b_leftover: Vec<T>,
}

/// Common shape shared by effect rows and places rows
pub(super) trait RowEntry: Clone {
    /// True if this entry is an unbound type variable acting as the row's open tail.
    /// Entries must already be zonked.
    fn is_open(&self) -> bool;

    /// The type actually compared/bound when this entry is a row's open end
    fn inner_type(&self) -> &Type;

    /// A fresh, unbound entry usable as a row's open tail
    fn fresh(next_var: &mut impl FnMut() -> Type) -> Self;

    /// The sort-grouping key for an already-followed leaf type, if concrete (else `None`)
    fn row_head(typ: &Type) -> Option<Type>;

    /// If `typ` is this row-kind's row type, return its raw entries (`Some(None)` = closed row)
    fn as_row(typ: &Type) -> Option<Row<Self>>;

    /// Rebuild a flattened leaf entry from `self` with its inner type replaced by `typ`
    fn flattened_leaf(&self, typ: Type, bindings: &TypeBindings, more_bindings: &TypeBindings) -> Self;

    /// Overwrite this entry's inner (comparison) type in place; other fields untouched.
    fn set_inner(&mut self, typ: Type);

    /// Build a row type from an already-canonical (flattened/followed/sorted/deduped) list.
    fn row_from_canonical(list: Vec<Self>) -> Type;
}

impl RowEntry for Effect {
    fn is_open(&self) -> bool {
        matches!(self.typ, Type::Variable(_))
    }

    fn inner_type(&self) -> &Type {
        &self.typ
    }

    fn fresh(next_var: &mut impl FnMut() -> Type) -> Self {
        Effect { id: next_var(), typ: next_var() }
    }

    fn row_head(typ: &Type) -> Option<Type> {
        let no_bindings = TypeBindings::default();
        typ.effect_head(&no_bindings, &no_bindings).map(Type::UserDefined)
    }

    fn as_row(typ: &Type) -> Option<Option<Arc<Vec<Effect>>>> {
        match typ {
            Type::Effects(row) => Some(row.clone()),
            _ => None,
        }
    }

    fn flattened_leaf(&self, typ: Type, bindings: &TypeBindings, more_bindings: &TypeBindings) -> Effect {
        Effect { id: self.id.follow_two(bindings, more_bindings), typ }
    }

    fn set_inner(&mut self, typ: Type) {
        self.typ = typ;
    }

    fn row_from_canonical(list: Vec<Effect>) -> Type {
        if list.is_empty() { Type::pure() } else { Type::Effects(Some(Arc::new(list))) }
    }
}

/// Flatten every entry reachable from `entries` into `found`
pub(super) fn flatten_row_into<T: RowEntry>(entries: &[T], found: &mut Vec<T>, bindings: &TypeBindings, more_bindings: &TypeBindings) {
    for entry in entries {
        let followed = entry.inner_type().follow_two(bindings, more_bindings);
        match T::as_row(&followed) {
            Some(Some(row)) => flatten_row_into(&row, found, bindings, more_bindings),
            Some(None) => (),
            None => found.push(entry.flattened_leaf(followed, bindings, more_bindings)),
        }
    }
}

/// Zonk each entry's inner type in place
pub(super) fn follow_row<T: RowEntry>(entries: &mut [T], bindings: &TypeBindings, more_bindings: &TypeBindings) {
    for entry in entries.iter_mut() {
        if let Some(typ) = entry.inner_type().follow_all_opt(bindings, more_bindings) {
            entry.set_inner(typ);
        }
    }
}

/// Sort and deduplicate the given row. Entries must already be zonked.
pub(super) fn sort_and_dedup_row<T: RowEntry>(entries: &mut Vec<T>, mut on_merge: impl FnMut(&T, &T)) {
    let sort_key = |entry: &T| {
        let head = T::row_head(entry.inner_type());
        (entry.is_open(), head.is_none(), head)
    };
    entries.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)).then_with(|| a.inner_type().cmp(b.inner_type())));
    entries.dedup_by(|dropped, kept| {
        let same = dropped.inner_type() == kept.inner_type();
        if same {
            on_merge(dropped, kept);
        }
        same
    });
}

/// Flatten, follow, sort, and deduplicate `entries` (dedup is by exact equality, not unification)
pub(super) fn canonicalize_row<T: RowEntry>(
    entries: &[T], bindings: &TypeBindings, more_bindings: &TypeBindings, on_merge: impl FnMut(&T, &T),
) -> Vec<T> {
    let mut list = Vec::with_capacity(entries.len());
    flatten_row_into(entries, &mut list, bindings, more_bindings);
    follow_row(&mut list, bindings, more_bindings);
    sort_and_dedup_row(&mut list, on_merge);
    list
}

/// Construct a canonicalized row by flattening, following & deduplicating entries.
pub(super) fn construct_row<T: RowEntry>(list: &[T], bindings: &TypeBindings, more_bindings: &TypeBindings) -> Type {
    T::row_from_canonical(canonicalize_row(list, bindings, more_bindings, |_, _| ()))
}

impl RowEntry for Type {
    fn is_open(&self) -> bool {
        matches!(self, Type::Variable(_))
    }

    fn inner_type(&self) -> &Type {
        self
    }

    fn fresh(next_var: &mut impl FnMut() -> Type) -> Self {
        next_var()
    }

    fn row_head(typ: &Type) -> Option<Type> {
        if let Type::PlaceAtom(_) = typ { Some(typ.clone()) } else { None }
    }

    fn as_row(typ: &Type) -> Option<Option<Arc<Vec<Type>>>> {
        match typ {
            Type::Places(row) => Some(row.clone()),
            _ => None,
        }
    }

    fn flattened_leaf(&self, typ: Type, _bindings: &TypeBindings, _more_bindings: &TypeBindings) -> Type {
        typ
    }

    fn set_inner(&mut self, typ: Type) {
        *self = typ;
    }

    fn row_from_canonical(list: Vec<Type>) -> Type {
        if list.is_empty() { Type::Places(None) } else { Type::Places(Some(Arc::new(list))) }
    }
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Partition `a_list`/`b_list` into open vs. concrete entries, then pair up
    /// concrete entries between the two sides using `try_match`. Entries must already
    /// be flattened, zonked, and deduplicated.
    pub(super) fn match_row_entries<T: RowEntry>(a_list: Vec<T>, b_list: Vec<T>, mut try_match: impl FnMut(&T, &T) -> bool) -> RowMatch<T> {
        let (a_open, a_concrete): (Vec<T>, Vec<T>) = a_list.into_iter().partition(RowEntry::is_open);
        let (b_open, b_concrete): (Vec<T>, Vec<T>) = b_list.into_iter().partition(RowEntry::is_open);

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

        let b_leftover = b_concrete.into_iter().zip(b_matched).filter_map(|(item, matched)| (!matched).then_some(item)).collect();
        RowMatch { a_open, b_open, a_leftover, b_leftover }
    }

    /// Flattens two effect rows and matches `a`'s concrete effects against `b`'s.
    /// Returns `None` if either row contains an error type
    fn match_rows(&self, a: &Type, b: &Type, variance: Variance, new_bindings: &mut TypeBindings) -> Option<RowMatch<Effect>> {
        let a_list = self.collect_and_merge_effects(a, new_bindings);
        let b_list = self.collect_and_merge_effects(b, new_bindings);

        if a_list.iter().chain(b_list.iter()).any(|effect| effect.typ.is_error()) {
            return None;
        }

        Some(Self::match_row_entries(a_list, b_list, |a_effect, b_effect| {
            self.subtype_matching_effect(std::slice::from_ref(b_effect), |_| false, a_effect, variance, new_bindings)
                .is_some()
        }))
    }

    /// Row-subtype two rows of `T`: is `a`'s actual set of entries permitted by `b`'s expected set?
    /// `skip_leftover` lets a caller ignore certain unmatched `a` entries when `b`'s row is closed
    pub(super) fn row_subtype_generic<T: RowEntry>(
        &self, m: RowMatch<T>, skip_leftover: impl Fn(&T) -> bool, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let RowMatch { a_open, b_open, mut a_leftover, mut b_leftover } = m;

        // What is left of `b`'s row end after it absorbs the entries `a` has that `b` didn't list
        let b_residual = match (b_open.first(), a_leftover.is_empty()) {
            (open, true) => open.cloned(),
            (Some(open), false) => {
                let fresh = T::fresh(&mut || self.next_type_variable());
                a_leftover.push(fresh.clone());
                let binding = construct_row(&a_leftover, &self.bindings, new_bindings);
                self.subtype(open.inner_type(), &binding, Variance::Invariant, RowMode::Exact, new_bindings)?;
                Some(fresh)
            },
            (None, false) if a_leftover.iter().all(&skip_leftover) => None,
            (None, false) => return Err(()),
        };

        let Some(a_open_first) = a_open.first() else { return Ok(()) };

        match b_residual {
            // Binding `a_open_first` to a row containing itself would create an infinitely recursive type
            Some(residual) if self.identical_entries(&residual, a_open_first, new_bindings) => return Ok(()),
            Some(residual) => b_leftover.push(residual),
            None => (),
        }

        let binding = construct_row(&b_leftover, &self.bindings, new_bindings);
        self.subtype(a_open_first.inner_type(), &binding, Variance::Invariant, RowMode::Exact, new_bindings)
    }

    /// Row-subtype two effect rows: is `a`'s actual set of effects permitted by `b`'s expected set?
    pub(super) fn row_subtype(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_rows(a, b, Variance::Contravariant, new_bindings) else { return Ok(()) };
        // TODO: Hack: review & potentionally remove `is_implicit_effect_placeholder`
        self.row_subtype_generic(m, |effect: &Effect| self.is_implicit_effect_placeholder(&effect.typ), new_bindings)
    }

    /// Unify two rows of `T`: both must end up with the same set of entries.
    pub(super) fn row_unify_generic<T: RowEntry>(&self, m: RowMatch<T>, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let RowMatch { a_open, b_open, mut a_leftover, mut b_leftover } = m;

        let both_closed =
            |a_leftover: &[T], b_leftover: &[T]| (a_leftover.is_empty() && b_leftover.is_empty()).then_some(()).ok_or(());
        match (a_open.first(), b_open.first()) {
            (None, None) => both_closed(&a_leftover, &b_leftover),
            (Some(a_end), None) if a_leftover.is_empty() => self.bind_row_end(a_end, &b_leftover, new_bindings),
            (None, Some(b_end)) if b_leftover.is_empty() => self.bind_row_end(b_end, &a_leftover, new_bindings),
            (Some(_), None) | (None, Some(_)) => Err(()),
            (Some(a_end), Some(b_end)) if self.identical_entries(a_end, b_end, new_bindings) => {
                both_closed(&a_leftover, &b_leftover)
            },
            (Some(a_end), Some(b_end)) => {
                // Each end absorbs what the other side has that it lacks, sharing a fresh end
                let fresh = T::fresh(&mut || self.next_type_variable());
                a_leftover.push(fresh.clone());
                b_leftover.push(fresh);
                self.bind_row_end(a_end, &b_leftover, new_bindings)?;
                self.bind_row_end(b_end, &a_leftover, new_bindings)
            },
        }
    }

    /// Unify two rows
    pub(super) fn row_unify(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_rows(a, b, Variance::Invariant, new_bindings) else { return Ok(()) };
        self.row_unify_generic(m, new_bindings)
    }

    /// Bind a row's open end to the row of `entries`
    fn bind_row_end<T: RowEntry>(&self, end: &T, entries: &[T], new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let binding = construct_row(entries, &self.bindings, new_bindings);
        self.subtype(end.inner_type(), &binding, Variance::Invariant, RowMode::Exact, new_bindings)
    }

    /// True if both entries are exactly equal after following type variables
    fn identical_entries<T: RowEntry>(&self, a: &T, b: &T, new_bindings: &TypeBindings) -> bool {
        a.inner_type().follow_two(&self.bindings, new_bindings) == b.inner_type().follow_two(&self.bindings, new_bindings)
    }
}
