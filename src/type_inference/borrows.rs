//! This module contains the borrow-checking pass which occurs after type checking (but before
//! definitions are generalized). The purpose of this pass is to:
//! - Issue an error when a reference escapes the scope in which it is valid
//! - Issue an error when a non-`Copy` value is referenced after it has already been moved
//! - Issue an error when a reference is used after a place it may refer to has been moved
//!
//! Each reference in Ante has a 'place' which is analogous to a lifetime in Rust. Each place
//! is a set of places a lifetime may refer to. The result of `if rand () then ref foo else ref bar`
//! for example may refer to either `foo` or `bar` so its place is `'(foo, bar)`.
//!
//! Anonymous places are tagged with the stack depth in which they're valid for.
//!
//! This module walks the CST in [BorrowChecker::walk_expr], keeping track of the current scope nesting.
//! - The result type of each block is checked to ensure it contains no places that refer to names
//! declared within the block.
//! - Each move of a variable is recorded in the current block. We merge these moves when branches
//! are merged and check at the end of each loop to ensure each variable moved was declared within
//! the loop body.
//! - On each scope exit, we check each variable only mentions places from that scope or prior.
//! - Each mutation is recorded like a move. Using a value with an unstable reference like
//! `elem: ref 'v! t` is an error if `v` was mutated since the value's holder was assigned.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    diagnostics::{Diagnostic, Location, RepeatedContext},
    incremental::{GetItem, GetType},
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{self, Expr, KindAnnotation, Pattern},
        ids::{ExprId, NameId, NameStore, PathId, PatternId, TopLevelName},
    },
    type_inference::{
        Locateable, TypeChecker,
        affine::{MoveScope, MoveTracker},
    },
};

use super::{
    free_variables::FreeVars,
    generics::Generic,
    patterns::WILDCARD_PATTERN,
    places::{EntryRoot, Place, PlaceEntry, PlaceRoot, ScopeDepth},
    types::{GenericSubstitutions, PrimitiveType, Type, TypeBindings},
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_borrows(&mut self, definition: &cst::Definition, depth: ScopeDepth) {
        let mut checker = BorrowChecker {
            tc: self,
            name_depths: FxHashMap::default(),
            function_depth: None,
            reported_escapes: FxHashSet::default(),
            uses: Vec::new(),
            binding_places: FxHashMap::default(),
            borrow_sites: Vec::new(),
            parents: FxHashMap::default(),
            moves: MoveTracker::default(),
        };
        checker.record_name_depths_in_pattern(definition.pattern, depth);
        checker.walk_expr(definition.rhs, depth, true, true);
    }
}

struct BorrowChecker<'a, 'local, 'inner> {
    tc: &'a mut TypeChecker<'local, 'inner>,

    /// The scope depth each local name was bound at
    name_depths: FxHashMap<NameId, ScopeDepth>,

    /// The scope depth of the current function's body, used to check `return` statements
    function_depth: Option<ScopeDepth>,

    /// Used so we only issue an error for each [Place] once
    reported_escapes: FxHashSet<Place>,

    /// The first use of each holder of unstable references within each enclosing loop or lambda body
    uses: Vec<Uses>,

    /// The places reachable from each binding whose type mentions any.
    /// Bindings with no places are left out to avoid unnecessary iterations over this later.
    binding_places: FxHashMap<NameId, Vec<Place>>,

    /// Every expression whose type mentions a place, in walk order. Used to work out the location
    /// of an escaping place after the fact.
    borrow_sites: Vec<ExprId>,

    /// Maps each expression to the one it is nested within, so that [Self::escape_location] can
    /// widen a borrow outwards to the expression which actually extended its lifetime.
    parents: FxHashMap<ExprId, ExprId>,

    moves: MoveTracker,
}

