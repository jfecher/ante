//! This file has any type-checker specific code for places - ante's version of lifetimes.
//! Places are sets of variables or values a reference may refer to.

use std::sync::Arc;

use inc_complete::DbGet;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostic,
    incremental::{DbHandle, GetItem},
    name_resolution::{Origin, ResolutionResult},
    parser::{
        cst::{self, Expr},
        ids::{ExprId, NameId, NameStore},
    },
    type_inference::{
        TypeChecker,
        row::{RowMatch, canonicalize_row, construct_row, flatten_row_into, follow_row, sort_and_dedup_row},
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

/// A concrete place a reference may point to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlaceAtom {
    Variable(NameId),
    Anonymous(ExprId, ScopeDepth),
}

impl Type {
    /// Flatten every place reachable from `places` into `found`
    pub(crate) fn flatten_places_into(
        places: &[Type], found: &mut Vec<Type>, bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) {
        flatten_row_into(places, found, bindings, more_bindings);
    }

    /// Flatten, follow, sort, and deduplicate `places`.
    /// Deduplication is done via exact equality rather than unification.
    pub(crate) fn canonicalize_places(
        places: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings,
    ) -> Vec<Type> {
        canonicalize_row(places, bindings, more_bindings, |_: &Type, _: &Type| ())
    }

    /// Zonk each entry in place
    pub(crate) fn follow_places(places: &mut [Type], bindings: &TypeBindings, more_bindings: &TypeBindings) {
        follow_row(places, bindings, more_bindings);
    }

    /// Sort and deduplicate the given places row. Entries must already be zonked.
    pub(crate) fn sort_and_dedup_places(places: &mut Vec<Type>) {
        sort_and_dedup_row(places, |_: &Type, _: &Type| ());
    }

    /// Construct a canonicalized places row by following & deduplicating entries.
    pub(crate) fn places(list: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings) -> Type {
        construct_row(list, bindings, more_bindings)
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
    /// Canonicalize a places row for printing
    pub(super) fn canonicalize_place_entries(&self, places: &Option<Arc<Vec<Type>>>) -> Vec<Type> {
        let places = places.as_deref().map_or(&[][..], Vec::as_slice);
        Type::canonicalize_places(places, self.bindings, &Default::default())
            .into_iter()
            .filter(|t| matches!(t, Type::PlaceAtom(_) | Type::Generic(_)))
            .collect()
    }

    /// Print a reference's place argument, prefixed with a space and `'`, or nothing at all
    /// when it has no concrete places to show
    pub(super) fn fmt_place_arg(&self, places: &Type, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match places.follow(self.bindings) {
            Type::Variable(_) => Ok(()),
            Type::Places(row) => {
                let entries = self.canonicalize_place_entries(row);
                if entries.is_empty() {
                    Ok(())
                } else {
                    write!(f, " '")?;
                    self.fmt_place_entries(&entries, f)
                }
            },
            generic @ Type::Generic(_) => {
                write!(f, " '")?;
                self.fmt_type(generic, false, f)
            },
            _ => Ok(()),
        }
    }

    pub(super) fn fmt_place_atom(&self, atom: PlaceAtom, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match atom {
            PlaceAtom::Variable(name) => {
                if let Some(name) = self.names.try_get_name(name) {
                    write!(f, "{name}")
                } else {
                    write!(f, "#name-not-in-context")
                }
            },
            PlaceAtom::Anonymous(expr, _scope) => write!(f, "_{}", expr.index()),
        }
    }

    fn fmt_place_entry(&self, entry: &Type, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match entry {
            Type::PlaceAtom(atom) => self.fmt_place_atom(*atom, f),
            other => self.fmt_type(other, false, f),
        }
    }

    pub(super) fn fmt_place_entries(&self, entries: &[Type], f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if entries.len() == 1 {
            self.fmt_place_entry(&entries[0], f)
        } else {
            write!(f, "(")?;
            for (i, entry) in entries.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                self.fmt_place_entry(entry, f)?;
            }
            write!(f, ")")
        }
    }
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Flattens two places rows and matches `a`'s concrete places against `b`'s
    fn match_places(&self, a: &Type, b: &Type, new_bindings: &TypeBindings) -> Option<RowMatch<Type>> {
        let a_list = self.collect_and_merge_places(a, new_bindings);
        let b_list = self.collect_and_merge_places(b, new_bindings);

        if a_list.iter().chain(b_list.iter()).any(|place| place.is_error()) {
            return None;
        }

        Some(Self::match_row_entries(a_list, b_list, |a_place, b_place| a_place == b_place))
    }

