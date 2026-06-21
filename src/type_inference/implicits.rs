use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::Arc;

use crate::{
    diagnostics::{Diagnostic, ImportSuggestion, Location},
    incremental::{ExportedDefinitions, GetCrateGraph, GetItem, VisibleImplicits},
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, Integer, IntegerKind},
    name_resolution::{Origin, namespace::CrateId},
    parser::{
        cst::{self, Name, Pattern, TopLevelItemKind},
        ids::{ExprId, NameId, PatternId},
    },
    type_inference::{
        Locateable, TypeChecker,
        Variance::Covariant,
        errors::TypeErrorKind,
        types::{FunctionType, ParameterType, PrimitiveType, Type, TypeBindings, TypeVariableId},
    },
};

/// Any more than this arbitrary value and we stop looking for impls to populate the error
/// message with and instead display `..`.
const MULTIPLE_MATCHING_IMPLS_CUTOFF: usize = 5;

#[derive(Clone)]
struct DeferredClosureCheck {
    lambda: ExprId,
    environment: Type,
    self_name: Option<NameId>,
    is_move: bool,
}

#[derive(Default, Clone)]
pub(super) struct ImplicitsContext {
    /// Any implicits introduced in the current scope. To find all implicits in scope, it is
    /// necessary to traverse all levels of `TypeChecker::implicits`, in addition to querying
    /// implicits in global scope separately.
    implicits_in_scope: Vec<NameId>,

    /// Contains implicits for which we need to delay checking for a value for until the end of the
    /// current item when more types are inferred. Without this, for example, we'd see `0i32 < 3`
    /// and would fail searching for an implicit for `Cmp _` since we'd check `<` before its
    /// arguments while its type is still unknown.
    delayed_implicits: Vec<DelayedImplicit>,

    /// Closure checks deferred for coercion wrapper lambdas or lambdas whose implicit scope was
    /// delayed. These are run after `delayed_implicits` are resolved so that free-variable analysis
    /// sees the freshly added implicit arguments.
    deferred_closure_checks: Vec<DeferredClosureCheck>,

    /// Any type variables created for integer literals for polymorphic integer types.
    /// If not bound by the end of a scope they will be defaulted to I32.
    /// This is a tuple of (the integer's value, the integer type variable, location to use for errors)
    integer_type_variables: Vec<(Integer, TypeVariableId, Location)>,

    /// Similar to polymorphic integers, we track polymorphic floats as well. Their value is not stored
    /// since we do not check if the float value fits in the resulting type.
    float_type_variables: Vec<(TypeVariableId, Location)>,
}

#[derive(Clone, Copy)]
struct DelayedImplicit {
    /// The [ExprId] which originally requested an implicit value.
    /// This is often an ability method like `cast` or `+`
    source: ExprId,

    /// The destination to emplace the implicit value into
    destination: ExprId,

    /// The parameter index the implicit should slot into on the `self.source` expr.
    /// Used in error messages.
    parameter_index: usize,
}

impl ImplicitsContext {
    pub(super) fn push_deferred_closure_check(
        &mut self, lambda: ExprId, environment: Type, self_name: Option<NameId>, is_move: bool,
    ) {
        self.deferred_closure_checks.push(DeferredClosureCheck { lambda, environment, self_name, is_move })
    }

    fn extend(&mut self, other: ImplicitsContext) {
        self.implicits_in_scope.extend(other.implicits_in_scope);
        self.delayed_implicits.extend(other.delayed_implicits);
        self.deferred_closure_checks.extend(other.deferred_closure_checks);
        self.integer_type_variables.extend(other.integer_type_variables);
        self.float_type_variables.extend(other.float_type_variables);
    }
}

/// Result of a successful implicit-parameter coercion.
pub(super) enum CoercionKind {
    /// A wrapper lambda expression was produced and should be inserted at the function's ExprId.
    Wrapper(cst::Expr),
    /// The enclosing Call had its arguments rewritten in place; the function expression is
    /// unchanged and should not be re-checked against the reduced expected type.
    DirectCallInsertion,
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Perform an implicit parameter coercion.
    ///
    /// Given a function `expr` which requires some implicit parameters present in the `actual`
    /// type but not the `expected` type, find values for those implicits (issuing errors for any
    /// that cannot be found).
    ///
    /// If the function is immediately used in a Call (`call` is `Some`), the resolved implicit
    /// arguments are spliced directly into that Call's argument list and this returns
    /// [CoercionKind::DirectCallInsertion]. Otherwise a new wrapper lambda is created, e.g.:
    ///
    /// ```ante
    /// fn a c -> <expr> a <new-implicit> c
    /// ```
    ///
    /// In the case a matching implicit value cannot be found, an error is issued and an error
    /// expression is slotted in as the argument instead.
    pub(super) fn implicit_parameter_coercion(
        &mut self, actual: Arc<FunctionType>, expected: Arc<FunctionType>, function: ExprId, call: Option<ExprId>,
    ) -> Option<CoercionKind> {
        // Looking for implicit parameters that are in `actual` but not `expected`.
        // The reverse would be a type error.
        let mut new_expected = Vec::new();

        let actual_params = actual.parameters.iter();
        let mut expected_params = expected.parameters.iter().cloned();
        let mut current_expected = expected_params.next();

        // For each parameter, this is either `None` if no new implicit was inserted
        // at that position, or it is `Some(expr_id)` of the new expression.
        let mut implicits_added = Vec::new();

        for actual in actual_params {
            match (actual.is_implicit, current_expected.as_ref()) {
                // actual is implicit, but expected isn't, search for an implicit in scope
                (true, expected) if expected.is_none_or(|param| !param.is_implicit) => {
                    let value = self.delay_find_implicit_value(&actual.typ, new_expected.len(), function, call);
                    implicits_added.push(Some(value));
                    new_expected.push(ParameterType::implicit(self.expr_types[&value].clone()));
                },
                _ => {
                    let Some(expected) = current_expected else {
                        // User underprovided explicit args - avoid OOB indexing into the call's args.
                        return None;
                    };
                    new_expected.push(expected);
                    implicits_added.push(None);
                    current_expected = expected_params.next();
                },
            }
        }

        if let Some(call_expr) = call {
            // Only rewrite the call if we actually inserted an implicit.
            if !implicits_added.iter().any(|param| param.is_some()) {
                return None;
            }

            // Allow `foo ()` to call an implicit-only function by dropping the trailing `()`.
            if current_expected.is_some() && !self.drop_trailing_unit_arg(call_expr) {
                return None;
            }

            let new_fn = Type::Function(Arc::new(FunctionType {
                parameters: new_expected,
                environment: expected.environment.clone(),
                return_type: expected.return_type.clone(),
            }));
            self.unify(&Type::Function(actual), &new_fn, TypeErrorKind::General, function);

            Some(CoercionKind::DirectCallInsertion)
        } else {
            self.create_closure_wrapper_for_implicit(function, implicits_added, new_expected).map(CoercionKind::Wrapper)
        }
    }

