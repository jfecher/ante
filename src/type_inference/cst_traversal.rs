use std::{borrow::Cow, collections::BTreeMap, sync::Arc};

use rustc_hash::FxHashSet;

use crate::{
    diagnostics::{Diagnostic, Location, RepeatedContext, UnimplementedItem},
    incremental::{AllDefinitions, ExportedDefinitions, GetItemRaw, GetType, Resolve},
    iterator_extensions::mapvec,
    lexer::token::Integer,
    name_resolution::{Origin, builtin::Builtin, namespace::SourceFileId},
    parser::{
        cst::{self, Definition, Expr, Literal, Name, Pattern, ReferenceKind},
        ids::{ExprId, NameId, PathId, PatternId, TopLevelId, TopLevelName},
    },
    type_inference::{
        Locateable, TypeChecker,
        errors::TypeErrorKind,
        get_type::{get_partial_type, try_get_generalized_type},
        types::{self, FunctionType, ParameterType, Type},
    },
};

/// `handle` exprs involve closures with special behavior.
/// `Default` on this is intended to be the default behavior for a non-handle lambda.
#[derive(Default)]
struct LambdaOptions {
    /// When `Some`, the lambda's body is a context that may execute more than
    /// once (e.g. a handler branch), and moves of outer non-Copy variables inside
    /// it should be reported. The set is the names visible before the branch
    /// introduces its own pattern bindings.
    repeated_context: Option<(RepeatedContext, FxHashSet<NameId>)>,
}

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_definition(&mut self, definition: &Definition, is_top_level: bool) {
        let next_id = &mut self.next_type_variable_id.get();
        let expected_type =
            get_partial_type(definition, self.current_context(), &self.current_resolve(), self.compiler, next_id);

        self.next_type_variable_id.set(*next_id);

        self.check_pattern(definition.pattern, &expected_type);

        // Track mutable definitions so closure capture analysis can wrap them in reference types
        if definition.mutable {
            self.record_mutable_pattern(definition.pattern);
        }

        // If the RHS is a lambda, call check_lambda directly so we can pass the definition's
        // own name as `self_name`. This prevents self-recursive local functions (such as the
        // `recur` helper produced by loop desugaring) from treating themselves as a captured
        // free variable.
        let self_name = match &self.current_extended_context()[definition.pattern] {
            Pattern::Variable(name) => Some(*name),
            _ => None,
        };

        let rhs = definition.rhs;
        let rhs_expr = match self.current_extended_context().extended_expr(rhs) {
            Some(e) => Cow::Owned(e.clone()),
            None => Cow::Borrowed(&self.current_context()[rhs]),
        };

        match rhs_expr.as_ref() {
            Expr::Lambda(lambda) => {
                let lambda = lambda.clone();
                self.expr_types.insert(rhs, expected_type.clone());
                self.check_lambda(&lambda, &expected_type, rhs, self_name);
            },
            _ => self.check_expr(rhs, &expected_type),
        }

        if definition.implicit {
            // Local definitions without types are fine, they won't cause globally cascading
            // errors on every implicit search site if their type is too general.
            let has_type = !is_top_level
                || try_get_generalized_type(definition, self.current_context(), self.current_resolve(), self.compiler)
                    .is_some();

            if has_type {
                self.add_implicit(definition.pattern);
            } else {
                let location = definition.pattern.locate(self);
                self.compiler.accumulate(Diagnostic::TopLevelImplicitTypeAnnotationRequired { location });
            }
        }

        self.check_for_main(definition.pattern, &expected_type);
    }

    /// Check an expression's type matches the expected type.
    fn check_expr(&mut self, id: ExprId, expected: &Type) {
        self.check_expr_inner(id, expected, None);
    }

    fn check_expr_inner(&mut self, id: ExprId, expected: &Type, call_expr: Option<ExprId>) {
        self.expr_types.insert(id, expected.clone());

        let expr = match self.current_extended_context().extended_expr(id) {
            Some(expr) => Cow::Owned(expr.clone()),
            None => Cow::Borrowed(&self.current_context()[id]),
        };

        match expr.as_ref() {
            Expr::Literal(literal) => self.check_literal(literal, id, expected),
            Expr::Variable(path) => self.check_path(*path, expected, Some(id), call_expr),
            Expr::Call(call) => self.check_call(call, expected, id),
            Expr::Lambda(lambda) => self.check_lambda(lambda, expected, id, None),
            Expr::Sequence(items) => {
                self.push_implicits_scope();
                for (i, item) in items.iter().enumerate() {
                    let expected_type = if i == items.len() - 1 { expected } else { &self.next_type_variable() };
                    self.check_expr(item.expr, expected_type);
                }
                self.pop_implicits_scope();
            },
            Expr::Definition(definition) => {
                self.check_definition(definition, false);
                self.unify(&Type::UNIT, expected, TypeErrorKind::General, id);
            },
            Expr::MemberAccess(member_access) => self.check_member_access(member_access, expected, id),
            Expr::If(if_) => self.check_if(if_, expected, id),
            Expr::Match(match_) => self.check_match(match_, expected, id),
            Expr::Reference(reference) => self.check_reference(reference, expected, id),
            Expr::Is(_) => unreachable!("Expr::Is should be desugared during GetItem"),
            Expr::Do(_) => unreachable!("Expr::Do should be desugared during GetItem"),
            Expr::TypeAnnotation(type_annotation) => {
                let annotation = self.from_cst_type(&type_annotation.rhs, true);
                self.unify(expected, &annotation, TypeErrorKind::TypeAnnotationMismatch, id);
                self.check_expr(type_annotation.lhs, &annotation);
            },
            Expr::Handle(handle) => self.check_handle(handle, expected),
            Expr::Constructor(constructor) => self.check_constructor(constructor, expected, id),
            Expr::Quoted(_) => {
                let location = id.locate(self);
                UnimplementedItem::Comptime.issue(self.compiler, location);
            },
            Expr::Loop(_) => unreachable!("Loops should be desugared before type inference"),
            Expr::While(while_) => self.check_while(while_, expected, id),
            Expr::For(for_) => self.check_for(for_, expected, id),
            // Allow break/continue to return any type
            // TODO: Add bottom type
            Expr::Break | Expr::Continue => (),
            Expr::Return(return_) => self.check_return(return_.expression, id),
            Expr::Assignment(assignment) => self.check_assignment(assignment, expected, id),
            Expr::Error => (),
            Expr::Extern(_) => (),
            Expr::InterpolatedString(_) => {
                unreachable!("InterpolatedString should be desugared before type inference")
            },
            Expr::ArrayLiteral(elements) => self.check_array_literal(elements, expected, id),
        }
    }

    fn check_literal(&mut self, literal: &Literal, locator: impl Locateable + Copy, expected: &Type) {
        let actual = match literal {
            Literal::Unit => Type::UNIT,
            Literal::Integer(value, Some(kind)) => {
                self.check_int_fits(*value, *kind, locator);
                Type::integer(*kind)
            },
            Literal::Float(_, Some(kind)) => Type::float(*kind),
            Literal::Bool(_) => Type::BOOL,
            Literal::Integer(value, None) => {
                let type_variable = self.next_type_variable_id();
                self.push_inferred_int(*value, type_variable, locator.locate(self));
                Type::Variable(type_variable)
            },
            Literal::Float(_, None) => {
                let type_variable = self.next_type_variable_id();
                self.push_inferred_float(type_variable, locator.locate(self));
                Type::Variable(type_variable)
            },
            Literal::String(_) => self.get_string_type(),
            Literal::Char(_) => Type::CHAR,
        };
        self.unify(&actual, expected, TypeErrorKind::General, locator);
    }

    pub(super) fn check_name(&mut self, name: NameId, actual: &Type) {
        if let Some(existing) = self.name_types.get(&name) {
            self.unify(actual, &existing.clone(), TypeErrorKind::General, name);
        } else {
            self.name_types.insert(name, actual.clone());
        }
    }

    fn check_pattern(&mut self, id: PatternId, expected: &Type) {
        self.pattern_types.insert(id, expected.clone());

        let pattern = match self.current_extended_context().extended_pattern(id) {
            Some(pattern) => Cow::Owned(pattern.clone()),
            None => Cow::Borrowed(&self.current_context()[id]),
        };

        match pattern.as_ref() {
            Pattern::Error => (),
            Pattern::Variable(name) | Pattern::MethodName { item_name: name, .. } => {
                self.check_name(*name, expected);
            },
            Pattern::Literal(literal) => self.check_literal(literal, id, expected),
            Pattern::Constructor(path, args) => {
                let parameters = mapvec(args, |_| types::ParameterType::explicit(self.next_type_variable()));

                let expected_function_type = if args.is_empty() {
                    expected.clone()
                } else {
                    Type::Function(Arc::new(FunctionType {
                        parameters: parameters.clone(),
                        // Any type constructor we can match on shouldn't be a closure
                        environment: Type::NO_CLOSURE_ENV,
                        return_type: expected.clone(),
                    }))
                };

                self.check_path(*path, &expected_function_type, None, None);
                for (expected_arg_type, arg) in parameters.into_iter().zip(args) {
                    self.check_pattern(*arg, &expected_arg_type.typ);
                }
            },
            Pattern::TypeAnnotation(inner_pattern, typ) => {
                let annotated = self.from_cst_type(typ, true);
                self.unify(expected, &annotated, TypeErrorKind::TypeAnnotationMismatch, id);
                self.check_pattern(*inner_pattern, expected);
            },
            Pattern::Or(alts) => {
                for alt in alts {
                    self.check_pattern(*alt, expected);
                }
            },
        };
    }

    fn check_path(&mut self, path: PathId, expected: &Type, expr: Option<ExprId>, call_expr: Option<ExprId>) {
        let actual = match self.path_origin(path) {
            Some(Origin::TopLevelDefinition(id)) => self.type_of_top_level_name(&id, path),
            Some(Origin::Local(name)) => {
                let Some(typ) = self.name_types.get(&name).cloned() else {
                    // Name wasn't defined, name resolution should already have emitted an error
                    self.name_types.insert(name, expected.clone());
                    self.path_types.insert(path, expected.clone());
                    return;
                };

                let move_path = super::affine::MovePath::Variable(name);
                if !self.suppress_move_check {
                    self.check_use_of_move_path(&move_path, path);
                }
                if !self.suppress_move_record && !self.type_is_copy(&typ) {
                    let location = path.locate(self);
                    self.move_tracker.record_move(move_path, location);
                }
                typ
            },
            Some(Origin::TypeResolution) => self.resolve_type_resolution(path, expected),
            Some(Origin::Builtin(builtin)) => self.check_builtin(builtin, path),
            None => return,
        };
        if let Some(expr) = expr {
            match self.try_coercion(&actual, expected, expr, call_expr) {
                super::CoercionOutcome::ReplacedExpr => {
                    // If the coercion wrapped a local variable (e.g. auto-ref `seq` → `ref seq`),
                    // the move recorded above no longer applies.
                    if let Some(Origin::Local(name)) = self.path_origin(path) {
                        self.move_tracker.clear_moves(&super::affine::MovePath::Variable(name));
                    }
                    self.check_expr(expr, expected);
                    return;
                    // no need to unify or modify self.path_types, that will be handled in the
                    // recursive check_expr call since we've just changed the expression at this ExprId.
                },
                super::CoercionOutcome::InPlaceCall => {
                    // The Call at `call_expr` was rewritten with the resolved implicit
                    // arguments already spliced in. The function expression itself is
                    // unchanged, so record its actual (full-implicit) type and return.
                    // `implicit_parameter_coercion` already performed the unification
                    // needed for `check_call`'s argument loop to see the right types.
                    self.path_types.insert(path, actual);
                    return;
                },
                super::CoercionOutcome::None => {},
            }
        }
        self.unify(&actual, expected, TypeErrorKind::General, path);
        self.path_types.insert(path, actual);
    }

    /// Returns the instantiated type of the given TopLevelName.
    ///
    /// Stores the result of the instantiation (if any) to the given [PathId].
    pub(super) fn type_of_top_level_name(&mut self, name: &TopLevelName, path: PathId) -> Type {
        if let Some(typ) = self.item_types.get(name) {
            typ.clone()
        } else {
            let typ = GetType(*name).get(self.compiler);
            let (typ, bindings) = self.instantiate(typ);
            if let Some(bindings) = bindings {
                self.current_extended_context_mut().insert_instantiation(path, bindings);
            }
            typ
        }
    }

    /// Returns the type of a [TopLevelName], possibly instantiating it and returning the bindings,
    /// if any, along with the type.
    pub(super) fn type_and_bindings_of_top_level_name(&mut self, name: &TopLevelName) -> (Type, Option<Vec<Type>>) {
        if let Some(typ) = self.item_types.get(name) {
            (typ.clone(), None)
        } else {
            let typ = GetType(*name).get(self.compiler);
            self.instantiate(typ)
        }
    }

    /// Instantiate the given type, returning the instantiated type and the instantiation bindings.
    ///
    /// This function should not be used outside of [Self::type_and_bindings_of_top_level_name] or [Self::type_of_top_level_name]
    /// since the resulting bindings always need to be remembered.
    fn instantiate(&mut self, typ: Type) -> (Type, Option<Vec<Type>>) {
        match typ {
            Type::Forall(generics, old_type) => {
                assert!(!generics.is_empty());
                let substitutions = generics.iter().map(|generic| (*generic, self.next_type_variable())).collect();
                let typ = old_type.substitute(&substitutions, &self.bindings);

                let bindings = mapvec(generics.iter(), |generic| substitutions[generic].clone());
                (typ, Some(bindings))
            },
            other => (other, None),
        }
    }

    fn resolve_type_resolution(&mut self, path: PathId, expected: &Type) -> Type {
        let path_value = &self.current_context()[path];
        assert_eq!(path_value.components.len(), 1, "Only single-component paths should have Origin::TypeResolution");
        let name = path_value.last_ident();

        let Some(id) = self.try_find_type_namespace_for_type_resolution(expected, name) else {
            return self.issue_name_not_in_scope_error(path);
        };

        // Remember what this `Origin::TypeResolution` path actually refers to from now on
        self.current_extended_context_mut().insert_path_origin(path, Origin::TopLevelDefinition(id));
        self.type_of_top_level_name(&id, path)
    }

    /// Issue a NameNotInScope error and return Type::Error
    fn issue_name_not_in_scope_error(&self, path: PathId) -> Type {
        let name = Arc::new(self.current_context()[path].last_ident().to_owned());
        let location = self.current_context().path_location(path).clone();
        self.compiler.accumulate(Diagnostic::NameNotInScope { name, location });
        Type::ERROR
    }

    fn try_find_type_namespace_for_type_resolution(&self, typ: &Type, constructor_name: &str) -> Option<TopLevelName> {
        match self.follow_type(typ) {
            Type::UserDefined(Origin::TopLevelDefinition(id)) => {
                // We found which type this name belongs to, but if it is a variant we have to
                // check which constructor we want.
                let (_, item_context) = GetItemRaw(id.top_level_item).get(self.compiler);
                let resolve = Resolve(id.top_level_item).get(self.compiler);
                let name_id = resolve
                    .top_level_names
                    .iter()
                    .find(|&&name| item_context.names[name].as_str() == constructor_name)?;

                Some(TopLevelName { top_level_item: id.top_level_item, local_name_id: *name_id })
            },
            Type::Function(function_type) => {
                self.try_find_type_namespace_for_type_resolution(&function_type.return_type, constructor_name)
            },
            Type::Application(constructor, _) => {
                self.try_find_type_namespace_for_type_resolution(constructor, constructor_name)
            },
            _ => None,
        }
    }

    /// Returns the instantiated type of a builtin value
    ///
    /// Will error if passed a builtin type
    fn check_builtin(&mut self, builtin: Builtin, locator: impl Locateable) -> Type {
        match builtin {
            Builtin::Unit | Builtin::Char | Builtin::Bool | Builtin::Ptr | Builtin::Array | Builtin::Never => {
                let typ = Arc::new(builtin.to_string());
                let location = locator.locate(self);
                self.compiler.accumulate(Diagnostic::ValueExpected { location, typ });
                Type::ERROR
            },
            // This needs to match various different function types generally in the form
            // `fn String ... -> a`. For simplicity a type variable is issued here, those working
            // on the stdlib should take care to only use intrinsics with the proper types.
            Builtin::Intrinsic => self.next_type_variable(),
        }
    }

    fn check_call(&mut self, call: &cst::Call, expected: &Type, call_expr: ExprId) {
        // If the function is a MemberAccess, try to resolve it as a method call.
        // E.g. `vec.push 3` is rewritten to `push (mut vec) 3`.
        if self.try_rewrite_method_call(call, expected, call_expr) {
            return;
        }

        let expected_parameter_types =
            mapvec(&call.arguments, |arg| types::ParameterType::new(self.next_type_variable(), arg.is_implicit));

        let expected_function_type = {
            let parameters = expected_parameter_types.clone();
            let environment = self.next_type_variable();
            let return_type = expected.clone();
            Type::Function(Arc::new(FunctionType { parameters, environment, return_type }))
        };

        self.check_expr_inner(call.function, &expected_function_type, Some(call_expr));
        for (arg, expected_arg_type) in call.arguments.iter().zip(expected_parameter_types) {
            self.check_expr(arg.expr, &expected_arg_type.typ);
        }
    }

    /// If `call` is `v.push 3` (MemberAccess + args), try to resolve `push` as a function
    /// in the module where `v`'s type is defined. If found, rewrite the Call expression to
    /// `push (mut v) 3` in the extended context and type-check that instead.
    fn try_rewrite_method_call(&mut self, call: &cst::Call, expected: &Type, call_expr: ExprId) -> bool {
        let func_expr = match self.current_extended_context().extended_expr(call.function) {
            Some(expr) => expr.clone(),
            None => self.current_context()[call.function].clone(),
        };

        let Expr::MemberAccess(member_access) = &func_expr else {
            return false;
        };

        let object = member_access.object;
        let member = member_access.member.clone();

        // Type-check the object to learn its type.
        // Suppress moves: if the rewrite fails, the normal call handling will
        // process the member access with proper partial-move tracking.
        let struct_type = self.next_type_variable();
        let old_suppress_check = self.suppress_move_check;
        let old_suppress_record = self.suppress_move_record;
        self.suppress_move_check = true;
        self.suppress_move_record = true;
        self.check_expr(object, &struct_type);
        self.suppress_move_check = old_suppress_check;
        self.suppress_move_record = old_suppress_record;

        // Resolve the method name to a top-level function
        let Some((method_name, func_type, bindings)) = self.resolve_method_for_type(&struct_type, &member) else {
            return false;
        };
        let method_type = Type::Function(func_type.clone());

        // Build the object argument, auto-ref'ing if the first parameter is a reference type
        let location = self.current_context().expr_location(call.function).clone();
        let Some(object_arg) = self.build_object_arg_with_auto_ref(
            object,
            &struct_type,
            &func_type.parameters[0].typ,
            &location,
            call_expr,
        ) else {
            return false;
        };

        // Create a Variable expression for the method, with a fresh path
        let method_path = self.push_path(
            cst::Path { components: vec![(member, location.clone())] },
            method_type.clone(),
            location.clone(),
        );

        self.current_extended_context_mut().insert_path_origin(method_path, Origin::TopLevelDefinition(method_name));

        if let Some(bindings) = bindings {
            self.current_extended_context_mut().insert_instantiation(method_path, bindings);
        }

        let method_var = self.push_expr(Expr::Variable(method_path), method_type, location);

        // Build the new Call: `push (mut v) 3`
        // If the call is in the form `obj.method ()` and the method only takes 1 parameter,
        // strip the `()`.
        let single_unit_arg = call.arguments.len() == 1
            && func_type.parameters.iter().filter(|p| !p.is_implicit).count() == 1
            && matches!(self.current_context()[call.arguments[0].expr], Expr::Literal(Literal::Unit));

        let mut new_arguments = vec![cst::Argument::explicit(object_arg)];
        if !single_unit_arg {
            new_arguments.extend_from_slice(&call.arguments);
        }

        let new_call = Expr::Call(cst::Call { function: method_var, arguments: new_arguments });
        self.current_extended_context_mut().insert_expr(call_expr, new_call);

        // Type-check the rewritten expression (handles implicit parameter resolution too)
        self.check_expr(call_expr, expected);
        true
    }

    fn check_lambda(&mut self, lambda: &cst::Lambda, expected: &Type, expr: ExprId, self_name: Option<NameId>) {
        self.check_lambda_impl(lambda, expected, expr, self_name, LambdaOptions::default());
    }

    fn check_lambda_impl(
        &mut self, lambda: &cst::Lambda, expected: &Type, expr: ExprId, self_name: Option<NameId>,
        options: LambdaOptions,
    ) {
        let function_type = match self.follow_type(expected) {
            Type::Function(function_type) => function_type.clone(),
            _ => {
                let parameters = mapvec(&lambda.parameters, |param| {
                    types::ParameterType::new(self.next_type_variable(), param.is_implicit)
                });
                let expected_parameter_count = parameters.len();
                let environment = self.next_type_variable();
                let return_type = self.next_type_variable();
                let new_type = Arc::new(FunctionType { parameters, environment, return_type });
                let function_type = Type::Function(new_type.clone());
                self.unify(expected, &function_type, TypeErrorKind::Lambda { expected_parameter_count }, expr);
                new_type
            },
        };

        // Remember the return type so that it can be checked by `return` statements
        let old_return_type =
            std::mem::replace(&mut self.function_return_type, Some(function_type.return_type.clone()));
        // Closures capture by reference, so moves inside the lambda don't affect the outer scope
        let old_move_tracker = std::mem::take(&mut self.move_tracker);

        self.push_implicits_scope();
        self.check_function_parameter_count(&function_type.parameters, lambda.parameters.len(), expr);
        let parameter_lengths_match = function_type.parameters.len() == lambda.parameters.len();

        for (parameter, expected_type) in lambda.parameters.iter().zip(function_type.parameters.iter()) {
            // Avoid extra errors if the parameter length isn't as expected
            let expected_type = if parameter_lengths_match { &expected_type.typ } else { &Type::ERROR };
            self.check_pattern(parameter.pattern, expected_type);

            if parameter.is_mutable {
                self.record_mutable_pattern(parameter.pattern);
            }

            if parameter.is_implicit {
                self.add_implicit(parameter.pattern);
            }
        }

        // Required in case `function_type` has fewer parameters, to ensure we check all of `lambda.parameters`
        for parameter in lambda.parameters.iter().skip(function_type.parameters.len()) {
            self.check_pattern(parameter.pattern, &Type::ERROR);
        }

        let return_type = if let Some(return_type) = lambda.return_type.as_ref() {
            let return_type = self.from_cst_type(return_type, true);
            self.unify(&return_type, &function_type.return_type, TypeErrorKind::TypeAnnotationMismatch, expr);
            Cow::Owned(return_type)
        } else {
            Cow::Borrowed(&function_type.return_type)
        };

        self.check_expr(lambda.body, &return_type);

        // If this lambda's body may execute more than once (e.g. a handler
        // branch), report any non-Copy outer variables moved inside it before
        // we discard the scope-local move tracker. `self.move_tracker` at this
        // point is the branch-local tracker that `mem::take` above started empty.
        if let Some((context, outer_names)) = options.repeated_context.as_ref() {
            self.check_moves_in_repeated_context(outer_names, *context);
        }

        self.function_return_type = old_return_type;
        self.move_tracker = old_move_tracker;

        // Must run before `check_for_closure` may be deferred, so later uses see the move.
        if lambda.is_move {
            self.record_move_captures(expr, self_name);
        }

        let delayed = self.pop_implicits_scope();

        // pop_implicits_scope modifies the function by inserting implicit arguments, we need
        // to check captures only after that step in case any of those arguments are captured.
        // When `delayed` is true, the scope's implicits were deferred to the parent and haven't
        // been resolved yet, so the closure check must also be deferred (same as coercion wrappers
        // whose argument slots are filled by the enclosing scope's pop_implicits_scope).
        if self.coercion_wrapper_exprs.contains(&expr) || delayed {
            if let Some(scope) = self.implicits.last_mut() {
                scope.push_deferred_closure_check(expr, function_type.environment.clone(), self_name, lambda.is_move);
            }
        } else {
            self.check_for_closure(expr, &function_type.environment, self_name, lambda.is_move);
        }
    }

    /// Check a function's parameter count using the given parameter types as the expected count.
    /// Issues an error if the expected count does not match the actual count.
    fn check_function_parameter_count(
        &mut self, parameters: &Vec<types::ParameterType>, actual_count: usize, expr: ExprId,
    ) {
        if actual_count != parameters.len() {
            self.compiler.accumulate(Diagnostic::FunctionArgCountMismatch {
                actual: actual_count,
                expected: parameters.len(),
                location: self.current_context().expr_location(expr).clone(),
            });
        }
    }

    fn check_member_access(&mut self, member_access: &cst::MemberAccess, expected: &Type, expr: ExprId) {
        let struct_type = self.next_type_variable();

        // Suppress moves on the object - we handle partial move tracking here at the field level
        let old_suppress_check = self.suppress_move_check;
        let old_suppress_record = self.suppress_move_record;
        self.suppress_move_check = true;
        self.suppress_move_record = true;
        self.check_expr(member_access.object, &struct_type);
        self.suppress_move_check = old_suppress_check;
        self.suppress_move_record = old_suppress_record;

        let fields = self.get_field_types(&struct_type, None);
        if let Some((field, field_index)) = fields.get(&member_access.member) {
            let field = field.clone();
            let field_index = *field_index;
            self.current_extended_context_mut().push_member_access_index(expr, field_index);

            // If the struct is a reference or pointer, field types are wrapped in the same reference/pointer.
            //
            // If it is a reference (and not a pointer), auto-deref the field unless the expected type is known to be a reference.
            let struct_is_ref = struct_type.reference_element(&self.bindings).is_some();
            let struct_is_indirect = struct_is_ref || struct_type.pointer_element(&self.bindings).is_some();

            // Track partial moves only when the struct is not behind a reference or pointer.
            if !struct_is_indirect {
                if let Some(parent_path) = self.try_build_move_path(member_access.object) {
                    let move_path = super::affine::MovePath::Field(Box::new(parent_path), member_access.member.clone());
                    if !old_suppress_check {
                        self.check_use_of_move_path(&move_path, expr);
                    }
                    if !old_suppress_record && !self.type_is_copy(&field) {
                        let location = expr.locate(self);
                        self.move_tracker.record_move(move_path, location);
                    }
                }
            }

            let expected_is_ref = expected.reference_element(&self.bindings).is_some();

            if struct_is_ref && !expected_is_ref {
                if let Some((_, inner_field_type)) = field.reference_element(&self.bindings) {
                    if self.type_is_copy(&inner_field_type) {
                        self.unify(&inner_field_type, expected, TypeErrorKind::General, expr);
                        let new_expr = self.auto_deref_coercion(expr, inner_field_type);
                        self.current_extended_context_mut().insert_expr(expr, new_expr);
                        self.check_expr(expr, expected);
                        return;
                    }
                }
            }

            match self.try_coercion(&field, expected, expr, None) {
                super::CoercionOutcome::ReplacedExpr => self.check_expr(expr, expected),
                super::CoercionOutcome::InPlaceCall | super::CoercionOutcome::None => {
                    self.unify(&field, expected, TypeErrorKind::General, expr);
                },
            }
        } else if matches!(self.follow_type(&struct_type), Type::Variable(_)) {
            let location = self.current_context().expr_location(expr).clone();
            self.compiler.accumulate(Diagnostic::TypeMustBeKnownMemberAccess { location });
        } else {
            let typ = self.type_to_string(&struct_type);
            let location = self.current_context().expr_location(expr).clone();
            let name = Arc::new(member_access.member.clone());
            self.compiler.accumulate(Diagnostic::NoSuchFieldForType { typ, location, name });
        }
    }

    /// Resolve a method name on a type to its top-level function definition.
    /// Returns the method's name, function type, and optional generic bindings.
    fn resolve_method_for_type(
        &mut self, struct_type: &Type, member: &str,
    ) -> Option<(TopLevelName, Arc<FunctionType>, Option<Vec<Type>>)> {
        let (source_file, type_top_level_id) = self.find_type_info(struct_type)?;

        // Only resolve methods on actual type definitions (structs/enums),
        // not on traits or effects (whose declarations also live in `methods`).
        let (item, _) = GetItemRaw(type_top_level_id).get(self.compiler);
        if !matches!(item.kind, cst::TopLevelItemKind::TypeDefinition(_)) {
            return None;
        }

        // Within the same file, non-exported methods should be visible (matching
        // how regular definitions work). Across files, only exported methods are visible.
        let current_file = self.current_item.expect("current_item set").source_file;
        let definitions = if source_file == current_file {
            AllDefinitions(source_file).get(self.compiler)
        } else {
            ExportedDefinitions(source_file).get(self.compiler)
        };

        let member_name = Arc::new(member.to_owned());
        let name = *definitions.methods.get(&type_top_level_id)?.get(&member_name)?;

        let (method_type, bindings) = self.type_and_bindings_of_top_level_name(&name);

        let Type::Function(func_type) = &method_type else {
            return None;
        };

        if func_type.parameters.is_empty() {
            return None;
        }

        Some((name, func_type.clone(), bindings))
    }

    /// Find the source file and TopLevelId where a type is defined, unwrapping references and type applications.
    fn find_type_info(&self, typ: &Type) -> Option<(SourceFileId, TopLevelId)> {
        match self.follow_type(typ) {
            Type::UserDefined(Origin::TopLevelDefinition(id)) => {
                Some((id.top_level_item.source_file, id.top_level_item))
            },
            Type::Application(constructor, _) => match typ.reference_element(&self.bindings) {
                Some((_, element)) => self.find_type_info(&element),
                _ => self.find_type_info(constructor),
            },
            _ => None,
        }
    }

    /// Build the object argument for a method call, automatically wrapping it
    /// in a reference if the method's first parameter expects one.
    fn build_object_arg_with_auto_ref(
        &mut self, mut object: ExprId, struct_type: &Type, first_param: &Type, location: &Location, call_expr: ExprId,
    ) -> Option<ExprId> {
        // If the first parameter expects a reference type, unwrap both sides
        // and auto-ref the object if needed.
        let (param_type, struct_base, auto_ref) = if let Some((ref_kind, inner_type)) =
            first_param.reference_element(&self.bindings)
        {
            let (struct_base_type, should_wrap_object_in_ref) = match struct_type.reference_element(&self.bindings) {
                Some((_, element)) => (element, false),
                None => (struct_type.clone(), true),
            };
            let auto_ref = should_wrap_object_in_ref.then_some(ref_kind);
            (inner_type, struct_base_type, auto_ref)
        } else {
            (first_param.clone(), struct_type.clone(), None)
        };

        self.unify(&param_type, &struct_base, TypeErrorKind::General, call_expr);

        if let Some(ref_kind) = auto_ref {
            let ref_expr = Expr::Reference(cst::Reference { kind: ref_kind, rhs: object });
            object = self.push_expr(ref_expr, first_param.clone(), location.clone());
        }

        Some(object)
    }

    fn check_if(&mut self, if_: &cst::If, expected: &Type, expr: ExprId) {
        self.check_expr(if_.condition, &Type::BOOL);

        // If there's an else clause our expected return type should match the then/else clauses'
        // types. Otherwise, the then body may be any type.
        let expected = if if_.else_.is_some() {
            Cow::Borrowed(expected)
        } else {
            self.unify(&Type::UNIT, expected, TypeErrorKind::IfStatement, expr);
            Cow::Owned(self.next_type_variable())
        };

        // Save move state before branches so each branch sees the same pre-branch state
        let pre_branch_moves = self.move_tracker.clone();

        self.push_implicits_scope();
        self.check_expr(if_.then, &expected);
        self.pop_implicits_scope();
        let then_moves = self.move_tracker.clone();

        // TODO: No way to identify if `then_type != else_type`. This would be useful to point out
        // for error messages.
        let then_diverges = self.expr_always_diverges(if_.then);

        if let Some(else_) = if_.else_ {
            // Reset to pre-branch state for else branch
            self.move_tracker = pre_branch_moves.clone();
            self.push_implicits_scope();
            self.check_expr(else_, &expected);
            self.pop_implicits_scope();
            let else_moves = self.move_tracker.clone();
            let else_diverges = self.expr_always_diverges(else_);

            // After if/else, exclude moves from branches that always diverge (return)
            // since execution never reaches the merge point from those branches.
            let mut branches = Vec::new();
            if !then_diverges {
                branches.push(then_moves);
            }
            if !else_diverges {
                branches.push(else_moves);
            }
            self.move_tracker = super::affine::MoveTracker::merge_branches(&pre_branch_moves, &branches);
        } else {
            // If-without-else: if the then-branch always returns, moves don't carry forward
            if then_diverges {
                self.move_tracker = pre_branch_moves;
            } else {
                self.move_tracker = super::affine::MoveTracker::merge_branches(&pre_branch_moves, &[then_moves]);
            }
        }
    }

    fn check_match(&mut self, match_: &cst::Match, expected: &Type, expr: ExprId) {
        let expr_type = self.next_type_variable();

        // Push an implicits scope here so we can default any integers used in the match
        // to an `I32` before the decision tree checks occur. This lets us compile `match 1 | ...`
        // without errors that the type of `1` is not yet known.
        self.push_implicits_scope();
        self.check_expr(match_.expression, &expr_type);

        // Save move state before branches
        let pre_branch_moves = self.move_tracker.clone();
        let mut branch_trackers = Vec::new();

        for (pattern, branch) in match_.cases.iter() {
            self.move_tracker = pre_branch_moves.clone();
            self.check_pattern(*pattern, &expr_type);
            // TODO: Specify if branch_type != type of first branch for better error messages
            self.push_implicits_scope();
            self.check_expr(*branch, expected);
            self.pop_implicits_scope();
            branch_trackers.push(self.move_tracker.clone());
        }
        self.move_tracker = super::affine::MoveTracker::merge_branches(&pre_branch_moves, &branch_trackers);
        self.pop_implicits_scope();

        // Now compile the match into a decision tree. The `match expr | ...` expression will be
        // replaced with `<fresh> = expr; <decision tree>`
        let location = self.current_context().expr_location(match_.expression).clone();
        let (match_var, match_var_name) = self.fresh_variable("match_var", expr_type.clone(), location.clone());

        // `<match_var> = <expression being matched>`
        let preamble = self.let_binding(match_var_name, match_.expression);

        if let Some(tree) = self.compile_decision_tree(match_var, &match_.cases, expr_type, location) {
            let context = self.current_extended_context_mut();
            context.insert_decision_tree(expr, preamble, tree);
        }
    }

    fn check_reference(&mut self, reference: &cst::Reference, expected: &Type, expr: ExprId) {
        let actual = Type::reference(reference.kind);

        let expected_element_type = match self.follow_type(expected) {
            Type::Application(constructor, args) => {
                let constructor = constructor.clone();
                let args = args.clone();
                let element_arg = args.get(1);

                match self.follow_type(&constructor) {
                    Type::Primitive(types::PrimitiveType::Reference(..)) => {
                        self.unify(&actual, &constructor, TypeErrorKind::ReferenceKind, expr);

                        // Expect incorrect arg counts to be resolved beforehand
                        element_arg.unwrap().clone()
                    },
                    _ => {
                        if self.unify(&actual, expected, TypeErrorKind::ExpectedNonReference, expr) {
                            element_arg.unwrap().clone()
                        } else {
                            Type::ERROR
                        }
                    },
                }
            },
            Type::Variable(id) => {
                let id = *id;
                let lifetime = self.next_type_variable();
                let element = self.next_type_variable();
                let expected = Type::Application(Arc::new(actual), Arc::new(vec![lifetime, element.clone()]));
                self.bindings.insert(id, expected);
                element
            },
            _ => {
                self.unify(&actual, expected, TypeErrorKind::ExpectedNonReference, expr);
                Type::ERROR
            },
        };

        // A reference doesn't move its rhs, but it still reads it, so we must
        // check that the rhs isn't already moved.
        let old_suppress_record = self.suppress_move_record;
        self.suppress_move_record = true;
        self.check_expr(reference.rhs, &expected_element_type);
        self.suppress_move_record = old_suppress_record;
    }

    fn check_constructor(&mut self, constructor: &cst::Constructor, expected: &Type, id: ExprId) {
        let typ = self.from_cst_type(&constructor.typ, true);
        self.unify(&typ, expected, TypeErrorKind::Constructor, id);

        // Map each field name to its index in the type's declaration order.
        // This is used when lowering to MIR when structs are converted into tuples.
        let mut field_order = BTreeMap::new();
        let field_types = self.get_field_types(&typ, None);

        for (name, expr) in &constructor.fields {
            let name_string = &self.current_context()[*name];
            let (expected_field_type, field_index) = field_types.get(name_string).cloned().unwrap_or((Type::ERROR, 0));

            self.check_expr(*expr, &expected_field_type);
            self.check_name(*name, &expected_field_type);

            field_order.insert(*name, field_index);
        }

        self.current_extended_context_mut().push_constructor_field_order(id, field_order);
    }

    fn check_handle(&mut self, handle: &cst::Handle, expected: &Type) {
        // `can expected_effect, e`
        let expected_and_e = self.next_type_variable();

        let handler_effect_type = self.next_type_variable();
        self.name_types.insert(handle.handler_name, handler_effect_type.clone());

        // The parser wraps the handled expression in `fn () -> <body>` to serve as the
        // coroutine's init function.
        let body_env = self.next_type_variable();
        let body_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![ParameterType::explicit(Type::UNIT)],
            environment: body_env,
            return_type: expected.clone(),
        }));
        self.expr_types.insert(handle.expression, body_type.clone());

        // Prevent any names visible from before the handler branches from being moved
        // TODO: This is inefficient, remove the need for collecting here
        let outer_names = self.name_types.keys().copied().collect::<FxHashSet<_>>();

        // For each case:
        // - pattern.function: fn args.. {Effect..} -> r can e  (the effect op declared type)
        // - branch lambda:    fn args.. resume -> expected can expected_effect
        // - resume:           fn r -> expected can expected_effect
        //
        // `resume` doesn't raise `e` since handlers in Ante are deep: each call to
        // resume is automatically handled by the same handler.
        for (pattern, branch) in &handle.cases {
            let mut parameter_types = mapvec(&pattern.args, |_| ParameterType::explicit(self.next_type_variable()));

            // The effect operation has an implicit trailing parameter of its parent
            // effect type (e.g. `Emit a`, `Fail`).
            parameter_types.push(ParameterType::implicit(handler_effect_type.clone()));
            let r = self.next_type_variable();
            let e = self.next_type_variable();

            let function_type = Type::Function(Arc::new(FunctionType {
                parameters: parameter_types.clone(),
                environment: Type::Application(Arc::new(Type::POINTER), Arc::new(vec![Type::UNIT])),
                return_type: r.clone(),
            }));
            self.check_path(pattern.function, &function_type, None, None);
            self.unify(&e, &expected_and_e, TypeErrorKind::General, pattern.function);

            // Branches accept the operation's explicit args plus `resume`.
            parameter_types.pop();

            // resume is a closure capturing its environment by reference.
            // The coroutine lowering pass supplies a closure with an env pointing to `(coro, handlers..)`.
            let resume_type = Type::Function(Arc::new(FunctionType {
                parameters: vec![ParameterType::explicit(r)],
                environment: Type::Primitive(types::PrimitiveType::Pointer),
                return_type: expected.clone(),
            }));

            let mut handler_params = parameter_types;
            handler_params.push(ParameterType::explicit(resume_type));
            let handler_type = Type::Function(Arc::new(FunctionType {
                parameters: handler_params,
                environment: self.next_type_variable(),
                return_type: expected.clone(),
            }));

            // Only allow moving variables into this branch if `resume` is never mentioned.
            // This notably keeps handlers like `try_or` working.
            let repeated_context = self
                .handler_branch_uses_resume(pattern.resume_name, *branch)
                .then(|| (RepeatedContext::HandlerBranch, outer_names.clone()));

            let options = LambdaOptions { repeated_context };

            let branch_lambda = self.unwrap_lambda(*branch);
            self.expr_types.insert(*branch, handler_type.clone());
            self.check_lambda_impl(&branch_lambda, &handler_type, *branch, None, options);
        }

        self.push_implicits_scope();
        self.add_implicit_name(handle.handler_name);

        let options = LambdaOptions::default();
        let body_lambda = self.unwrap_lambda(handle.expression);

        // `Some(handler_name)` exempts the variable from being captured as a closure.
        // This is instead handled as a special case in mir generation
        self.check_lambda_impl(&body_lambda, &body_type, handle.expression, Some(handle.handler_name), options);
        self.pop_implicits_scope();
    }

    /// Retrieve the [`cst::Lambda`] at `expr_id` or panic otherwise.
    fn unwrap_lambda(&self, expr_id: ExprId) -> &'local cst::Lambda {
        match &self.current_context()[expr_id] {
            Expr::Lambda(lambda) => lambda,
            other => unreachable!("Expected a lambda, found {other:?}"),
        }
    }

    fn check_assignment(&mut self, assignment: &cst::Assignment, expected: &Type, id: ExprId) {
        let lhs_type = self.next_type_variable();

        // Allow `x := v` to use `x` even if moved but `x += v` cannot since it reads `x`
        let is_plain = assignment.op.is_none();
        if is_plain {
            let old_suppress_check = std::mem::replace(&mut self.suppress_move_check, true);
            let old_suppress_record = std::mem::replace(&mut self.suppress_move_record, true);
            self.check_expr(assignment.lhs, &lhs_type);
            self.suppress_move_check = old_suppress_check;
            self.suppress_move_record = old_suppress_record;
        } else {
            self.check_expr(assignment.lhs, &lhs_type);
        }

        if let Err((name, location)) = self.check_lhs_mutable(assignment.lhs) {
            self.compiler.accumulate(Diagnostic::AssignToImmutable { name, location });
        }

        // If the LHS is a reference type (e.g. `p.x` where `p: mut Point` yields `mut I32`),
        // the RHS should match the inner (pointee) type rather than the reference wrapper.
        let lhs_followed = self.follow_type(&lhs_type);
        let lhs_is_ref = matches!(&lhs_followed, Type::Application(c, _)
            if matches!(self.follow_type(c), Type::Primitive(types::PrimitiveType::Reference(_))));

        let value_type = if lhs_is_ref {
            match self.follow_type(&lhs_type) {
                // Reference applications carry a lifetime in slot 0 and the element in slot 1.
                Type::Application(_, args) => args[1].clone(),
                _ => unreachable!(),
            }
        } else {
            lhs_type.clone()
        };

        // For compound assignments (+=, -=, etc.), resolve the operator function through
        // implicits dispatch. The operator has type `fn value_type value_type -> value_type`.
        if let Some((_, op_expr)) = assignment.op {
            let expected_fn_type = Type::Function(Arc::new(FunctionType {
                parameters: vec![
                    ParameterType::explicit(value_type.clone()),
                    ParameterType::explicit(value_type.clone()),
                ],
                environment: self.next_type_variable(),
                return_type: value_type.clone(),
            }));
            self.check_expr(op_expr, &expected_fn_type);
        }

        self.check_expr(assignment.rhs, &value_type);

        // The LHS always holds a value after an assignment
        if let Some(path) = self.try_build_move_path(assignment.lhs) {
            self.move_tracker.clear_moves(&path);
        }

        self.unify(&Type::UNIT, expected, TypeErrorKind::General, id);
    }

    /// Walk a pattern declared with `var` and add every variable name it
    /// introduces to `mutable_definitions`.
    fn record_mutable_pattern(&mut self, pattern: PatternId) {
        match &self.current_context()[pattern] {
            Pattern::Variable(name) => {
                self.mutable_definitions.insert(*name);
            },
            Pattern::TypeAnnotation(inner, _) => {
                let inner = *inner;
                self.record_mutable_pattern(inner);
            },
            Pattern::Error => (),
            Pattern::Literal(_) => (),
            Pattern::Constructor(_, patterns) => {
                patterns.iter().for_each(|pattern| self.record_mutable_pattern(*pattern))
            },
            // This may be reachable on a parse error but these should only be for
            // top-level methods which should never be mutable
            Pattern::MethodName { .. } => (),
            Pattern::Or(_) => unreachable!("`|` pattern in record_mutable_pattern"),
        }
    }

    /// A place is mutable if any of these hold:
    /// - it is declared with `var`
    /// - the local's type is a `mut`/`uniq` reference
    /// - it is a deref of a mutable place
    /// - it is a field access `l.r` where `l` is a mutable place
    ///
    /// On error, returns the first offending name not matching the above rules if found
    fn check_lhs_mutable(&self, lhs: ExprId) -> Result<(), (Option<Name>, Location)> {
        match &self.current_extended_context()[lhs] {
            Expr::Variable(path) => {
                let path_id = *path;
                match self.path_origin(path_id) {
                    Some(Origin::Local(name)) => {
                        if self.mutable_definitions.contains(&name) {
                            return Ok(());
                        }
                        if let Some(typ) = self.name_types.get(&name) {
                            if self.is_mut_or_uniq_reference(typ) {
                                return Ok(());
                            }
                        }
                        let name = self.current_extended_context()[name].clone();
                        Err((Some(name), path_id.locate(self)))
                    },
                    // Top-level definitions, builtins, and type-resolution paths are not assignable.
                    Some(_) => {
                        let path = self.current_extended_context()[path_id].to_string();
                        Err((Some(Arc::new(path)), path_id.locate(self)))
                    },
                    // Unresolved name, ignore further errors
                    None => Ok(()),
                }
            },
            Expr::TypeAnnotation(ta) => self.check_lhs_mutable(ta.lhs),
            Expr::MemberAccess(access) => {
                let object = access.object;
                // TODO: This allows `x: ref (mut a, b)` to assign to the inner `mut a`
                if let Some(typ) = self.expr_types.get(&object) {
                    if self.is_mut_or_uniq_reference(typ) {
                        return Ok(());
                    }
                }
                self.check_lhs_mutable(object)
            },
            Expr::Call(call) => {
                // If this is a call, just assume it is something like `a.* :=` or `a.[0] :=` and check the obj type.
                // TODO: Make this check more rigorous
                if let Some(obj) = call.arguments.first() {
                    self.check_lhs_mutable(obj.expr)
                } else {
                    let location = self.current_extended_context().expr_location(lhs);
                    Err((None, location))
                }
            },
            // TODO: We could have a different variant for lvalues instead of reusing ExprIds
            _ => Ok(()),
        }
    }

    fn is_mut_or_uniq_reference(&self, typ: &Type) -> bool {
        match typ.reference_element(&self.bindings) {
            Some((kind, _)) => matches!(kind, ReferenceKind::Mut | ReferenceKind::Uniq),
            None => false,
        }
    }

    /// Return can unify with any type locally so we don't need the expected type here
    fn check_return(&mut self, returned_expr: ExprId, id: ExprId) {
        match self.function_return_type.as_ref().cloned() {
            Some(expected_return) => {
                self.check_expr(returned_expr, &expected_return);
            },
            None => {
                let location = id.locate(self);
                self.compiler.accumulate(Diagnostic::ReturnNotInFunction { location });
            },
        }
    }

    fn check_while(&mut self, while_: &cst::While, expected: &Type, id: ExprId) {
        // Both the condition and body may execute more than once, so moves of
        // outer non-Copy values inside either are unsound.
        let outer_names = self.name_types.keys().copied().collect::<FxHashSet<_>>();
        let old_tracker = std::mem::take(&mut self.move_tracker);

        self.check_expr(while_.condition, &Type::BOOL);
        self.check_expr(while_.body, &Type::UNIT);

        self.check_moves_in_repeated_context(&outer_names, RepeatedContext::WhileLoop);
        self.move_tracker = old_tracker;

        self.unify(&Type::UNIT, expected, TypeErrorKind::General, id);
    }

    fn check_for(&mut self, for_: &cst::For, expected: &Type, id: ExprId) {
        let int_ty = self.next_type_variable_id();
        // Hack: use 0 for the integer value here since it fits into all integer types.
        // We just need this to ensure the user actually uses integer ranges. Any actual integer
        // they choose will already have its range checked by the Literal code path.
        self.push_inferred_int(Integer::positive(0), int_ty, id.locate(self));
        let int_ty = Type::Variable(int_ty);

        // Range expressions run exactly once, so normal move semantics apply here.
        self.check_expr(for_.start, &int_ty);
        self.check_expr(for_.end, &int_ty);

        // Snapshot outer names before introducing the loop variable so the
        // loop variable itself is not counted as an outer binding.
        let outer_names = self.name_types.keys().copied().collect::<FxHashSet<_>>();
        self.name_types.insert(for_.variable, int_ty);

        let old_tracker = std::mem::take(&mut self.move_tracker);
        self.check_expr(for_.body, &Type::UNIT);
        self.check_moves_in_repeated_context(&outer_names, RepeatedContext::ForLoop);
        self.move_tracker = old_tracker;

        self.unify(&Type::UNIT, expected, TypeErrorKind::General, id);
    }

    /// Check if an expression always diverges (e.g. ends with a `return`).
    /// Used to exclude moves in diverging branches from post-branch merge.
    ///
    /// TODO: Replace this with a check for the bottom type
    fn expr_always_diverges(&self, id: ExprId) -> bool {
        let expr = match self.current_extended_context().extended_expr(id) {
            Some(expr) => Cow::Owned(expr.clone()),
            None => Cow::Borrowed(&self.current_context()[id]),
        };
        match expr.as_ref() {
            Expr::Return(_) => true,
            Expr::Sequence(items) => items.last().map_or(false, |item| self.expr_always_diverges(item.expr)),
            Expr::If(if_) => {
                let then_diverges = self.expr_always_diverges(if_.then);
                let else_diverges = if_.else_.map_or(false, |e| self.expr_always_diverges(e));
                then_diverges && else_diverges
            },
            _ => false,
        }
    }

    pub(super) fn check_comptime(&self, _comptime: &cst::Comptime) {
        let location = self.current_context().location().clone();
        UnimplementedItem::Comptime.issue(self.compiler, location);
    }

    fn check_array_literal(&mut self, elements: &[ExprId], expected: &Type, id: ExprId) {
        let element_type = expected.array_element(&self.bindings).unwrap_or_else(|| self.next_type_variable());
        for element in elements {
            self.check_expr(*element, &element_type);
        }
        let array_args = Arc::new(vec![Type::U32(elements.len() as u32), element_type]);
        let array_type = Type::Application(Arc::new(Type::ARRAY), array_args);
        self.unify(&array_type, expected, TypeErrorKind::General, id);
    }
}