    /// Row-subtype two places rows: is `a`'s actual set of places permitted by `b`'s expected set?
    pub(super) fn place_subtype(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_places(a, b, new_bindings) else { return Ok(()) };
        self.row_subtype_generic(m, |_: &Type| false, new_bindings)
    }

    /// Unify two places rows: both must end up with the same set of places.
    pub(super) fn place_unify(&self, a: &Type, b: &Type, new_bindings: &mut TypeBindings) -> Result<(), ()> {
        let Some(m) = self.match_places(a, b, new_bindings) else { return Ok(()) };
        self.row_unify_generic(m, new_bindings)
    }

    fn collect_and_merge_places(&self, places: &Type, new_bindings: &TypeBindings) -> Vec<Type> {
        let mut places = self.collect_places(places, new_bindings);
        Type::follow_places(&mut places, &self.bindings, new_bindings);
        Type::sort_and_dedup_places(&mut places);
        places
    }

    /// Flatten `places` into a list of places
    fn collect_places(&self, places: &Type, new_bindings: &TypeBindings) -> Vec<Type> {
        match places.follow_two(&self.bindings, new_bindings) {
            Type::Places(row) => {
                let mut found = Vec::new();
                if let Some(row) = row {
                    Type::flatten_places_into(&row, &mut found, &self.bindings, new_bindings);
                }
                found
            },
            typ @ (Type::PlaceAtom(_) | Type::Generic(_) | Type::Variable(_)) => vec![typ],
            // Any remaining variant should be a kind error emitted elsewhere
            _ => Vec::new(),
        }
    }

    /// Returns the set of places a reference to the given expression may refer to.
    /// This is generally either a single variable or an anonymous place.
    pub(super) fn infer_place(&self, expr: ExprId) -> Type {
        match &self.current_extended_context()[expr] {
            Expr::Variable(path) => match self.path_origin(*path) {
                Some(Origin::Local(name)) => {
                    let atom = self.binding_place(name).root_variable();
                    self.open_place(PlaceAtom::Variable(atom))
                },
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
                    _ => self.infer_place(object),
                }
            },
            // A diverging expr may be coerced to any place
            _ if self.expr_types.get(&expr).is_some_and(|t| self.diverges(t)) => self.next_type_variable(),
            // Otherwise we have an anonymous local like `ref my_call ()`
            _ => self.open_place(PlaceAtom::Anonymous(expr, self.current_scope_depth())),
        }
    }

    /// A places row containing just `atom` plus a fresh open tail
    pub(crate) fn open_place(&self, atom: PlaceAtom) -> Type {
        let fresh = self.next_type_variable();
        Type::places(&[Type::PlaceAtom(atom), fresh], &self.bindings, &TypeBindings::default())
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

/// This is the place inferred when a place in a return type position is elided
pub enum PlaceElision {
    /// There is one candidate input place, and it is an `ImplicitPlace`.
    Elided,
    /// There is one candidate input place, and it is already named.
    Named(Origin),
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

/// Collect every place in `typ`
fn collect_places(resolve: &ResolutionResult, typ: &cst::Type, implicit_count: &mut usize, named: &mut Vec<Origin>) {
    match &typ.kind {
        cst::TypeKind::Application(f, args) => {
            if matches!(f.kind, cst::TypeKind::Reference(_))
                && let Some(place_arg) = args.first()
            {
                match &place_arg.kind {
                    cst::TypeKind::ImplicitPlace => *implicit_count += 1,
                    cst::TypeKind::Place(name) => {
                        if let Some(origin) = resolve.name_origins.get(name).copied()
                            && !named.contains(&origin)
                        {
                            named.push(origin);
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
pub fn function_place_elision(resolve: &ResolutionResult, function: &cst::FunctionType) -> PlaceElision {
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
        (0, 1) => PlaceElision::Named(named[0]),
        _ => PlaceElision::Ambiguous,
    }
}