    /// Remove the last argument of the call if it is a unit literal, returning true if one was removed.
    fn drop_trailing_unit_arg(&mut self, call_expr: ExprId) -> bool {
        if !self.call_ends_with_unit_arg(call_expr) {
            return false;
        }
        if let Some(cst::Expr::Call(call)) = self.current_extended_context_mut().extended_expr_mut(call_expr) {
            call.arguments.truncate(call.arguments.len() - 1);
            true
        } else {
            false
        }
    }

    pub(super) fn call_ends_with_unit_arg(&self, call_expr: ExprId) -> bool {
        let expr = match self.current_extended_context().extended_expr(call_expr) {
            Some(expr) => expr,
            None => &self.current_context()[call_expr],
        };
        let cst::Expr::Call(call) = expr else { return false };
        call.arguments.last().is_some_and(|arg| self.is_unit_literal(arg.expr))
    }

    fn is_unit_literal(&self, expr: ExprId) -> bool {
        let expr = match self.current_extended_context().extended_expr(expr) {
            Some(expr) => expr,
            None => &self.current_context()[expr],
        };
        matches!(expr, cst::Expr::Literal(cst::Literal::Unit))
    }

    /// If the expression is a variable, return its name
    fn try_get_name(&self, expr: ExprId) -> Option<String> {
        match &self.current_extended_context()[expr] {
            cst::Expr::Variable(path) => Some(self.current_extended_context()[*path].last_ident().to_string()),
            _ => None,
        }
    }

    pub(super) fn push_implicits_scope(&mut self) {
        self.implicits.push(Default::default());
    }

    /// Pop an implicit scope:
    /// - Removes implicits in the current scope from being used by outer scopes
    /// - Solves any implicits queued in the current scope
    /// - Defaults any integer type variables to I32 that are still unbound in the current scope
    /// - Runs any deferred closure free variable checks after adding implicit arguments
    /// Returns `true` if the scope was delayed (extended into the parent) rather than resolved now.
    pub(super) fn pop_implicits_scope(&mut self) -> bool {
        let mut scope = self.implicits.pop().expect("More pops than pushes to `TypeChecker::implicits`");

        // If there are no implicits defined in the current scope, delay checking them until later
        // so we get as much type information as possible. This particularly helps for polymorphic
        // integer literals.
        if scope.implicits_in_scope.is_empty()
            && let Some(top) = self.implicits.last_mut()
        {
            let has_delayed_implicits = !scope.delayed_implicits.is_empty();
            top.extend(scope);
            return has_delayed_implicits;
        }

        let mut any_bubbled = false;

        // The scope has local implicits. Partition delayed implicits so ones that can't possibly
        // bind to a local name are deferred to the parent. Then transitively pull along any
        // bubbled candidate that shares a type variable with a staying implicit's target type:
        // resolving it at this scope is required to commit the bindings the staying implicit
        // depends on (e.g. `print v.data.[i]` keeps `Print u` here, and `Extract (Ptr t) Usz u`
        // shares `u` so it must also resolve here to bind `u` before the `Print` phase-2 retry).
        if !self.implicits.is_empty() {
            let local_implicits = scope.implicits_in_scope.clone();
            let mut stays_here = Vec::new();
            let mut candidates_to_bubble = Vec::new();
            for implicit in std::mem::take(&mut scope.delayed_implicits) {
                if self.delayed_implicit_could_match_local(&implicit, &local_implicits) {
                    stays_here.push(implicit);
                } else {
                    candidates_to_bubble.push(implicit);
                }
            }

            let mut kept_vars = stays_here
                .iter()
                .flat_map(|implicit| self.expr_types[&implicit.destination].free_vars(&self.bindings))
                .filter_map(|generic| generic.as_inferred())
                .collect::<HashSet<_>>();

            let bubbles_up = self.pull_transitive_implicits(&mut stays_here, candidates_to_bubble, &mut kept_vars);

            if stays_here.is_empty() {
                let parent = self.implicits.last_mut().unwrap();
                scope.delayed_implicits = bubbles_up;
                scope.implicits_in_scope.clear(); // local names die with this scope, must not leak up
                parent.extend(scope);
                return true;
            }

            // Keep here only int/float vars whose id appears in a kept implicit's target. The
            // rest bubble up so defaulting waits for more type info.
            let ints = std::mem::take(&mut scope.integer_type_variables);
            let (keep_ints, bubble_ints) = self.partition_by_target_tvar(ints, &kept_vars, |(_, tvar, _)| *tvar);

            let floats = std::mem::take(&mut scope.float_type_variables);
            let (keep_floats, bubble_floats) = self.partition_by_target_tvar(floats, &kept_vars, |(tvar, _)| *tvar);

            any_bubbled = !bubbles_up.is_empty() || !bubble_ints.is_empty() || !bubble_floats.is_empty();

            let parent = self.implicits.last_mut().unwrap();
            parent.delayed_implicits.extend(bubbles_up);
            parent.integer_type_variables.extend(bubble_ints);
            parent.float_type_variables.extend(bubble_floats);
            if any_bubbled {
                parent.deferred_closure_checks.append(&mut scope.deferred_closure_checks);
            }

            scope.delayed_implicits = stays_here;
            scope.integer_type_variables = keep_ints;
            scope.float_type_variables = keep_floats;
        }

        self.resolve_scope(scope);
        any_bubbled
    }

