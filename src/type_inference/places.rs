//! This file has any type-checker specific code for places.
//! Places are sets of variables or values a reference may refer to like `'a` or `'(x, y)`.

use inc_complete::DbGet;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostic,
    incremental::{DbHandle, GetItem},
    iterator_extensions::mapvec,
    name_resolution::{Origin, ResolutionResult},
    parser::{
        cst::{self, Expr, PlaceSegment, ReferenceKind, Segments},
        ids::{ExprId, NameId, NameStore},
    },
    shared_arc::Shared,
    type_inference::{
        TypeChecker,
        row::{RowKind, RowMatch, RowMode, canonicalize_row, construct_row, flatten_row_into},
    },
};

use super::types::{Type, TypeBindings, TypePrinter};

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

/// A concrete place a reference may point to, like `x.a!.b`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Place {
    pub root: PlaceRoot,
    pub segments: Vec<PlaceSegment>,
}

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

    /// Check if `self` is a descendant of `ancestor` but is not itself the ancestor.
    /// E.g. `x.a.b` is a descendant of `x.a` and `x`, but not of `x.a.b`.
    pub fn is_descendant_of(&self, ancestor: &Place) -> bool {
        self.root == ancestor.root
            && self.segments.len() > ancestor.segments.len()
            && self.segments.starts_with(&ancestor.segments)
    }

    /// True if mutating `mutated` may invalidate a reference to `self`
    pub fn invalidated_by(&self, mutated: &Place) -> bool {
        if self.root != mutated.root {
            return false;
        }
        let common = self.segments.iter().zip(&mutated.segments).take_while(|(a, b)| a == b).count();
        match (self.segments.get(common), mutated.segments.get(common)) {
            (None, _) => false,
            (Some(_), None) => self.segments[common..].contains(&PlaceSegment::Unstable),
            (Some(segment), Some(_)) => *segment == PlaceSegment::Unstable,
        }
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
    fn match_places(&self, a: &Type, b: &Type, new_bindings: &TypeBindings) -> Option<RowMatch> {
        let a_list = self.collect_and_merge_places(a, new_bindings);
        let b_list = self.collect_and_merge_places(b, new_bindings);

        if a_list.iter().chain(b_list.iter()).any(|place| place.is_error()) {
            return None;
        }

        Some(Self::match_row_entries(a_list, b_list, |a_place, b_place| a_place == b_place))
    }

    /// Row-subtype two places rows: is `a`'s actual set of places permitted by `b`'s expected set?
    pub(super) fn place_subtype(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(mut m) = self.match_places(a, b, new_bindings) else { return Ok(()) };
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
        let Some(mut m) = self.match_places(a, b, new_bindings) else { return Ok(()) };
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
                        t.reference_places(&self.bindings).unwrap_or_else(|| self.next_type_variable())
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
            | cst::TypeKind::Pointer
            | cst::TypeKind::NoClosureEnv
            | cst::TypeKind::Hole
            | cst::TypeKind::Unit
            | cst::TypeKind::Place(_)
            | cst::TypeKind::Pure
            | cst::TypeKind::IntegerConstant(_) => (),
        }
    }
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

/// This is the place inferred when a place in a return type position is elided
pub enum PlaceElision<'a> {
    /// There is one candidate input place, and it is an `ImplicitPlace`.
    Elided,
    /// There is one candidate input place, and it is already named.
    Named(&'a cst::Type),
    /// No input place, this occurs in e.g. `ptr_to_ref: fn (Ptr t) -> ref t`.
    Free,
    /// Two or more candidate input places: an error should be issued.
    Ambiguous,
}

/// Does `typ` contain at least one `ImplicitPlace`?
///
/// TODO: Kinds for type variables aren't tracked yet so only places on reference types are
/// checked at the moment.
pub fn cst_type_has_elided_place(typ: &cst::Type) -> bool {
    match &typ.kind {
        cst::TypeKind::Application(f, args) => {
            let is_elided_ref_place = matches!(f.kind, cst::TypeKind::Reference(_))
                && args.first().is_some_and(|arg| matches!(arg.kind, cst::TypeKind::ImplicitPlace));
            is_elided_ref_place || args.iter().any(cst_type_has_elided_place)
        },
        cst::TypeKind::Tuple(elements) => elements.iter().any(cst_type_has_elided_place),
        _ => false,
    }
}

/// What a written place refers to, used to tell whether two written places are the same
type PlaceKey<'a> = Vec<(Option<Origin>, &'a [PlaceSegment])>;

fn place_key<'a>(resolve: &ResolutionResult, paths: &'a [cst::PlacePath]) -> PlaceKey<'a> {
    mapvec(paths, |path| (resolve.name_origins.get(&path.root).copied(), path.segments.as_slice()))
}

/// Collect every place in `typ`
fn collect_places<'a>(
    resolve: &ResolutionResult, typ: &'a cst::Type, implicit_count: &mut usize,
    named: &mut Vec<(PlaceKey<'a>, &'a cst::Type)>,
) {
    match &typ.kind {
        cst::TypeKind::Application(f, args) => {
            if matches!(f.kind, cst::TypeKind::Reference(_))
                && let Some(place_arg) = args.first()
            {
                match &place_arg.kind {
                    cst::TypeKind::ImplicitPlace => *implicit_count += 1,
                    cst::TypeKind::Place(paths) => {
                        let key = place_key(resolve, paths);
                        if named.iter().all(|(other, _)| *other != key) {
                            named.push((key, place_arg));
                        }
                    },
                    _ => (),
                }
            }
            for arg in args.iter() {
                collect_places(resolve, arg, implicit_count, named);
            }
        },
        cst::TypeKind::Tuple(elements) => {
            for element in elements {
                collect_places(resolve, element, implicit_count, named);
            }
        },
        _ => (),
    }
}

/// Which input place an elided return place should be inferred from
pub fn function_place_elision<'a>(resolve: &ResolutionResult, function: &'a cst::FunctionType) -> PlaceElision<'a> {
    let mut implicit_count = 0;
    let mut named = Vec::new();
    for param in &function.parameters {
        collect_places(resolve, &param.typ, &mut implicit_count, &mut named);
    }
    if let Some(environment) = function.environment.as_ref() {
        collect_places(resolve, environment, &mut implicit_count, &mut named);
    }
    match (implicit_count, named.len()) {
        (0, 0) => PlaceElision::Free,
        (1, 0) => PlaceElision::Elided,
        (0, 1) => PlaceElision::Named(named[0].1),
        _ => PlaceElision::Ambiguous,
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
    fn different_roots() {
        assert!(!invalidates("x", "y!"));
        assert!(!invalidates("_", "x!"));
    }
}