impl<'a, 'local, 'inner> BorrowChecker<'a, 'local, 'inner> {
    /// Walk `expr` at `depth`, checking that each reference is still valid when used and each
    /// variable is not moved more than once.
    ///
    /// `check_move` is false on assignment's lhs and a member access' object.
    /// `record_move` is false on the rhs of reference expressions.
    fn walk_expr(&mut self, expr: ExprId, depth: ScopeDepth, check_move: bool, record_move: bool) {
        for child in self.child_exprs(expr) {
            self.parents.insert(child, expr);
        }

        match self.tc.resolved_expr(expr).into_owned() {
            Expr::Sequence(items) => {
                let new_depth = depth.deeper();
                for item in &items {
                    self.walk_expr(item.expr, new_depth, check_move, record_move);
                }
                if let Some(tail) = items.last() {
                    self.check_escapes(tail.expr, new_depth);
                }
                self.check_outer_bindings(new_depth);
            },
            Expr::Definition(definition) => {
                self.walk_expr(definition.rhs, depth, check_move, record_move);
                self.record_name_depths_in_pattern(definition.pattern, depth);
            },
            Expr::If(if_) => self.walk_if(&if_, depth, check_move, record_move),
            Expr::Match(match_) => self.walk_match(&match_, depth, check_move, record_move),
            Expr::Lambda(_) => {
                self.borrow_sites.push(expr);
                self.walk_lambda(expr, depth, None, Some(RepeatedContext::Closure))
            },
            Expr::While(while_) => self.walk_while(&while_, depth, check_move, record_move),
            Expr::For(for_) => self.walk_for(&for_, depth, check_move, record_move),
            Expr::Return(ret) => {
                self.walk_expr(ret.expression, depth, check_move, record_move);

                if let Some(boundary) = self.function_depth {
                    self.check_escapes(ret.expression, boundary);
                } else {
                    // return outside of a function, name resolution should already error here
                }
            },
            Expr::Assignment(assignment) => self.walk_assignment(&assignment, expr, depth, check_move, record_move),
            Expr::Reference(reference) => {
                self.borrow_sites.push(expr);
                self.walk_expr(reference.rhs, depth, check_move, false)
            },
            Expr::MemberAccess(access) => {
                self.walk_expr(access.object, depth, false, false);
                self.check_member_access_move(&access, expr, check_move, record_move);
            },
            Expr::Call(call) => {
                self.walk_expr(call.function, depth, check_move, record_move);
                for arg in &call.arguments {
                    self.walk_expr(arg.expr, depth, check_move, record_move);
                }
                let mutated = self.call_mutated_places(call.function);
                self.record_mutation(mutated, expr);
            },
            Expr::TypeAnnotation(annotation) => self.walk_expr(annotation.lhs, depth, check_move, record_move),
            Expr::Constructor(constructor) => {
                for (_, field_expr) in &constructor.fields {
                    self.walk_expr(*field_expr, depth, check_move, record_move);
                }
            },
            Expr::Handle(handle) => self.walk_handle(&handle, depth),
            Expr::ArrayLiteral(elements) => {
                for element in &elements {
                    self.walk_expr(*element, depth, check_move, record_move);
                }
            },
            Expr::Variable(path) => self.walk_variable(path, expr, check_move, record_move),
            Expr::Literal(_) | Expr::Break | Expr::Continue | Expr::Error | Expr::Extern(_) | Expr::Quoted(_) => {},
            Expr::Is(_) | Expr::Do(_) | Expr::Loop(_) | Expr::InterpolatedString(_) => {
                unreachable!("desugared before type inference")
            },
        }
    }

    /// Record the scope depth `name` was bound at, along with any places its type may refer to.
    fn bind_name(&mut self, name: NameId, depth: ScopeDepth) {
        self.name_depths.insert(name, depth);

        let Some(typ) = self.tc.name_types.get(&name) else { return };
        let mut entries = Vec::new();
        self.tc.collect_place_entries(typ, &mut entries);

        // Mutations before now can't invalidate this binding's references
        if !entries.is_empty() {
            self.moves.clear_moves(&self.tc.binding_place(name));
        }

        let places: Vec<_> = entries.into_iter().filter_map(PlaceEntry::concrete).collect();
        if !places.is_empty() {
            self.binding_places.insert(name, places);
        }
    }

    /// Record the scope depth of every name a pattern binds.
    fn record_name_depths_in_pattern(&mut self, pattern: PatternId, depth: ScopeDepth) {
        match self.tc.pattern_of(pattern).as_ref() {
            Pattern::Variable(name) | Pattern::MethodName { item_name: name, .. } => {
                self.bind_name(*name, depth);
            },
            Pattern::Alias(name, inner) => {
                self.bind_name(*name, depth);
                self.record_name_depths_in_pattern(*inner, depth);
            },
            Pattern::Constructor(_, args) => {
                for arg in args {
                    self.record_name_depths_in_pattern(*arg, depth);
                }
            },
            Pattern::ConstructorRest(_, args, name) => {
                for arg in args {
                    self.record_name_depths_in_pattern(*arg, depth);
                }
                if let Some(name) = name {
                    self.bind_name(*name, depth);
                }
            },
            Pattern::TypeAnnotation(inner, _) => self.record_name_depths_in_pattern(*inner, depth),
            Pattern::Or(alts) => {
                for alt in alts {
                    self.record_name_depths_in_pattern(*alt, depth);
                }
            },
            Pattern::Error | Pattern::Literal(_) => {},
        }
    }

    /// Whether `pattern` binds any part of the value it matches. Wildcards are excluded.
    fn pattern_binds_name(&self, pattern: PatternId) -> bool {
        match self.tc.pattern_of(pattern).as_ref() {
            Pattern::Variable(name) => self.tc.current_extended_context()[*name].as_str() != WILDCARD_PATTERN,
            Pattern::MethodName { .. } | Pattern::Alias(..) => true,
            Pattern::Constructor(_, args) => args.iter().any(|arg| self.pattern_binds_name(*arg)),
            Pattern::ConstructorRest(_, args, name) => {
                name.is_some() || args.iter().any(|arg| self.pattern_binds_name(*arg))
            },
            Pattern::TypeAnnotation(inner, _) => self.pattern_binds_name(*inner),
            Pattern::Or(alts) => alts.iter().any(|alt| self.pattern_binds_name(*alt)),
            Pattern::Error | Pattern::Literal(_) => false,
        }
    }

    fn walk_and_check_escapes(&mut self, expr: ExprId, depth: ScopeDepth, check_move: bool, record_move: bool) {
        self.walk_expr(expr, depth, check_move, record_move);
        self.check_escapes(expr, depth);
        self.check_outer_bindings(depth);
    }

    /// The scope depth `e` in `x := e` must be valid for
    fn assignment_target_depth(&self, lhs: ExprId) -> Option<ScopeDepth> {
        if let Some(depth) = self.reference_target_depth(lhs) {
            return Some(depth);
        }
        if let Expr::Call(call) = self.tc.resolved_expr(lhs).as_ref() {
            if let Some(arg) = call.arguments.first() {
                return self.assignment_target_depth(arg.expr);
            }
        }
        None
    }

