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
        affine::MovePath,
        errors::TypeErrorKind,
        get_type::{get_partial_type, try_get_generalized_type},
        types::{self, FunctionType, ParameterType, Type, TypeBindings},
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
            get_partial_type(definition, self.current_context(), self.current_resolve(), self.compiler, next_id);

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
                self.infer_lambda(&lambda, &expected_type, rhs, self_name);
            },
            _ => {
                self.check_expr(rhs, &expected_type, TypeErrorKind::TypeAnnotationMismatch);
            },
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

    /// Infer an expression's type and return it.
    ///
    /// `expected` is only used for resolving names with [Origin::TypeResolution],
    /// it is not unified against.
    pub(super) fn infer_expr(&mut self, id: ExprId, expected: &Type) -> Type {
        // Pre-insert the hint so coercions copying this expression mid-inference
        // can read a type for it. This is overwritten with the inferred type below.
        self.expr_types.insert(id, expected.clone());

        let expr = match self.current_extended_context().extended_expr(id) {
            Some(expr) => Cow::Owned(expr.clone()),
            None => Cow::Borrowed(&self.current_context()[id]),
        };

        let typ = match expr.as_ref() {
            Expr::Literal(literal) => self.infer_literal(literal, id),
            Expr::Variable(path) => self.infer_path(*path, expected),
            Expr::Call(call) => self.infer_call(call, expected, id),
            Expr::Lambda(lambda) => self.infer_lambda(lambda, expected, id, None),
            Expr::Sequence(items) => {
                self.push_implicits_scope();
                let mut result = Type::UNIT;
                for (i, item) in items.iter().enumerate() {
                    let expected_type = if i == items.len() - 1 { expected } else { &self.next_type_variable() };
                    result = self.infer_expr(item.expr, expected_type);
                }
                self.pop_implicits_scope();
                result
            },
            Expr::Definition(definition) => {
                self.check_definition(definition, false);
                Type::UNIT
            },
            Expr::MemberAccess(member_access) => self.infer_member_access(member_access, expected, id),
            Expr::If(if_) => self.infer_if(if_, expected, id),
            Expr::Match(match_) => self.infer_match(match_, expected, id),
            Expr::Reference(reference) => self.infer_reference(reference, expected),
            Expr::Is(_) => unreachable!("Expr::Is should be desugared during GetItem"),
            Expr::Do(_) => unreachable!("Expr::Do should be desugared during GetItem"),
            Expr::TypeAnnotation(type_annotation) => {
                let annotation = self.from_cst_type(&type_annotation.rhs, true);
                let actual = self.infer_expr(type_annotation.lhs, &annotation);
                self.unify(&actual, &annotation, TypeErrorKind::TypeAnnotationMismatch, id);
                annotation
            },
            Expr::Handle(handle) => self.infer_handle(handle, expected),
            Expr::Constructor(constructor) => self.infer_constructor(constructor, expected, id),
            Expr::Quoted(_) => {
                let location = id.locate(self);
                UnimplementedItem::Comptime.issue(self.compiler, location);
                Type::ERROR
            },
            Expr::Loop(_) => unreachable!("Loops should be desugared before type inference"),
            Expr::While(while_) => self.infer_while(while_),
            Expr::For(for_) => self.infer_for(for_, id),
            // Allow break/continue to return any type
            // TODO: Add bottom type
            Expr::Break | Expr::Continue => Type::NEVER,
            Expr::Return(return_) => {
                self.check_return(return_.expression, id);
                Type::NEVER
            },
            Expr::Assignment(assignment) => self.infer_assignment(assignment),
            // Error expressions assume the expected type to suppress cascading errors.
            // This also preserves the recorded types of implicit-argument placeholder
            // slots (which are Expr::Error) when their wrapper is re-inferred.
            Expr::Error => expected.clone(),
            Expr::Extern(_) => self.next_type_variable(),
            Expr::InterpolatedString(_) => {
                unreachable!("InterpolatedString should be desugared before type inference")
            },
            Expr::ArrayLiteral(elements) => self.infer_array_literal(elements, expected),
        };

        self.expr_types.insert(id, typ.clone());
        typ
    }

    /// Infer the expression's type, then unify it against `expected`, reporting an
    /// error with the given kind located at the expression on failure.
    /// Returns the inferred type.
    pub(super) fn check_expr(&mut self, id: ExprId, expected: &Type, kind: TypeErrorKind) -> Type {
        let actual = self.infer_expr(id, expected);
        self.unify(&actual, expected, kind, id);
        actual
    }

    fn infer_literal(&mut self, literal: &Literal, locator: impl Locateable + Copy) -> Type {
        match literal {
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
        }
    }

    pub(super) fn check_name(&mut self, name: NameId, actual: &Type) {
        if let Some(existing) = self.name_types.get(&name) {
            self.unify(actual, &existing.clone(), TypeErrorKind::NameAlreadyBound, name);
        } else {
            self.name_types.insert(name, actual.clone());
        }
    }

    /// Read the pattern for `id`, preferring one added by this type-checking pass and
    /// falling back to the original parsed pattern.
    fn pattern_of(&self, id: PatternId) -> Cow<'local, Pattern> {
        match self.current_extended_context().extended_pattern(id) {
            Some(pattern) => Cow::Owned(pattern.clone()),
            None => Cow::Borrowed(&self.current_context()[id]),
        }
    }

    fn check_pattern(&mut self, id: PatternId, expected: &Type) {
        self.pattern_types.insert(id, expected.clone());

        let pattern = self.pattern_of(id);

        match pattern.as_ref() {
            Pattern::Error => (),
            Pattern::Variable(name) | Pattern::MethodName { item_name: name, .. } => {
                self.check_name(*name, expected);
            },
            Pattern::Literal(literal) => {
                let actual = self.infer_literal(literal, id);
                self.unify(&actual, expected, TypeErrorKind::Pattern, id);
            },
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
                        effects: Type::pure(),
                    }))
                };

                let actual = self.infer_path(*path, &expected_function_type);
                self.unify(&actual, &expected_function_type, TypeErrorKind::Pattern, *path);
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
            Pattern::Alias(name, inner_pattern) => {
                self.check_name(*name, expected);
                self.check_pattern(*inner_pattern, expected);
                // Runs after `check_pattern` so `pattern_types` is populated for the whole subtree.
                let root = super::affine::MovePath::Variable(*name);
                self.assign_binding_places(*inner_pattern, root);
            },
        };
    }

    /// Assign each binding introduced by `pattern` the place it denotes, rooted at `place`, so a
    /// binding and the equivalent member access (`whole.field`) resolve to the same `MovePath`.
    fn assign_binding_places(&mut self, id: PatternId, place: super::affine::MovePath) {
        let pattern = self.pattern_of(id);
        match pattern.as_ref() {
            Pattern::Variable(name) | Pattern::MethodName { item_name: name, .. } => {
                self.binding_places.insert(*name, place);
            },
            Pattern::Alias(name, inner) => {
                self.binding_places.insert(*name, place.clone());
                self.assign_binding_places(*inner, place);
            },
            Pattern::TypeAnnotation(inner, _) => self.assign_binding_places(*inner, place),
            Pattern::Or(alts) => {
                for alt in alts {
                    self.assign_binding_places(*alt, place.clone());
                }
            },
            Pattern::Constructor(_, args) => {
                // Prefer real struct field names; enum payloads and tuples fall back to indices.
                let names_by_index = self.field_names_by_index(id);
                for (i, arg) in args.iter().enumerate() {
                    let field = names_by_index.get(&(i as u32)).cloned().unwrap_or_else(|| i.to_string());
                    let child = super::affine::MovePath::field(place.clone(), field);
                    self.assign_binding_places(*arg, child);
                }
            },
            Pattern::Literal(_) | Pattern::Error => (),
        }
    }

    /// Map each field index of the constructor pattern `id` to its declared field name.
    /// Returns an empty map for non-struct types (enum variants, tuples).
    fn field_names_by_index(&mut self, id: PatternId) -> BTreeMap<u32, String> {
        let Some(typ) = self.pattern_types.get(&id).cloned() else {
            return BTreeMap::default();
        };
        self.get_field_types(&typ, None).into_iter().map(|(name, (_, index))| (index, name.to_string())).collect()
    }

    fn infer_path(&mut self, path: PathId, expected: &Type) -> Type {
        let actual = match self.path_origin(path) {
            Some(Origin::TopLevelDefinition(id)) => self.type_of_top_level_name(&id, path),
            Some(Origin::Local(name)) => {
                let Some(typ) = self.name_types.get(&name).cloned() else {
                    // Name wasn't defined, name resolution should already have emitted an error
                    self.name_types.insert(name, expected.clone());
                    self.path_types.insert(path, expected.clone());
                    return expected.clone();
                };

                let move_path = self.binding_place(name);
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
            None => expected.clone(),
        };
        self.path_types.insert(path, actual.clone());
        actual
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

    fn infer_call(&mut self, call: &cst::Call, expected: &Type, call_expr: ExprId) -> Type {
        // If the function is a MemberAccess, try to resolve it as a method call.
        // E.g. `vec.push 3` is rewritten to `push (mut vec) 3`.
        if let Some(new_expr) = self.try_rewrite_method_call(call, call_expr) {
            return self.infer_expr(new_expr, expected);
        }

        let expected_parameter_types =
            mapvec(&call.arguments, |arg| ParameterType::new(self.next_type_variable(), arg.is_implicit));

        let effects_var = self.fresh_effect_row();
        let mut expected_function_type = Arc::new(FunctionType {
            parameters: expected_parameter_types.clone(),
            environment: self.next_type_variable(),
            return_type: expected.clone(),
            effects: effects_var.clone(),
        });
        let actual_function_type = self.infer_expr(call.function, &Type::Function(expected_function_type.clone()));

        let actual_return_type = self.next_type_variable();
        Arc::make_mut(&mut expected_function_type).return_type = actual_return_type.clone();

        let implicit_count_before_call = self.delayed_implicits_count();

        // This coerce covers inserting any necessary implicit arguments to this function call
        self.coerce(
            &actual_function_type,
            &Type::Function(expected_function_type),
            call.function,
            Some(call_expr),
            TypeErrorKind::Callee,
            None,
        );

        let current_row = self.current_effect_row.clone();
        self.unify(&effects_var, &current_row, TypeErrorKind::Effects, call.function);
        self.current_effect_row = self.canonical_effects_row(&current_row, &TypeBindings::default());

        // FIXME: This is a hack. Type inference benefits if we can push down more expected types by
        // binding the return, which can affect argument types, but it can also lead to coercion errors.
        // As a compromise, we only unify non-type variable returns currently just because it
        // happened to keep most examples working.
        if !matches!(self.follow_type(&actual_return_type), Type::Variable(_))
            && let Ok(bindings) = self.try_unify(&actual_return_type, expected)
        {
            self.bindings.extend(bindings);
        }

        // Infer the arguments. For `+ - * / %`, allow auto-deref of operands.
        let deref_operands = self.is_arithmetic_operator(call.function);
        for (index, (arg, expected_arg_type)) in call.arguments.iter().zip(expected_parameter_types).enumerate() {
            let kind = TypeErrorKind::CallArgument { index };
            self.infer_and_coerce(arg.expr, &expected_arg_type.typ, kind, deref_operands);
        }

        // FIXME: Another related hack. Try to bind the return type now, this time if it has no unbound
        // type variables. Doing so results in some better results when resolving implicits early below.
        if expected.free_vars(&self.bindings).is_empty()
            && let Ok(bindings) = self.try_unify(&actual_return_type, expected)
        {
            self.bindings.extend(bindings);
        }

        // A lot of Extract implicits (.[]) break without this
        self.resolve_new_delayed_implicits(implicit_count_before_call);

        // Ideally we only coerce on call arguments, but this is currently needed.
        // TODO: Take another stab at cleaning up these call rules, but this took much iteration.
        self.coerce(&actual_return_type, expected, call_expr, None, TypeErrorKind::CallReturn, None)
    }

    /// If `call` is `v.push 3` (MemberAccess + args), try to resolve `push` as a function
    /// in the module where `v`'s type is defined. If found, rewrite the Call expression to
    /// `push (mut v) 3` in the extended context and type-check that instead.
    fn try_rewrite_method_call(&mut self, call: &cst::Call, call_expr: ExprId) -> Option<ExprId> {
        let func_expr = match self.current_extended_context().extended_expr(call.function) {
            Some(expr) => expr.clone(),
            None => self.current_context()[call.function].clone(),
        };

        let Expr::MemberAccess(member_access) = &func_expr else {
            return None;
        };

        let object = member_access.object;
        let member = member_access.member.clone();

        // Type-check the object to learn its type.
        // Suppress moves: if the rewrite fails, the normal call handling will
        // process the member access with proper partial-move tracking.
        let hint = self.next_type_variable();
        let old_suppress_check = self.suppress_move_check;
        let old_suppress_record = self.suppress_move_record;
        self.suppress_move_check = true;
        self.suppress_move_record = true;
        let struct_type = self.infer_expr(object, &hint);
        self.suppress_move_check = old_suppress_check;
        self.suppress_move_record = old_suppress_record;

        // Resolve the method name to a top-level function
        let Some((method_name, func_type, bindings)) = self.resolve_method_for_type(&struct_type, &member) else {
            return None;
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
            return None;
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
        Some(call_expr)
    }

    fn infer_lambda(&mut self, lambda: &cst::Lambda, expected: &Type, expr: ExprId, self_name: Option<NameId>) -> Type {
        self.infer_lambda_impl(lambda, expected, expr, self_name, LambdaOptions::default())
    }

    /// Convert a lambda's effects clause to an effect row or a fresh open row if omitted.
    fn effects_from_lambda_clause(&mut self, lambda: &cst::Lambda) -> Type {
        let Some(effects) = &lambda.effects else { return self.fresh_effect_row() };
        let mut local_kinds = types::LocalKinds::default();
        let mut next_id = self.next_type_variable_id.get();
        let typ = Type::from_cst_effects_clause(
            Some(effects),
            self.current_resolve(),
            self.compiler,
            &mut next_id,
            &mut local_kinds,
            true,
            true,
        );
        self.next_type_variable_id.set(next_id);
        typ
    }

    fn infer_lambda_impl(
        &mut self, lambda: &cst::Lambda, expected: &Type, expr: ExprId, self_name: Option<NameId>,
        options: LambdaOptions,
    ) -> Type {
        let function_type = match self.follow_type(expected) {
            Type::Function(function_type) => function_type.clone(),
            _ => {
                let parameters = mapvec(&lambda.parameters, |param| {
                    types::ParameterType::new(self.next_type_variable(), param.is_implicit)
                });
                let expected_parameter_count = parameters.len();
                let environment = self.next_type_variable();
                let return_type = self.next_type_variable();
                let effects = self.effects_from_lambda_clause(lambda);
                let new_type = Arc::new(FunctionType { parameters, environment, return_type, effects });
                let function_type = Type::Function(new_type.clone());
                self.unify(expected, &function_type, TypeErrorKind::Lambda { expected_parameter_count }, expr);
                new_type
            },
        };

        // Remember the return type so that it can be checked by `return` statements
        let old_return_type = self.function_return_type.replace(function_type.return_type.clone());
        let old_effect_row = std::mem::replace(&mut self.current_effect_row, function_type.effects.clone());
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

        self.check_expr(lambda.body, &return_type, TypeErrorKind::FunctionBody);

        // If this lambda's body may execute more than once (e.g. a handler
        // branch), report any non-Copy outer variables moved inside it before
        // we discard the scope-local move tracker. `self.move_tracker` at this
        // point is the branch-local tracker that `mem::take` above started empty.
        if let Some((context, outer_names)) = options.repeated_context.as_ref() {
            self.check_moves_in_repeated_context(outer_names, *context);
        }

        self.function_return_type = old_return_type;
        self.current_effect_row = old_effect_row;
        self.move_tracker = old_move_tracker;

        // Must run before `check_for_closure` may be deferred, so later uses see the move.
        if lambda.is_move {
            self.record_move_captures(expr, self_name);
        }

        let delayed = self.pop_implicits_scope();

        // pop_implicits_scope modifies the function by inserting implicit arguments, we need
        // to check captures only after that step in case any of those arguments are captured.
        // When `delayed` is true, the scope's implicits were deferred to the parent and haven't
        // been resolved yet, so the closure check must also be deferred.
        if self.coercion_wrapper_exprs.contains(&expr) || delayed {
            if let Some(scope) = self.implicits.last_mut() {
                scope.push_deferred_closure_check(expr, function_type.environment.clone(), self_name, lambda.is_move);
            }
        } else {
            self.check_for_closure(expr, &function_type.environment, self_name, lambda.is_move);
        }

        Type::Function(function_type)
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

    fn infer_member_access(&mut self, member_access: &cst::MemberAccess, expected: &Type, expr: ExprId) -> Type {
        let hint = self.next_type_variable();

        // Suppress moves on the object - we handle partial move tracking here at the field level
        let old_suppress_check = self.suppress_move_check;
        let old_suppress_record = self.suppress_move_record;
        self.suppress_move_check = true;
        self.suppress_move_record = true;
        let struct_type = self.infer_expr(member_access.object, &hint);
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
            if !struct_is_indirect && let Some(parent_path) = self.try_build_move_path(member_access.object) {
                let move_path = MovePath::field(parent_path, member_access.member.clone());
                if !old_suppress_check {
                    self.check_use_of_move_path(&move_path, expr);
                }
                if !old_suppress_record && !self.type_is_copy(&field) {
                    let location = expr.locate(self);
                    self.move_tracker.record_move(move_path, location);
                }
            }

            // Copy the field if the expected is not a reference and the field is Copy
            if expected.reference_element(&self.bindings).is_none()
                && let Some((_, inner_field_type)) = field.reference_element(&self.bindings)
                && self.type_is_copy(&inner_field_type)
            {
                let new_expr = self.auto_deref_coercion(expr, inner_field_type);
                self.current_extended_context_mut().insert_expr(expr, new_expr);
                return self.infer_expr(expr, expected);
            }
            field
        } else if matches!(self.follow_type(&struct_type), Type::Variable(_)) {
            let location = expr.locate(self);
            self.compiler.accumulate(Diagnostic::TypeMustBeKnownMemberAccess { location });
            Type::ERROR
        } else {
            let typ = self.type_to_string(&struct_type);
            let location = expr.locate(self);
            let name = Arc::new(member_access.member.clone());
            self.compiler.accumulate(Diagnostic::NoSuchFieldForType { typ, location, name });
            Type::ERROR
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

        self.unify(&param_type, &struct_base, TypeErrorKind::MethodObject, call_expr);

        if let Some(ref_kind) = auto_ref {
            let ref_expr = Expr::Reference(cst::Reference { kind: ref_kind, rhs: object });
            object = self.push_expr(ref_expr, first_param.clone(), location.clone());
        }

        Some(object)
    }

    fn infer_if(&mut self, if_: &cst::If, expected: &Type, expr: ExprId) -> Type {
        self.check_expr(if_.condition, &Type::BOOL, TypeErrorKind::Condition);

        // With an else clause both branches must match; without one the if always
        // returns Unit and the then body may be any type.
        let branch_expected =
            if if_.else_.is_some() { Cow::Borrowed(expected) } else { Cow::Owned(self.next_type_variable()) };

        // Save move state before branches so each branch sees the same pre-branch state
        let pre_branch_moves = self.move_tracker.clone();

        self.push_implicits_scope();
        let then_type = self.infer_expr(if_.then, &branch_expected);
        self.pop_implicits_scope();
        let then_moves = self.move_tracker.clone();

        let then_diverges = self.diverges(&then_type);

        if let Some(else_) = if_.else_ {
            // Reset to pre-branch state for else branch
            self.move_tracker = pre_branch_moves.clone();
            self.push_implicits_scope();
            let else_type = self.infer_expr(else_, &branch_expected);
            self.pop_implicits_scope();

            let else_moves = self.move_tracker.clone();
            let else_diverges = self.diverges(&else_type);

            // Take whichever branch does not diverge
            let result = if then_diverges {
                else_type
            } else if else_diverges {
                then_type
            } else {
                self.unify(&else_type, &then_type, TypeErrorKind::Else, else_);
                then_type
            };

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
            result
        } else {
            // If-without-else: if the then-branch always returns, moves don't carry forward
            if then_diverges {
                self.move_tracker = pre_branch_moves;
            } else {
                self.move_tracker = super::affine::MoveTracker::merge_branches(&pre_branch_moves, &[then_moves]);
            }

            let ok = self.unify(&Type::UNIT, expected, TypeErrorKind::IfStatement, expr);
            // Return error on failure to help prevent cascading errors
            if ok { Type::UNIT } else { Type::ERROR }
        }
    }

    fn infer_match(&mut self, match_: &cst::Match, expected: &Type, expr: ExprId) -> Type {
        let scrutinee_hint = self.next_type_variable();

        // Push an implicits scope here so we can default any integers used in the match
        // to an `I32` before the decision tree checks occur. This lets us compile `match 1 | ...`
        // without errors that the type of `1` is not yet known.
        self.push_implicits_scope();
        let expr_type = self.infer_expr(match_.expression, &scrutinee_hint);

        // Save move state before branches
        let pre_branch_moves = self.move_tracker.clone();
        let mut branch_trackers = Vec::new();
        let mut result_type = Type::NEVER;

        for (pattern, branch) in match_.cases.iter() {
            self.move_tracker = pre_branch_moves.clone();
            self.check_pattern(*pattern, &expr_type);
            self.push_implicits_scope();
            if self.diverges(&result_type) {
                result_type = self.infer_expr(*branch, expected);
            } else {
                self.check_expr(*branch, &result_type, TypeErrorKind::MatchBranch);
            }
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

        result_type
    }

    fn infer_reference(&mut self, reference: &cst::Reference, expected: &Type) -> Type {
        let constructor = Type::reference(reference.kind);

        // Reborrow: `mut x` / `uniq x` where `x` is already a `mut`/`uniq` reference does not
        // nest into `mut (mut t)`; it reborrows the same place, producing a reference of the
        // requested kind to the inner element. This lets index-assignment desugaring wrap the
        // receiver in `mut` uniformly without double-referencing already-mutable receivers
        // (e.g. a `mut Array` parameter). Only mut/uniq reborrow; `ref`/`imm` keep nesting.
        if matches!(reference.kind, ReferenceKind::Mut | ReferenceKind::Uniq) {
            let hint = self.next_type_variable();
            let old_suppress_record = self.suppress_move_record;
            self.suppress_move_record = true;
            let rhs_type = self.infer_expr(reference.rhs, &hint);
            self.suppress_move_record = old_suppress_record;

            let element = match self.follow_type(&rhs_type).reference_element(&self.bindings) {
                Some((inner_kind, inner_element)) if matches!(inner_kind, ReferenceKind::Mut | ReferenceKind::Uniq) => {
                    inner_element
                },
                // Not a reborrow: wrap `rhs_type` in the requested reference kind.
                _ => rhs_type,
            };

            let lifetime = self.next_type_variable();
            return Type::Application(Arc::new(constructor), Arc::new(vec![lifetime, element]));
        }

        // Use the expected element type as a hint when it is available
        let element_hint = match self.follow_type(expected) {
            Type::Application(_, args) if args.len() == 2 => args[1].clone(),
            _ => self.next_type_variable(),
        };

        // A reference doesn't move its rhs, but it still reads it, so we must
        // check that the rhs isn't already moved.
        let old_suppress_record = self.suppress_move_record;
        self.suppress_move_record = true;
        let element = self.infer_expr(reference.rhs, &element_hint);
        self.suppress_move_record = old_suppress_record;

        let lifetime = self.next_type_variable();
        Type::Application(Arc::new(constructor), Arc::new(vec![lifetime, element]))
    }

    fn infer_constructor(&mut self, constructor: &cst::Constructor, expected: &Type, id: ExprId) -> Type {
        let (mut typ, kind) = self.from_cst_type_and_kind(&constructor.typ, true);

        // Type arguments on constructors are optional (both `Clone t with ..` and `Clone with ..` are allowed)
        // So fill in any empty slots with fresh type variables.
        let required_argument_count = kind.required_argument_count();
        if required_argument_count != 0 {
            let args = mapvec(0..required_argument_count, |_| self.next_type_variable());
            typ = Type::Application(Arc::new(typ), Arc::new(args));

            // Eagerly unify with the expected type so the fields below are checked against
            // concrete types where possible. Errors are ignored here: if unification fails,
            // the caller will report the mismatch when unifying our return type.
            if let Ok(bindings) = self.try_unify(&typ, expected) {
                self.bindings.extend(bindings);
            }
        }

        // Map each field name to its index in the type's declaration order.
        // This is used when lowering to MIR when structs are converted into tuples.
        let mut field_order = BTreeMap::new();
        let field_types = self.get_field_types(&typ, None);

        for (name, expr) in &constructor.fields {
            let name_string = &self.current_context()[*name];
            let (expected_field_type, field_index) = field_types.get(name_string).cloned().unwrap_or((Type::ERROR, 0));

            self.check_expr(*expr, &expected_field_type, TypeErrorKind::ConstructorField);
            self.check_name(*name, &expected_field_type);

            field_order.insert(*name, field_index);
        }

        self.current_extended_context_mut().push_constructor_field_order(id, field_order);
        typ
    }

    fn infer_handle(&mut self, handle: &cst::Handle, expected: &Type) -> Type {
        // The type of the handled expression and of every handler branch.
        // The expected type is threaded through (rather than a fresh variable) so
        // type resolution in branches (e.g. `None` in `fail () -> None`) sees it,
        // and so `Never`-typed branches unify against it as the actual type.
        let result_type = expected.clone();

        // TODO: Add a way for users to use this handler name to manually specify
        // an effect handler when there are multiple in scope.
        let handler_name_type = self.next_type_variable();
        self.name_types.insert(handle.handler_name, handler_name_type.clone());

        // The parser wraps the handled expression in `fn () -> <body>` to serve as the
        // coroutine's init function.
        let body_env = self.next_type_variable();
        let body_row = self.fresh_effect_row();
        let body_type = Type::Function(Arc::new(FunctionType {
            parameters: vec![ParameterType::explicit(Type::UNIT)],
            environment: body_env,
            return_type: result_type.clone(),
            effects: body_row.clone(),
        }));
        self.expr_types.insert(handle.expression, body_type.clone());

        // Prevent any names visible from before the handler branches from being moved
        // TODO: This is inefficient, remove the need for collecting here
        let outer_names = self.name_types.keys().copied().collect::<FxHashSet<_>>();

        let mut handled_effect: Option<Type> = None;

        // The effects each branch performs
        let mut branch_rows = Vec::with_capacity(handle.cases.len());

        // For each case:
        // - pattern.function: fn args.. {Effect..} -> r can e
        // - branch lambda:    fn args.. resume -> expected can expected_effect
        // - resume:           fn r -> expected
        for (pattern, branch) in &handle.cases {
            let parameter_types = mapvec(&pattern.args, |_| ParameterType::explicit(self.next_type_variable()));
            let r = self.next_type_variable();
            let e = self.next_type_variable();

            // The effect operation is now an ordinary top-level function (see `build_method_types`).
            let function_type = Type::Function(Arc::new(FunctionType {
                parameters: parameter_types.clone(),
                environment: Type::NO_CLOSURE_ENV,
                return_type: r.clone(),
                effects: e.clone(),
            }));
            let actual = self.infer_path(pattern.function, &function_type);
            self.unify(&actual, &function_type, TypeErrorKind::EffectPattern, pattern.function);

            match &handled_effect {
                None => handled_effect = Some(e),
                Some(existing) => {
                    self.unify(&e, existing, TypeErrorKind::General, pattern.function);
                },
            }

            // resume is a closure capturing its environment by reference.
            // The coroutine lowering pass supplies a closure with an env pointing to `(coro, handlers..)`.
            let resume_type = Type::Function(Arc::new(FunctionType {
                parameters: vec![ParameterType::explicit(r)],
                environment: Type::Primitive(types::PrimitiveType::Pointer),
                return_type: result_type.clone(),
                effects: Type::pure(),
            }));

            let mut handler_params = parameter_types;
            handler_params.push(ParameterType::explicit(resume_type));
            let branch_row = self.fresh_effect_row();
            let handler_type = Type::Function(Arc::new(FunctionType {
                parameters: handler_params,
                environment: self.next_type_variable(),
                return_type: result_type.clone(),
                effects: branch_row.clone(),
            }));
            branch_rows.push(branch_row);

            // Only allow moving variables into this branch if `resume` is never mentioned.
            // This notably keeps handlers like `try_or` working.
            let repeated_context = self
                .handler_branch_uses_resume(pattern.resume_name, *branch)
                .then(|| (RepeatedContext::HandlerBranch, outer_names.clone()));

            let options = LambdaOptions { repeated_context };

            let branch_lambda = self.unwrap_lambda(*branch);
            self.expr_types.insert(*branch, handler_type.clone());
            self.infer_lambda_impl(branch_lambda, &handler_type, *branch, None, options);
        }

        // There's always at least one case, so `handled_effect` is always set by now.
        let handled_effect = handled_effect.as_ref().unwrap();
        self.unify(&handler_name_type, handled_effect, TypeErrorKind::General, handle.expression);

        let options = LambdaOptions::default();
        let body_lambda = self.unwrap_lambda(handle.expression);

        // `Some(handler_name)` exempts the variable from being captured as a closure.
        // This is instead handled as a special case in mir generation
        self.infer_lambda_impl(body_lambda, &body_type, handle.expression, Some(handle.handler_name), options);

        // Any unhandled effects escape this handler
        let mut new_bindings = TypeBindings::default();
        let leftover =
            self.discharge_effect(&body_row, handled_effect, &mut new_bindings).unwrap_or_else(|_| body_row.clone());

        self.bindings.extend(new_bindings);

        // TODO: Is it really necessary to reunify combined with each row
        let combined = self.fresh_effect_row();
        self.unify(&leftover, &combined, TypeErrorKind::Effects, handle.expression);

        for branch_row in &branch_rows {
            self.unify(branch_row, &combined, TypeErrorKind::Effects, handle.expression);
        }
        let current_row = self.current_effect_row.clone();
        self.unify(&combined, &current_row, TypeErrorKind::Effects, handle.expression);

        result_type
    }

    /// Peel `effect` out of `row`'s canonicalized effect list, returning the row with that entry removed.
    /// Returns Err if `effect` isn't present in `row`.
    fn discharge_effect(&self, row: &Type, effect: &Type, new_bindings: &mut TypeBindings) -> Result<Type, ()> {
        let row = self.canonical_effects_row(row, new_bindings);
        let effect = self.canonical_effects_row(effect, new_bindings);
        let Type::Effects(list, tail) = &row else { unreachable!("canonical_effects_row always returns Effects") };
        let Type::Effects(to_remove, _) = &effect else { unreachable!("canonical_effects_row always returns Effects") };

        let mut new_list = list.to_vec();
        let mut found = false;
        for entry in to_remove.iter() {
            if let Some(pos) = self.subtype_matching_effect(&new_list, |_| false, entry, new_bindings)? {
                new_list.remove(pos);
                found = true;
            }
        }

        if !found {
            return Err(());
        }
        Ok(Type::effects(new_list, tail.as_deref().cloned()))
    }

    /// Retrieve the [`cst::Lambda`] at `expr_id` or panic otherwise.
    fn unwrap_lambda(&self, expr_id: ExprId) -> &'local cst::Lambda {
        match &self.current_context()[expr_id] {
            Expr::Lambda(lambda) => lambda,
            other => unreachable!("Expected a lambda, found {other:?}"),
        }
    }

    fn infer_assignment(&mut self, assignment: &cst::Assignment) -> Type {
        let lhs_hint = self.next_type_variable();

        // Allow `x := v` to use `x` even if moved but `x += v` cannot since it reads `x`
        let is_plain = assignment.op.is_none();
        let lhs_type = if is_plain {
            let old_suppress_check = std::mem::replace(&mut self.suppress_move_check, true);
            let old_suppress_record = std::mem::replace(&mut self.suppress_move_record, true);
            let lhs_type = self.infer_expr(assignment.lhs, &lhs_hint);
            self.suppress_move_check = old_suppress_check;
            self.suppress_move_record = old_suppress_record;
            lhs_type
        } else {
            self.infer_expr(assignment.lhs, &lhs_hint)
        };

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
                effects: self.fresh_effect_row(),
            }));
            // The operator's ability constraint is an implicit parameter, so coerce
            // before unifying like other function positions
            let actual = self.infer_expr(op_expr, &expected_fn_type);
            match self.try_coercion(&actual, &expected_fn_type, op_expr, None) {
                super::CoercionOutcome::ReplacedExpr | super::CoercionOutcome::AutoRef => {
                    self.check_expr(op_expr, &expected_fn_type, TypeErrorKind::CompoundOperator);
                },
                super::CoercionOutcome::InPlaceCall | super::CoercionOutcome::None => {
                    self.unify(&actual, &expected_fn_type, TypeErrorKind::CompoundOperator, op_expr);
                },
            }
        }

        self.check_expr(assignment.rhs, &value_type, TypeErrorKind::Assignment);

        // The LHS always holds a value after an assignment
        if let Some(path) = self.try_build_move_path(assignment.lhs) {
            self.move_tracker.clear_moves(&path);
        }

        Type::UNIT
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
            Pattern::Alias(name, inner) => {
                self.mutable_definitions.insert(*name);
                self.record_mutable_pattern(*inner);
            },
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
                        if let Some(typ) = self.name_types.get(&name)
                            && (self.is_mut_or_uniq_reference(typ) || self.is_shared_mut_user_defined(typ))
                        {
                            return Ok(());
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
                if let Some(typ) = self.expr_types.get(&object)
                    && (self.is_mut_or_uniq_reference(typ) || self.is_shared_mut_user_defined(typ))
                {
                    return Ok(());
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
                self.check_expr(returned_expr, &expected_return, TypeErrorKind::Return);
            },
            None => {
                let location = id.locate(self);
                self.compiler.accumulate(Diagnostic::ReturnNotInFunction { location });
            },
        }
    }

    fn infer_while(&mut self, while_: &cst::While) -> Type {
        // Both the condition and body may execute more than once, so moves of
        // outer non-Copy values inside either are unsound.
        let outer_names = self.name_types.keys().copied().collect::<FxHashSet<_>>();
        let old_tracker = std::mem::take(&mut self.move_tracker);

        self.check_expr(while_.condition, &Type::BOOL, TypeErrorKind::Condition);
        self.check_expr(while_.body, &Type::UNIT, TypeErrorKind::LoopBody);

        self.check_moves_in_repeated_context(&outer_names, RepeatedContext::WhileLoop);
        self.move_tracker = old_tracker;

        Type::UNIT
    }

    fn infer_for(&mut self, for_: &cst::For, id: ExprId) -> Type {
        let int_ty = self.next_type_variable_id();
        // Hack: use 0 for the integer value here since it fits into all integer types.
        // We just need this to ensure the user actually uses integer ranges. Any actual integer
        // they choose will already have its range checked by the Literal code path.
        self.push_inferred_int(Integer::positive(0), int_ty, id.locate(self));
        let int_ty = Type::Variable(int_ty);

        // Range expressions run exactly once, so normal move semantics apply here.
        self.check_expr(for_.start, &int_ty, TypeErrorKind::LoopRange);
        self.check_expr(for_.end, &int_ty, TypeErrorKind::LoopRange);

        // Snapshot outer names before introducing the loop variable so the
        // loop variable itself is not counted as an outer binding.
        let outer_names = self.name_types.keys().copied().collect::<FxHashSet<_>>();
        self.name_types.insert(for_.variable, int_ty);

        let old_tracker = std::mem::take(&mut self.move_tracker);
        self.check_expr(for_.body, &Type::UNIT, TypeErrorKind::LoopBody);
        self.check_moves_in_repeated_context(&outer_names, RepeatedContext::ForLoop);
        self.move_tracker = old_tracker;

        Type::UNIT
    }

    pub(super) fn check_comptime(&self, _comptime: &cst::Comptime) {
        let location = self.current_context().location().clone();
        UnimplementedItem::Comptime.issue(self.compiler, location);
    }

    fn infer_array_literal(&mut self, elements: &[ExprId], expected: &Type) -> Type {
        let element_type = expected.array_element(&self.bindings).unwrap_or_else(|| self.next_type_variable());
        for element in elements {
            self.check_expr(*element, &element_type, TypeErrorKind::ArrayElement);
        }
        let array_args = Arc::new(vec![Type::U32(elements.len() as u32), element_type]);
        Type::Application(Arc::new(Type::ARRAY), array_args)
    }
}
