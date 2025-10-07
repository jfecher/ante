use std::sync::Arc;

use crate::{
    diagnostics::Diagnostic,
    incremental::GetType,
    iterator_extensions::vecmap,
    name_resolution::{builtin::Builtin, Origin},
    parser::{
        cst::{self, Definition, Expr, Literal, Pattern},
        ids::{ExprId, NameId, PathId, PatternId},
    },
    type_inference::{
        errors::TypeErrorKind,
        get_type::try_get_type,
        type_id::TypeId,
        types::{self, Type},
        Locateable, TypeChecker,
    },
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_definition(&mut self, definition: &Definition) -> TypeId {
        let expected_generalized_type = try_get_type(definition, self.current_context(), &self.current_resolve());
        let expected_type = match expected_generalized_type.as_ref() {
            Some(typ) => typ.as_type(&mut self.types),
            None => self.next_type_variable(),
        };

        self.check_expr(definition.rhs, expected_type);
        self.check_pattern(definition.pattern, expected_type);
        expected_type
    }

    fn check_expr(&mut self, expr: ExprId, expected: TypeId) {
        match &self.current_context().exprs[expr] {
            Expr::Literal(literal) => self.check_literal(literal, expr, expected),
            Expr::Variable(path) => self.check_path(*path, expected),
            Expr::Call(call) => self.check_call(call, expected),
            Expr::Lambda(lambda) => self.check_lambda(lambda, expected, expr),
            Expr::Sequence(items) => {
                for (i, item) in items.iter().enumerate() {
                    let expected_type = if i == items.len() - 1 { expected } else { self.next_type_variable() };
                    self.check_expr(item.expr, expected_type);
                }
            },
            Expr::Definition(definition) => {
                self.check_definition(definition);
            },
            Expr::MemberAccess(member_access) => self.check_member_access(member_access, expected, expr),
            Expr::Index(index) => self.check_index(index, expected),
            Expr::If(if_) => self.check_if(if_, expected, expr),
            Expr::Match(match_) => self.check_match(match_, expected),
            Expr::Reference(reference) => self.check_reference(reference, expected, expr),
            Expr::TypeAnnotation(type_annotation) => {
                let annotation = self.convert_ast_type(&type_annotation.rhs);
                self.unify(expected, annotation, TypeErrorKind::TypeAnnotationMismatch, expr);
                self.check_expr(type_annotation.lhs, annotation);
            },
            Expr::Handle(handle) => self.check_handle(handle, expected),
            Expr::Constructor(constructor) => self.check_constructor(constructor, expected, expr),
            Expr::Quoted(_) => todo!("type check Expr::Quoted"),
            Expr::Error => (),
        };
        self.expr_types.insert(expr, expected);
    }

    fn check_literal(&mut self, literal: &Literal, locator: impl Locateable, expected: TypeId) {
        let actual = match literal {
            Literal::Unit => TypeId::UNIT,
            Literal::Integer(_, Some(kind)) => TypeId::integer(*kind),
            Literal::Float(_, Some(kind)) => TypeId::float(*kind),
            Literal::Bool(_) => TypeId::BOOL,
            Literal::Integer(_, None) => TypeId::I32, // TODO: Polymorphic integers
            Literal::Float(_, None) => TypeId::F64,   // TODO: Polymorphic floats
            Literal::String(_) => TypeId::STRING,
            Literal::Char(_) => TypeId::CHAR,
        };
        self.unify(actual, expected, TypeErrorKind::General, locator);
    }

    fn check_name(&mut self, name: NameId, expected: TypeId) {
        if let Some(existing) = self.name_types.get(&name) {
            self.unify(expected, *existing, TypeErrorKind::General, name);
        } else {
            self.name_types.insert(name, expected);
        }
    }

    fn check_pattern(&mut self, pattern: PatternId, expected: TypeId) {
        match &self.current_context().patterns[pattern] {
            Pattern::Error => (),
            Pattern::Variable(name) | Pattern::MethodName { item_name: name, .. } => {
                if let Some(existing) = self.name_types.get(name) {
                    self.unify(expected, *existing, TypeErrorKind::General, pattern);
                } else {
                    self.name_types.insert(*name, expected);
                }
            },
            Pattern::Literal(literal) => self.check_literal(literal, pattern, expected),
            Pattern::Constructor(path, args) => {
                let parameters = vecmap(args, |_| self.next_type_variable());

                let expected_function_type = if args.is_empty() {
                    expected
                } else {
                    let function = Type::Function(types::FunctionType {
                        parameters: parameters.clone(),
                        return_type: expected,
                        effects: self.next_type_variable(),
                    });
                    self.types.get_or_insert_type(function)
                };

                self.check_path(*path, expected_function_type);
                for (expected_arg_type, arg) in parameters.into_iter().zip(args) {
                    self.check_pattern(*arg, expected_arg_type);
                }
            },
            Pattern::TypeAnnotation(inner_pattern, typ) => {
                let annotated = self.convert_ast_type(typ);
                self.unify(expected, annotated, TypeErrorKind::TypeAnnotationMismatch, pattern);
                self.check_pattern(*inner_pattern, expected);
            },
        };
    }

    fn check_path(&mut self, path: PathId, expected: TypeId) {
        let actual = match self.current_resolve().path_origins.get(&path).copied() {
            Some(Origin::TopLevelDefinition(id)) => {
                if let Some(typ) = self.item_types.get(&id) {
                    *typ
                } else {
                    let typ = GetType(id).get(self.compiler);
                    self.instantiate(&typ)
                }
            },
            Some(Origin::Local(name)) => self.name_types[&name],
            Some(Origin::TypeResolution) => todo!("Type check Origin::TypeResolution"),
            Some(Origin::Builtin(builtin)) => {
                self.check_builtin(builtin, expected, path);
                return;
            },
            None => return,
        };
        self.unify(actual, expected, TypeErrorKind::General, path);
        self.path_types.insert(path, expected);
    }

    /// Returns the instantiated type of a builtin value
    ///
    /// Will error if passed a builtin type
    fn check_builtin(&mut self, builtin: Builtin, expected: TypeId, locator: impl Locateable) {
        let actual = match builtin {
            Builtin::Unit => TypeId::UNIT,
            Builtin::Int | Builtin::Char | Builtin::Float | Builtin::String | Builtin::Ptr | Builtin::PairType => {
                let typ = Arc::new(builtin.to_string());
                let location = locator.locate(self);
                self.compiler.accumulate(Diagnostic::ValueExpected { location, typ });
                return;
            },
            Builtin::PairConstructor => {
                // Fast-track to avoid creating unnecessary type variables and a function type
                // since each `a, b` will match any argument type anyway.
                if let Type::Function(function) = self.types.get_type(expected) {
                    if function.parameters.len() == 2 {
                        if let Type::Application(constructor, args) = self.types.get_type(function.return_type) {
                            if args.len() == 2 {
                                self.unify(TypeId::PAIR, *constructor, TypeErrorKind::General, locator);
                                return;
                            }
                        }
                    }
                }

                let a = self.next_type_variable();
                let b = self.next_type_variable();
                let pair = self.types.get_or_insert_type(Type::Application(TypeId::PAIR, vec![a, b]));
                let function = Type::Function(types::FunctionType {
                    parameters: vec![a, b],
                    return_type: pair,
                    effects: TypeId::UNIT,
                });
                self.types.get_or_insert_type(function)
            },
        };
        self.unify(actual, expected, TypeErrorKind::General, locator);
    }

    fn check_call(&mut self, call: &cst::Call, expected: TypeId) {
        let expected_parameter_types = vecmap(&call.arguments, |_| self.next_type_variable());

        let expected_function_type = {
            let parameters = expected_parameter_types.clone();
            let effects = self.next_type_variable();
            let function = Type::Function(types::FunctionType { parameters, return_type: expected, effects });
            self.types.get_or_insert_type(function)
        };

        self.check_expr(call.function, expected_function_type);
        for (arg, expected_arg_type) in call.arguments.iter().zip(expected_parameter_types) {
            self.check_expr(*arg, expected_arg_type);
        }
    }

    fn check_lambda(&mut self, lambda: &cst::Lambda, expected: TypeId, expr: ExprId) {
        let mut function_type = match self.follow_type(expected) {
            Type::Function(function_type) => function_type.clone(),
            _ => {
                let parameters = vecmap(&lambda.parameters, |_| self.next_type_variable());
                let expected_parameter_count = parameters.len();
                let return_type = self.next_type_variable();
                let effects = self.next_type_variable();
                let new_type = types::FunctionType { parameters, return_type, effects };
                let new_id = self.types.get_or_insert_type(Type::Function(new_type.clone()));
                self.unify(expected, new_id, TypeErrorKind::Lambda { expected_parameter_count }, expr);
                new_type
            },
        };

        self.check_function_parameter_count(&mut function_type.parameters, lambda.parameters.len(), expr);
        assert_eq!(lambda.parameters.len(), function_type.parameters.len());

        for (parameter, expected_type) in lambda.parameters.iter().zip(function_type.parameters) {
            self.check_pattern(parameter.pattern, expected_type);
        }

        // TODO: Check lambda.effects
        let return_type = if let Some(return_type) = lambda.return_type.as_ref() {
            let return_type = self.convert_ast_type(return_type);
            self.unify(return_type, function_type.return_type, TypeErrorKind::TypeAnnotationMismatch, expr);
            return_type
        } else {
            function_type.return_type
        };

        self.check_expr(lambda.body, return_type);
    }

    /// Check a function's parameter count using the given parameter types as the expected count.
    /// Issues an error if the expected count does not match the actual count, and resizes the
    /// given parameter type Vec.
    fn check_function_parameter_count(&mut self, parameters: &mut Vec<TypeId>, actual_count: usize, expr: ExprId) {
        if actual_count != parameters.len() {
            self.compiler.accumulate(Diagnostic::FunctionArgCountMismatch {
                actual: actual_count,
                expected: parameters.len(),
                location: self.current_context().expr_locations[expr].clone(),
            });
            parameters.resize_with(actual_count, || self.next_type_variable());
        }
    }

    fn check_member_access(&mut self, member_access: &cst::MemberAccess, expected: TypeId, expr: ExprId) {
        let struct_type = self.next_type_variable();
        self.check_expr(member_access.object, struct_type);

        let fields = self.get_field_types(struct_type, None);
        if let Some(field) = fields.get(&member_access.member) {
            // TODO: How should we differentiate between shared and owned variants?
            let result = match member_access.ownership {
                cst::OwnershipMode::Owned => *field,
                cst::OwnershipMode::Borrow => {
                    let reference = Type::Application(TypeId::REF, vec![*field]);
                    self.types.get_or_insert_type(reference)
                },
                cst::OwnershipMode::BorrowMut => {
                    let reference = Type::Application(TypeId::REF_MUT, vec![*field]);
                    self.types.get_or_insert_type(reference)
                },
            };

            self.unify(result, expected, TypeErrorKind::General, expr);
        } else if matches!(self.follow_type(struct_type), Type::Variable(_)) {
            let location = self.current_context().expr_locations[expr].clone();
            self.compiler.accumulate(Diagnostic::TypeMustBeKnownMemberAccess { location });
        } else {
            let typ = self.type_to_string(struct_type);
            let location = self.current_context().expr_locations[expr].clone();
            let name = Arc::new(member_access.member.clone());
            self.compiler.accumulate(Diagnostic::NoSuchFieldForType { typ, location, name });
        }
    }

    fn check_index(&mut self, _index: &cst::Index, _expected: TypeId) {
        todo!()
    }

    fn check_if(&mut self, if_: &cst::If, expected: TypeId, expr: ExprId) {
        self.check_expr(if_.condition, TypeId::BOOL);

        // If there's an else clause our expected return type should match the then/else clauses'
        // types. Otherwise, the then body may be any type.
        let expected = if if_.else_.is_some() {
            expected
        } else {
            self.unify(TypeId::UNIT, expected, TypeErrorKind::IfStatement, expr);
            self.next_type_variable()
        };

        self.check_expr(if_.then, expected);

        // TODO: No way to identify if `then_type != else_type`. This would be useful to point out
        // for error messages.
        if let Some(else_) = if_.else_ {
            self.check_expr(else_, expected);
        }
    }

    fn check_match(&mut self, match_: &cst::Match, expected: TypeId) {
        let expr_type = self.next_type_variable();
        self.check_expr(match_.expression, expr_type);

        for (pattern, branch) in match_.cases.iter() {
            self.check_pattern(*pattern, expr_type);
            // TODO: Specify if branch_type != type of first branch for better error messages
            self.check_expr(*branch, expected);
        }
    }

    fn check_reference(&mut self, reference: &cst::Reference, expected: TypeId, expr: ExprId) {
        let actual = TypeId::reference(reference.mutability, reference.sharedness);
        let expected_element_type = match self.follow_type(expected) {
            Type::Application(constructor, args) => {
                let first_arg = args.first().copied();
                match self.follow_type(*constructor) {
                    Type::Primitive(types::PrimitiveType::Reference(..)) => {
                        self.unify(actual, *constructor, TypeErrorKind::ReferenceKind, expr);

                        // Expect incorrect arg counts to be resolved beforehand
                        first_arg.unwrap()
                    }
                    _ => {
                        if self.unify(actual, expected, TypeErrorKind::ExpectedNonReference, expr) {
                            first_arg.unwrap()
                        } else {
                            TypeId::ERROR
                        }
                    }
                }
            },
            Type::Variable(id) => {
                let id = *id;
                let element = self.next_type_variable();
                let expected = Type::Application(actual, vec![element]);
                let expected = self.types.get_or_insert_type(expected);
                self.bindings.insert(id, expected);
                element
            },
            _ => {
                self.unify(actual, expected, TypeErrorKind::ExpectedNonReference, expr);
                TypeId::ERROR
            }
        };

        self.check_expr(reference.rhs, expected_element_type);
    }

    fn check_constructor(&mut self, constructor: &cst::Constructor, expected: TypeId, id: ExprId) {
        let typ = self.convert_ast_type(&constructor.typ);
        self.unify(typ, expected, TypeErrorKind::Constructor, id);

        let field_types = self.get_field_types(typ, None);

        for (name, expr) in &constructor.fields {
            let name_string = &self.current_context().names[*name];
            let expected_field_type = field_types.get(name_string).copied().unwrap_or(TypeId::ERROR);

            self.check_expr(*expr, expected_field_type);
            self.check_name(*name, expected_field_type);
        }
    }

    fn check_handle(&mut self, _handle: &cst::Handle, _expected: TypeId) {
        todo!("check_handle")
    }

    pub(super) fn check_impl(&self, _trait_impl: &cst::TraitImpl) -> TypeId {
        unreachable!("impls should be simplified into definitions by this point")
    }

    pub(super) fn check_extern(&mut self, extern_: &cst::Extern) -> TypeId {
        let typ = self.convert_ast_type(&extern_.declaration.typ);
        self.check_name(extern_.declaration.name, typ);
        typ
    }

    pub(super) fn check_comptime(&self, _comptime: &cst::Comptime) -> TypeId {
        todo!("check_comptime")
    }
}