    /// If `lhs`'s type is a reference, return the shallowest depth among the places it may reference
    fn reference_target_depth(&self, lhs: ExprId) -> Option<ScopeDepth> {
        let typ = self.tc.expr_types.get(&lhs)?;
        let places = typ.reference_places(&self.tc.bindings)?;
        let mut atoms = Vec::new();
        self.collect_place_atoms(&places, &mut atoms);
        atoms.iter().filter_map(|atom| self.atom_depth(atom)).min()
    }

    /// The scope depth `atom` is valid for, if known.
    fn atom_depth(&self, atom: &Place) -> Option<ScopeDepth> {
        match atom.root {
            PlaceRoot::Variable(name) => self.name_depths.get(&name).copied(),
            PlaceRoot::Anonymous(_, depth) => Some(depth),
        }
    }

    /// Check that `value`'s type carries only places that outlive `boundary`.
    fn check_escapes(&mut self, value: ExprId, boundary: ScopeDepth) {
        let Some(typ) = self.tc.expr_types.get(&value) else { return };

        let mut offending = Vec::new();
        self.collect_place_atoms(typ, &mut offending);
        offending.retain(|atom| self.atom_depth(atom).is_some_and(|depth| depth >= boundary));

        let location = self.narrow_value_expr(value);
        for atom in offending {
            self.report_escape(atom, location);
        }
    }

    /// A binding declared outside the scope being left may have been widened, by a store through a
    /// `mut` reference or an assignment, to refer to a place declared inside it. The block tail and
    /// branch checks only look at one value's type, so nothing else catches this.
    fn check_outer_bindings(&mut self, boundary: ScopeDepth) {
        let mut escapes = Vec::new();
        for (name, places) in &self.binding_places {
            if self.name_depths.get(name).is_none_or(|depth| *depth >= boundary) {
                continue;
            }
            // Skipping already-reported places here rather than in report_escape keeps us from
            // searching for an error location we would only throw away.
            let offending = places.iter().filter(|atom| {
                !self.reported_escapes.contains(atom) && self.atom_depth(atom).is_some_and(|d| d >= boundary)
            });
            escapes.extend(offending.cloned().map(|atom| (atom, *name)));
        }

        for (atom, name) in escapes {
            let carrier = self.tc.name_types.get(&name).cloned();
            match carrier.and_then(|carrier| self.escape_location(&atom, name, &carrier)) {
                Some(location) => self.report_escape(atom, location),
                None => self.report_escape(atom, name),
            }
        }

        // Drop the bindings this scope owned so they aren't rechecked later
        let Self { binding_places, name_depths, .. } = self;
        binding_places.retain(|name, _| name_depths.get(name).is_some_and(|depth| *depth < boundary));
    }

    /// The expression to blame for `atom` escaping out of `binding`, if one is known.
    fn escape_location(&self, atom: &Place, binding: NameId, carrier: &Type) -> Option<ExprId> {
        if let PlaceRoot::Anonymous(origin, _) = atom.root {
            return Some(origin);
        }

        let target = self.row_carrying(carrier, atom)?;

        if let Some(location) = self.widening_argument(atom, binding, &target) {
            return Some(location);
        }

        let mut last_origin = None;
        for origin in self.borrow_sites.iter().rev().copied() {
            if !self.places_originated_by(origin).contains(atom) {
                continue;
            }
            last_origin.get_or_insert(origin);

            let mut current = Some(origin);
            while let Some(expr) = current {
                let row = self.tc.expr_types.get(&expr).and_then(|typ| self.row_carrying(typ, atom));
                if row.is_some_and(|row| is_subrow(&row, &target)) {
                    return Some(expr);
                }
                current = self.parents.get(&expr).copied();
            }
        }

        last_origin
    }

    /// Find a call which is passed a borrow of `binding`, and return the argument beside it which
    /// carries `atom` in `target`.
    fn widening_argument(&self, atom: &Place, binding: NameId, target: &[Type]) -> Option<ExprId> {
        let binding_place = self.tc.binding_place(binding);

        for borrow in self.borrow_sites.iter().rev().copied() {
            if !self.places_originated_by(borrow).contains(&binding_place) {
                continue;
            }

            let Some(parent) = self.parents.get(&borrow).copied() else { continue };
            let Expr::Call(call) = self.tc.resolved_expr(parent).into_owned() else { continue };

            let stored = call.arguments.iter().map(|arg| arg.expr).find(|arg| {
                *arg != borrow
                    && self
                        .tc
                        .expr_types
                        .get(arg)
                        .and_then(|typ| self.row_carrying(typ, atom))
                        .is_some_and(|row| is_subrow(&row, target))
            });

            if stored.is_some() {
                return stored;
            }
        }

        None
    }

    /// The places `expr` creates, as opposed to just carries.
    fn places_originated_by(&self, expr: ExprId) -> Vec<Place> {
        let mut places = Vec::new();
        match self.tc.resolved_expr(expr).as_ref() {
            Expr::Reference(reference) => {
                let row = self.tc.infer_place(reference.rhs);
                self.collect_place_atoms(&row, &mut places);
            },
            Expr::Lambda(_) => {
                if let Some(typ) = self.tc.expr_types.get(&expr)
                    && let Type::Function(function) = typ.follow(&self.tc.bindings)
                {
                    self.collect_place_atoms(&function.environment, &mut places);
                }
            },
            _ => (),
        }
        places
    }