    /// Move bubble candidates sharing a type variable with a staying target into `stays_here` to a fixpoint,
    /// returning the candidates to be bubbled up.
    fn pull_transitive_implicits(
        &mut self, stays_here: &mut Vec<DelayedImplicit>, mut candidates_to_bubble: Vec<DelayedImplicit>,
        vars_to_keep: &mut HashSet<TypeVariableId>,
    ) -> Vec<DelayedImplicit> {
        loop {
            let before = candidates_to_bubble.len();
            let mut i = 0;
            while i < candidates_to_bubble.len() {
                let implicit = candidates_to_bubble[i];
                let target = self.expr_types[&implicit.destination].clone().follow_all(&self.bindings);
                let free_vars = target.free_unification_vars(&self.bindings);

                if free_vars.iter().any(|tv| vars_to_keep.contains(tv)) {
                    vars_to_keep.extend(free_vars);
                    stays_here.push(implicit);
                    candidates_to_bubble.swap_remove(i);
                } else {
                    i += 1;
                }
            }
            if candidates_to_bubble.len() == before {
                return candidates_to_bubble;
            }
        }
    }

    /// Partition into (keep here, bubble up) by whether each entry's tvar is in `keep_tvars`. An
    /// already-bound tvar is kept since its type is settled and delaying it gains nothing.
    fn partition_by_target_tvar<T>(
        &self, entries: Vec<T>, vars_to_keep: &HashSet<TypeVariableId>, get_var: impl Fn(&T) -> TypeVariableId,
    ) -> (Vec<T>, Vec<T>) {
        entries.into_iter().partition(|entry| match Type::Variable(get_var(entry)).follow(&self.bindings) {
            Type::Variable(id) => vars_to_keep.contains(id),
            _ => true,
        })
    }

    /// Resolve everything queued in `scope` while it is still visible in `self.implicits`, then pop.
    fn resolve_scope(&mut self, scope: ImplicitsContext) {
        // Queued requests query all of `self.implicits`, so keep the scope on the stack for now
        self.implicits.push(scope);

        // We must perform any queued requests before popping any implicits which should be visible
        let scope = self.implicits.last_mut().unwrap();

        let implicits = std::mem::take(&mut scope.delayed_implicits);
        let closures = std::mem::take(&mut scope.deferred_closure_checks);
        let integers = std::mem::take(&mut scope.integer_type_variables);
        let floats = std::mem::take(&mut scope.float_type_variables);

        // Phase 1: resolve to a fixpoint, since binding one implicit can unblock another.
        // FIXME: Likely performance issue. Remove the fixpoint maybe with eagerly trying each implicit
        // and accepting any regressions.
        let implicits_in_scope = self.collect_implicits_in_scope();

        let mut pending = implicits;
        let failed_implicits = loop {
            let mut still_pending = Vec::new();
            let mut progressed = false;
            for implicit in pending {
                match self.find_implicit_value(implicit, &implicits_in_scope) {
                    Ok(()) => progressed = true,
                    Err(error) => still_pending.push((implicit, error)),
                }
            }
            if !progressed || still_pending.is_empty() {
                break still_pending;
            }
            pending = still_pending.into_iter().map(|(implicit, _)| implicit).collect();
        };

        // Default any still-unbound integers to I32 and ensure their value fits
        // in whatever type they are now.
        for (value, type_variable, location) in integers {
            self.try_default_integer_to_i32(value, type_variable, location);
        }

        for (type_variable, location) in floats {
            self.try_default_float_to_f64(type_variable, location);
        }

        // Phase 2: retry phase 1 failures now that ints & floats are defaulted
        for (implicit, mut original_error) in failed_implicits {
            if self.find_implicit_value(implicit, &implicits_in_scope).is_err() {
                self.try_attach_import_suggestions(&implicit, &mut original_error);
                self.compiler.accumulate(original_error);
            }
        }

        // Run deferred closure checks after all implicits (including retries) are resolved
        // so that free-variable analysis sees the fully-resolved implicit arguments.
        for check in closures {
            self.check_for_closure(check.lambda, &check.environment, check.self_name, check.is_move);
        }

        self.implicits.pop().expect("More pops than pushes to `TypeChecker::implicits`");
    }

