//! This file has any type-checker specific code for places - ante's version of lifetimes.
//! Places are sets of variables or values a reference may refer to.

use std::sync::Arc;

use inc_complete::DbGet;
use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::Diagnostic,
    incremental::{DbHandle, GetItem},
    name_resolution::Origin,
    parser::{
        cst::{self, Expr},
        ids::{ExprId, NameId, NameStore},
    },
    type_inference::TypeChecker,
};

use super::{
    RowEntry, RowMatch,
    types::{Type, TypeBindings, TypePrinter},
};

/// Anonymous references record the scope depth they're valid in for escape analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ScopeDepth(pub(crate) u32);

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
        for place in places {
            match place.follow_two(bindings, more_bindings) {
                Type::Places(Some(row)) => Self::flatten_places_into(&row, found, bindings, more_bindings),
                Type::Places(None) => (),
                typ => found.push(typ),
            }
        }
    }

    /// Sort key used to canonicalize a places row: concrete atoms first,
    /// then rigid generics, then unbound variables last.
    fn place_sort_key(typ: &Type) -> (bool, bool, Option<PlaceAtom>) {
        match typ {
            Type::PlaceAtom(atom) => (false, false, Some(*atom)),
            Type::Variable(_) => (true, true, None),
            _ => (false, true, None),
        }
    }

    /// Flatten, follow, sort, and deduplicate `places`.
    /// Deduplication is done via exact equality rather than unification.
    pub(crate) fn canonicalize_places(places: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings) -> Vec<Type> {
        let mut list = Vec::with_capacity(places.len());
        Self::flatten_places_into(places, &mut list, bindings, more_bindings);
        Self::follow_places(&mut list, bindings, more_bindings);
        Self::sort_and_dedup_places(&mut list);
        list
    }

    /// Zonk each entry in place
    pub(crate) fn follow_places(places: &mut [Type], bindings: &TypeBindings, more_bindings: &TypeBindings) {
        for place in places {
            if let Some(typ) = place.follow_all_opt(bindings, more_bindings) {
                *place = typ;
            }
        }
    }

    /// Sort and deduplicate the given places row. Entries must already be zonked.
    pub(crate) fn sort_and_dedup_places(places: &mut Vec<Type>) {
        places.sort_by(|a, b| Self::place_sort_key(a).cmp(&Self::place_sort_key(b)).then_with(|| a.cmp(b)));
        places.dedup();
    }

    /// Construct a canonicalized places row by following & deduplicating entries.
    pub(crate) fn places(list: &[Type], bindings: &TypeBindings, more_bindings: &TypeBindings) -> Type {
        Self::places_from_canonical(Self::canonicalize_places(list, bindings, more_bindings))
    }

    /// Construct a places row from an already canonical place set
    pub(crate) fn places_from_canonical(list: Vec<Type>) -> Type {
        if list.is_empty() { Type::Places(None) } else { Type::Places(Some(Arc::new(list))) }
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
    /// Canonicalize a places row for printing, returning just its concrete atoms.
    pub(super) fn canonicalize_place_atoms(&self, places: &Option<Arc<Vec<Type>>>) -> Vec<PlaceAtom> {
        let places = places.as_deref().map_or(&[][..], Vec::as_slice);
        let canonical = Type::canonicalize_places(places, self.bindings, &Default::default());
        canonical.iter().filter_map(|t| if let Type::PlaceAtom(a) = t { Some(*a) } else { None }).collect()
    }

    /// Print a reference's place argument, prefixed with a space and `'`, or nothing at all
    /// when it has no concrete places to show
    pub(super) fn fmt_place_arg(&self, places: &Type, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match places.follow(self.bindings) {
            Type::Variable(_) => Ok(()),
            Type::Places(row) => {
                let atoms = self.canonicalize_place_atoms(row);
                if atoms.is_empty() {
                    Ok(())
                } else {
                    write!(f, " '")?;
                    self.fmt_place_atoms(&atoms, f)
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

    pub(super) fn fmt_place_atoms(&self, atoms: &[PlaceAtom], f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if atoms.len() == 1 {
            self.fmt_place_atom(atoms[0], f)
        } else {
            write!(f, "(")?;
            for (i, atom) in atoms.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ")?;
                }
                self.fmt_place_atom(*atom, f)?;
            }
            write!(f, ")")
        }
    }
}

impl RowEntry for Type {
    fn is_open(&self) -> bool {
        matches!(self, Type::Variable(_))
    }

    fn inner_type(&self) -> &Type {
        self
    }

    fn row_of(list: &[Self], bindings: &TypeBindings, more_bindings: &TypeBindings) -> Type {
        Type::places(list, bindings, more_bindings)
    }

    fn fresh(next_var: &mut impl FnMut() -> Type) -> Self {
        next_var()
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

    /// Compute the set of places that a reference expression's RHS may point to
    pub(super) fn infer_place(&self, expr: ExprId) -> Type {
        match &self.current_extended_context()[expr] {
            Expr::Variable(path) => match self.path_origin(*path) {
                Some(Origin::Local(name)) => {
                    let atom = self.binding_place(name).root_variable();
                    self.open_place(PlaceAtom::Variable(atom))
                },
                _ => Type::Places(None),
            },
            Expr::MemberAccess(access) => {
                let object = access.object;
                match self.expr_types.get(&object) {
                    // Object is already a reference
                    Some(t) if t.reference_element(&self.bindings).is_some() => {
                        t.reference_places(&self.bindings).unwrap_or(Type::Places(None))
                    },
                    _ => self.infer_place(object),
                }
            },
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