    /// The canonicalized places row within `typ` which carries `atom`, if any.
    fn row_carrying(&self, typ: &Type, atom: &Place) -> Option<Vec<Type>> {
        match typ.follow(&self.tc.bindings) {
            Type::Application(constructor, args) => self
                .row_carrying(constructor, atom)
                .or_else(|| args.iter().find_map(|arg| self.row_carrying(arg, atom))),
            Type::Tuple(elements) => elements.iter().find_map(|element| self.row_carrying(element, atom)),
            Type::Places(_) | Type::Place(_) | Type::PlaceProjection(..) => {
                let row =
                    Type::canonicalize_places(std::slice::from_ref(typ), &self.tc.bindings, &TypeBindings::default());
                row.contains(&Type::Place(atom.clone())).then_some(row)
            },
            Type::Forall(_, typ) => self.row_carrying(typ, atom),
            Type::Effects(effects) => {
                effects.as_ref().and_then(|effects| effects.iter().find_map(|effect| self.row_carrying(effect, atom)))
            },
            Type::Function(function) => self
                .row_carrying(&function.environment, atom)
                .or_else(|| self.row_carrying(&function.effects, atom))
                .or_else(|| self.row_carrying(&function.return_type, atom))
                .or_else(|| function.parameters.iter().find_map(|p| self.row_carrying(&p.typ, atom))),
            Type::Primitive(_) | Type::Generic(_) | Type::Variable(_) | Type::UserDefined(_) | Type::U32(_) => None,
        }
    }

    /// Every expression directly nested within `expr`
    fn child_exprs(&self, expr: ExprId) -> Vec<ExprId> {
        match self.tc.resolved_expr(expr).as_ref() {
            Expr::Sequence(items) => mapvec(items, |item| item.expr),
            Expr::Definition(definition) => vec![definition.rhs],
            Expr::MemberAccess(access) => vec![access.object],
            Expr::Call(call) => {
                let mut children = vec![call.function];
                children.extend(call.arguments.iter().map(|arg| arg.expr));
                children
            },
            Expr::Lambda(lambda) => vec![lambda.body],
            Expr::If(if_) => {
                let mut children = vec![if_.condition, if_.then];
                children.extend(if_.else_);
                children
            },
            Expr::Match(match_) => {
                let mut children = vec![match_.expression];
                children.extend(match_.cases.iter().map(|(_, branch)| *branch));
                children
            },
            Expr::Handle(handle) => {
                let mut children = vec![handle.expression];
                children.extend(handle.cases.iter().map(|(_, branch)| *branch));
                children
            },
            Expr::Reference(reference) => vec![reference.rhs],
            Expr::TypeAnnotation(annotation) => vec![annotation.lhs],
            Expr::Constructor(constructor) => constructor.fields.iter().map(|(_, field)| *field).collect(),
            Expr::While(while_) => vec![while_.condition, while_.body],
            Expr::For(for_) => vec![for_.start, for_.end, for_.body],
            Expr::Return(ret) => vec![ret.expression],
            Expr::Assignment(assignment) => {
                let mut children = vec![assignment.lhs, assignment.rhs];
                children.extend(assignment.op.map(|(_, op_expr)| op_expr));
                children
            },
            Expr::ArrayLiteral(elements) => elements.clone(),
            Expr::Literal(_)
            | Expr::Variable(_)
            | Expr::Break
            | Expr::Continue
            | Expr::Error
            | Expr::Extern(_)
            | Expr::Quoted(_) => Vec::new(),
            Expr::Is(_) | Expr::Do(_) | Expr::Loop(_) | Expr::InterpolatedString(_) => {
                unreachable!("desugared before type inference")
            },
        }
    }

    /// Issue an escaping reference error for `atom`, avoiding multiple errors for the same place
    fn report_escape(&mut self, atom: Place, location: impl Locateable) {
        if !self.reported_escapes.insert(atom.clone()) {
            return;
        }
        let location = location.locate(self.tc);
        let (name, declared_at) = match atom.root {
            PlaceRoot::Variable(root) => {
                let name = atom.display_name(self.tc.current_extended_context());
                (Some(name), root.locate(self.tc))
            },
            PlaceRoot::Anonymous(origin, _) => (None, origin.locate(self.tc)),
        };
        self.tc.compiler.accumulate(Diagnostic::EscapingReference { name, location, declared_at });
    }

    /// Make an expr's location a bit more precise by returning the last expr of a block
    /// or the lhs of a type annotation.
    fn narrow_value_expr(&self, value: ExprId) -> ExprId {
        match self.tc.resolved_expr(value).as_ref() {
            Expr::Sequence(items) => match items.last() {
                Some(item) => self.narrow_value_expr(item.expr),
                None => value,
            },
            Expr::TypeAnnotation(annotation) => self.narrow_value_expr(annotation.lhs),
            _ => value,
        }
    }

    /// Recursively collect every concrete [Place] reachable from `typ`
    fn collect_place_atoms(&self, typ: &Type, out: &mut Vec<Place>) {
        let mut entries = Vec::new();
        self.tc.collect_place_entries(typ, &mut entries);
        out.extend(entries.into_iter().filter_map(PlaceEntry::concrete));
    }