    pub(super) fn push_inferred_int(&mut self, value: Integer, type_variable: TypeVariableId, location: Location) {
        self.integer_literal_vars.insert(type_variable);
        self.implicits.last_mut().unwrap().integer_type_variables.push((value, type_variable, location));
    }

    pub(super) fn push_inferred_float(&mut self, type_variable: TypeVariableId, location: Location) {
        self.float_literal_vars.insert(type_variable);
        self.implicits.last_mut().unwrap().float_type_variables.push((type_variable, location));
    }

    /// Check if a type variable is a polymorphic integer literal.
    pub(super) fn is_integer_type_variable(&self, id: TypeVariableId) -> bool {
        self.integer_literal_vars.contains(&id)
    }

    /// Same as [Self::is_integer_type_variable] but for polymorphic float literals.
    pub(super) fn is_float_type_variable(&self, id: TypeVariableId) -> bool {
        self.float_literal_vars.contains(&id)
    }

    /// Delay finding an implicit value until later when more types are known.
    ///
    /// This returns a fresh [ExprId] where the implicit value will be emplaced into when found.
    ///
    /// If `call` is `Some`, an [`cst::Argument::implicit`] referencing the fresh [ExprId] is
    /// spliced into that Call's argument list at `parameter_index`. Because implicits are
    /// inserted in order of `parameter_index` (matching the ordering of `actual.parameters`
    /// traversal in [`Self::implicit_parameter_coercion`]), later insertions see earlier ones
    /// already in place and `parameter_index` is directly the correct insertion position.
    fn delay_find_implicit_value(
        &mut self, target_type: &Type, parameter_index: usize, function: ExprId, call: Option<ExprId>,
    ) -> ExprId {
        let location = function.locate(self);
        let typ = target_type.clone();
        let fresh_id = self.push_expr(cst::Expr::Error, typ, location);
        let delayed = DelayedImplicit { source: function, destination: fresh_id, parameter_index };

        if let Some(call_expr) = call
            && let cst::Expr::Call(mut existing) = self.current_extended_context()[call_expr].clone()
        {
            existing.arguments.insert(parameter_index, cst::Argument::implicit(fresh_id));
            self.current_extended_context_mut().insert_expr(call_expr, cst::Expr::Call(existing));
        }

        // Try to resolve immediately. Slows down type inference but can help it in some cases.
        let in_scope = self.collect_implicits_in_scope();
        if self.find_implicit_value(delayed, &in_scope).is_err() {
            self.implicits.last_mut().unwrap().delayed_implicits.push(delayed);
        }

        fresh_id
    }

    /// Try to default the given integer to an I32, issuing an error if it is bound
    /// to a non-integer type or the literal cannot fit into an I32.
    fn try_default_integer_to_i32(&mut self, value: Integer, type_variable: TypeVariableId, location: Location) {
        let kind = match Type::Variable(type_variable).follow(&self.bindings) {
            Type::Variable(id) => {
                self.bindings.insert(*id, Type::Primitive(PrimitiveType::Int(IntegerKind::I32)));
                IntegerKind::I32
            },
            Type::Primitive(PrimitiveType::Int(kind)) => *kind,
            Type::Primitive(PrimitiveType::Error) => return,
            _ => {
                // Unification rejects non-integer bindings for literal type variables,
                // so this arm is only reachable when the variable was bound to `Never`.
                let actual = self.type_to_string(&Type::Variable(type_variable));
                self.compiler.accumulate(Diagnostic::TypeError {
                    actual,
                    expected: "an integer type".to_string(),
                    kind: TypeErrorKind::General,
                    function_environments_differ: false,
                    location,
                });
                return;
            },
        };

        // Now ensure the literal fits in the chosen kind
        self.check_int_fits(value, kind, location);
    }

    /// Try to default the given float to a F64, issuing an error if it is bound
    /// to a non-float type.
    fn try_default_float_to_f64(&mut self, type_variable: TypeVariableId, location: Location) {
        match Type::Variable(type_variable).follow(&self.bindings) {
            Type::Variable(id) => {
                self.bindings.insert(*id, Type::Primitive(PrimitiveType::Float(FloatKind::F64)));
            },
            Type::Primitive(PrimitiveType::Float(_)) => (),
            Type::Primitive(PrimitiveType::Error) => (),
            _ => {
                // Unification rejects non-float bindings for literal type variables,
                // so this arm is only reachable when the variable was bound to `Never`.
                let actual = self.type_to_string(&Type::Variable(type_variable));
                self.compiler.accumulate(Diagnostic::TypeError {
                    actual,
                    expected: "a float type".to_string(),
                    kind: TypeErrorKind::General,
                    function_environments_differ: false,
                    location,
                });
            },
        }
    }

    /// Collect all implicits in scope into a single Vec
    pub(super) fn collect_implicits_in_scope(&self) -> Vec<NameId> {
        self.implicits.iter().flat_map(|scope| &scope.implicits_in_scope).copied().collect()
    }

