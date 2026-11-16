//! Finds the `Copy` and `Drop` impls of each value the borrow walk may copy or drop. These are
//! searched for as optional implicits during inference.
//!
//! We only know which values are dropped, and where, once the borrow walk has tracked moves. Yet
//! which impls drop them depends on the implicits in scope where each value is dropped, and on
//! types which are only known once inference ends. So inference records [ValueImpls] for each
//! value it creates, holding the implicits in scope at the end of the value's scope since that is
//! the latest it can be dropped. Its impls are then searched for once its type is known:
//! - When the lambda the value is created in is checked for captures, since a drop inside the
//!   lambda may call a local implicit which the lambda then needs to capture.
//! - Otherwise once every item of the SCC is inferred, see [TypeChecker::resolve_value_impls].
//!
//! The borrow walk only reads the results, see [TypeChecker::copy_impl_found] and
//! [TypeChecker::drop_dictionary]. It never searches for implicits itself.

use std::{collections::BTreeSet, rc::Rc};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::Location,
    parser::{
        cst::{self, PlaceSegment},
        ids::{ExprId, NameId, PatternId},
    },
    type_inference::{
        Locateable, TypeChecker,
        free_variables::FreeVars,
        places::{Place, PlaceRoot},
        type_body::TypeBody,
        types::Type,
    },
};