    /// Record that `mutated` was mutated at `locator`
    fn record_mutation(&mut self, mutated: Vec<PlaceEntry>, locator: impl Locateable) {
        if !mutated.is_empty() {
            let location = locator.locate(self.tc);
            self.moves.record_mutation(mutated, location);
        }
    }

    /// The places in each `Mutate` effect calling `function` performs
    fn call_mutated_places(&self, function: ExprId) -> Vec<PlaceEntry> {
        match self.tc.expr_types.get(&function).map(|typ| typ.follow(&self.tc.bindings)) {
            Some(Type::Function(function)) => self.tc.mutated_entries(&function.effects),
            _ => Vec::new(),
        }
    }

    /// Error if a place `name` mutates is instantiated at `path` to one overlapping another of its place generics
    fn check_mutated_places_distinct(&mut self, path: PathId, name: TopLevelName) {
        let Some(instantiation) = self.tc.current_extended_context().get_instantiation(path) else { return };
        let Type::Forall(generics, body) = GetType(name).get(self.tc.compiler) else { return };
        let Type::Function(function) = body.as_ref() else { return };
        let substitutions: GenericSubstitutions = generics.iter().copied().zip(instantiation.iter().cloned()).collect();

        let mut mutated = Vec::new();
        let mut effect_mutations = Vec::new();
        for effect in self.tc.collect_effects(&function.effects, &TypeBindings::default()) {
            match (self.tc.mutated_places(&effect), effect) {
                (Some(places), _) => self.collect_place_generics_in_row(&places, &mut mutated),
                (None, Type::Generic(generic)) => {
                    let places = self.tc.mutated_entries(&substitutions[&generic]);
                    if !places.is_empty() {
                        effect_mutations.push((generic, places));
                    }
                },
                (None, _) => (),
            }
        }
        if mutated.is_empty() && effect_mutations.is_empty() {
            return;
        }

        let mut place_generics = Vec::new();
        for parameter in &function.parameters {
            self.collect_place_generics(&parameter.typ, &mut place_generics);
        }
        self.collect_place_generics(&function.return_type, &mut place_generics);
        let place_generics =
            mapvec(place_generics, |generic| (generic, self.tc.place_entries(&substitutions[&generic])));

        let (_, names) = GetItem(name.top_level_item).get(self.tc.compiler);
        let named = |generic| match generic {
            Generic::Named(Origin::Local(name)) => Some(name),
            _ => None,
        };
        let place_name =
            |generic| named(generic).map_or("an inferred place".to_string(), |name| format!("'{}", names[name]));
        let effect_name = |generic| {
            let name = named(generic).filter(|name| !names.is_synthetic_name(*name));
            name.map_or("its arguments".to_string(), |name| names[name].to_string())
        };
        let function_name = &names[name.local_name_id];

        for (i, (first, first_places)) in place_generics.iter().enumerate() {
            for (second, second_places) in &place_generics[i + 1..] {
                if (mutated.contains(first) || mutated.contains(second))
                    && let Some(place) = overlapping(first_places, second_places)
                {
                    self.report_alias(function_name, place_name(*first), place_name(*second), place, path);
                }
            }

            for (effect, mutated) in &effect_mutations {
                if let Some(place) = overlapping(mutated, first_places) {
                    let effect = format!("a place mutated by {}", effect_name(*effect));
                    self.report_alias(function_name, place_name(*first), effect, place, path);
                }
            }
        }
    }

    fn report_alias(
        &self, function: &str, first: String, second: String, place: &PlaceEntry, locator: impl Locateable,
    ) {
        let function = function.to_string();
        let place = self.entry_display_name(place);
        let location = locator.locate(self.tc);
        self.tc.compiler.accumulate(Diagnostic::PlacesMayAlias { function, first, second, place, location });
    }

    /// The name of a place in the current item for error messages, if it has one
    fn entry_display_name(&self, place: &PlaceEntry) -> Option<String> {
        let names = self.tc.current_extended_context();
        let root = match place.root {
            EntryRoot::Concrete(root) => Place { root, segments: Vec::new() }.display_name(names),
            EntryRoot::Generic(Generic::Named(Origin::Local(name))) => names.try_get_name(name)?.to_string(),
            EntryRoot::Generic(_) | EntryRoot::Unbound(_) => return None,
        };
        Some(format!("'{root}{}", cst::Segments(&place.segments)))
    }

    /// Collect each generic used as a place within the generalized type `typ`, except in function parameters
    fn collect_place_generics(&self, typ: &Type, out: &mut Vec<Generic>) {
        match typ {
            Type::Application(constructor, args) => {
                let place_parameters = match constructor.as_ref() {
                    Type::Primitive(PrimitiveType::Reference(_)) => vec![true, false],
                    Type::UserDefined(Origin::TopLevelDefinition(name)) => {
                        let (item, _) = GetItem(name.top_level_item).get(self.tc.compiler);
                        match &item.kind {
                            cst::TopLevelItemKind::TypeDefinition(definition) => {
                                mapvec(&definition.generics, |param| param.kind == Some(KindAnnotation::Place))
                            },
                            _ => Vec::new(),
                        }
                    },
                    _ => Vec::new(),
                };
                for (i, arg) in args.iter().enumerate() {
                    if place_parameters.get(i).copied().unwrap_or(false) {
                        self.collect_place_generics_in_row(arg, out);
                    } else {
                        self.collect_place_generics(arg, out);
                    }
                }
            },
            Type::Places(_) | Type::Place(_) | Type::PlaceProjection(..) => {
                self.collect_place_generics_in_row(typ, out)
            },
            // A function parameter's places are given by the callee rather than to it
            Type::Function(function) => {
                self.collect_place_generics(&function.environment, out);
                self.collect_place_generics(&function.return_type, out);
                self.collect_place_generics(&function.effects, out);
            },
            Type::Tuple(elements) => {
                for element in elements.iter() {
                    self.collect_place_generics(element, out);
                }
            },
            Type::Effects(Some(effects)) => {
                for effect in effects.iter() {
                    self.collect_place_generics(effect, out);
                }
            },
            Type::Forall(_, typ) => self.collect_place_generics(typ, out),
            Type::Effects(None)
            | Type::Primitive(_)
            | Type::Generic(_)
            | Type::Variable(_)
            | Type::UserDefined(_)
            | Type::U32(_) => (),
        }
    }

