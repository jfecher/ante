//! This file has any type-checker specific code for places.
//! Places are sets of variables or values a reference may refer to like `'a` or `'(x, y)`.

use inc_complete::DbGet;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostic,
    incremental::{DbHandle, GetItem, GetItemRaw, RawItem},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{self, Expr, PlaceSegment, ReferenceKind, Segments, TopLevelItemKind, TypeDefinitionBody},
        ids::{ExprId, NameId, NameStore},
    },
    shared_arc::Shared,
    type_inference::{
        TypeChecker, Variance,
        row::{RowEnd, RowKind, RowMatch, RowMode, canonicalize_row, construct_row, flatten_row_into, is_open},
    },
};

use super::{
    TypeErrorKind,
    generics::Generic,
    types::{FunctionType, PrimitiveType, Type, TypeBindings, TypePrinter, TypeVariableId},
};

/// Anonymous references record the scope depth they're valid in for escape analysis
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ScopeDepth(pub(crate) u32);

impl ScopeDepth {
    pub fn deeper(self) -> Self {
        Self(self.0 + 1)
    }
}

/// An explicit mutability modifier on a place, as in `mut 'p` or `read 'p`
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlaceMode {
    Mut,
    Read,
}

impl PlaceMode {
    /// The mode a reference keyword applies to its place, `None` for the polymorphic `ref`
    pub fn of_reference(kind: ReferenceKind) -> Option<PlaceMode> {
        match kind {
            ReferenceKind::Ref => None,
            ReferenceKind::Mut => Some(PlaceMode::Mut),
            ReferenceKind::Read => Some(PlaceMode::Read),
        }
    }

    fn mutability(self) -> Mutability {
        match self {
            PlaceMode::Mut => Mutability::Mutable,
            PlaceMode::Read => Mutability::ReadOnly,
        }
    }
}

impl std::fmt::Display for PlaceMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PlaceMode::Mut => write!(f, "mut"),
            PlaceMode::Read => write!(f, "read"),
        }
    }
}

/// How a reference may access a concrete place
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlaceAccess {
    Plain {
        mutable: bool,
    },
    Mode(PlaceMode),
}

impl PlaceAccess {
    fn mutability(self) -> Mutability {
        match self {
            PlaceAccess::Plain { mutable: true } => Mutability::Mutable,
            PlaceAccess::Plain { mutable: false } => Mutability::ReadOnly,
            PlaceAccess::Mode(mode) => mode.mutability(),
        }
    }
}

/// What a reference may do through a places row entry
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mutability {
    Mutable,
    ReadOnly,
    Polymorphic,
}

impl Mutability {
    /// The mutability of a reference through any of `entries`: mutable only if all are
    pub fn join(entries: impl IntoIterator<Item = Mutability>) -> Mutability {
        entries.into_iter().fold(Mutability::Mutable, |joined, entry| match (joined, entry) {
            (Mutability::ReadOnly, _) | (_, Mutability::ReadOnly) => Mutability::ReadOnly,
            (Mutability::Polymorphic, _) | (_, Mutability::Polymorphic) => Mutability::Polymorphic,
            _ => Mutability::Mutable,
        })
    }

    /// May an entry with mutability `self` be used where one with `expected` is?
    /// Both must refer to the same place.
    pub(super) fn subtype(self, expected: Mutability, row_mode: RowMode) -> bool {
        match row_mode {
            RowMode::Exact => self == expected,
            RowMode::Coercible => match expected {
                Mutability::ReadOnly => true,
                Mutability::Mutable => self == Mutability::Mutable,
                Mutability::Polymorphic => self != Mutability::ReadOnly,
            },
        }
    }
}

/// The mutability of a flattened, followed places row entry
pub(crate) fn entry_mutability(entry: &Type) -> Mutability {
    match entry {
        Type::Place(_, access) => access.mutability(),
        Type::PlaceProjection(_, _, Some(mode)) => mode.mutability(),
        // Errors are mutable to avoid cascading diagnostics
        Type::Primitive(PrimitiveType::Error) => Mutability::Mutable,
        _ => Mutability::Polymorphic,
    }
}

/// The modifier written on an entry, if any, for printing
pub(crate) fn entry_mode(entry: &Type) -> Option<PlaceMode> {
    match entry {
        Type::Place(_, PlaceAccess::Mode(mode)) | Type::PlaceProjection(_, _, Some(mode)) => Some(*mode),
        _ => None,
    }
}

/// The modifier every entry besides open tails shares, if there are any and they all have one
fn uniform_mode(entries: &[Type]) -> Option<PlaceMode> {
    let mut entries = entries.iter().filter(|entry| !is_unbound_place(entry));
    let mode = entry_mode(entries.next()?)?;
    entries.all(|entry| entry_mode(entry) == Some(mode)).then_some(mode)
}

/// `entry` without any modifier, for rows where mutability is irrelevant like `Mutate 'p`
fn without_mode(entry: Type) -> Type {
    match entry {
        Type::Place(place, PlaceAccess::Mode(_)) => Type::Place(place, PlaceAccess::Plain { mutable: true }),
        Type::PlaceProjection(base, segments, Some(_)) if segments.is_empty() => base.as_ref().clone(),
        Type::PlaceProjection(base, segments, Some(_)) => Type::PlaceProjection(base, segments, None),
        other => other,
    }
}

/// True if `a` and `b` are the same place, ignoring how each is accessed
pub(super) fn same_place(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Place(a, _), Type::Place(b, _)) => a == b,
        (Type::PlaceProjection(a_base, a_segments, _), Type::PlaceProjection(b_base, b_segments, _)) => {
            a_base == b_base && a_segments == b_segments
        },
        (Type::PlaceProjection(base, segments, _), other) | (other, Type::PlaceProjection(base, segments, _)) => {
            segments.is_empty() && base.as_ref() == other
        },
        _ => a == b,
    }
}

/// Join two entries for the same place into one usable however either is
pub(super) fn join_entries(a: Type, b: Type) -> Type {
    let (a_mutability, b_mutability) = (entry_mutability(&a), entry_mutability(&b));
    if a_mutability == b_mutability {
        // Prefer the entry without a modifier for printing
        return if entry_mode(&b).is_none() { b } else { a };
    }
    match Mutability::join([a_mutability, b_mutability]) {
        Mutability::ReadOnly if a_mutability == Mutability::ReadOnly => a,
        Mutability::ReadOnly => b,
        // Mutable and polymorphic join to the polymorphic entry
        _ if a_mutability == Mutability::Polymorphic => a,
        _ => b,
    }
}

