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
        TypeChecker,
        row::{RowKind, RowMatch, RowMode, canonicalize_row, construct_row, flatten_row_into, is_open},
    },
};

use super::{
    TypeErrorKind,
    generics::Generic,
    types::{FunctionType, Type, TypeBindings, TypePrinter, TypeVariableId},
};

/// Anonymous references record the scope depth they're valid in for escape analysis
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ScopeDepth(pub(crate) u32);

impl ScopeDepth {
    pub fn deeper(self) -> Self {
        Self(self.0 + 1)
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
            Type::Place(place) => (EntryRoot::Concrete(place.root), place.segments.clone()),
            Type::PlaceProjection(base, segments) => match base.as_ref() {
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
        if segments.is_empty() {
            return base.clone();
        }
        match base.follow_two(bindings, more_bindings) {
            Type::Place(place) => Type::Place(place.project(segments)),
            Type::Places(Some(entries)) => {
                let entries =
                    mapvec(entries.iter(), |entry| Type::project_place(entry, segments, bindings, more_bindings));
                Type::places(&entries, bindings, more_bindings)
            },
            Type::PlaceProjection(inner, inner_segments) => {
                let segments = [inner_segments.as_slice(), segments].concat();
                Type::project_place(&inner, &segments, bindings, more_bindings)
            },
            base @ (Type::Variable(_) | Type::Generic(_)) => {
                Type::PlaceProjection(Shared::new(base), Shared::new(segments.to_vec()))
            },
            // Any other type is a kind error reported elsewhere
            other => other,
        }
    }

    /// If this is a reference application, return its places argument
    pub fn reference_places(&self, bindings: &TypeBindings) -> Option<Type> {
        match self.follow(bindings) {
            Type::Application(constructor, args) if args.len() >= 2 => {
                constructor.reference_constructor(bindings).map(|_| args[0].clone())
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

    /// Print a place argument as ` 'p`, ` '()` if closed and empty, or ` _` if unbound and not `elidable`
    pub(super) fn fmt_place_arg(&self, places: &Type, elidable: bool, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.printed_places(places) {
            None => write!(f, " '()"),
            Some(entries) if !entries.is_empty() => {
                write!(f, " '")?;
                self.fmt_place_entries(&entries, f)
            },
            Some(_) if elidable => Ok(()),
            Some(_) => write!(f, " _"),
        }
    }

    pub(super) fn fmt_place_entries(&self, entries: &[Type], f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if entries.len() == 1 {
            self.fmt_type(&entries[0], false, f)
        } else {
            write!(f, "(")?;
            for (i, entry) in entries.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                self.fmt_type(entry, false, f)?;
            }
            write!(f, ")")
        }
    }
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Flattens two places rows and matches `a`'s concrete places against `b`'s
    fn match_places(&self, a: &Type, b: &Type, row_mode: RowMode, new_bindings: &TypeBindings) -> Option<RowMatch> {
        let mut a_list = self.collect_and_merge_places(a, new_bindings);
        let b_list = self.collect_and_merge_places(b, new_bindings);

        if a_list.iter().chain(b_list.iter()).any(|place| place.is_error()) {
            return None;
        }

        // A place is a subtype of any stable place containing it, like `'m.value <: 'm`
        if row_mode == RowMode::Coercible {
            let b_entries: Vec<_> = b_list.iter().filter_map(PlaceEntry::new).collect();
            a_list.retain(|a_place| {
                let a_place = PlaceEntry::new(a_place);
                !a_place.is_some_and(|a_place| b_entries.iter().any(|b_place| a_place.is_field_of(b_place)))
            });
        }

        Some(Self::match_row_entries(a_list, b_list, |a_place, b_place| a_place == b_place))
    }

    /// Row-subtype two places rows: is `a`'s actual set of places permitted by `b`'s expected set?
    pub(super) fn place_subtype(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(mut m) = self.match_places(a, b, RowMode::Coercible, new_bindings) else { return Ok(()) };
        if m.b_open.is_empty() {
            self.absorb_into_projections(&mut m.b_leftover, &mut m.a_leftover, RowMode::Coercible, new_bindings)?;
            self.bind_projections(&mut m.a_leftover, &m.b_leftover, new_bindings)?;
        }
        self.row_subtype_generic(RowKind::Places, m, new_bindings)
    }

    /// Bind each open projection like `?t!` in `absorbers` to the `entries` ending in its segments
    fn absorb_into_projections(
        &self, absorbers: &mut Vec<Type>, entries: &mut Vec<Type>, row_mode: RowMode, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        for absorber in std::mem::take(absorbers) {
            let Some((base, segments)) = open_projection(&absorber) else {
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

            if row_mode == RowMode::Coercible {
                absorbed.push(self.next_type_variable());
            }
            self.bind_row_end(RowKind::Places, base, &absorbed, new_bindings)?;
        }
        Ok(())
    }

    /// Bind and remove each open projection in `entries` using the `candidates` ending in its segments
    fn bind_projections(
        &self, entries: &mut Vec<Type>, candidates: &[Type], new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        for entry in std::mem::take(entries) {
            let Some((base, segments)) = open_projection(&entry) else {
                entries.push(entry);
                continue;
            };
            let stripped: Vec<_> =
                candidates.iter().filter_map(|candidate| strip_segments(candidate, segments)).collect();
            self.bind_row_end(RowKind::Places, base, &stripped, new_bindings)?;
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
                let kind = constructor_type.reference_constructor(&self.bindings)?;
                let mut args = args.as_ref().clone();
                args[0] = self.reopen_places(&args[0], new_bindings);

                // A `mut` element must stay shared so stores through it widen the original
                if matches!(kind, ReferenceKind::Ref | ReferenceKind::Imm) {
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

    /// Subtype or unify two places rows depending on `row_mode`
    pub(super) fn relate_places(
        &self, a: &Type, b: &Type, row_mode: RowMode, new_bindings: &mut TypeBindings,
    ) -> Result<(), ()> {
        match row_mode {
            RowMode::Coercible => self.place_subtype(a, b, new_bindings),
            RowMode::Exact => self.place_unify(a, b, new_bindings),
        }
    }

    /// Unify two places rows: both must end up with the same set of places.
    pub(super) fn place_unify(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(mut m) = self.match_places(a, b, RowMode::Exact, new_bindings) else { return Ok(()) };
        if m.b_open.is_empty() {
            self.absorb_into_projections(&mut m.b_leftover, &mut m.a_leftover, RowMode::Exact, new_bindings)?;
        }
        if m.a_open.is_empty() {
            self.absorb_into_projections(&mut m.a_leftover, &mut m.b_leftover, RowMode::Exact, new_bindings)?;
        }
        if m.b_open.is_empty() {
            self.bind_projections(&mut m.a_leftover, &[], new_bindings)?;
        }
        if m.a_open.is_empty() {
            self.bind_projections(&mut m.b_leftover, &[], new_bindings)?;
        }
        self.row_unify_generic(RowKind::Places, m, new_bindings)
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
            | Type::Place(_)
            | Type::Generic(_)
            | Type::Variable(_)) => Type::canonicalize_places(std::slice::from_ref(&typ), &self.bindings, new_bindings),
            // Any remaining variant should be a kind error emitted elsewhere
            _ => Vec::new(),
        }
    }

    /// `Mutate places`
    pub(super) fn mutate_effect(&self, places: Type) -> Type {
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
                    // A closed row can't name an inferred place, as in a trait method's parameter
                    places.retain(|place| {
                        PlaceEntry::new(place).is_some_and(|place| {
                            mentioned.iter().any(|m| place.overlaps(m))
                                && !(closed && matches!(place.root, EntryRoot::Unbound(_)))
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
                if args.len() == 2 && constructor.reference_constructor(&self.bindings).is_some() =>
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
            Type::Places(_) | Type::Place(_) | Type::PlaceProjection(..) => out.extend(self.place_entries(typ)),
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
        if let Type::Application(constructor, args) = typ.follow(&self.bindings)
            && let Some(kind) = constructor.reference_constructor(&self.bindings)
        {
            return (args[1].clone(), Some(MatchedReference { kind, places: args[0].clone() }));
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
        Type::Application(Shared::new(Type::reference(reference.kind)), Shared::new(vec![places, field]))
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
        Type::places(&[Type::Place(atom), fresh], &self.bindings, &TypeBindings::default())
    }

    /// Walk a field type and emit a `MissingExplicitPlace` diagnostic for
    /// every `ImplicitPlace` placeholder.
    pub(super) fn reject_implicit_places(typ: &cst::Type, db: &DbHandle) {
        match &typ.kind {
            cst::TypeKind::ImplicitPlace => {
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
            | cst::TypeKind::Pure
            | cst::TypeKind::IntegerConstant(_) => (),
        }
    }
}

/// A reference a pattern matches through, which each field it binds is reached through
pub(super) struct MatchedReference {
    kind: ReferenceKind,
    places: Type,
}

/// True if `entry` is shown when printing places
fn is_printed_place(entry: &Type) -> bool {
    match entry {
        Type::Place(_) | Type::Generic(_) => true,
        Type::PlaceProjection(base, _) => matches!(base.as_ref(), Type::Generic(_)),
        _ => false,
    }
}

/// If `entry` is an unbound variable followed by segments like `?t!`, return both
fn open_projection(entry: &Type) -> Option<(&Type, &[PlaceSegment])> {
    match entry {
        Type::PlaceProjection(base, segments) if matches!(base.as_ref(), Type::Variable(_)) => Some((base, segments)),
        _ => None,
    }
}

/// `entry` without `suffix` at the end of its path, if it ends in `suffix`
fn strip_segments(entry: &Type, suffix: &[PlaceSegment]) -> Option<Type> {
    match entry {
        Type::Place(place) => {
            let segments = place.segments.strip_suffix(suffix)?.to_vec();
            Some(Type::Place(Place { root: place.root, segments }))
        },
        Type::PlaceProjection(base, segments) => {
            let none = TypeBindings::default();
            Some(Type::project_place(base, segments.strip_suffix(suffix)?, &none, &none))
        },
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