    /// Collect each generic in the generalized places row `places`
    fn collect_place_generics_in_row(&self, places: &Type, out: &mut Vec<Generic>) {
        for entry in self.tc.place_entries(places) {
            if let EntryRoot::Generic(generic) = entry.root
                && !out.contains(&generic)
            {
                out.push(generic);
            }
        }
    }

    /// Check that no place `typ` may refer to was moved, or mutated since `holder` was assigned
    fn check_referents(&mut self, typ: &Type, locator: impl Locateable, holder: Option<&Place>) {
        let mut entries = Vec::new();
        self.tc.collect_place_entries(typ, &mut entries);

        for entry in &entries {
            let EntryRoot::Concrete(root @ PlaceRoot::Variable(_)) = entry.root else { continue };
            let path = Place { root, segments: entry.segments.clone() }.stable_prefix();

            if let Some((moved_path, moved_in)) = self.moves.moved(&path)
                && !self.moves.errored.contains(moved_path)
            {
                let (moved_path, moved_in) = (moved_path.clone(), moved_in.clone());
                let name = moved_path.display_name(self.tc.current_extended_context());
                let location = locator.locate(self.tc);
                self.tc.compiler.accumulate(Diagnostic::ReferenceToMovedValue { name, location, moved_in });
                self.moves.errored.insert(moved_path);
            }
        }

        let unstable: Vec<_> = entries.into_iter().filter(PlaceEntry::is_unstable).collect();
        let Some(holder) = holder.filter(|_| !unstable.is_empty()) else { return };
        let location = locator.locate(self.tc);

        if let Some((name, invalidated_at)) = self.invalidation(holder, &unstable) {
            let location = location.clone();
            self.tc.compiler.accumulate(Diagnostic::UseOfInvalidatedReference { name, location, invalidated_at });
        }

        if let Some(uses) = self.uses.last_mut() {
            uses.entry(holder.clone()).or_insert((location, unstable));
        }
    }

    /// The name of `holder` and where it was invalidated, if a mutation since it was assigned invalidates `entries`
    fn invalidation(&mut self, holder: &Place, entries: &[PlaceEntry]) -> Option<(String, Location)> {
        let invalidated_at = self.moves.invalidated(holder, entries)?.clone();
        let names = self.tc.current_extended_context();
        self.moves.errored.insert(holder.clone()).then(|| (holder.display_name(names), invalidated_at))
    }

    /// Check if using `path` is valid (not already moved or partially moved).
    /// Emits a diagnostic if the path was already moved.
    /// Only emits the first error per path to avoid noisy duplicate diagnostics.
    fn check_use_of_move_path(&mut self, path: &Place, locator: impl Locateable) {
        if self.moves.errored.contains(path) {
            return;
        }

        if let Some((_, moved_in)) = self.moves.moved(path) {
            let name = path.display_name(self.tc.current_extended_context());
            let location = locator.locate(self.tc);
            let moved_in = moved_in.clone();
            self.tc.compiler.accumulate(Diagnostic::UseOfMovedValue { name, location, moved_in });
            self.moves.errored.insert(path.clone());
        }
    }

    /// Emit errors for any variable declared outside `body_scope` that was moved within it
    fn check_moves_in_repeated_context(
        &mut self, body_scope: &MoveScope, body_depth: ScopeDepth, context: RepeatedContext,
    ) {
        let moved =
            body_scope.moved_paths().filter(|(path, _)| self.atom_depth(path).is_some_and(|depth| depth < body_depth));
        let outer_moves = mapvec(moved, |(path, location)| (path.clone(), location.clone()));

        for (path, location) in outer_moves {
            let name = path.display_name(self.tc.current_extended_context());
            self.tc.compiler.accumulate(Diagnostic::MoveInRepeatedContext { name, context, location });
        }
    }

    /// Emit errors for any outer holder the current body uses then may invalidate before running again
    fn check_invalidations_in_repeated_context(
        &mut self, uses: &Uses, body_depth: ScopeDepth, context: RepeatedContext,
    ) {
        for (holder, (location, entries)) in uses {
            if self.atom_depth(holder).is_none_or(|depth| depth >= body_depth) {
                continue;
            }
            if let Some((name, invalidated_at)) = self.invalidation(holder, entries) {
                let location = location.clone();
                let diagnostic = Diagnostic::InvalidatedInRepeatedContext { name, context, location, invalidated_at };
                self.tc.compiler.accumulate(diagnostic);
            }
        }
    }