    /// The current number of delayed implicits. Used with [Self::resolve_new_delayed_implicits] later
    /// on to resolve a subset of implicits since a given point.
    pub(super) fn delayed_implicits_count(&self) -> usize {
        self.implicits.last().map(|scope| scope.delayed_implicits.len()).unwrap_or_default()
    }

    /// Eagerly resolve the delayed implicits registered in the innermost scope since `previous`
    /// was snapshotted. Implicits that do not resolve to a unique impl now are left to be solved
    /// again on scope's end. No diagnostics are emitted.
    pub(super) fn resolve_new_delayed_implicits(&mut self, previous_length: usize) {
        // TODO: See if we can remove this method when we add eager solving of implicits
        let all = match self.implicits.last_mut() {
            Some(scope) => &mut scope.delayed_implicits,
            None => return,
        };
        let to_resolve = all.drain(previous_length..).collect::<Vec<_>>();
        if to_resolve.is_empty() {
            return;
        }
        let implicits_in_scope = self.collect_implicits_in_scope();
        for implicit in to_resolve {
            if self.find_implicit_value(implicit, &implicits_in_scope).is_err() {
                self.implicits.last_mut().unwrap().delayed_implicits.push(implicit);
            }
        }
    }

    /// Find an implicit value & modify the current cst to insert the implicit if found,
    /// or report an error otherwise.
    ///
    /// Accepts `implicits_in_scope` as a parameter to avoid repeated work collecting it
    /// across nested & repeated calls.
    fn find_implicit_value(
        &mut self, implicit: DelayedImplicit, implicits_in_local_scope: &[NameId],
    ) -> Result<(), Diagnostic> {
        // TODO: This prevents infinite recursion but still slows us down when searching for an implicit
        // N levels deep. We could check for `Type::ERROR` object/implicit types to help catch this early.
        let arbitrary_recursion_limit = 8;
        // The type bindings parameter is for recursive calls when we need to find implicits
        // to slot in for another implicit function's arguments.
        let no_bindings = TypeBindings::default();
        self.find_implicit_value_inner(implicit, implicits_in_local_scope, &no_bindings, arbitrary_recursion_limit)
    }

    fn find_implicit_value_inner(
        &mut self, implicit: DelayedImplicit, implicits_in_local_scope: &[NameId], type_bindings: &TypeBindings,
        fuel: u32,
    ) -> Result<(), Diagnostic> {
        let target_type = self.expr_types[&implicit.destination].clone();
        let target_type = target_type.follow_all_two(&self.bindings, type_bindings);

        let parameter_index = implicit.parameter_index;
        let function = implicit.source;
        let destination = implicit.destination;

        // If every argument of the target type is an unbound type variable, any implicit
        // of the same constructor will match. This happens often when types aren't known
        // so detect this and end early with a helpful error if there is >1 implicit.
        let unbound = Self::type_args_all_unbound(&target_type);

        // Each matching implicit candidate we find. We're hoping this will be exactly 1 item
        let mut candidates = Vec::new();

        for name in implicits_in_local_scope.iter().copied() {
            if candidates.len() > MULTIPLE_MATCHING_IMPLS_CUTOFF {
                // Multiple matching impls, don't waste time looking for more.
                // There are many Eq impls we could waste time on for example.
                break;
            }

            let name_type = self.name_types[&name].follow_two(type_bindings, &self.bindings);

            let origin = Origin::Local(name);
            let name = self.current_extended_context()[name].clone();
            self.check_implicit_candidate(
                name_type,
                None,
                &target_type,
                name,
                origin,
                &mut candidates,
                parameter_index,
                function,
                implicits_in_local_scope,
                type_bindings,
                fuel,
            );
        }

        // Need to check globally visible implicits separately
        if candidates.len() <= MULTIPLE_MATCHING_IMPLS_CUTOFF
            && let Some(item) = self.current_item
        {
            let visible_implicits = VisibleImplicits(item.source_file).get(self.compiler);

            if unbound && !visible_implicits.at_most_1_candidate(&target_type) {
                return Err(self.ambiguous_implicits_error(&target_type, parameter_index, function));
            }

            visible_implicits.iter_possibly_matching_impls(&target_type, |name, name_id| {
                if candidates.len() > MULTIPLE_MATCHING_IMPLS_CUTOFF {
                    // Multiple matching impls, don't waste time looking for more.
                    // There are many Eq impls we could waste time on for example.
                    return true;
                }

                let (name_type, impl_bindings) = self.type_and_bindings_of_top_level_name(name_id);

                let origin = Origin::TopLevelDefinition(*name_id);
                self.check_implicit_candidate(
                    name_type,
                    impl_bindings,
                    &target_type,
                    name.clone(),
                    origin,
                    &mut candidates,
                    parameter_index,
                    function,
                    implicits_in_local_scope,
                    type_bindings,
                    fuel,
                );
                false
            });
        }

        if candidates.is_empty() {
            Err(self.no_implicit_found_error(&target_type, parameter_index, function))
        } else if candidates.len() == 1 {
            let candidate = candidates.remove(0);
            let location = function.locate(self);
            self.create_implicit_argument_expr(candidate, destination, location);
            Ok(())
        } else {
            Err(self.multiple_matching_implicits_error(candidates, &target_type, parameter_index, function))
        }
    }