/// A value the borrow walk may copy or drop. This is either the value an [OwnedRoot] holds, or
/// a part of it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) struct Owned {
    pub(super) root: OwnedRoot,
    pub(super) path: Vec<OwnedSegment>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(super) enum OwnedRoot {
    /// The value a name is bound to
    Name(NameId),
    /// The value of an expression
    Value(ExprId),
    /// The value a pattern matches
    Pattern(PatternId),
    /// The value pointed to by the mutable reference an expression evaluates to
    Referent(ExprId),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(super) enum OwnedSegment {
    Field(cst::Name),
    /// The field at this index of a union variant
    VariantField(NameId, usize),
}

impl ValueImpls {
    fn new(path: Vec<OwnedSegment>, implicits_in_scope: Option<Rc<[NameId]>>, with_parts: bool) -> ValueImpls {
        ValueImpls {
            path,
            implicits_in_scope,
            with_parts,
            copy_found: false,
            drop: None,
            drop_final: false,
            settled: false,
        }
    }
}

impl Owned {
    pub(super) fn new(root: OwnedRoot) -> Owned {
        Owned { root, path: Vec::new() }
    }

    pub(super) fn part(&self, segment: OwnedSegment) -> Owned {
        let mut path = self.path.clone();
        path.push(segment);
        Owned { root: self.root, path }
    }
}

#[derive(Default)]
pub(super) struct ValueImplTable {
    by_root: FxHashMap<OwnedRoot, Vec<ValueImpls>>,

    /// The implicits in scope when the last value's impls were requested. Later values with the
    /// same implicits share this instead of allocating their own.
    last_scope: Option<Rc<[NameId]>>,
}

#[derive(Clone)]
struct ValueImpls {
    path: Vec<OwnedSegment>,

    /// The local implicits in scope at the end of the scope the value was created in. This is
    /// `None` until then. It is also `None` for parts of a value with its own impls since those
    /// parts share the value's implicits.
    implicits_in_scope: Option<Rc<[NameId]>>,

    /// True if we should also search for the impls of each part of the value once its type is known
    with_parts: bool,

    /// True once a `Copy` impl was found. We only care whether there is one, not which.
    copy_found: bool,

    /// The destination of the `Drop` impl search, once we've searched for it
    drop: Option<ExprId>,

    /// True once we shouldn't repeat the search for its `Drop` impl, either because one was found
    /// or because the search failed when its type was already known
    drop_final: bool,

    /// True once searched for after inference ended, when its type can't be refined any further
    settled: bool,
}

impl TypeChecker<'_, '_> {
    /// Request that the impls of `owned`, created in the innermost scope, be searched for. Does
    /// nothing if its type is trivially `Copy` or its impls were already requested. If `with_parts`
    /// is set, the impls of each of its parts are requested too once its type is known.
    pub(super) fn request_value_impls(&mut self, owned: Owned, with_parts: bool) {
        if !self.requests_value_impls {
            return;
        }
        if let Some(impls) = self.find_value_impls_mut(&owned) {
            impls.with_parts |= with_parts;
            return;
        }
        if self.owned_type(&owned).is_some_and(|typ| self.is_trivially_copy(&typ)) {
            return;
        }
        let implicits_in_scope = self.await_scope_end(&owned);
        let impls = ValueImpls::new(owned.path, implicits_in_scope, with_parts);
        self.item.value_impls.by_root.entry(owned.root).or_default().push(impls);
    }

    /// Arrange for the impls of `owned` to be searched for with the implicits in scope at the end
    /// of the innermost scope. If there is no scope, the implicits in scope now are returned.
    fn await_scope_end(&mut self, owned: &Owned) -> Option<Rc<[NameId]>> {
        match self.implicits.last_mut() {
            Some(scope) => {
                scope.value_impls.push(owned.clone());
                None
            },
            None => Some(self.implicits_snapshot()),
        }
    }

    /// The innermost scope is about to end, so give each value created in it the implicits now in scope
    pub(super) fn end_value_impls_scope(&mut self) {
        let Some(scope) = self.implicits.last_mut() else { return };
        let owned = std::mem::take(&mut scope.value_impls);
        if owned.is_empty() {
            return;
        }
        let implicits_in_scope = self.implicits_snapshot();
        for owned in owned {
            if let Some(impls) = self.find_value_impls_mut(&owned) {
                impls.implicits_in_scope.get_or_insert_with(|| implicits_in_scope.clone());
            }
        }
    }

    /// Returns the implicits in scope where `owned` is dropped. If it has no impls of its own, these
    /// are those of the value it is a part of. If the scope it was created in hasn't ended yet, these
    /// are the implicits in scope now.
    fn scope_of(&mut self, owned: &Owned) -> Rc<[NameId]> {
        let mut path = &owned.path[..];
        loop {
            let value = Owned { root: owned.root, path: path.to_vec() };
            if let Some(scope) = self.find_value_impls(&value).and_then(|impls| impls.implicits_in_scope.clone()) {
                return scope;
            }
            match path.split_last() {
                Some((_, parent)) => path = parent,
                None => return self.implicits_snapshot(),
            }
        }
    }

    /// Returns the local implicits in scope, reusing the previous value's if they're the same
    fn implicits_snapshot(&mut self) -> Rc<[NameId]> {
        let in_scope = self.collect_implicits_in_scope();
        match &self.item.value_impls.last_scope {
            Some(last) if **last == *in_scope => last.clone(),
            _ => {
                let scope: Rc<[NameId]> = in_scope.into();
                self.item.value_impls.last_scope = Some(scope.clone());
                scope
            },
        }
    }

    fn find_value_impls(&self, owned: &Owned) -> Option<&ValueImpls> {
        self.item.value_impls.by_root.get(&owned.root)?.iter().find(|impls| impls.path == owned.path)
    }

    fn find_value_impls_mut(&mut self, owned: &Owned) -> Option<&mut ValueImpls> {
        self.item.value_impls.by_root.get_mut(&owned.root)?.iter_mut().find(|impls| impls.path == owned.path)
    }

    /// The expression of `from` was moved to `to`, so give the values rooted at `to` the impls of
    /// those rooted at `from`
    pub(super) fn copy_value_impls(&mut self, from: OwnedRoot, to: OwnedRoot) {
        let Some(values) = self.item.value_impls.by_root.get(&from).cloned() else { return };
        for impls in &values {
            if impls.implicits_in_scope.is_none() {
                self.await_scope_end(&Owned { root: to, path: impls.path.clone() });
            }
        }
        self.item.value_impls.by_root.insert(to, values);
    }

    /// Returns each value rooted at `root` whose impls we're searching for
    fn values_with_impls(&self, root: OwnedRoot) -> Vec<Owned> {
        let values = self.item.value_impls.by_root.get(&root).map_or(&[][..], Vec::as_slice);
        values.iter().map(|impls| Owned { root, path: impls.path.clone() }).collect()
    }

    /// Add to the free variables of `lambda` whatever the `Drop` impls of values it may drop refer
    /// to. These values are those it owns, plus what it captures by value. We have
    /// to do this here since drops are only inserted once the lambda is borrow checked, after its
    /// environment is decided. If we can't search for a value's impl yet, any local `Drop` impl in
    /// its scope could drop it, so we capture them all.
    pub(super) fn capture_drop_impls(&mut self, lambda: &mut FreeVars, is_move: bool, moved: &BTreeSet<NameId>) {
        // Dictionaries only refer to locals that are implicits in scope, so we only need to
        // capture those visible outside of the lambda
        if self.implicits_in_scope().all(|name| lambda.defined_in_fn.contains(&name)) {
            return;
        }
        let values = lambda.exprs.iter().flat_map(|expr| [OwnedRoot::Value(*expr), OwnedRoot::Referent(*expr)]);
        let mut roots: Vec<_> = values.collect();
        roots.extend(lambda.defined_in_fn.iter().map(|name| OwnedRoot::Name(*name)));
        let by_value = lambda.free_vars.iter().filter(|name| is_move || moved.contains(name));
        roots.extend(by_value.map(|name| OwnedRoot::Name(*name)));
        for root in roots {
            // We only know the value's parts once we know its type
            for owned in self.values_with_impls(root) {
                self.expand_parts(&owned);
            }
            for owned in self.values_with_impls(root) {
                match self.resolve_drop_impl_early(&owned) {
                    Ok(Some(dictionary)) => lambda.find_free_variables(dictionary, self),
                    Ok(None) => (),
                    Err(implicits_in_scope) => {
                        let (drop, _) = self.drop_trait();
                        for name in implicits_in_scope.iter() {
                            if self.name_types.get(name).is_some_and(|typ| self.trait_subject(typ, drop).is_some()) {
                                lambda.check_name(*name);
                            }
                        }
                    },
                }
            }
        }
    }

    /// The borrow walk may own the value of `expr` until the end of its statement unless `expr`
    /// reads a place, see [super::borrows::BorrowChecker::own_temporary]. So we search for the impls
    /// of each expression in such a position: a statement whose value is unused or an operand used
    /// in place.
    pub(super) fn request_temporary_impls(&mut self, expr: ExprId, with_parts: bool) {
        let owned = self.owned_value(expr);
        self.request_value_impls(owned, with_parts);
    }

    /// Returns the value `expr` evaluates to. This is the place it reads if that place is local,
    /// otherwise it is a new value.
    pub(super) fn owned_value(&self, expr: ExprId) -> Owned {
        match self.try_build_move_path(expr) {
            Some(place) => self.owned_place(&place).unwrap_or_else(|| Owned::new(OwnedRoot::Value(expr))),
            None => Owned::new(OwnedRoot::Value(expr)),
        }
    }

    /// Returns the value held at `place`. For a temporary the borrow walk named, this is the value
    /// of its expression.
    pub(super) fn owned_place(&self, place: &Place) -> Option<Owned> {
        let PlaceRoot::Variable(name) = place.root else { return None };
        let root = match self.current_extended_context().drops().temporary_expr(name) {
            Some(expr) => OwnedRoot::Value(expr),
            None => OwnedRoot::Name(name),
        };
        let path = place.segments.iter().map(|segment| match segment {
            PlaceSegment::Field(field) => Some(OwnedSegment::Field(field.clone())),
            PlaceSegment::Unstable => None,
        });
        Some(Owned { root, path: path.collect::<Option<_>>()? })
    }

    /// Returns the type of `owned`, if it is known
    pub(super) fn owned_type(&mut self, owned: &Owned) -> Option<Type> {
        let mut typ = match owned.root {
            OwnedRoot::Name(name) => self.name_types.get(&name)?.clone(),
            OwnedRoot::Value(expr) => self.expr_types.get(&expr)?.clone(),
            OwnedRoot::Pattern(pattern) => self.pattern_types.get(&pattern)?.clone(),
            OwnedRoot::Referent(expr) => self.expr_types.get(&expr)?.reference_element(&self.bindings)?.1,
        };
        for segment in &owned.path {
            typ = match segment {
                OwnedSegment::Field(field) => self.get_field_types(&typ, None).remove(field)?.0,
                OwnedSegment::VariantField(variant, index) => {
                    let (name, arguments) = typ.definition_head(&self.bindings)?;
                    let TypeBody::Sum(variants) = name.type_body(arguments, self.compiler) else { return None };
                    let (_, _, fields) = variants.into_iter().find(|(name, _, _)| name == variant)?;
                    fields.into_iter().nth(*index)?.1
                },
            };
        }
        Some(typ)
    }

    fn owned_location(&self, owned: &Owned) -> Location {
        match owned.root {
            OwnedRoot::Name(name) => name.locate(self),
            OwnedRoot::Value(expr) | OwnedRoot::Referent(expr) => expr.locate(self),
            OwnedRoot::Pattern(pattern) => pattern.locate(self),
        }
    }

    /// Now that the type of `owned` may be known, request impls for each of its parts if they were
    /// asked for. Its parts are its fields, or the fields of each variant if it is a union.
    fn expand_parts(&mut self, owned: &Owned) {
        let Some(impls) = self.find_value_impls(owned) else { return };
        if !impls.with_parts {
            return;
        }
        let Some(typ) = self.owned_type(owned) else { return };
        if matches!(self.follow_type(&typ), Type::Variable(_)) {
            return;
        }
        self.find_value_impls_mut(owned).expect("checked above").with_parts = false;

        let fields = self.get_field_types(&typ, None).into_keys();
        let mut parts: Vec<_> = fields.map(OwnedSegment::Field).collect();
        if let Some((name, arguments)) = typ.definition_head(&self.bindings)
            && let TypeBody::Sum(variants) = name.type_body(arguments, self.compiler)
        {
            for (variant, _, fields) in variants {
                parts.extend((0..fields.len()).map(|index| OwnedSegment::VariantField(variant, index)));
            }
        }
        for segment in parts {
            let part = owned.part(segment);
            if self.find_value_impls(&part).is_none() {
                let impls = ValueImpls::new(part.path, None, false);
                self.item.value_impls.by_root.entry(owned.root).or_default().push(impls);
            }
        }
    }

    /// Search for the `Drop` impl of `owned` while its lambda is checked for captures, returning
    /// the dictionary if one was found. If its type isn't known well enough to search yet, this
    /// returns `Err` with each local implicit the impl may turn out to be.
    pub(super) fn resolve_drop_impl_early(&mut self, owned: &Owned) -> Result<Option<ExprId>, Rc<[NameId]>> {
        self.expand_parts(owned);
        let Some(impls) = self.find_value_impls(owned) else { return Ok(None) };
        if impls.drop_final {
            return Ok(impls.drop.filter(|destination| self.implicit_was_found(*destination)));
        }
        let scope = self.scope_of(owned);
        let Some(typ) = self.owned_type(owned) else { return Ok(None) };
        if self.is_trivially_copy(&typ) {
            return Ok(None);
        }
        let location = self.owned_location(owned);
        let target = self.drop_type(typ.clone());
        let previous = self.find_value_impls(owned).and_then(|impls| impls.drop);
        let (destination, result) = self.search_value_impl(previous, target, location, &scope, false);
        let known = result.is_ok() || !typ.has_unbound_type_variables(&self.bindings);
        let impls = self.find_value_impls_mut(owned).expect("checked above");
        (impls.drop, impls.drop_final) = (Some(destination), known);
        match result {
            Ok(()) => Ok(Some(destination)),
            Err(_) if known => Ok(None),
            Err(_) => Err(scope),
        }
    }

    /// Search for the impls of every value in the current item which needs them. This is done once
    /// every item in the SCC is inferred so that types can't be refined any further.
    pub(super) fn resolve_value_impls(&mut self) {
        let roots: Vec<OwnedRoot> = self.item.value_impls.by_root.keys().copied().collect();
        for root in roots {
            // Expanding parts may add more values for this root, so we resolve those as well
            let mut index = 0;
            while let Some(impls) = self.item.value_impls.by_root.get(&root).and_then(|values| values.get(index)) {
                let owned = Owned { root, path: impls.path.clone() };
                index += 1;
                if !impls.settled {
                    self.expand_parts(&owned);
                    self.settle_value_impls(&owned);
                }
            }
        }
    }

    fn settle_value_impls(&mut self, owned: &Owned) {
        let impls = self.find_value_impls_mut(owned).expect("only called on a value with impls");
        impls.settled = true;
        let (previous_drop, drop_final) = (impls.drop, impls.drop_final);
        let scope = self.scope_of(owned);

        let Some(typ) = self.owned_type(owned) else { return };
        if self.is_trivially_copy(&typ) || typ.contains_error(&self.bindings) {
            return;
        }
        let location = self.owned_location(owned);

        let copy_found = self.type_is_copy_in(&typ, location.clone(), &scope);
        self.find_value_impls_mut(owned).expect("only called on a value with impls").copy_found = copy_found;

        // A `Copy` value is never dropped
        if copy_found || drop_final {
            return;
        }
        let target = self.drop_type(typ);
        let (drop, _) = self.search_value_impl(previous_drop, target, location, &scope, true);
        self.find_value_impls_mut(owned).expect("only called on a value with impls").drop = Some(drop);
    }

    /// True if a `Copy` impl was found for the value `owned` of type `typ`
    pub(super) fn copy_impl_found(&self, owned: &Owned, typ: &Type) -> bool {
        self.is_trivially_copy(typ) || self.find_value_impls(owned).is_some_and(|impls| impls.copy_found)
    }

    /// Returns the `Drop` dictionary found for `owned`, if any
    pub(super) fn drop_dictionary(&self, owned: &Owned) -> Option<ExprId> {
        let destination = self.find_value_impls(owned)?.drop?;
        self.implicit_was_found(destination).then_some(destination)
    }

    /// Returns the type `Drop typ e`, with a fresh type variable for each of its other arguments
    pub(super) fn drop_type(&mut self, typ: Type) -> Type {
        let drop = self.drop_trait();
        self.trait_type(drop, typ)
    }
}