/// What a [Place] is rooted in
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlaceRoot {
    Variable(NameId),
    Anonymous(ExprId, ScopeDepth),
}

/// What any places row entry is rooted in, including generic and unbound places
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum EntryRoot {
    Concrete(PlaceRoot),
    Generic(Generic),
    Unbound(TypeVariableId),
}

/// A place a reference may point to, like `x.a!.b`. Concrete unless `R` is [EntryRoot].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Place<R = PlaceRoot> {
    pub root: R,
    pub segments: Vec<PlaceSegment>,
}

/// Any places row entry, which may be rooted in a generic or unbound place
pub type PlaceEntry = Place<EntryRoot>;

impl Place {
    pub fn variable(name: NameId) -> Place {
        Place { root: PlaceRoot::Variable(name), segments: Vec::new() }
    }

    pub fn anonymous(expr: ExprId, depth: ScopeDepth) -> Place {
        Place { root: PlaceRoot::Anonymous(expr, depth), segments: Vec::new() }
    }

    /// Return `self.field`
    pub fn field(mut self, field: cst::Name) -> Place {
        self.segments.push(PlaceSegment::Field(field));
        self
    }

    /// Return `self` followed by each of `segments`
    pub fn project(mut self, segments: &[PlaceSegment]) -> Place {
        self.segments.extend_from_slice(segments);
        self
    }

    /// `self` up to its first `!`
    pub fn stable_prefix(mut self) -> Place {
        if let Some(index) = self.segments.iter().position(|segment| *segment == PlaceSegment::Unstable) {
            self.segments.truncate(index);
        }
        self
    }

    /// Build a display name for error messages
    pub fn display_name(&self, names: &impl NameStore) -> String {
        let root = match self.root {
            PlaceRoot::Variable(name) => match names.try_get_name(name) {
                Some(name) => name.to_string(),
                // TODO: Is this reachable?
                None => "#name-not-in-context".to_string(),
            },
            PlaceRoot::Anonymous(expr, _) => format!("_{}", expr.index()),
        };
        format!("{root}{}", Segments(&self.segments))
    }
}

impl PlaceEntry {
    /// Split a flattened places row entry into its root and segments
    pub fn new(entry: &Type) -> Option<PlaceEntry> {
        let (root, segments) = match entry {
            Type::Place(place, _) => (EntryRoot::Concrete(place.root), place.segments.clone()),
            Type::PlaceProjection(base, segments, _) => match base.as_ref() {
                Type::Generic(generic) => (EntryRoot::Generic(*generic), segments.to_vec()),
                Type::Variable(id) => (EntryRoot::Unbound(*id), segments.to_vec()),
                _ => return None,
            },
            Type::Generic(generic) => (EntryRoot::Generic(*generic), Vec::new()),
            Type::Variable(id) => (EntryRoot::Unbound(*id), Vec::new()),
            _ => return None,
        };
        Some(Place { root, segments })
    }

    /// This entry as a [Place], if it is concrete
    pub fn concrete(self) -> Option<Place> {
        match self.root {
            EntryRoot::Concrete(root) => Some(Place { root, segments: self.segments }),
            _ => None,
        }
    }
}

impl From<Place> for PlaceEntry {
    fn from(place: Place) -> Self {
        Place { root: EntryRoot::Concrete(place.root), segments: place.segments }
    }
}

impl<R: PartialEq> Place<R> {
    /// Check if `self` is a descendant of `ancestor` but is not itself the ancestor.
    /// E.g. `x.a.b` is a descendant of `x.a` and `x`, but not of `x.a.b`.
    pub fn is_descendant_of(&self, ancestor: &Self) -> bool {
        self.root == ancestor.root
            && self.segments.len() > ancestor.segments.len()
            && self.segments.starts_with(&ancestor.segments)
    }

    /// True if `self` is a descendant of `place` reached only through fields, like `x.a.b` from `x`
    pub fn is_field_of(&self, place: &Self) -> bool {
        self.is_descendant_of(place) && !self.segments[place.segments.len()..].contains(&PlaceSegment::Unstable)
    }

    /// True if `self` is `place` or one of its descendants
    pub fn is_within(&self, place: &Self) -> bool {
        self == place || self.is_descendant_of(place)
    }

    /// True if `self` and `place` may refer to the same memory
    pub fn overlaps(&self, place: &Self) -> bool {
        self.is_within(place) || place.is_within(self)
    }

    /// True if mutating `mutated` may invalidate a reference to `self`
    pub fn invalidated_by(&self, mutated: &Self) -> bool {
        if self.root != mutated.root {
            return false;
        }
        let mut owned = mutated.segments.clone();
        owned.push(PlaceSegment::Unstable);
        may_be_within(&self.segments, &owned)
    }

    /// True if `self` contains a `!`
    pub fn is_unstable(&self) -> bool {
        self.segments.contains(&PlaceSegment::Unstable)
    }
}

/// True if a place with `segments` may be `region` or below it.
/// Since a place like `'p.foo!` coerces to `'p!`, this needs to be a bit more involved
/// than just checking the ancestors of two places all match. For example, mutating
/// `'p.foo` needs to invalidate `'p!`.
fn may_be_within(segments: &[PlaceSegment], region: &[PlaceSegment]) -> bool {
    let after_unstable = |segments: &[PlaceSegment]| {
        segments.iter().position(|segment| *segment == PlaceSegment::Unstable).map(|index| index + 1)
    };
    match (segments.split_first(), region.split_first()) {
        (_, None) => true,
        // An ancestor holds `region` inline only if no `!` leads to it
        (None, Some(_)) => !region.contains(&PlaceSegment::Unstable),
        (Some((segment, rest)), Some((region_segment, region_rest))) if segment == region_segment => {
            may_be_within(rest, region_rest)
        },
        (Some((PlaceSegment::Unstable, rest)), Some(_)) => match after_unstable(region) {
            Some(index) => may_be_within(rest, &region[index..]),
            None => true,
        },
        (Some(_), Some((PlaceSegment::Unstable, region_rest))) => match after_unstable(segments) {
            Some(index) => may_be_within(&segments[index..], region_rest),
            None => false,
        },
        (Some(_), Some(_)) => false,
    }
}

