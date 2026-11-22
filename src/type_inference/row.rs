//! Row polymorphism helpers shared by effect rows and places rows. Both are lists of entries with
//! an open tail which are compared by subtyping or unification. A places row is a set, but an
//! effect row may repeat an effect and the order of those repeats matters. Effects also can't be
//! moved past a row's row variables. See [sort_and_dedup_row].
use crate::iterator_extensions::{mapvec, sort_runs};
use crate::shared_arc::Shared;

use crate::type_inference::{
    TypeChecker, Variance,
    kinds::Kind,
    places::{join_entries, same_place},
    types::{Type, TypeBindings, TypeVariableId},
};

pub type Row<T> = Option<Shared<Vec<T>>>;

/// Whether an effect row being compared sits in a covariant or invariant position
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowMode {
    /// A position where the mir builder can adapt a wider effect set so subtyping is possible
    Coercible,
    /// Both rows must unify exactly
    Exact,
}

/// Which side of a comparison a row's open end being bound is on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowEnd {
    /// The end of the expected row, bound to actual entries
    Expected,
    /// The end of the actual row, bound to expected entries
    Actual,
    /// Either end when unifying
    Exact,
}

/// Controls how the entries of one effect row are resolved against those of another. See
/// [resolve_row].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowRelation {
    /// Both rows must hold the same entries, slot for slot. This is required for two function types
    /// to be equal.
    Same,
    /// Each entry of the first row must be provided by some entry of the second. E.g. when
    /// performing an effect somewhere the second row is allowed, or when using a function as a
    /// function with the second row.
    Within,
}

/// Resolves each entry of the effect row `narrow` to the entry of `wide` which provides it. Returns
/// the index of that entry for each, or `None` if `wide` has no such entry.
///
/// An effect is a label along with its arguments, and a row may contain the same effect several
/// times. Entries sharing a `head` are distinguished by their order, with the first being the
/// nearest handler. We can't go by their arguments alone since differing arguments may later be
/// substituted to the same type. `same` is used to decide whether two entries are the same effect.
/// - [RowRelation::Same]: each entry resolves to the next remaining entry in `wide` with the same
///   head. Returns `Err` if `same` is false for that entry.
/// - [RowRelation::Within]: each entry resolves to the next remaining entry in `wide` for which
///   `same` is true. If there are none left, it shares the last entry taken for which `same` is
///   true. This lets a row which repeats an effect more often than `wide` still resolve.
///
/// Entries which aren't `ordered` have no repeats to distinguish, so they only need `same` to be
/// true.
pub(crate) fn resolve_row<T, Head: PartialEq>(
    narrow: &[T], wide: &[T], relation: RowRelation, head: impl Fn(&T) -> Option<Head>, ordered: impl Fn(&T) -> bool,
    mut same: impl FnMut(&T, &T) -> bool,
) -> Result<Vec<Option<usize>>, ()> {
    let wide_heads = mapvec(wide, &head);
    let mut taken = vec![false; wide.len()];
    let mut slots = Vec::with_capacity(narrow.len());

    for entry in narrow {
        let entry_head = head(entry);
        let slot = if relation == RowRelation::Same && entry_head.is_some() && ordered(entry) {
            match (0..wide.len()).find(|i| !taken[*i] && wide_heads[*i] == entry_head) {
                Some(next) if same(entry, &wide[next]) => Some(next),
                Some(_) => return Err(()),
                None => None,
            }
        } else {
            // Effects with different heads are never the same
            let mut is_same = |i: usize| {
                let same_head = entry_head.is_none() || wide_heads[i].is_none() || wide_heads[i] == entry_head;
                same_head && same(entry, &wide[i])
            };
            match (0..wide.len()).find(|i| !taken[*i] && is_same(*i)) {
                None if relation == RowRelation::Within => (0..wide.len()).rev().find(|i| taken[*i] && is_same(*i)),
                slot => slot,
            }
        };
        if let Some(slot) = slot {
            taken[slot] = true;
        }
        slots.push(slot);
    }
    Ok(slots)
}

