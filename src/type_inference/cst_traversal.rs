use std::{borrow::Cow, collections::BTreeMap, sync::Arc};

use crate::{
    diagnostics::{Diagnostic, UnimplementedItem},
    incremental::{GetItemRaw, GetType, Resolve},
    iterator_extensions::mapvec,
    name_resolution::{Origin, builtin::Builtin},
    parser::{
        cst::{self, Definition, Expr, Literal, Pattern},
        ids::{ExprId, NameId, PathId, PatternId, TopLevelName},
    },
    type_inference::{
        Locateable, TypeChecker,
        errors::TypeErrorKind,
        get_type::try_get_type,
        types::{self, Type},
    },
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_definition(&mut self, definition: &Definition) {
        let expected_generalized_type = try_get_type(definition, self.current_context(), &self.current_resolve());
        let expected_type = match expected_generalized_type {
            Some(typ) => typ,
            None => self.next_type_variable(),
        };

        self.check_pattern(definition.pattern, &expected_type);
        self.check_expr(definition.rhs, &expected_type);
    }

    /// Check an expression's type matches the expected type.
    fn check_expr(&mut self, id: ExprId, expected: &Type) {
        self.expr_types.insert(id, expected.clone());

        let expr = match self.current_extended_context().extended_expr(id) {
            Some(expr) => Cow::Owned(expr.clone()),
            None => Cow::Borrowed(&self.current_context().exprs[id]),
        };

        match expr.as_ref() {
            Expr::Literal(literal) => self.check_literal(literal, id, expected),
            Expr::Variable(path) => self.check_path(*path, expected, Some(id)),
            Expr::Call(call) => self.check_call(call, expected),
            Expr::Lambda(lambda) => self.check_lambda(lambda, expected, id),
            Expr::Sequence(items) => {
                for (i, item) in items.iter().enumerate() {
                    let expected_type = if i == items.len() - 1 { expected } else { &self.next_type_variable() };
                    self.check_expr(item.expr, expected_type);
                }
            },
            Expr::Definition(definition) => {
                self.check_definition(definition);
            },
            Expr::MemberAccess(member_access) => self.check_member_access(member_access, expected, id),
            Expr::If(if_) => self.check_if(if_, expected, id),
            Expr::Match(match_) => self.check_match(match_, expected, id),
            Expr::Reference(reference) => self.check_reference(reference, expected, id),
            Expr::TypeAnnotation(type_annotation) => {
                let annotation = self.from_cst_type(&type_annotation.rhs);
                self.unify(expected, &annotation, TypeErrorKind::TypeAnnotationMismatch, id);
                self.check_expr(type_annotation.lhs, &annotation);
            },
            Expr::Handle(handle) => self.check_handle(handle, expected, id),
            Expr::Constructor(constructor) => self.check_constructor(constructor, expected, id),
            Expr::Quoted(_) => {
                let location = id.locate(self);
                UnimplementedItem::Comptime.issue(self.compiler, location);
            },
            Expr::Error => (),
        }
    }

    fn check_literal(&mut self, literal: &Literal, locator: impl Locateable, expected: &Type) {
        let actual = match literal {
            Literal::Unit => Type::UNIT,
            Literal::Integer(_, Some(kind)) => Type::integer(*kind),
            Literal::Float(_, Some(kind)) => Type::float(*kind),
            Literal::Bool(_) => Type::BOOL,
            Literal::Integer(_, None) => Type::I32, // TODO: Polymorphic integers
            Literal::Float(_, None) => Type::F64,   // TODO: Polymorphic floats
            Literal::String(_) => Type::STRING,
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
            None => Cow::Borrowed(&self.current_context().patterns[id]),
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
                    Type::Function(Arc::new(types::FunctionType {
                        parameters: parameters.clone(),
                        return_type: expected.clone(),
                        effects: self.next_type_variable(),
                    }))
                };

                self.check_path(*path, &expected_function_type, None);
                for (expected_arg_type, arg) in parameters.into_iter().zip(args) {
                    self.check_pattern(*arg, &expected_arg_type.typ);
                }
            },
            Pattern::TypeAnnotation(inner_pattern, typ) => {
                let annotated = self.from_cst_type(typ);
                self.unify(expected, &annotated, TypeErrorKind::TypeAnnotationMismatch, id);
                self.check_pattern(*inner_pattern, expected);
            },
        };
    }

    fn check_path(&mut self, path: PathId, expected: &Type, expr: Option<ExprId>) {
        let origin = self.current_resolve().path_origins.get(&path).copied().or_else(|| {
            self.current_extended_context().path_origin(path)
        });

        let actual = match origin {
            Some(Origin::TopLevelDefinition(id)) => {
                if let Some(typ) = self.item_types.get(&id) {
                    typ.clone()
                } else {
                    let typ = GetType(id).get(self.compiler);
                    self.instantiate(typ)
                }
            },
            Some(Origin::Local(name)) => self.name_types[&name].clone(),
            Some(Origin::TypeResolution) => self.resolve_type_resolution(path, expected),
            Some(Origin::Builtin(builtin)) => self.check_builtin(builtin, path),
            None => return,
        };
        if let Some(expr) = expr {
            if self.try_coercion(&actual, expected, path, expr) {
                self.check_expr(expr, expected);
                return;
                // no need to unify or modify self.path_types, that will be handled in the
                // recursive check_expr call since we've just changed the expression at this ExprId.
            }
        }
        self.unify(&actual, expected, TypeErrorKind::General, path);
        self.path_types.insert(path, actual);
    }

    fn resolve_type_resolution(&mut self, path: PathId, expected: &Type) -> Type {
        let path_value = &self.current_context().paths[path];
        assert_eq!(path_value.components.len(), 1, "Only single-component paths should have Origin::TypeResolution");
        let name = path_value.last_ident();

        let Some(id) = self.try_find_type_namespace_for_type_resolution(expected, name) else {
            return self.issue_name_not_in_scope_error(path);
        };

        // Remember what this `Origin::TypeResolution` path actually refers to from now on
        self.current_extended_context_mut().insert_path_origin(path, Origin::TopLevelDefinition(id));

        let result = GetType(id).get(self.compiler);
        self.instantiate(result)
    }

    /// Issue a NameNotInScope error and return Type::Error
    fn issue_name_not_in_scope_error(&self, path: PathId) -> Type {
        let name = Arc::new(self.current_context().paths[path].last_ident().to_owned());
        let location = self.current_context().path_locations[path].clone();
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
            Builtin::Unit => Type::UNIT,
            Builtin::Int
            | Builtin::Char
            | Builtin::Bool
            | Builtin::Float
            | Builtin::String
            | Builtin::Ptr
            | Builtin::PairType => {
                let typ = Arc::new(builtin.to_string());
                let location = locator.locate(self);
                self.compiler.accumulate(Diagnostic::ValueExpected { location, typ });
                return Type::ERROR;
            },
            Builtin::PairConstructor => {
                let a = self.next_type_variable();
                let b = self.next_type_variable();
                let pair = Type::Application(Arc::new(Type::PAIR), Arc::new(vec![a.clone(), b.clone()]));
                Type::Function(Arc::new(types::FunctionType {
                    parameters: vec![types::ParameterType::explicit(a), types::ParameterType::explicit(b)],
                    return_type: pair,
                    effects: Type::UNIT,
                }))
            },
        }
    }

    fn check_call(&mut self, call: &cst::Call, expected: &Type) {
        let expected_parameter_types =
            mapvec(&call.arguments, |arg| types::ParameterType::new(self.next_type_variable(), arg.is_implicit));

        let expected_function_type = {
            let parameters = expected_parameter_types.clone();
            let effects = self.next_type_variable();
            let return_type = expected.clone();
            Type::Function(Arc::new(types::FunctionType { parameters, return_type, effects }))
        };

        self.check_expr(call.function, &expected_function_type);
        for (arg, expected_arg_type) in call.arguments.iter().zip(expected_parameter_types) {
            self.check_expr(arg.expr, &expected_arg_type.typ);
        }
    }

    fn check_lambda(&mut self, lambda: &cst::Lambda, expected: &Type, expr: ExprId) {
        let function_type = match self.follow_type(expected) {
            Type::Function(function_type) => function_type.clone(),
            _ => {
                let parameters =
                    mapvec(&lambda.parameters, |_| types::ParameterType::explicit(self.next_type_variable()));
                let expected_parameter_count = parameters.len();
                let return_type = self.next_type_variable();
                let effects = self.next_type_variable();
                let new_type = Arc::new(types::FunctionType { parameters, return_type, effects });
                let function_type = Type::Function(new_type.clone());
                self.unify(expected, &function_type, TypeErrorKind::Lambda { expected_parameter_count }, expr);
                new_type
            },
        };

        self.check_function_parameter_count(&function_type.parameters, lambda.parameters.len(), expr);
        let parameter_lengths_match = function_type.parameters.len() == lambda.parameters.len();

        for (parameter, expected_type) in lambda.parameters.iter().zip(function_type.parameters.iter()) {
            // Avoid extra errors if the parameter length isn't as expected
            let expected_type = if parameter_lengths_match { &expected_type.typ } else { &Type::ERROR };
            self.check_pattern(parameter.pattern, expected_type);
        }

        // Required in case `function_type` has fewer parameters, to ensure we check all of `lambda.parameters`
        for parameter in lambda.parameters.iter().skip(function_type.parameters.len()) {
            self.check_pattern(parameter.pattern, &Type::ERROR);
        }

        // TODO: Check lambda.effects
        let return_type = if let Some(return_type) = lambda.return_type.as_ref() {
            let return_type = self.from_cst_type(return_type);
            self.unify(&return_type, &function_type.return_type, TypeErrorKind::TypeAnnotationMismatch, expr);
            Cow::Owned(return_type)
        } else {
            Cow::Borrowed(&function_type.return_type)
        };

        self.check_expr(lambda.body, &return_type);
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
                location: self.current_context().expr_locations[expr].clone(),
            });
        }
    }

    fn check_member_access(&mut self, member_access: &cst::MemberAccess, expected: &Type, expr: ExprId) {
        let struct_type = self.next_type_variable();
        self.check_expr(member_access.object, &struct_type);

        let fields = self.get_field_types(&struct_type, None);
        if let Some((field, field_index)) = fields.get(&member_access.member) {
            self.current_extended_context_mut().push_member_access_index(expr, *field_index);
            self.unify(field, expected, TypeErrorKind::General, expr);
        } else if matches!(self.follow_type(&struct_type), Type::Variable(_)) {
            let location = self.current_context().expr_locations[expr].clone();
            self.compiler.accumulate(Diagnostic::TypeMustBeKnownMemberAccess { location });
        } else {
            let typ = self.type_to_string(&struct_type);
            let location = self.current_context().expr_locations[expr].clone();
            let name = Arc::new(member_access.member.clone());
            self.compiler.accumulate(Diagnostic::NoSuchFieldForType { typ, location, name });
        }
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

        self.check_expr(if_.then, &expected);

        // TODO: No way to identify if `then_type != else_type`. This would be useful to point out
        // for error messages.
        if let Some(else_) = if_.else_ {
            self.check_expr(else_, &expected);
        }
    }

    fn check_match(&mut self, match_: &cst::Match, expected: &Type, expr: ExprId) {
        let expr_type = self.next_type_variable();
        self.check_expr(match_.expression, &expr_type);

        for (pattern, branch) in match_.cases.iter() {
            self.check_pattern(*pattern, &expr_type);
            // TODO: Specify if branch_type != type of first branch for better error messages
            self.check_expr(*branch, expected);
        }

        // Now compile the match into a decision tree. The `match expr | ...` expression will be
        // replaced with `<fresh> = expr; <decision tree>`
        let location = self.current_context().expr_locations[match_.expression].clone();
        let (match_var, match_var_name) = self.fresh_match_variable(expr_type.clone(), location.clone());

        // `<match_var> = <expression being matched>`
        let preamble = self.let_binding(match_var_name, match_.expression);

        if let Some(tree) = self.compile_decision_tree(match_var, &match_.cases, expr_type, location) {
            let context = self.current_extended_context_mut();
            context.insert_decision_tree(expr, preamble, tree);
        }
    }

    fn check_reference(&mut self, reference: &cst::Reference, expected: &Type, expr: ExprId) {
        let actual = Type::reference(reference.mutability, reference.sharedness);

        let expected_element_type = match self.follow_type(expected) {
            Type::Application(constructor, args) => {
                let constructor = constructor.clone();
                let args = args.clone();
                let first_arg = args.first();

                match self.follow_type(&constructor) {
                    Type::Primitive(types::PrimitiveType::Reference(..)) => {
                        self.unify(&actual, &constructor, TypeErrorKind::ReferenceKind, expr);

                        // Expect incorrect arg counts to be resolved beforehand
                        first_arg.unwrap().clone()
                    },
                    _ => {
                        if self.unify(&actual, expected, TypeErrorKind::ExpectedNonReference, expr) {
                            first_arg.unwrap().clone()
                        } else {
                            Type::ERROR
                        }
                    },
                }
            },
            Type::Variable(id) => {
                let id = *id;
                let element = self.next_type_variable();
                let expected = Type::Application(Arc::new(actual), Arc::new(vec![element.clone()]));
                self.bindings.insert(id, expected);
                element
            },
            _ => {
                self.unify(&actual, expected, TypeErrorKind::ExpectedNonReference, expr);
                Type::ERROR
            },
        };

        self.check_expr(reference.rhs, &expected_element_type);
    }

    fn check_constructor(&mut self, constructor: &cst::Constructor, expected: &Type, id: ExprId) {
        let typ = self.from_cst_type(&constructor.typ);
        self.unify(&typ, expected, TypeErrorKind::Constructor, id);

        // Map each field name to its index in the type's declaration order.
        // This is used when lowering to MIR when structs are converted into tuples.
        let mut field_order = BTreeMap::new();
        let field_types = self.get_field_types(&typ, None);

        for (name, expr) in &constructor.fields {
            let name_string = &self.current_context().names[*name];
            let (expected_field_type, field_index) = field_types.get(name_string).cloned().unwrap_or((Type::ERROR, 0));

            self.check_expr(*expr, &expected_field_type);
            self.check_name(*name, &expected_field_type);

            field_order.insert(*name, field_index);
        }

        self.current_extended_context_mut().push_constructor_field_order(id, field_order);
    }

    fn check_handle(&mut self, _handle: &cst::Handle, _expected: &Type, expr: ExprId) {
        let location = self.current_context().expr_locations[expr].clone();
        UnimplementedItem::Effects.issue(self.compiler, location);
    }

    pub(super) fn check_extern(&mut self, extern_: &cst::Extern) {
        let typ = self.from_cst_type(&extern_.declaration.typ);
        self.check_name(extern_.declaration.name, &typ);
    }

    pub(super) fn check_comptime(&self, _comptime: &cst::Comptime) {
        let location = self.current_context().location.clone();
        UnimplementedItem::Comptime.issue(self.compiler, location);
    }
}