impl Type {
    /// Flatten every place reachable from `places` into `found`
    pub(crate) fn flatten_places_into(
        places: &[Type], found: &mut Vec<Type>, bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) {
        flatten_row_into(RowKind::Places, places, found, bindings, more_bindings);
    }

    /// Flatten, follow, sort, and deduplicate `places`.
    /// Deduplication is done via exact equality rather than unification.
    pub(crate) fn canonicalize_places(
        places: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) -> Vec<Type> {
        canonicalize_row(RowKind::Places, places, bindings, more_bindings)
    }

    /// Construct a canonicalized places row by following & deduplicating entries.
    pub(crate) fn places(list: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings) -> Type {
        construct_row(RowKind::Places, list, bindings, more_bindings)
    }

    /// `base` followed by `segments`, applied to each place `base` is bound to
    pub(crate) fn project_place(
        base: &Type, segments: &[PlaceSegment], bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) -> Type {
        Type::project_place_with_mode(base, segments, None, bindings, more_bindings)
    }

    /// Apply `mode` to every entry of the places row `places`
    pub(crate) fn apply_place_mode(
        places: &Type, mode: PlaceMode, bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) -> Type {
        Type::project_place_with_mode(places, &[], Some(mode), bindings, more_bindings)
    }

    /// `base` followed by `segments` and accessed through `mode` if any (which overrides any
    /// mode the base has), applied to each place `base` is bound to
    pub(crate) fn project_place_with_mode(
        base: &Type, segments: &[PlaceSegment], mode: Option<PlaceMode>, bindings: &TypeBindings,
        more_bindings: &TypeBindings,
    ) -> Type {
        if segments.is_empty() && mode.is_none() {
            return base.clone();
        }
        match base.follow_two(bindings, more_bindings) {
            Type::Place(place, access) => {
                let access = mode.map_or(access, PlaceAccess::Mode);
                Type::Place(place.project(segments), access)
            },
            Type::Places(Some(entries)) => {
                let entries = mapvec(entries.iter(), |entry| {
                    Type::project_place_with_mode(entry, segments, mode, bindings, more_bindings)
                });
                Type::places(&entries, bindings, more_bindings)
            },
            Type::PlaceProjection(inner, inner_segments, inner_mode) => {
                let segments = [inner_segments.as_slice(), segments].concat();
                Type::project_place_with_mode(&inner, &segments, mode.or(inner_mode), bindings, more_bindings)
            },
            base @ (Type::Variable(_) | Type::Generic(_)) => {
                Type::PlaceProjection(Shared::new(base), Shared::new(segments.to_vec()), mode)
            },
            // Any other type is a kind error reported elsewhere
            other => other,
        }
    }

    /// If this is a reference application, return its places argument
    pub fn reference_places(&self, bindings: &TypeBindings) -> Option<Type> {
        match self.follow(bindings) {
            Type::Application(constructor, args)
                if args.len() >= 2 && constructor.is_reference_constructor(bindings) =>
            {
                Some(args[0].clone())
            },
            _ => None,
        }
    }
}