    /// True if a `Type::Application`'s arguments are all unbound (false for non-type applications)
    fn type_args_all_unbound(target: &Type) -> bool {
        let Some((_ctor, args)) = target.as_application() else { return false };
        args.iter().all(|arg| matches!(arg, Type::Variable(_)))
    }

    /// Check if the given `implicit_type` matches the `target_type` directly, or if it can be
    /// called as a function to produce the target type. If either are true, push the candidate to
    /// the candidates list.
    ///
    /// TODO: Way too many parameters.
    fn check_implicit_candidate(
        &mut self, implicit_type: Type, instantiation_bindings: Option<Vec<Type>>, target_type: &Type, name: Name,
        origin: Origin, candidates: &mut Vec<Candidate>, parameter_index: usize, function: ExprId,
        implicits_in_local_scope: &[NameId], type_bindings: &TypeBindings, fuel: u32,
    ) {
        match self.implicit_type_matches(&implicit_type, target_type, type_bindings) {
            // Prevent infinite recursion
            _ if fuel == 0 => (),
            ImplicitMatch::NoMatch => (),
            ImplicitMatch::MatchedAsIs(type_bindings) => {
                candidates.push(Candidate {
                    name,
                    origin,
                    instantiation_bindings,
                    type_bindings,
                    typ: implicit_type,
                    arguments: Vec::new(),
                });
            },
            ImplicitMatch::Call(function_type, type_bindings) => {
                // TODO: Make this algorithm iterative instead of recursive
                let mut arguments = Vec::new();
                for parameter in &function_type.parameters {
                    if parameter.is_implicit {
                        let arg_type = parameter.typ.clone();
                        let arg_location = function.locate(self);
                        let destination = self.push_expr(cst::Expr::Error, arg_type, arg_location);

                        let implicit = DelayedImplicit { source: function, destination, parameter_index };

                        if self
                            .find_implicit_value_inner(implicit, implicits_in_local_scope, &type_bindings, fuel - 1)
                            .is_ok()
                        {
                            arguments.push(cst::Argument::implicit(destination));
                        }
                    }
                }

                if arguments.len() == function_type.parameters.len() {
                    candidates.push(Candidate {
                        name,
                        origin,
                        instantiation_bindings,
                        type_bindings,
                        typ: implicit_type,
                        arguments,
                    });
                }
            },
        }
    }

    /// Given the type of an implicit value, and the target type to search for, return whether the
    /// given implicit is a match for the target type, whether it can produce such a type by
    /// calling it as a function, or whether there is no match.
    fn implicit_type_matches(
        &mut self, implicit_type: &Type, target_type: &Type, type_bindings: &TypeBindings,
    ) -> ImplicitMatch {
        // TODO: These shouldn't be large in practice (they should be empty unless this is a
        // recursive call) but we should try to reduce the number of clones since this happens for
        // every candidate. Reducing the number of candidates beforehand (e.g. keying them) would also help.
        let mut fresh_bindings = type_bindings.clone();

        if self.subtype(implicit_type, target_type, Covariant, &mut fresh_bindings).is_ok() {
            ImplicitMatch::MatchedAsIs(fresh_bindings)
        } else if let Type::Function(f) = implicit_type {
            let mut fresh_bindings = type_bindings.clone();

            if self.subtype(&f.return_type, target_type, Covariant, &mut fresh_bindings).is_ok() {
                ImplicitMatch::Call(f.clone(), fresh_bindings)
            } else {
                ImplicitMatch::NoMatch
            }
        } else {
            ImplicitMatch::NoMatch
        }
    }

    // error: No implicit found for parameter N of type T
    fn no_implicit_found_error(&self, implicit_type: &Type, parameter_index: usize, function: ExprId) -> Diagnostic {
        let type_string = self.type_to_string(implicit_type);
        let function_name = self.try_get_name(function);
        let location = function.locate(self);
        Diagnostic::NoImplicitFound { type_string, function_name, parameter_index, location, suggestions: Vec::new() }
    }

    // error: Implicit type T is ambiguous, type annotations needed
    fn ambiguous_implicits_error(&self, implicit_type: &Type, parameter_index: usize, function: ExprId) -> Diagnostic {
        let type_string = self.type_to_string(implicit_type);
        let function_name = self.try_get_name(function);
        let location = function.locate(self);
        Diagnostic::AmbiguousImplicit { type_string, function_name, parameter_index, location }
    }

    // error: Multiple implicits found for parameter N of type T
    fn multiple_matching_implicits_error(
        &self, matching: Vec<Candidate>, implicit_type: &Type, parameter_index: usize, function: ExprId,
    ) -> Diagnostic {
        let type_string = self.type_to_string(implicit_type);
        let function_name = self.try_get_name(function);
        let location = function.locate(self);

        let mut matches = mapvec(matching, |candidate| candidate.name);
        if matches.len() > MULTIPLE_MATCHING_IMPLS_CUTOFF {
            matches.truncate(MULTIPLE_MATCHING_IMPLS_CUTOFF);
            matches.push(Arc::new("..".to_string()));
        }

        Diagnostic::MultipleImplicitsFound { matches, type_string, function_name, parameter_index, location }
    }

    /// If `error` is `NoImplicitFound`, populate its `suggestions` field with
    /// out-of-scope implicits that match the missing type. This is done separately
    /// to avoid unnecessary work if the initial error is later fixed by defaulting
    /// integer/float type variables.
    fn try_attach_import_suggestions(&mut self, implicit: &DelayedImplicit, error: &mut Diagnostic) {
        let Diagnostic::NoImplicitFound { suggestions, .. } = error else { return };
        let target = self.expr_types[&implicit.destination].clone();
        let target = target.follow_all_two(&self.bindings, &TypeBindings::default());
        *suggestions = self.collect_import_suggestions(&target);
    }