    /// A `move` closure captures its free variables by value. Record a move for each
    /// non-Copy free variable against the outer move tracker.
    fn record_move_captures(&mut self, id: ExprId, self_name: Option<NameId>) {
        let mut context = FreeVars::default();
        if let Some(name) = self_name {
            context.defined_in_fn.insert(name);
        }
        context.find_free_variables(id, self.tc);

        let location = id.locate(self.tc);
        for name in &context.free_vars {
            let typ = self.tc.name_types[name].clone();
            if !self.tc.expr_is_copy(context.use_sites[name], &typ) {
                let move_path = self.tc.binding_place(*name);
                self.moves.record_move(move_path, location.clone());
            }
        }
    }

    fn move_path_of_assignment_lhs(&self, lhs: ExprId) -> Option<Place> {
        match self.tc.resolved_expr(lhs).as_ref() {
            Expr::Reference(reference) => self.tc.try_build_move_path(reference.rhs),
            _ => self.tc.try_build_move_path(lhs),
        }
    }

    fn walk_variable(&mut self, path: PathId, expr: ExprId, check_move: bool, record_move: bool) {
        let name = match self.tc.path_origin(path) {
            Some(Origin::Local(name)) => name,
            Some(Origin::TopLevelDefinition(name)) => return self.check_mutated_places_distinct(path, name),
            _ => return,
        };

        let move_path = self.tc.binding_place(name);
        let typ = self.tc.path_types[&path].clone();
        if check_move {
            self.check_use_of_move_path(&move_path, path);
            self.check_referents(&typ, path, Some(&move_path));
        }

        if record_move && !self.tc.expr_is_copy(expr, &typ) {
            let location = path.locate(self.tc);
            self.moves.record_move(move_path, location);
        }
    }

    /// The field path move check for a member access. `access.object` is walked separately, with moves
    /// fully suppressed.
    fn check_member_access_move(
        &mut self, access: &cst::MemberAccess, expr: ExprId, check_move: bool, record_move: bool,
    ) {
        let struct_type = self.tc.expr_types[&access.object].clone();
        let indirect = struct_type.reference_element(&self.tc.bindings).is_some()
            || struct_type.pointer_element(&self.tc.bindings).is_some();

        // A field reached through a reference is held by the object rather than the field
        let parent_path = self.tc.try_build_move_path(access.object);
        let holder = parent_path.map(|parent| if indirect { parent } else { parent.field(access.member.clone()) });

        if check_move {
            let typ = self.tc.expr_types[&expr].clone();
            self.check_referents(&typ, expr, holder.as_ref());
        }

        let Some(move_path) = holder.filter(|_| !indirect) else { return };
        if check_move {
            self.check_use_of_move_path(&move_path, expr);
        }

        if record_move {
            let field_type = self.tc.expr_types[&expr].clone();
            if !self.tc.expr_is_copy(expr, &field_type) {
                let location = expr.locate(self.tc);
                self.moves.record_move(move_path, location);
            }
        }
    }

    fn walk_if(&mut self, if_: &cst::If, depth: ScopeDepth, check_move: bool, record_move: bool) {
        self.walk_expr(if_.condition, depth, check_move, record_move);
        let new_depth = depth.deeper();

        self.moves.push_scope();
        self.walk_and_check_escapes(if_.then, new_depth, check_move, record_move);
        let then_scope = self.moves.pop_scope();
        let then_type = self.tc.expr_types[&if_.then].clone();
        let then_scope = (!self.tc.diverges(&then_type)).then_some(then_scope);

        let else_scope = match if_.else_ {
            Some(else_) => {
                self.moves.push_scope();
                self.walk_and_check_escapes(else_, new_depth, check_move, record_move);
                let else_scope = self.moves.pop_scope();
                let else_type = self.tc.expr_types[&else_].clone();
                (!self.tc.diverges(&else_type)).then_some(else_scope)
            },
            None => Some(MoveScope::default()),
        };

        let branches = then_scope.into_iter().chain(else_scope).collect();
        self.moves.merge_branches_into_parent(branches);
    }

    fn walk_match(&mut self, match_: &cst::Match, depth: ScopeDepth, check_move: bool, record_move: bool) {
        // `infer_match` pushes an implicits scope before checking the expression, so its
        // real scope depth is one deeper than the match's own
        let new_depth = depth.deeper();

        // Only patterns which bind part of the pattern move it, e.g. `x is None` does not move `x`
        let binds = match_.cases.iter().any(|(pattern, _)| self.pattern_binds_name(*pattern));
        self.walk_expr(match_.expression, new_depth, check_move, record_move && binds);

        let branch_depth = new_depth.deeper();
        let mut branches = Vec::with_capacity(match_.cases.len());
        for (pattern, branch) in &match_.cases {
            self.moves.push_scope();
            self.record_name_depths_in_pattern(*pattern, branch_depth);
            self.walk_and_check_escapes(*branch, branch_depth, check_move, record_move);
            branches.push(self.moves.pop_scope());
        }
        self.moves.merge_branches_into_parent(branches);
    }

    fn walk_while(&mut self, while_: &cst::While, depth: ScopeDepth, check_move: bool, record_move: bool) {
        let body_depth = depth.deeper();
        self.enter_body();
        self.walk_expr(while_.condition, depth, check_move, record_move);
        self.walk_and_check_escapes(while_.body, body_depth, check_move, record_move);
        self.finish_body(body_depth, Some(RepeatedContext::WhileLoop));
    }