/// Which kind of row a list of entries belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowKind {
    Effects,
    Places,
}

impl RowKind {
    /// The kind of each entry or open end variable in a row of this kind
    fn variable_kind(self) -> Kind {
        match self {
            RowKind::Effects => Kind::Effect,
            RowKind::Places => Kind::Place,
        }
    }

    /// The sort-grouping key for an already-followed leaf type, if concrete (else `None`)
    fn head(self, typ: &Type) -> Option<Type> {
        match self {
            RowKind::Effects => {
                let no_bindings = TypeBindings::default();
                typ.effect_head(&no_bindings, &no_bindings).map(Type::UserDefined)
            },
            RowKind::Places => {
                if let Type::Place(..) = typ {
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
        let row = (!list.is_empty()).then(|| Shared::new(list));
        match self {
            RowKind::Effects => Type::Effects(row),
            RowKind::Places => Type::Places(row),
        }
    }
}

/// The result of matching one row's concrete entries against another's (effects or places).
pub(super) struct RowMatch {
    pub(super) a_open: Vec<Type>,
    pub(super) b_open: Vec<Type>,
    /// `a`'s concrete entries with no match in `b`
    pub(super) a_leftover: Vec<Type>,
    /// `b`'s concrete entries with no match in `a`
    pub(super) b_leftover: Vec<Type>,
}

/// True if this entry is an unbound type variable acting as the row's open tail, possibly
/// with a mode for places like `mut ?t`. Entries must already be zonked.
pub(super) fn is_open(typ: &Type) -> bool {
    match typ {
        Type::Variable(_) => true,
        Type::PlaceProjection(base, segments, Some(_)) => {
            segments.is_empty() && matches!(base.as_ref(), Type::Variable(_))
        },
        _ => false,
    }
}

/// Remove any open tails both rows share
fn cancel_shared_open(mut a_open: Vec<Type>, b_open: Vec<Type>) -> (Vec<Type>, Vec<Type>) {
    let b_open = b_open
        .into_iter()
        .filter(|b| match a_open.iter().position(|a| a == b) {
            Some(i) => {
                a_open.remove(i);
                false
            },
            None => true,
        })
        .collect();
    (a_open, b_open)
}

/// Flatten every entry reachable from `entries` into `found`
pub(super) fn flatten_row_into(
    kind: RowKind, entries: &[Type], found: &mut Vec<Type>, bindings: &TypeBindings, more_bindings: &TypeBindings,
) {
    for entry in entries {
        let followed = entry.follow_two(bindings, more_bindings);
        // Push the segments into whatever the base is bound to
        if let Type::PlaceProjection(base, segments, mode) = &followed {
            match Type::project_place_with_mode(base, segments, *mode, bindings, more_bindings) {
                projection @ Type::PlaceProjection(..) => found.push(projection),
                projected => flatten_row_into(kind, &[projected], found, bindings, more_bindings),
            }
            continue;
        }
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

/// Sort the given row, merging repeated places. Entries must already be zonked.
///
/// The handler an effect reaches is determined by its position among the other effects sharing its
/// head. This position can't change when type arguments or row variables are substituted, so we
/// keep effects with the same head in order and never merge them. We also never move an effect past
/// a row variable since the variable may later be bound to effects with the same head. This leaves
/// only the effects between two row variables to be sorted by head.
pub(super) fn sort_and_dedup_row(kind: RowKind, entries: &mut Vec<Type>) {
    match kind {
        RowKind::Effects => {
            let head = |entry: &Type| kind.head(entry);
            sort_runs(
                entries,
                |entry| head(entry).is_none(),
                |a, b| {
                    let positional = !is_places_effect(a) && !is_places_effect(b);
                    let by_head = head(a).cmp(&head(b));
                    by_head.then_with(|| if positional { std::cmp::Ordering::Equal } else { a.cmp(b) })
                },
            );
            // `Mutate` is never handled so repeats of it are meaningless and can be merged
            entries.dedup_by(|entry, previous| is_places_effect(entry) && entry == previous);
        },
        RowKind::Places => {
            let sort_key = |entry: &Type| {
                let head = kind.head(entry);
                (is_open(entry), head.is_none(), head)
            };
            entries.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)).then_with(|| a.cmp(b)));
            entries.dedup_by(|entry, previous| {
                let same = same_place(entry, previous);
                if same {
                    *previous = join_entries(previous.clone(), entry.clone());
                }
                same
            })
        },
    }
}

/// Moves each effect of a row ahead of its row variables. This is done on rows as they were written
/// so that the order effects and row variables are written in doesn't matter.
pub(super) fn effects_before_row_variables(entries: &mut [Type]) {
    entries.sort_by_key(|entry| RowKind::Effects.head(entry).is_none());
}

/// True if this is an effect over places like `Mutate 'p`. These have no handlers, so there is no
/// need to distinguish between repeats of them.
fn is_places_effect(effect: &Type) -> bool {
    match effect {
        Type::Application(_, arguments) => matches!(arguments.first(), Some(Type::Places(_))),
        _ => false,
    }
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
        let slots = mapvec(&a_concrete, |a_item| {
            let slot = (0..b_concrete.len()).find(|i| !b_matched[*i] && try_match(a_item, &b_concrete[*i]));
            if let Some(slot) = slot {
                b_matched[slot] = true;
            }
            slot
        });
        Self::row_match_from_slots(a_open, a_concrete, b_open, b_concrete, slots)
    }

    /// Splits the concrete entries of each row into the pairs matched by `slots` and the entries
    /// left over. `slots` holds the index into `b_concrete` that each entry of `a_concrete` was
    /// matched to.
    fn row_match_from_slots(
        a_open: Vec<Type>, a_concrete: Vec<Type>, b_open: Vec<Type>, b_concrete: Vec<Type>, slots: Vec<Option<usize>>,
    ) -> RowMatch {
        let mut b_matched = vec![false; b_concrete.len()];
        slots.iter().flatten().for_each(|slot| b_matched[*slot] = true);

        let a_leftover =
            a_concrete.into_iter().zip(slots).filter_map(|(item, slot)| slot.is_none().then_some(item)).collect();
        let b_leftover =
            b_concrete.into_iter().zip(b_matched).filter_map(|(item, matched)| (!matched).then_some(item)).collect();
        RowMatch { a_open, b_open, a_leftover, b_leftover }
    }

    /// Flattens two effect rows and resolves `a`'s concrete effects against `b`'s according to
    /// `relation`, see [resolve_row]. Returns `None` if either row contains an error type.
    fn match_rows(
        &self, a: &Type, b: &Type, variance: Variance, relation: RowRelation, new_bindings: &mut TypeBindings,
    ) -> Result<Option<RowMatch>, ()> {
        let a_list = self.collect_and_merge_effects(a, new_bindings);
        let b_list = self.collect_and_merge_effects(b, new_bindings);

        if a_list.iter().chain(b_list.iter()).any(|effect| effect.is_error()) {
            return Ok(None);
        }

        let (a_open, a_concrete): (Vec<Type>, Vec<Type>) = a_list.into_iter().partition(is_open);
        let (b_open, b_concrete): (Vec<Type>, Vec<Type>) = b_list.into_iter().partition(is_open);

        let head = |effect: &Type| RowKind::Effects.head(effect);
        let ordered = |effect: &Type| !is_places_effect(effect);
        // `subtype_matching_effect` tries each comparison on a copy of the bindings, so a failed
        // comparison doesn't leave any bindings behind
        let same = |a_effect: &Type, b_effect: &Type| {
            let b_effect = std::slice::from_ref(b_effect);
            self.subtype_matching_effect(b_effect, |_| false, a_effect, variance, new_bindings).is_some()
        };
        let slots = resolve_row(&a_concrete, &b_concrete, relation, head, ordered, same)?;

        let mut row_match = Self::row_match_from_slots(a_open, a_concrete, b_open, b_concrete, slots);

        // The effects performed within a row are a set, so remove any repeats from the leftover
        // effects and row variables
        if relation == RowRelation::Within {
            let dedup = |entries: &mut Vec<Type>| {
                let mut kept: Vec<Type> = Vec::with_capacity(entries.len());
                entries.retain(|entry| {
                    let new = !kept.contains(entry);
                    if new {
                        kept.push(entry.clone());
                    }
                    new
                });
            };
            dedup(&mut row_match.a_leftover);
            dedup(&mut row_match.a_open);
        }
        Ok(Some(row_match))
    }

    /// Row-subtype two rows, adding `a`'s open ends to `b` rather than binding them to `b`'s other entries.
    /// `sides` are how `a`'s and `b`'s ends are bound, see [RowEnd].
    pub(super) fn row_subtype_generic(
        &self, kind: RowKind, m: RowMatch, sides: (RowEnd, RowEnd), new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let RowMatch { mut a_open, b_open, mut a_leftover, b_leftover } = m;
        a_open.retain(|a_end| !b_open.contains(a_end));

        // `b` grows through its newest open end, even one `a` shares, rather than another row within it
        match b_open.last() {
            Some(_) if a_leftover.is_empty() && a_open.is_empty() => Ok(()),
            Some(b_end) => {
                a_leftover.extend(a_open);
                a_leftover.push(self.next_variable(kind.variable_kind()));
                self.bind_end(kind, b_end, &a_leftover, sides.1, new_bindings)
            },
            None if !a_leftover.is_empty() => Err(()),
            None => a_open.iter().try_for_each(|a_end| self.bind_end(kind, a_end, &b_leftover, sides.0, new_bindings)),
        }
    }

    /// Row-subtype two effect rows: is `a`'s actual set of effects permitted by `b`'s expected set?
    pub(super) fn row_subtype(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_rows(a, b, Variance::Contravariant, RowRelation::Within, new_bindings)? else {
            return Ok(());
        };
        self.row_subtype_generic(RowKind::Effects, m, (RowEnd::Exact, RowEnd::Exact), new_bindings)
    }

    /// Unify two rows, both must end up with the same set of entries.
    /// `sides` are how `a`'s and `b`'s ends are bound, see [RowEnd].
    pub(super) fn row_unify_generic(
        &self, kind: RowKind, m: RowMatch, sides: (RowEnd, RowEnd), new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let RowMatch { a_open, b_open, mut a_leftover, mut b_leftover } = m;
        let (mut a_open, mut b_open) = cancel_shared_open(a_open, b_open);

        // Against a closed row, every open end is bound to that row's remaining entries
        match (a_open.is_empty(), b_open.is_empty()) {
            (false, true) if a_leftover.is_empty() => {
                return a_open.iter().try_for_each(|end| self.bind_end(kind, end, &b_leftover, sides.0, new_bindings));
            },
            (true, false) if b_leftover.is_empty() => {
                return b_open.iter().try_for_each(|end| self.bind_end(kind, end, &a_leftover, sides.1, new_bindings));
            },
            _ => (),
        }

        // Tails past the first must be absorbed by the other side like any other entry
        let (a_end, b_end) = (a_open.pop(), b_open.pop());
        a_leftover.extend(a_open);
        b_leftover.extend(b_open);

        let both_closed = |a_leftover: &[Type], b_leftover: &[Type]| {
            (a_leftover.is_empty() && b_leftover.is_empty()).then_some(()).ok_or(())
        };
        match (a_end.as_ref(), b_end.as_ref()) {
            (None, None) => both_closed(&a_leftover, &b_leftover),
            (Some(_), None) | (None, Some(_)) => Err(()),
            (Some(a_end), Some(b_end)) if self.identical_entries(a_end, b_end, new_bindings) => {
                both_closed(&a_leftover, &b_leftover)
            },
            (Some(a_end), Some(b_end)) => {
                // Each end absorbs what the other side has that it lacks, sharing a fresh end
                let fresh = self.next_variable(kind.variable_kind());
                a_leftover.push(fresh.clone());
                b_leftover.push(fresh);
                self.bind_end(kind, a_end, &b_leftover, sides.0, new_bindings)?;
                self.bind_end(kind, b_end, &a_leftover, sides.1, new_bindings)
            },
        }
    }

    /// Unify two effect rows
    pub(super) fn row_unify(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_rows(a, b, Variance::Invariant, RowRelation::Same, new_bindings)? else {
            return Ok(());
        };
        self.row_unify_generic(RowKind::Effects, m, (RowEnd::Exact, RowEnd::Exact), new_bindings)
    }

    /// Bind a row's open end to the row of `entries`, checking any mode a places end has
    fn bind_end(
        &self, kind: RowKind, end: &Type, entries: &[Type], side: RowEnd, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        match kind {
            RowKind::Effects => self.bind_row_end(kind, end, entries, new_bindings),
            RowKind::Places => self.bind_places_end(end, entries, side, new_bindings),
        }
    }

    /// Bind a row's open end to the row of `entries`
    pub(super) fn bind_row_end(
        &self, kind: RowKind, end: &Type, entries: &[Type], new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        if kind == RowKind::Effects {
            self.check_effect_bound(end, entries, new_bindings)?;
        }
        let binding = construct_row(kind, entries, &self.bindings, new_bindings);
        self.subtype(end, &binding, Variance::Invariant, RowMode::Exact, new_bindings)
    }

    /// Checks that each of `entries` is allowed by the bound on `end`, if any, before `end` is
    /// bound to them. An open end bounded by a row (see [TypeChecker::effect_bounds]) may only
    /// be bound to effects from that row. Entries that are open ends themselves are skipped
    /// since they inherit the same bound, see [Self::effect_bounds_of].
    fn check_effect_bound(&self, end: &Type, entries: &[Type], new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Type::Variable(id) = end.follow_two(&self.bindings, new_bindings) else { return Ok(()) };
        for bound in self.effect_bounds_of(id, new_bindings) {
            let allowed = self.collect_effects(&bound, new_bindings);

            for entry in entries {
                if let Type::Variable(_) = entry.follow_two(&self.bindings, new_bindings) {
                    continue;
                }
                let matches = allowed.iter().any(|effect| {
                    let mut trial = new_bindings.clone();
                    let matches = self.subtype(entry, effect, Variance::Invariant, RowMode::Exact, &mut trial).is_ok();
                    if matches {
                        *new_bindings = trial;
                    }
                    matches
                });
                if !matches {
                    return Err(());
                }
            }
        }
        Ok(())
    }

    /// Returns every row bounding the open end `id`. This includes its own bound along with the
    /// bound of any other open end whose binding now ends in `id`. We derive these from the
    /// bindings rather than recording them when an end is bound so that if a failed unification
    /// discards a binding, its bound is discarded with it.
    fn effect_bounds_of(&self, id: TypeVariableId, new_bindings: &TypeBindings) -> Vec<Type> {
        let mut entries = Vec::new();
        let mut bounds = Vec::new();
        for (bounded, bound) in &self.effect_bounds {
            let end = Type::Variable(*bounded);
            let bounds_id = *bounded == id || {
                entries.clear();
                flatten_row_into(RowKind::Effects, &[end], &mut entries, &self.bindings, new_bindings);
                entries.contains(&Type::Variable(id))
            };
            if bounds_id {
                bounds.push(bound.clone());
            }
        }
        bounds
    }

    /// True if both entries are exactly equal after following type variables
    fn identical_entries(&self, a: &Type, b: &Type, new_bindings: &TypeBindings) -> bool {
        a.follow_two(&self.bindings, new_bindings) == b.follow_two(&self.bindings, new_bindings)
    }
}