impl<Db, Names> TypePrinter<'_, Db, Names>
where
    Db: DbGet<GetItem>,
    Names: NameStore,
{
    /// The places to print for `places`, or `None` if it is closed and empty
    pub(super) fn printed_places(&self, places: &Type) -> Option<Vec<Type>> {
        let mut entries = Type::canonicalize_places(std::slice::from_ref(places), self.bindings, &Default::default());
        if entries.is_empty() {
            return None;
        }
        entries.retain(is_printed_place);
        Some(entries)
    }

    /// The modifier every entry of `places` shares, if it has entries and they all have one
    pub(super) fn uniform_place_mode(&self, places: &Type) -> Option<PlaceMode> {
        let entries = Type::canonicalize_places(std::slice::from_ref(places), self.bindings, &Default::default());
        uniform_mode(&entries)
    }

    /// Print a place argument as ` 'p`, ` '()` if closed and empty, or ` _` if unbound and not `elidable`.
    /// If `strip_modes` is set, entries are printed without their modifiers.
    pub(super) fn fmt_place_arg(
        &self, places: &Type, elidable: bool, strip_modes: bool, f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        match self.printed_places(places) {
            None => write!(f, " '()"),
            Some(entries) if !entries.is_empty() => {
                write!(f, " '")?;
                self.fmt_place_entries(&entries, strip_modes, f)
            },
            Some(_) if elidable => Ok(()),
            Some(_) => write!(f, " _"),
        }
    }

    /// Print a place argument outside a reference, as ` (mut 'p)` if every entry is mutable
    pub(super) fn fmt_moded_place_arg(&self, places: &Type, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.in_effect.get() {
            return self.fmt_place_arg(places, false, true, f);
        }
        match self.uniform_place_mode(places) {
            Some(mode) => {
                write!(f, " ({mode}")?;
                self.fmt_place_arg(places, false, true, f)?;
                write!(f, ")")
            },
            None => self.fmt_place_arg(places, false, false, f),
        }
    }

    pub(super) fn fmt_place_entries(
        &self, entries: &[Type], strip_modes: bool, f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        if entries.len() == 1 {
            self.fmt_place_entry(&entries[0], strip_modes, f)
        } else {
            write!(f, "(")?;
            for (i, entry) in entries.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                self.fmt_place_entry(entry, strip_modes, f)?;
            }
            write!(f, ")")
        }
    }

    /// Print a single places row entry like `x.a!` or `mut p`
    pub(super) fn fmt_place_entry(
        &self, entry: &Type, strip_mode: bool, f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        if let Some(mode) = entry_mode(entry).filter(|_| !strip_mode) {
            write!(f, "{mode} ")?;
        }
        match entry {
            Type::Place(atom, _) => write!(f, "{}", atom.display_name(self.names)),
            Type::PlaceProjection(base, segments, _) => {
                self.fmt_type(base, false, f)?;
                write!(f, "{}", Segments(segments))
            },
            other => self.fmt_type(other, false, f),
        }
    }
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Flattens two places rows and matches `a`'s concrete places against `b`'s.
    /// Returns `Err` if a place both rows share is accessed incompatibly per `mutability`.
    fn match_places(
        &self, a: &Type, b: &Type, row_mode: RowMode, mutability: RowMode, new_bindings: &TypeBindings,
    ) -> Result<Option<RowMatch>, ()> {
        let mut a_list = self.collect_and_merge_places(a, new_bindings);
        let b_list = self.collect_and_merge_places(b, new_bindings);

        if a_list.iter().chain(b_list.iter()).any(|place| place.is_error()) {
            return Ok(None);
        }

        // A place is a subtype of any stable place containing it, like `'m.value <: 'm`
        if row_mode == RowMode::Coercible {
            let b_entries: Vec<_> = b_list.iter().filter_map(PlaceEntry::new).collect();
            a_list.retain(|a_place| {
                let a_place = PlaceEntry::new(a_place);
                !a_place.is_some_and(|a_place| b_entries.iter().any(|b_place| a_place.is_field_of(b_place)))
            });
        }

        let mut compatible = true;
        let m = Self::match_row_entries(a_list, b_list, |a_place, b_place| {
            let same = same_place(a_place, b_place);
            if same {
                compatible &= entry_mutability(a_place).subtype(entry_mutability(b_place), mutability);
            }
            same
        });
        if compatible { Ok(Some(m)) } else { Err(()) }
    }

    /// Row-subtype two places rows: is `a`'s actual set of places permitted by `b`'s expected set?
    fn place_subtype(
        &self, a: &Type, b: &Type, mutability: RowMode, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let Some(mut m) = self.match_places(a, b, RowMode::Coercible, mutability, new_bindings)? else { return Ok(()) };
        let sides = Self::end_sides(mutability);
        if m.b_open.is_empty() {
            self.absorb_into_projections(&mut m.b_leftover, &mut m.a_leftover, true, sides.1, new_bindings)?;
            self.bind_projections(&mut m.a_leftover, &m.b_leftover, sides.0, new_bindings)?;
        }
        self.row_subtype_generic(RowKind::Places, m, sides, new_bindings)
    }

    /// How the ends of an actual and expected places row are bound when comparing mutability per `mutability`
    fn end_sides(mutability: RowMode) -> (RowEnd, RowEnd) {
        match mutability {
            RowMode::Coercible => (RowEnd::Actual, RowEnd::Expected),
            RowMode::Exact => (RowEnd::Exact, RowEnd::Exact),
        }
    }

    /// Bind each open projection like `?t!` in `absorbers` to the `entries` ending in its segments.
    /// If `widen` is set, each absorber may still grow afterward.
    fn absorb_into_projections(
        &self, absorbers: &mut Vec<Type>, entries: &mut Vec<Type>, widen: bool, side: RowEnd,
        new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        for absorber in std::mem::take(absorbers) {
            let Some((end, segments)) = open_projection(&absorber) else {
                absorbers.push(absorber);
                continue;
            };

            let mut absorbed = Vec::new();
            entries.retain(|entry| match strip_segments(entry, segments) {
                Some(stripped) => {
                    absorbed.push(stripped);
                    false
                },
                None => true,
            });
            if absorbed.is_empty() {
                absorbers.push(absorber);
                continue;
            }

            if widen {
                absorbed.push(self.next_type_variable());
            }
            self.bind_places_end(&end, &absorbed, side, new_bindings)?;
        }
        Ok(())
    }

    /// Bind and remove each open projection in `entries` using the `candidates` ending in its segments
    fn bind_projections(
        &self, entries: &mut Vec<Type>, candidates: &[Type], side: RowEnd, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        for entry in std::mem::take(entries) {
            let Some((end, segments)) = open_projection(&entry) else {
                entries.push(entry);
                continue;
            };
            let stripped: Vec<_> =
                candidates.iter().filter_map(|candidate| strip_segments(candidate, segments)).collect();
            self.bind_places_end(&end, &stripped, side, new_bindings)?;
        }
        Ok(())
    }

    /// A places row with every place in `places` plus a fresh tail to widen into
    fn reopen_places(&self, places: &Type, new_bindings: &TypeBindings) -> Type {
        let mut entries = self.collect_and_merge_places(places, new_bindings);
        entries.push(self.next_type_variable());
        Type::places(&entries, &self.bindings, new_bindings)
    }

    /// Reopen each places row in a covariant position of `typ` so a variable bound to it
    /// widens without widening `typ`. Returns `None` if there are no such rows.
    pub(super) fn reopen_covariant_places(&self, typ: &Type, new_bindings: &TypeBindings) -> Option<Type> {
        match typ {
            Type::Places(_) => Some(self.reopen_places(typ, new_bindings)),
            Type::Application(constructor, args) => {
                let constructor_type = constructor.follow_two(&self.bindings, new_bindings);
                if !constructor_type.is_reference_constructor(&self.bindings) {
                    return None;
                }
                let mut args = args.as_ref().clone();
                args[0] = self.reopen_places(&args[0], new_bindings);

                // A possibly mutable element must stay shared so stores through it widen the original
                if self.places_mutability(&args[0], new_bindings) == Mutability::ReadOnly {
                    let element = args[1].follow_two(&self.bindings, new_bindings);
                    if let Some(element) = self.reopen_covariant_places(&element, new_bindings) {
                        args[1] = element;
                    }
                }
                Some(Type::Application(constructor.clone(), Shared::new(args)))
            },
            _ => None,
        }
    }

    /// A supertype of a branch's type for later branches to widen without widening that branch
    pub(super) fn branch_join_type(&self, typ: Type) -> Type {
        let followed = typ.follow(&self.bindings);
        self.reopen_covariant_places(followed, &TypeBindings::default()).unwrap_or(typ)
    }

    /// Subtype or unify two places rows depending on `row_mode`. Mutability is compared
    /// by subtyping unless `variance` is invariant.
    pub(super) fn relate_places(
        &self, a: &Type, b: &Type, row_mode: RowMode, variance: Variance, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let mutability = if variance == Variance::Invariant { RowMode::Exact } else { RowMode::Coercible };
        match row_mode {
            RowMode::Coercible => self.place_subtype(a, b, mutability, new_bindings),
            RowMode::Exact => self.place_unify(a, b, mutability, new_bindings),
        }
    }

    /// Unify two places rows: both must end up with the same set of places.
    fn place_unify(&self, a: &Type, b: &Type, mutability: RowMode, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(mut m) = self.match_places(a, b, RowMode::Exact, mutability, new_bindings)? else { return Ok(()) };
        let sides = Self::end_sides(mutability);
        if m.b_open.is_empty() {
            self.absorb_into_projections(&mut m.b_leftover, &mut m.a_leftover, false, sides.1, new_bindings)?;
        }
        if m.a_open.is_empty() {
            self.absorb_into_projections(&mut m.a_leftover, &mut m.b_leftover, false, sides.0, new_bindings)?;
        }
        if m.b_open.is_empty() {
            self.bind_projections(&mut m.a_leftover, &[], sides.0, new_bindings)?;
        }
        if m.a_open.is_empty() {
            self.bind_projections(&mut m.b_leftover, &[], sides.1, new_bindings)?;
        }
        self.row_unify_generic(RowKind::Places, m, sides, new_bindings)
    }

    /// True if the unbound `variable` is itself an entry of `places`
    pub(super) fn places_entry_is(&self, places: &Type, variable: TypeVariableId, new_bindings: &TypeBindings) -> bool {
        let mut entries = Vec::new();
        Type::flatten_places_into(std::slice::from_ref(places), &mut entries, &self.bindings, new_bindings);
        entries.contains(&Type::Variable(variable))
    }

    /// Flatten, follow, sort, and deduplicate the places in `places`
    fn collect_and_merge_places(&self, places: &Type, new_bindings: &TypeBindings) -> Vec<Type> {
        match places.follow_two(&self.bindings, new_bindings) {
            typ @ (Type::Places(_)
            | Type::PlaceProjection(..)
            | Type::Place(..)
            | Type::Generic(_)
            | Type::Variable(_)) => Type::canonicalize_places(std::slice::from_ref(&typ), &self.bindings, new_bindings),
            // Any remaining variant should be a kind error emitted elsewhere
            _ => Vec::new(),
        }
    }

    /// `Mutate places`
    pub(super) fn mutate_effect(&self, places: Type) -> Type {
        // Whether a mutated place is mutable was checked already
        let none = TypeBindings::default();
        let entries = mapvec(Type::canonicalize_places(&[places], &self.bindings, &none), without_mode);
        let places = Type::places(&entries, &self.bindings, &none);
        let mutate = Type::UserDefined(Origin::TopLevelDefinition(self.mutate_name));
        Type::Application(Shared::new(mutate), Shared::new(vec![places]))
    }

    /// If `effect` is `Mutate 'p`, return `'p`
    pub(super) fn mutated_places(&self, effect: &Type) -> Option<Type> {
        let Type::Application(constructor, args) = effect.follow(&self.bindings) else { return None };
        match constructor.follow(&self.bindings) {
            Type::UserDefined(Origin::TopLevelDefinition(name)) if *name == self.mutate_name => args.first().cloned(),
            _ => None,
        }
    }

    /// Every place mutated by a `Mutate` effect in the effect row `effects`
    pub(super) fn mutated_entries(&self, effects: &Type) -> Vec<PlaceEntry> {
        let effects = self.collect_effects(effects, &TypeBindings::default());
        let mutated = effects.iter().filter_map(|effect| self.mutated_places(effect));
        mutated.flat_map(|places| self.place_entries(&places)).collect()
    }

    /// Split `effects` into the row that applies now and the entries only this function's boundary can decide
    pub(super) fn split_deferred_effects(&self, effects: &Type) -> (Type, Vec<Type>) {
        let none = TypeBindings::default();
        let mut now = Vec::new();
        let mut deferred = Vec::new();
        for entry in self.collect_effects(effects, &none) {
            match (self.mutated_places(&entry), entry) {
                (Some(places), _) => {
                    // A generic place is named by a signature, so it is never local
                    let places = self.collect_and_merge_places(&places, &none);
                    let (generic, local) = places.into_iter().partition(|place| {
                        PlaceEntry::new(place).is_some_and(|place| matches!(place.root, EntryRoot::Generic(_)))
                    });
                    self.push_mutate(&mut now, self.unmutated_places(generic, &self.current_effect_row));
                    self.push_mutate(&mut deferred, local);
                },
                (None, entry @ Type::Variable(_)) => deferred.push(entry),
                (None, entry) => now.push(entry),
            }
        }
        (Type::effects(&now, &self.bindings, &none), deferred)
    }

    /// Push `Mutate places` onto `effects` unless `places` is empty
    fn push_mutate(&self, effects: &mut Vec<Type>, places: Vec<Type>) {
        if !places.is_empty() {
            effects.push(self.mutate_effect(Type::places(&places, &self.bindings, &TypeBindings::default())));
        }
    }

    /// Each of `places` not within a place `effects` already mutates
    fn unmutated_places(&self, mut places: Vec<Type>, effects: &Type) -> Vec<Type> {
        if places.is_empty() {
            return places;
        }
        let mutated = self.mutated_entries(effects);
        places.retain(|place| !PlaceEntry::new(place).is_some_and(|place| mutated.iter().any(|m| place.is_within(m))));
        places
    }

    /// Apply a body's deferred effects to its function's row, handling `Mutate`s of places it doesn't mention
    pub(super) fn discharge_effects(
        &mut self, deferred: Vec<Type>, function: &FunctionType, lambda: ExprId, implicits_delayed: bool,
    ) {
        if deferred.is_empty() {
            return;
        }
        let none = TypeBindings::default();
        let row = self.collect_effects(&function.effects, &none);
        let closed = !row.iter().any(is_open);

        let mut mentioned = None;
        let mut kept = Vec::new();
        for entry in Type::canonicalize_effects(&deferred, &self.bindings, &none) {
            match (self.mutated_places(&entry), entry) {
                (Some(places), _) => {
                    let mentioned = mentioned.get_or_insert_with(|| self.mentioned_places(function, lambda));
                    let mut places = self.collect_and_merge_places(&places, &none);

                    // A closed row can't name an inferred place, nor can an impl name its method's places
                    let is_impl_method = self.item.impl_.as_ref().is_some_and(|impl_| impl_.methods.contains(&lambda));
                    places.retain(|place| {
                        PlaceEntry::new(place).is_some_and(|place| {
                            let unnameable = match place.root {
                                EntryRoot::Unbound(_) => true,
                                EntryRoot::Generic(Generic::Elided(_)) => is_impl_method,
                                _ => false,
                            };
                            mentioned.iter().any(|m| place.overlaps(m)) && !(closed && unnameable)
                        })
                    });
                    self.push_mutate(&mut kept, self.unmutated_places(places, &function.effects));
                },
                // Against a closed row, an open end nothing bound performs nothing
                (None, Type::Variable(id)) if closed && !implicits_delayed => {
                    let location = self.current_context().expr_location(lambda).clone();
                    self.try_default_effect_to_pure(id, location);
                },
                (None, entry) => kept.push(entry),
            }
        }
        let kept = Type::effects(&kept, &self.bindings, &none);
        self.unify(&kept, &function.effects, TypeErrorKind::Effects, lambda);
    }

    /// Every place a function's type mentions, including those of each variable it captures
    fn mentioned_places(&self, function: &FunctionType, lambda: ExprId) -> Vec<PlaceEntry> {
        let mut captures = self.lambda_free_vars_context(lambda);
        captures.find_free_variables(lambda, self);

        let mut entries = Vec::new();
        for parameter in &function.parameters {
            self.collect_place_entries(&parameter.typ, &mut entries);
        }
        self.collect_place_entries(&function.return_type, &mut entries);
        for name in &captures.free_vars {
            entries.push(self.binding_place(*name).into());
            if let Some(typ) = self.name_types.get(name) {
                self.collect_place_entries(typ, &mut entries);
            }
        }
        entries
    }

    /// Recursively collect every places row entry reachable from `typ`
    pub(super) fn collect_place_entries(&self, typ: &Type, out: &mut Vec<PlaceEntry>) {
        match typ.follow(&self.bindings) {
            // A reference's place may also be a lone generic or variable
            Type::Application(constructor, args)
                if args.len() == 2 && constructor.is_reference_constructor(&self.bindings) =>
            {
                out.extend(self.place_entries(&args[0]));
                self.collect_place_entries(&args[1], out);
            },
            Type::Application(constructor, args) => {
                self.collect_place_entries(constructor, out);
                for arg in args.iter() {
                    self.collect_place_entries(arg, out);
                }
            },
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    self.collect_place_entries(element, out);
                }
            },
            Type::Places(_) | Type::Place(..) | Type::PlaceProjection(..) => out.extend(self.place_entries(typ)),
            Type::Forall(_, typ) => self.collect_place_entries(typ, out),
            // Mutating a place doesn't need it to still be alive
            Type::Effects(effects) => {
                if let Some(effects) = effects.as_ref() {
                    for effect in effects.iter() {
                        if self.mutated_places(effect).is_none() {
                            self.collect_place_entries(effect, out);
                        }
                    }
                }
            },
            // A function holds its environment's places plus whatever its result may refer to
            Type::Function(function) => {
                self.collect_place_entries(&function.environment, out);
                self.collect_place_entries(&function.effects, out);

                // TODO: This keeps tests working but seems like a hack, investigate further
                let mut supplied = Vec::new();
                for parameter in &function.parameters {
                    self.collect_place_entries(&parameter.typ, &mut supplied);
                }
                let mut returned = Vec::new();
                self.collect_place_entries(&function.return_type, &mut returned);
                out.extend(returned.into_iter().filter(|entry| !supplied.contains(entry)));
            },
            Type::Primitive(_) | Type::Generic(_) | Type::Variable(_) | Type::UserDefined(_) | Type::U32(_) => (),
        }
    }

    /// Every entry of the places row `places`
    pub(super) fn place_entries(&self, places: &Type) -> Vec<PlaceEntry> {
        let mut flattened = Vec::new();
        Type::flatten_places_into(
            std::slice::from_ref(places),
            &mut flattened,
            &self.bindings,
            &TypeBindings::default(),
        );
        flattened.iter().filter_map(PlaceEntry::new).collect()
    }

    /// Returns the set of places a reference to the given expression may refer to.
    /// This is generally either a single variable or an anonymous place.
    pub(super) fn infer_place(&self, expr: ExprId) -> Type {
        match &self.current_extended_context()[expr] {
            Expr::Variable(path) => match self.path_origin(*path) {
                Some(Origin::Local(name)) => self.open_place(self.binding_place(name)),
                // TODO: Track places for globals
                _ => self.next_type_variable(),
            },
            Expr::MemberAccess(access) => {
                let object = access.object;
                match self.expr_types.get(&object) {
                    // Object is already a reference
                    Some(t) if t.reference_element(&self.bindings).is_some() => {
                        match t.reference_places(&self.bindings) {
                            Some(places) => {
                                let field = [PlaceSegment::Field(self.member_name(access).clone())];
                                Type::project_place(&places, &field, &self.bindings, &TypeBindings::default())
                            },
                            None => self.next_type_variable(),
                        }
                    },
                    _ => match self.try_build_move_path(expr) {
                        Some(path) => self.open_place(path),
                        None => self.infer_place(object),
                    },
                }
            },
            Expr::TypeAnnotation(annotation) => self.infer_place(annotation.lhs),
            // A diverging expr may be coerced to any place
            _ if self.expr_types.get(&expr).is_some_and(|t| self.diverges(t)) => self.next_type_variable(),
            // Otherwise we have an anonymous local like `ref my_call ()`
            _ => self.open_place(Place::anonymous(expr, self.current_scope_depth())),
        }
    }

    /// Split `typ` into the type a pattern matches and the reference it matches through, if any
    pub(super) fn match_through_reference(&self, typ: &Type) -> (Type, Option<MatchedReference>) {
        if let Some((places, element)) = typ.reference_element(&self.bindings) {
            return (element, Some(MatchedReference { places }));
        }
        (typ.clone(), None)
    }

    /// The type of `field` reached through `reference`, which is unstable inside a union's payload
    pub(super) fn field_through_reference(&self, reference: &MatchedReference, field: Type, unstable: bool) -> Type {
        let places = if unstable {
            Type::project_place(&reference.places, &[PlaceSegment::Unstable], &self.bindings, &TypeBindings::default())
        } else {
            reference.places.clone()
        };
        Type::reference(places, field)
    }

    /// The types `fields` of a `matched` constructor are bound at, reached through `reference` if any
    pub(super) fn fields_through_reference(
        &self, matched: &Type, reference: Option<&MatchedReference>, fields: Vec<Type>,
    ) -> Vec<Type> {
        let Some(reference) = reference else { return fields };
        let payload_count = match self.union_common_field_count(matched) {
            Some(common) => fields.len().saturating_sub(common),
            None => 0,
        };
        let fields = fields.into_iter().enumerate();
        mapvec(fields, |(i, field)| self.field_through_reference(reference, field, i < payload_count))
    }

    /// If `typ` is a union of more than one variant, return how many `with` fields it has
    pub(super) fn union_common_field_count(&self, typ: &Type) -> Option<usize> {
        let name = match typ.follow(&self.bindings) {
            Type::Application(constructor, _) => match constructor.follow(&self.bindings) {
                Type::UserDefined(Origin::TopLevelDefinition(name)) => *name,
                _ => return None,
            },
            Type::UserDefined(Origin::TopLevelDefinition(name)) => *name,
            _ => return None,
        };
        let RawItem(item, _) = GetItemRaw(name.top_level_item).get(self.compiler);
        match &item.kind {
            TopLevelItemKind::TypeDefinition(definition) if definition.name == name.local_name_id => {
                match &definition.body {
                    TypeDefinitionBody::Enum(variants, common) if variants.len() > 1 => Some(common.len()),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    /// A places row containing just `atom` plus a fresh open tail
    pub(crate) fn open_place(&self, atom: Place) -> Type {
        let fresh = self.next_type_variable();
        let access = PlaceAccess::Plain { mutable: self.place_root_is_mutable(&atom) };
        Type::places(&[Type::Place(atom, access), fresh], &self.bindings, &TypeBindings::default())
    }

    /// True if `place` is rooted in a `var`
    fn place_root_is_mutable(&self, place: &Place) -> bool {
        match place.root {
            PlaceRoot::Variable(name) => self.item.mutable_definitions.contains(&name),
            PlaceRoot::Anonymous(..) => false,
        }
    }

    /// `kind places element`, e.g. `ref (mut places) element` for `mut`
    pub(super) fn reference_type(&self, kind: ReferenceKind, places: Type, element: Type) -> Type {
        let places = match PlaceMode::of_reference(kind) {
            Some(mode) => Type::apply_place_mode(&places, mode, &self.bindings, &TypeBindings::default()),
            None => places,
        };
        Type::reference(places, element)
    }

    /// What a reference through `places` may do: mutate only if every entry is mutable.
    /// Unbound places are ignored since they may still be constrained, see [Self::constrain_mutable].
    pub(super) fn places_mutability(&self, places: &Type, new_bindings: &TypeBindings) -> Mutability {
        let entries = self.collect_and_merge_places(places, new_bindings);
        Mutability::join(entries.iter().filter(|entry| !is_unbound_place(entry)).map(entry_mutability))
    }

    /// The modifier every entry of `places` shares, if it has entries and they all have one
    pub(super) fn uniform_mode(&self, places: &Type) -> Option<PlaceMode> {
        uniform_mode(&self.collect_and_merge_places(places, &TypeBindings::default()))
    }

    /// Constrain each unbound place in `places` to be mutable since a reference through it was mutated
    pub(super) fn constrain_mutable(&mut self, places: &Type) {
        let mut new_bindings = TypeBindings::default();
        for entry in self.collect_and_merge_places(places, &new_bindings) {
            if is_unbound_place(&entry) {
                // Can only fail if the variable is already bound, which `entry` isn't
                let _ = self.constrain_place_variable(&entry, PlaceMode::Mut, &mut new_bindings);
            }
        }
        self.bindings.extend(new_bindings);
    }

    /// Bind the unbound variable at the root of `entry` so every place it later grows to has `mode`
    fn constrain_place_variable(
        &self, entry: &Type, mode: PlaceMode, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let base = match entry {
            Type::PlaceProjection(base, ..) => base.as_ref(),
            other => other,
        };
        let Type::Variable(id) = base.follow_two(&self.bindings, new_bindings) else { return Err(()) };
        let fresh = self.next_type_variable();
        let constrained = Type::PlaceProjection(Shared::new(fresh), Shared::new(Vec::new()), Some(mode));
        new_bindings.insert(id, Type::Places(Some(Shared::new(vec![constrained]))));
        Ok(())
    }

    /// Bind the open end `end` of a places row to `entries`. If `end` has a mode, each entry must be
    /// usable with it: for an `Expected` end the entries are actual places, otherwise they're expected.
    pub(super) fn bind_places_end(
        &self, end: &Type, entries: &[Type], side: RowEnd, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        let (base, mode) = match end {
            Type::PlaceProjection(base, segments, Some(mode)) if segments.is_empty() => (base.as_ref(), *mode),
            _ => return self.bind_row_end(RowKind::Places, end, entries, new_bindings),
        };

        let mut flattened = Vec::new();
        Type::flatten_places_into(entries, &mut flattened, &self.bindings, new_bindings);
        for entry in &flattened {
            if is_unbound_place(entry) {
                // Anything may be used as read-only, and a mutable place may be used as anything
                let unconstrained =
                    matches!((side, mode), (RowEnd::Expected, PlaceMode::Read) | (RowEnd::Actual, PlaceMode::Mut));
                if !unconstrained {
                    self.constrain_place_variable(entry, mode, new_bindings)?;
                }
                continue;
            }
            let (actual, expected) = match side {
                RowEnd::Expected => (entry_mutability(entry), mode.mutability()),
                RowEnd::Actual => (mode.mutability(), entry_mutability(entry)),
                RowEnd::Exact => {
                    if entry_mutability(entry) != mode.mutability() {
                        return Err(());
                    }
                    continue;
                },
            };
            if !actual.subtype(expected, RowMode::Coercible) {
                return Err(());
            }
        }
        self.bind_row_end(RowKind::Places, base, &flattened, new_bindings)
    }

    /// Walk a field type and emit a `MissingExplicitPlace` diagnostic for
    /// every `ImplicitPlace` placeholder.
    pub(super) fn reject_implicit_places(typ: &cst::Type, db: &DbHandle) {
        match &typ.kind {
            cst::TypeKind::ImplicitPlace { .. } => {
                db.accumulate(Diagnostic::MissingExplicitPlace { location: typ.location.clone() });
            },
            cst::TypeKind::Application(f, args) => {
                Self::reject_implicit_places(f, db);
                for arg in args {
                    Self::reject_implicit_places(arg, db);
                }
            },
            cst::TypeKind::Function(function) => {
                for parameter in &function.parameters {
                    Self::reject_implicit_places(&parameter.typ, db);
                }
                if let Some(env) = function.environment.as_ref() {
                    Self::reject_implicit_places(env, db);
                }
                Self::reject_implicit_places(&function.return_type, db);
            },
            cst::TypeKind::Tuple(elements) | cst::TypeKind::EffectUnion(elements) => {
                for element in elements {
                    Self::reject_implicit_places(element, db);
                }
            },
            cst::TypeKind::Forall(_, body) => Self::reject_implicit_places(body, db),
            cst::TypeKind::Error
            | cst::TypeKind::Named(_)
            | cst::TypeKind::Variable(_)
            | cst::TypeKind::Integer(_)
            | cst::TypeKind::Float(_)
            | cst::TypeKind::Char
            | cst::TypeKind::Reference(_)
            | cst::TypeKind::NoClosureEnv
            | cst::TypeKind::Hole
            | cst::TypeKind::RigidHole
            | cst::TypeKind::Unit
            | cst::TypeKind::Place(_)
            | cst::TypeKind::PlaceMode(..)
            | cst::TypeKind::Pure
            | cst::TypeKind::IntegerConstant(_) => (),
        }
    }
}

/// A reference a pattern matches through, which each field it binds is reached through
pub(super) struct MatchedReference {
    places: Type,
}

/// True if `entry` is shown when printing places
fn is_printed_place(entry: &Type) -> bool {
    match entry {
        Type::Place(..) | Type::Generic(_) => true,
        Type::PlaceProjection(base, ..) => matches!(base.as_ref(), Type::Generic(_)),
        _ => false,
    }
}

/// True if `entry` is an unbound variable, possibly followed by segments, without a mode
fn is_unbound_place(entry: &Type) -> bool {
    match entry {
        Type::Variable(_) => true,
        Type::PlaceProjection(base, _, None) => matches!(base.as_ref(), Type::Variable(_)),
        _ => false,
    }
}

/// If `entry` is an unbound variable followed by segments like `?t!`, return it as an end to bind:
/// the variable itself, or a projection of it with no segments if it has a mode.
fn open_projection(entry: &Type) -> Option<(Type, &[PlaceSegment])> {
    match entry {
        Type::PlaceProjection(base, segments, mode) if matches!(base.as_ref(), Type::Variable(_)) => {
            let end = match mode {
                Some(_) => Type::PlaceProjection(base.clone(), Shared::new(Vec::new()), *mode),
                None => base.as_ref().clone(),
            };
            Some((end, segments))
        },
        _ => None,
    }
}

/// `entry` without `suffix` at the end of its path, if it ends in `suffix`
fn strip_segments(entry: &Type, suffix: &[PlaceSegment]) -> Option<Type> {
    match entry {
        Type::Place(place, access) => {
            let segments = place.segments.strip_suffix(suffix)?.to_vec();
            Some(Type::Place(Place { root: place.root, segments }, *access))
        },
        Type::PlaceProjection(base, segments, mode) => {
            let none = TypeBindings::default();
            Some(Type::project_place_with_mode(base, segments.strip_suffix(suffix)?, *mode, &none, &none))
        },
        _ if suffix.is_empty() => Some(entry.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{Place, PlaceSegment, ScopeDepth, mapvec};
    use crate::parser::ids::{ExprId, NameId};

    /// Parse `path` like `x.a!.b`, where `x` and `y` are variables and `_` is a temporary
    fn place(path: &str) -> Place {
        let (root, rest) = path.split_at(1);
        let mut place = match root {
            "x" => Place::variable(NameId::new(0)),
            "y" => Place::variable(NameId::new(1)),
            "_" => Place::anonymous(ExprId::new(0), ScopeDepth(0)),
            _ => unreachable!("unknown root in {path}"),
        };
        place.segments = mapvec(rest.replace('!', ".!").split('.').skip(1), |segment| match segment {
            "!" => PlaceSegment::Unstable,
            field => PlaceSegment::Field(Arc::new(field.to_string())),
        });
        place
    }

    fn invalidates(mutated: &str, reference: &str) -> bool {
        place(reference).invalidated_by(&place(mutated))
    }

    #[test]
    fn mutating_a_parent_invalidates_unstable_children() {
        assert!(invalidates("x", "x!"));
        assert!(invalidates("x!", "x!!"));
        assert!(invalidates("x", "x.bar!.baz"));
        assert!(invalidates("x.bar", "x.bar!.baz"));
        assert!(invalidates("_", "_!"));
    }

    #[test]
    fn stable_references_survive_mutation() {
        assert!(!invalidates("x", "x"));
        assert!(!invalidates("x!", "x!"));
        assert!(!invalidates("x", "x.a"));
        assert!(!invalidates("x.bar!", "x.bar!.baz"));
        assert!(!invalidates("x.a", "x"));
        assert!(!invalidates("x!", "x"));
    }

    #[test]
    fn sibling_paths() {
        assert!(invalidates("x.data", "x!"));
        assert!(!invalidates("x!", "x.data"));
        assert!(!invalidates("x.a", "x.b"));
        assert!(!invalidates("x.b", "x.a!"));
        assert!(invalidates("x.a.b", "x.a!"));
    }

    #[test]
    fn summary_places() {
        // `x!` may be an element of `x.a`, so replacing it drops what that element owns
        assert!(invalidates("x!", "x.a!!"));
        assert!(invalidates("x!.b", "x.a!.b!"));
        assert!(!invalidates("x!.b", "x.a!.c!"));
        assert!(!invalidates("x!", "x.a!"));
    }

    #[test]
    fn mutating_an_element() {
        // Replacing an element drops only what it owns
        assert!(invalidates("x.a!", "x.a!!"));
        assert!(invalidates("x.a!", "x!!"));
        assert!(!invalidates("x.a!", "x.a!"));
        assert!(!invalidates("x.a!", "x!"));
        assert!(!invalidates("x.a!", "x.b!!"));
        assert!(!invalidates("x.a!", "x.a"));
    }

    #[test]
    fn fields_of_a_place() {
        assert!(place("x.a").is_field_of(&place("x")));
        assert!(place("x!.a.b").is_field_of(&place("x!")));
        assert!(!place("x!").is_field_of(&place("x")));
        assert!(!place("x.a!").is_field_of(&place("x")));
        assert!(!place("x").is_field_of(&place("x")));
        assert!(!place("y.a").is_field_of(&place("x")));
    }

    #[test]
    fn different_roots() {
        assert!(!invalidates("x", "y!"));
        assert!(!invalidates("_", "x!"));
    }
}
