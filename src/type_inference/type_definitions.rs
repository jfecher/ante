use rustc_hash::FxHashMap;

use crate::{
    diagnostics::Diagnostic,
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst,
        ids::{NameId, TopLevelId, TopLevelName},
    },
    shared_arc::Shared,
    type_inference::{
        Locateable, TypeChecker,
        generics::Generic,
        recursive_types,
        types::{self, GenericSubstitutions, ParameterType, Type},
    },
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_type_definition(&mut self, definition: &cst::TypeDefinition) {
        let id = self.current_item.unwrap();
        self.check_generic_defaults(definition);

        // Each constructor's name along with the name, if it has one, and type of each of its fields
        let constructors = match &definition.body {
            cst::TypeDefinitionBody::Error => Vec::new(),
            cst::TypeDefinitionBody::Alias(body) => {
                // Convert the body even though the result is unused to issue kind or recursion errors
                Self::reject_implicit_places(body, self.compiler);
                let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);
                let _ = self.from_cst_type_in_type_definition(body, false, &mut local_kinds);
                return;
            },
            cst::TypeDefinitionBody::EffectAlias(effects) => {
                // Convert each effect even though the result is unused to issue kind or recursion errors
                let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);
                let _ = self.with_next_id(|next_id| {
                    Type::from_cst_effects_clause(
                        Some(effects),
                        self.current_resolve(),
                        self.compiler,
                        next_id,
                        &mut local_kinds,
                        false,
                        false,
                    )
                });
                return;
            },
            cst::TypeDefinitionBody::Struct(fields) => {
                if definition.kind.is_effect() {
                    self.build_effect_operation_types(id, definition, fields);
                    return;
                }
                let fields = mapvec(fields, |(name, field_type)| (Some(*name), field_type));
                vec![(definition.name, fields)]
            },
            cst::TypeDefinitionBody::Enum(variants, _) => {
                mapvec(variants, |(name, fields)| (*name, mapvec(fields, |(name, field_type)| (*name, field_type))))
            },
        };

        let type_name = TopLevelName::new(id, definition.name);

        if !definition.kind.is_ability() && recursive_types::is_infinitely_sized(type_name, definition, self.compiler) {
            let typ = self.current_context()[definition.name].as_ref().clone();
            let location = definition.name.locate(self);
            self.compiler.accumulate(Diagnostic::RecursiveType { typ, location });
        }

        let generics = mapvec(&definition.generics, |p| Generic::Named(Origin::Local(p.name)));

        // Share kind info across all variant fields of this definition so that, e.g., two
        // fields that mention `n` agree on `n`'s kind.
        let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);

        for (constructor_name, fields) in constructors.iter() {
            let parameters = self.convert_constructor_parameters(definition, fields, &mut local_kinds);
            if definition.kind.is_trait() {
                self.build_trait_member_types(type_name, definition, &parameters);
            } else {
                // Give each field declaration a type and origin like any other name for tooling to use
                for ((field, _), field_type) in fields.iter().zip(&parameters) {
                    if let Some(field) = *field {
                        self.name_types.insert(field, field_type.clone());
                        let origin = Origin::TopLevelDefinition(TopLevelName::new(id, field));
                        self.current_extended_context_mut().insert_name_origin(field, origin);
                    }
                }
            }
            let actual = self.build_constructor_type(type_name, definition, &generics, parameters);
            self.check_name(*constructor_name, &actual);
        }
    }

    /// Convert each parameter default to issue errors
    fn check_generic_defaults(&mut self, definition: &cst::TypeDefinition) {
        let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);
        for param in &definition.generics {
            let Some(default) = &param.default else { continue };
            Self::reject_implicit_places(default, self.compiler);

            let kind = local_kinds[&param.name].clone();
            let _ = self.from_cst_type_with_local_kinds(default, kind, true, false, &mut local_kinds);
        }
    }

    /// Given a type definition such as `type Vec t = ...`, return it as a type.
    /// For a non-generic type this is simply the `Type::UserDefined(_)` referring
    /// to this type.
    ///
    /// If `instantiate` is true, this will instantiate the variables of a generic type,
    /// returning the substitutions along the type itself. For example, `Vec t` instantiated
    /// may return `(Vec _1, [t -> _1])`
    ///
    /// For generic types, the type will be instantiated with fresh type
    /// variables that are returned along with the `Type::Application` of the user-defined type.
    fn type_definition_type(
        &mut self, type_name: TopLevelName, item: &cst::TypeDefinition, instantiate: bool,
    ) -> (Type, GenericSubstitutions) {
        let mut substitutions = FxHashMap::default();
        let mut data_type = Type::UserDefined(Origin::TopLevelDefinition(type_name));

        if !item.generics.is_empty() {
            if instantiate {
                let fresh_vars = mapvec(&item.generics, |_| self.next_type_variable());
                substitutions = Self::datatype_generic_substitutions(item, &fresh_vars);
                data_type = Type::Application(Shared::new(data_type), Shared::new(fresh_vars));
            } else {
                let generics = mapvec(&item.generics, |p| Type::Generic(Generic::Named(Origin::Local(p.name))));
                data_type = Type::Application(Shared::new(data_type), Shared::new(generics));
            }
        }

        (data_type, substitutions)
    }

    /// Convert the parameter types of a constructor for a sum or product type, whose arguments
    /// are declared as `variant_args`. A trait's methods never have an environment
    /// of their own.
    fn convert_constructor_parameters(
        &mut self, item: &cst::TypeDefinition, variant_args: &[(Option<NameId>, &cst::Type)],
        local_kinds: &mut crate::type_inference::types::LocalKinds,
    ) -> Vec<Type> {
        // TODO: Change lifetime desugaring to work on function types better.
        // `fn (ref t) (ref t) -> Bool` should likely be `forall 'a. fn (ref 'a t) (ref 'a t) -> Bool`
        if !item.kind.is_ability() {
            for (_, arg) in variant_args {
                Self::reject_implicit_places(arg, self.compiler);
            }
        }

        // TODO: Review allowing abilities to desugar here; the new type variables need to be tracked.
        let insert_implicit_type_vars = item.kind.is_ability();

        // Effects are closed by default here which stops `can` clauses from being polymorphic by default.
        mapvec(variant_args, |(_, arg)| {
            let param = self.from_cst_type_in_type_definition(arg, insert_implicit_type_vars, local_kinds);
            if item.kind.is_trait() {
                self.without_method_environment(arg, param)
            } else {
                param
            }
        })
    }

    /// Build and returns a constructor type for a sum or product type, taking the given parameters.
    ///
    /// Expects the given sum or product type to be the current context. This cannot be called
    /// within another definition that merely references the type in question.
    ///
    /// `type_name` should be the name of the struct/product type rather than
    /// the name for each individual variant, in the case of sum types.
    ///
    /// Note that the order of the generics of type constructors must match the ordering of the
    /// generics of the type since this is what [TopLevelId::type_body] later expects.
    fn build_constructor_type(
        &mut self, type_name: TopLevelName, item: &cst::TypeDefinition, generics: &[Generic], parameters: Vec<Type>,
    ) -> Type {
        let (mut result, substitutions) = self.type_definition_type(type_name, item, false);
        assert!(substitutions.is_empty());

        if !parameters.is_empty() {
            let parameters = mapvec(parameters, types::ParameterType::explicit);
            result = Type::Function(Shared::new(types::FunctionType {
                parameters,
                environment: Type::NO_CLOSURE_ENV,
                return_type: result,
                effects: Type::pure(),
            }));
        }

        if !generics.is_empty() {
            result = Type::Forall(Shared::new(generics.to_vec()), Shared::new(result));
        }

        // Converting the parameters with effects closed is normally enough to keep types closed,
        // but ability method signatures may carry `ImplicitPlace` placeholders that become fresh
        // type variables. Promote any such inferred free vars into the surrounding
        // `Forall` so the constructor stays a closed polytype.
        let free_vars = result.free_vars(&self.bindings);
        if !free_vars.is_empty() {
            let mut inferred = Vec::new();
            let mut bad_named = false;
            for var in &free_vars {
                match var {
                    Generic::Inferred(_) => inferred.push(*var),
                    _ => bad_named = true,
                }
            }

            if bad_named {
                let location = self.current_context().name_location(type_name.local_name_id).clone();
                self.compiler.accumulate(Diagnostic::FreeVarsInTypeConstructor { location });
            }

            if !inferred.is_empty() {
                result = match result {
                    Type::Forall(existing, body) => {
                        let mut combined = Vec::with_capacity(existing.len() + inferred.len());
                        combined.extend(existing.iter().copied());
                        combined.extend(inferred);
                        Type::Forall(Shared::new(combined), body)
                    },
                    other => Type::Forall(Shared::new(inferred), Shared::new(other)),
                };
            }
        }

        result
    }

    /// For a trait, the name `Eq.eq` is publically visible. We want to give it the type:
    /// `Eq.eq: fn t t {Eq t i} -> Bool`
    ///
    /// It should be generalized to `forall t i. fn t t {Eq t i} -> Bool` later. `member_types` are
    /// the types of the trait's members as fields of its dictionary, see [Self::convert_constructor_parameters].
    /// A member whose type is a function is a method, see [crate::parser::get_item::TraitDefinition::method_flags].
    fn build_trait_member_types(
        &mut self, type_name: TopLevelName, definition: &cst::TypeDefinition, member_types: &[Type],
    ) {
        let cst::TypeDefinitionBody::Struct(members) = &definition.body else {
            unreachable!("a trait's body is always a struct")
        };
        let (dictionary_type, substitutions) = self.type_definition_type(type_name, definition, false);
        assert!(substitutions.is_empty());

        for ((member_name, _), member_type) in members.iter().zip(member_types) {
            let member_type = match self.follow_type(member_type) {
                Type::Function(_) => {
                    self.add_implicit_arg_to_function_type(member_type.clone(), dictionary_type.clone())
                },
                _ => member_type.clone(),
            };
            self.check_name(*member_name, &member_type);
        }
    }

    /// Effect operations are publicly visible (e.g. `Emit.emit`), so each needs its own type.
    /// Each operation in `E` gets a `can E` clause.
    fn build_effect_operation_types(
        &mut self, id: TopLevelId, definition: &cst::TypeDefinition, operations: &[(NameId, cst::Type)],
    ) {
        let type_name = TopLevelName::new(id, definition.name);

        for (operation_name, cst_operation_type) in operations.iter() {
            let (effect, substitutions) = self.type_definition_type(type_name, definition, false);
            assert!(substitutions.is_empty());
            // Each operation is generalized independently, so it gets a fresh kind map seeded
            // from the effect's own generic annotations.
            let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);

            // Omitting a type argument defaulting to `_` is an error, as it would be for a type's fields
            let mut operation_type = self.from_cst_type_in_type_definition(cst_operation_type, true, &mut local_kinds);

            match &cst_operation_type.kind {
                cst::TypeKind::Function(function) => {
                    if function.effects.is_some() {
                        let location = cst_operation_type.location.clone();
                        self.compiler.accumulate(Diagnostic::EffectOperationWithEffectClause { location });
                    }
                    operation_type = self.set_effect_on_function_type(operation_type, effect);
                },
                _ => {
                    let location = cst_operation_type.location.clone();
                    self.compiler.accumulate(Diagnostic::EffectOperationMustBeFunction { location });
                },
            }
            self.check_name(*operation_name, &operation_type);
        }
    }

    /// A trait method reads the data of its impl rather than capturing anything, so it never has an
    /// environment of its own. A method declared with `=>` is the same as one declared with `->`.
    /// Issues an error if the method of type `method_type`, declared as `cst_type`, has any other
    /// environment. Returns any other type unchanged.
    fn without_method_environment(&self, cst_type: &cst::Type, method_type: Type) -> Type {
        let Type::Function(function) = self.follow_type(&method_type) else { return method_type };
        let explicit_environment = match &cst_type.kind {
            cst::TypeKind::Function(function) => function
                .environment
                .as_ref()
                .filter(|env| !matches!(env.kind, cst::TypeKind::Hole | cst::TypeKind::RigidHole)),
            _ => None,
        };
        let location = match explicit_environment {
            Some(environment) => Some(environment.location.clone()),
            None => match self.follow_type(&function.environment) {
                Type::Variable(_) | Type::Primitive(types::PrimitiveType::NoClosureEnv) => None,
                _ => Some(cst_type.location.clone()),
            },
        };
        if let Some(location) = location {
            self.compiler.accumulate(Diagnostic::TraitMethodEnvironment { location });
        }

        let mut function = function.as_ref().clone();
        function.environment = Type::NO_CLOSURE_ENV;
        Type::Function(Shared::new(function))
    }

    /// Given a function type, return a new function type with the given argument added to the end
    /// as an implicit argument.
    fn add_implicit_arg_to_function_type(&self, method_type: Type, implicit_arg: Type) -> Type {
        Self::map_function_type(method_type, "add_implicit_arg_to_function_type", |function_type| {
            function_type.parameters.push(ParameterType::implicit(implicit_arg));
        })
    }

    /// Given an effect operation's function type, set its effect row to the closed singleton containing `effect_type`.
    fn set_effect_on_function_type(&self, method_type: Type, effect_type: Type) -> Type {
        Self::map_function_type(method_type, "set_effect_on_function_type", |function_type| {
            function_type.effects = Type::Effects(Some(Shared::new(vec![effect_type])));
        })
    }

    fn map_function_type(method_type: Type, caller: &str, f: impl FnOnce(&mut types::FunctionType)) -> Type {
        match method_type {
            Type::Function(function_type) => {
                let mut function_type = Shared::unwrap_or_clone(function_type);
                f(&mut function_type);
                Type::Function(Shared::new(function_type))
            },
            other => unreachable!("{caller} expected function type, found {other:?}"),
        }
    }
}