    fn walk_for(&mut self, for_: &cst::For, depth: ScopeDepth, check_move: bool, record_move: bool) {
        self.walk_expr(for_.start, depth, check_move, record_move);
        self.walk_expr(for_.end, depth, check_move, record_move);

        let body_depth = depth.deeper();
        self.bind_name(for_.variable, body_depth);

        self.enter_body();
        self.walk_and_check_escapes(for_.body, body_depth, check_move, record_move);
        self.finish_body(body_depth, Some(RepeatedContext::ForLoop));
    }

    /// Track a body's moves, mutations, and uses of unstable references
    fn enter_body(&mut self) {
        self.moves.push_scope();
        self.uses.push(FxHashMap::default());
    }

    /// Check what a body moves or invalidates outside it, erroring if `context` says it may run again
    fn finish_body(&mut self, body_depth: ScopeDepth, context: Option<RepeatedContext>) {
        let uses = self.uses.pop().unwrap();
        if let Some(context) = context {
            self.check_invalidations_in_repeated_context(&uses, body_depth, context);
        }

        let body_scope = self.moves.pop_scope();
        // A closure's body runs when it is called rather than where it is written
        if context != Some(RepeatedContext::Closure) {
            if let Some(context) = context {
                self.check_moves_in_repeated_context(&body_scope, body_depth, context);
            }
            self.moves.absorb_mutations(body_scope);
        }

        if let Some(parent) = self.uses.last_mut() {
            for (holder, used) in uses {
                parent.entry(holder).or_insert(used);
            }
        }
    }

    fn walk_assignment(
        &mut self, assignment: &cst::Assignment, expr: ExprId, depth: ScopeDepth, check_move: bool, record_move: bool,
    ) {
        // Allow `x := v` to use `x` even if moved, but `x += v` cannot since it reads `x`.
        // Writing through an existing reference also uses that reference.
        if assignment.op.is_some() {
            self.walk_expr(assignment.lhs, depth, check_move, record_move);
        } else {
            let writes_through = !matches!(self.tc.resolved_expr(assignment.lhs).as_ref(), Expr::Reference(_));
            self.walk_expr(assignment.lhs, depth, check_move && writes_through, false);
        }
        self.walk_expr(assignment.rhs, depth, check_move, record_move);
        if let Some((_, op_expr)) = assignment.op {
            self.walk_expr(op_expr, depth, check_move, record_move);
        }

        let target_depth = self.assignment_target_depth(assignment.lhs).unwrap_or(ScopeDepth(0));
        self.check_escapes(assignment.rhs, target_depth.deeper());

        if let Some(mutated) = self.tc.assignment_mutated_places(assignment.lhs) {
            let mutated = self.tc.place_entries(&mutated);
            self.record_mutation(mutated, expr);
        }

        // The LHS always holds a value after an assignment.
        if let Some(path) = self.move_path_of_assignment_lhs(assignment.lhs) {
            self.moves.clear_moves(&path);
        }
    }

    /// Both the handled body and each branch are lambdas, so both go through
    /// `walk_lambda` which will record that branch's `resume` parameter
    /// into `name_depths` so we don't need to do so here.
    fn walk_handle(&mut self, handle: &cst::Handle, depth: ScopeDepth) {
        self.walk_lambda(handle.expression, depth, Some(handle.handler_name), None);

        for (pattern, branch) in &handle.cases {
            // Only allow moving variables into this branch if `resume` is never mentioned.
            // This notably keeps handlers like `try_or` working.
            let uses_resume = self.tc.handler_branch_uses_resume(pattern.resume_name, *branch);
            let repeated_context = uses_resume.then_some(RepeatedContext::HandlerBranch);
            self.walk_lambda(*branch, depth, None, repeated_context);
        }
    }

    fn walk_lambda(
        &mut self, lambda_expr: ExprId, depth: ScopeDepth, self_name: Option<NameId>, context: Option<RepeatedContext>,
    ) {
        let Expr::Lambda(lambda) = self.tc.resolved_expr(lambda_expr).into_owned() else {
            unreachable!("walk_lambda called on a non-lambda expr")
        };

        let body_depth = depth.deeper();
        for parameter in &lambda.parameters {
            self.record_name_depths_in_pattern(parameter.pattern, body_depth);
        }

        let old_function_depth = self.function_depth.replace(body_depth);
        self.enter_body();
        self.walk_and_check_escapes(lambda.body, body_depth, true, true);
        self.function_depth = old_function_depth;
        self.finish_body(body_depth, context);

        if lambda.is_move {
            self.record_move_captures(lambda_expr, self_name);
        }
    }
}

/// True if every entry of the canonical row `row` is in `target`, as when `row` flowed into it
fn is_subrow(row: &[Type], target: &[Type]) -> bool {
    row.iter().all(|entry| target.contains(entry))
}

/// The first use of each holder of unstable references in a body, and those references
type Uses = FxHashMap<Place, (Location, Vec<PlaceEntry>)>;

/// An entry of `a` overlapping one of `b`
fn overlapping<'a>(a: &'a [PlaceEntry], b: &[PlaceEntry]) -> Option<&'a PlaceEntry> {
    a.iter().find(|a| b.iter().any(|b| a.overlaps(b)))
}