    /// Search out-of-scope implicits for "did you mean to import X.Y?" hints.
    ///
    /// Excludes unrelated user crates to limit the search space.
    fn collect_import_suggestions(&mut self, target_type: &Type) -> Vec<ImportSuggestion> {
        // TODO: Is the `or N more` at the end of the note message useful?
        const SUGGESTION_CAP: usize = 5;

        let Some(item) = self.current_item else { return Vec::new() };

        // Skip names already visible in the current file so we don't suggest imports the user already has.
        let visible = VisibleImplicits(item.source_file).get(self.compiler);
        let mut skip = BTreeSet::new();
        visible.iter_possibly_matching_impls(target_type, |_, top_level_name| {
            skip.insert(*top_level_name);
            false
        });

        // Candidate crates: stdlib + crates that define types referenced by target_type.
        let mut candidate_crates = BTreeSet::new();
        candidate_crates.insert(CrateId::STDLIB);
        collect_user_defined_crates(target_type, &mut candidate_crates);

        let mut suggestions = Vec::new();
        let crates = GetCrateGraph.get(self.compiler);

        for crate_id in &candidate_crates {
            let Some(crate_) = crates.get(crate_id) else { continue };
            for (rel_path, source_file_id) in &crate_.source_files {
                let exports = ExportedDefinitions(*source_file_id).get(self.compiler);

                for (name, top_level_name) in &exports.definitions {
                    if skip.contains(top_level_name) {
                        continue;
                    }

                    let (item, _ctx) = GetItem(top_level_name.top_level_item).get(self.compiler);
                    let TopLevelItemKind::Definition(definition) = &item.kind else { continue };
                    if !definition.implicit {
                        continue;
                    }

                    let (name_type, _) = self.type_and_bindings_of_top_level_name(top_level_name);

                    if matches!(
                        self.implicit_type_matches(&name_type, target_type, &TypeBindings::default()),
                        ImplicitMatch::NoMatch
                    ) {
                        continue;
                    }

                    suggestions.push(ImportSuggestion {
                        qualified_path: format_module_path(&crate_.name, rel_path, name),
                        location: top_level_name.location(self.compiler),
                    });
                    if suggestions.len() >= SUGGESTION_CAP {
                        return suggestions;
                    }
                }
            }
        }

        suggestions
    }

    /// Probe whether a delayed implicit's target type *could* match at least one of the given
    /// local implicit names. Used by `pop_implicits_scope` to decide whether the implicit must
    /// be resolved at this scope (because a local name is a candidate) or can be bubbled up to
    /// the parent (because no local name could possibly satisfy it).
    fn delayed_implicit_could_match_local(&mut self, implicit: &DelayedImplicit, local_implicits: &[NameId]) -> bool {
        let target_type = self.expr_types[&implicit.destination].clone();
        let target_type = target_type.follow_all(&self.bindings);
        let target_ctor = target_type.as_application().map(|(c, _)| c.clone());
        let no_bindings = TypeBindings::default();

        for name in local_implicits {
            let name_type = self.name_types[name].follow_two(&no_bindings, &self.bindings);
            if !matches!(self.implicit_type_matches(&name_type, &target_type, &no_bindings), ImplicitMatch::NoMatch) {
                return true;
            }
            // Same ability constructor as the target: the local may chain through a global function
            // impl like `eq_ref: {Eq t} -> Eq (ref t)` to reach the target. Bubbling past this
            // scope would drop the local and break the chain.
            if let (Some(tc), Some((lc, _))) = (&target_ctor, name_type.as_application())
                && tc.as_ref() == lc.as_ref()
            {
                return true;
            }
        }
        false
    }

    /// Try to add the given implicit into scope
    pub(super) fn add_implicit(&mut self, id: PatternId) {
        let name = match &self.current_extended_context()[id] {
            Pattern::Error => return,
            Pattern::Variable(name) => *name,
            Pattern::TypeAnnotation(inner_id, _) => return self.add_implicit(*inner_id),
            _ => {
                let location = id.locate(self);
                self.compiler.accumulate(Diagnostic::ImplicitNotAVariable { location });
                return;
            },
        };
        self.add_implicit_name(name);
    }

    /// Add the given implicit into scope
    pub(super) fn add_implicit_name(&mut self, name: NameId) {
        self.implicits.last_mut().unwrap().implicits_in_scope.push(name);
    }

    /// Given:
    /// - A function `f`
    /// - `implicits_added = [None, Some(i), None]` (e.g.)
    /// - `argument_types = [t, u, v]`
    ///
    /// Create:
    /// `fn (a: t) (c: v) -> f a {i} c`
    fn create_closure_wrapper_for_implicit(
        &mut self, function: ExprId, implicits_added: Vec<Option<ExprId>>, argument_types: Vec<ParameterType>,
    ) -> Option<cst::Expr> {
        // We should always have at least 1 added implicit parameter
        let implicit_added = implicits_added.iter().any(|param| param.is_some());

        // A type-error is expected when type checking this call
        if !implicit_added || implicits_added.len() != argument_types.len() {
            return None;
        }

        let mut parameters = Vec::new();
        let mut arguments = Vec::new();

        for (implicit, arg_type) in implicits_added.into_iter().zip(argument_types) {
            match implicit {
                // We want new implicit arguments to be in the call but not the lambda parameters
                Some(arg) => {
                    arguments.push(cst::Argument::implicit(arg));
                },
                None => {
                    let location = function.locate(self);
                    let (var_path, var_name) = self.fresh_variable("p", arg_type.typ.clone(), location.clone());

                    let pattern = self.push_pattern(cst::Pattern::Variable(var_name), location.clone());
                    let expr = self.push_expr(cst::Expr::Variable(var_path), arg_type.typ, location);

                    arguments.push(cst::Argument { is_implicit: arg_type.is_implicit, expr });
                    parameters.push(cst::Parameter::with_implicit(pattern, arg_type.is_implicit));
                },
            }
        }

        // Since `function` is the ExprId we'll be replacing, we can't use it directly here. We
        // have to copy it to a new id.
        let location = function.locate(self);
        let expr = self.current_extended_context()[function].clone();

        // This type should be overwritten later when cst_traversal traverses this new expr
        let function = self.push_expr(expr, Type::ERROR, location.clone());

        let body = cst::Expr::Call(cst::Call { function, arguments });
        let body_type = Type::ERROR;
        let body = self.push_expr(body, body_type, location);

        Some(cst::Expr::Lambda(cst::Lambda { parameters, body, return_type: None, is_move: false }))
    }

    /// Creates a new expression referring to the given implicit value.
    /// - 0 arguments: The expression is a variable
    /// - 1+ arguments: The expression is a function call to the given name, using the given arguments.
    fn create_implicit_argument_expr(&mut self, candidate: Candidate, destination: ExprId, location: Location) {
        let name = candidate.name.as_ref().clone();
        let path = self.push_path(cst::Path::ident(name, location.clone()), candidate.typ.clone(), location.clone());
        let variable = cst::Expr::Variable(path);

        // Commit type bindings from the prior `try_unify` call(s) made to find this implicit
        self.bindings.extend(candidate.type_bindings);

        let context = self.current_extended_context_mut();
        context.insert_path_origin(path, candidate.origin);

        // And remember the generic instantiation (if there was one) of any generic implicits
        // so the mir-builder can pick this up and mark it for monomorphization.
        if let Some(bindings) = candidate.instantiation_bindings {
            context.insert_instantiation(path, bindings);
        }

        let (expr, typ) = if candidate.arguments.is_empty() {
            (variable, candidate.typ)
        } else {
            let return_type = candidate.typ.return_type().unwrap().clone();
            let function = self.push_expr(variable, candidate.typ, location.clone());
            let call = cst::Expr::Call(cst::Call { function, arguments: candidate.arguments });
            (call, return_type)
        };

        self.current_extended_context_mut().insert_expr(destination, expr);
        self.expr_types.insert(destination, typ);
    }
}

/// Candidates when searching for an implicit value.
/// Contains the name, origin, instantiation type bindings, type for the implicit, along with any arguments to
/// call it with (if any) if we should call this implicit for its return value.
struct Candidate {
    name: Name,
    origin: Origin,
    instantiation_bindings: Option<Vec<Type>>,
    typ: Type,

    /// Bindings to commit to the current context on success.
    type_bindings: TypeBindings,

    /// Arguments to call this implicit with (if any) if it is an implicit function
    /// we want to call for its return value.
    arguments: Vec<cst::Argument>,
}

enum ImplicitMatch {
    NoMatch,
    MatchedAsIs(TypeBindings),
    Call(Arc<FunctionType>, TypeBindings),
}

/// Format an import path: `CrateName.module.path.itemName`.
fn format_module_path(crate_name: &str, rel_path: &Path, item_name: &str) -> String {
    let stem = rel_path.with_extension("");
    let dotted = stem.to_string_lossy().replace(std::path::MAIN_SEPARATOR, ".");
    if dotted.is_empty() { format!("{crate_name}.{item_name}") } else { format!("{crate_name}.{dotted}.{item_name}") }
}

/// Walk a type and collect the crate ids of any type it references.
/// references, used to scope an out-of-scope implicit search to relevant crates.
fn collect_user_defined_crates(typ: &Type, out: &mut BTreeSet<CrateId>) {
    // TODO: Should we switch to only collecting source files? This greatly limits
    // the search space, thus improving performance for these errors but also means
    // fewer possibly matching implicits will be found
    match typ {
        Type::UserDefined(Origin::TopLevelDefinition(name)) => {
            out.insert(name.top_level_item.source_file.crate_id);
        },
        Type::Application(ctor, args) => {
            collect_user_defined_crates(ctor, out);
            for arg in args.iter() {
                collect_user_defined_crates(arg, out);
            }
        },
        Type::Function(f) => {
            for p in &f.parameters {
                collect_user_defined_crates(&p.typ, out);
            }
            collect_user_defined_crates(&f.return_type, out);
            collect_user_defined_crates(&f.environment, out);
        },
        Type::Forall(_, t) => collect_user_defined_crates(t, out),
        Type::Tuple(ts) => {
            for t in ts.iter() {
                collect_user_defined_crates(t, out);
            }
        },
        Type::UserDefined(_) | Type::Variable(_) | Type::Generic(_) | Type::Primitive(_) | Type::U32(_) => {},
    }
}
