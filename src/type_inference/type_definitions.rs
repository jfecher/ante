use std::{borrow::Cow, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::Diagnostic,
    incremental::DbHandle,
    iterator_extensions::mapvec,
    name_resolution::{Origin, ResolutionResult, builtin::Builtin},
    parser::{
        cst,
        ids::{NameId, TopLevelId, TopLevelName},
    },
    type_inference::{
        Locateable, TypeChecker,
        generics::Generic,
        types::{self, GenericSubstitutions, ParameterType, Type},
    },
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    pub(super) fn check_type_definition(&mut self, definition: &cst::TypeDefinition) {
        let id = self.current_item.unwrap();

        let constructors = match &definition.body {
            cst::TypeDefinitionBody::Error => Cow::Owned(Vec::new()),
            cst::TypeDefinitionBody::Alias(body) => {
                // Convert the body even though the result is unused to issue kind or recursion errors
                Self::reject_implicit_lifetimes(body, self.compiler);
                let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);
                let _ = self.from_cst_type_with_local_kinds(body, false, false, &mut local_kinds);
                return;
            },
            cst::TypeDefinitionBody::Struct(fields) => {
                // Ability fields are publicly visible (e.g. `Eq.eq`), so each needs its own type.
                if definition.kind.is_ability() {
                    self.build_method_types(id, definition, fields);
                }
                if definition.kind.is_effect() {
                    return;
                }
                let fields = mapvec(fields, |(_, field_type)| field_type.clone());
                Cow::Owned(vec![(definition.name, fields)])
            },
            cst::TypeDefinitionBody::Enum(variants) => Cow::Borrowed(variants),
        };

        let type_name = TopLevelName::new(id, definition.name);

        // Detect self-referential types. TODO: this won't catch any indirect uses or polymorphic recursion.
        if !definition.shared && self.type_definition_is_recursive_and_unboxed(definition, type_name) {
            let typ = self.current_context()[definition.name].as_ref().clone();
            let location = definition.name.locate(self);
            self.compiler.accumulate(Diagnostic::RecursiveType { typ, location });
        }

        let generics = mapvec(&definition.generics, |p| Generic::Named(Origin::Local(p.name)));

        // Share kind info across all variant fields of this definition so that, e.g., two
        // fields that mention `n` agree on `n`'s kind.
        let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);

        for (constructor_name, args) in constructors.iter() {
            let actual = self.build_constructor_type(type_name, definition, &generics, args, &mut local_kinds);
            self.check_name(*constructor_name, &actual);
        }
    }

    fn type_definition_is_recursive_and_unboxed(
        &self, definition: &cst::TypeDefinition, type_name: TopLevelName,
    ) -> bool {
        let resolve = self.current_resolve();
        let target = Origin::TopLevelDefinition(type_name);
        match &definition.body {
            cst::TypeDefinitionBody::Enum(variants) => {
                variants.iter().any(|(_, args)| args.iter().any(|t| Self::type_uses_target_unboxed(t, target, resolve)))
            },
            cst::TypeDefinitionBody::Struct(fields) => {
                fields.iter().any(|(_, t)| Self::type_uses_target_unboxed(t, target, resolve))
            },
            _ => false,
        }
    }

    /// Walk a field type and emit a `MissingExplicitLifetime` diagnostic for
    /// every `ImplicitLifetime` placeholder. Lifetimes on references must be written out
    /// in type-definition bodies.
    fn reject_implicit_lifetimes(typ: &cst::Type, db: &DbHandle) {
        match &typ.kind {
            cst::TypeKind::ImplicitLifetime => {
                db.accumulate(Diagnostic::MissingExplicitLifetime { location: typ.location.clone() });
            },
            cst::TypeKind::Application(f, args) => {
                Self::reject_implicit_lifetimes(f, db);
                for arg in args {
                    Self::reject_implicit_lifetimes(arg, db);
                }
            },
            cst::TypeKind::Function(function) => {
                for parameter in &function.parameters {
                    Self::reject_implicit_lifetimes(&parameter.typ, db);
                }
                if let Some(env) = function.environment.as_ref() {
                    Self::reject_implicit_lifetimes(env, db);
                }
                Self::reject_implicit_lifetimes(&function.return_type, db);
            },
            cst::TypeKind::Tuple(elements) => {
                for element in elements {
                    Self::reject_implicit_lifetimes(element, db);
                }
            },
            cst::TypeKind::Forall(_, body) => Self::reject_implicit_lifetimes(body, db),
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
            | cst::TypeKind::Lifetime(_)
            | cst::TypeKind::IntegerConstant(_) => (),
        }
    }

    /// Used to check for recursively infinitely sized types.
    fn type_uses_target_unboxed(typ: &cst::Type, target: Origin, resolve: &ResolutionResult) -> bool {
        match &typ.kind {
            cst::TypeKind::Named(path) => resolve.path_origins.get(path).copied() == Some(target),
            cst::TypeKind::Application(f, args) => {
                if Self::is_pointer_constructor(f, resolve) {
                    return false;
                }
                Self::type_uses_target_unboxed(f, target, resolve)
                    || args.iter().any(|a| Self::type_uses_target_unboxed(a, target, resolve))
            },
            cst::TypeKind::Tuple(elements) => {
                elements.iter().any(|e| Self::type_uses_target_unboxed(e, target, resolve))
            },
            cst::TypeKind::Forall(_, body) => Self::type_uses_target_unboxed(body, target, resolve),
            cst::TypeKind::Function(typ) => {
                if let Some(env) = typ.environment.as_ref() {
                    Self::type_uses_target_unboxed(env, target, resolve)
                } else {
                    false
                }
            }

            cst::TypeKind::Variable(_)
            | cst::TypeKind::Reference(_)
            | cst::TypeKind::Pointer
            | cst::TypeKind::NoClosureEnv
            | cst::TypeKind::Hole
            | cst::TypeKind::Error
            | cst::TypeKind::Unit
            | cst::TypeKind::Char
            | cst::TypeKind::Integer(_)
            | cst::TypeKind::Float(_)
            | cst::TypeKind::Lifetime(_)
            | cst::TypeKind::ImplicitLifetime
            | cst::TypeKind::IntegerConstant(_) => false,
        }
    }

    fn is_pointer_constructor(typ: &cst::Type, resolve: &ResolutionResult) -> bool {
        match &typ.kind {
            cst::TypeKind::Pointer => true,
            cst::TypeKind::Named(path) => {
                matches!(resolve.path_origins.get(path).copied(), Some(Origin::Builtin(Builtin::Ptr)))
            },
            _ => false,
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
                data_type = Type::Application(Arc::new(data_type), Arc::new(fresh_vars));
            } else {
                let generics = mapvec(&item.generics, |p| Type::Generic(Generic::Named(Origin::Local(p.name))));
                data_type = Type::Application(Arc::new(data_type), Arc::new(generics));
            }
        }

        (data_type, substitutions)
    }

    /// Build and returns a constructor type for a sum or product type.
    ///
    /// Expects the given sum or product type to be the current context. This cannot be called
    /// within another definition that merely references the type in question.
    ///
    /// `type_name` should be the name of the struct/product type rather than
    /// the name for each individual variant, in the case of sum types.
    ///
    /// Note that the order of the generics of type constructors must match the ordering of the
    /// generics of the type since this is what [TopLevelId::type_body] later expects.
    fn build_constructor_type<'a>(
        &mut self, type_name: TopLevelName, item: &cst::TypeDefinition, generics: &[Generic],
        variant_args: &[cst::Type], local_kinds: &mut crate::type_inference::types::LocalKinds,
    ) -> Type {
        let (mut result, substitutions) = self.type_definition_type(type_name, item, false);
        assert!(substitutions.is_empty());

        // TODO: Change lifetime desugaring to work on function types better.
        // `fn (ref t) (ref t) -> Bool` should likely be `forall 'a. fn (ref 'a t) (ref 'a t) -> Bool`
        if !item.kind.is_ability() {
            for arg in variant_args {
                Self::reject_implicit_lifetimes(arg, self.compiler);
            }
        }

        // TODO: Review allowing abilities to desugar here; the new type variables need to be tracked.
        let insert_implicit_type_vars = item.kind.is_ability();

        // `false` here stops `can` clauses from being polymorphic by default.
        let parameters = mapvec(variant_args, |arg| {
            let param = self.from_cst_type_with_local_kinds(arg, insert_implicit_type_vars, false, local_kinds);
            types::ParameterType::explicit(param)
        });

        if !variant_args.is_empty() {
            result = Type::Function(Arc::new(types::FunctionType {
                parameters,
                environment: Type::NO_CLOSURE_ENV,
                return_type: result,
                effects: Type::pure(),
            }));
        }

        if !generics.is_empty() {
            result = Type::Forall(Arc::new(generics.to_vec()), Arc::new(result));
        }

        // The `false` flag above is normally enough to keep types closed, but ability
        // method signatures may carry `ImplicitLifetime` placeholders that become fresh
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
                        Type::Forall(Arc::new(combined), body)
                    },
                    other => Type::Forall(Arc::new(inferred), Arc::new(other)),
                };
            }
        }

        result
    }

    /// For a trait, the name `Eq.eq` is publically visible. We want to give it the type:
    /// `Eq.eq: fn t t {Eq t} -> Bool`
    ///
    /// It should be generalized to `forall t. fn t t {Eq t} -> Bool` later.
    ///
    /// The function's closure environment is hard-coded to `Pointer` by the ability
    /// desugarer so every ability value has uniform size.
    ///
    /// For effects, each function in `E` gets a `can E` clause.
    fn build_method_types(&mut self, id: TopLevelId, definition: &cst::TypeDefinition, fields: &[(NameId, cst::Type)]) {
        let type_name = TopLevelName::new(id, definition.name);
        let is_effect = definition.kind.is_effect();

        for (method_name, method_type) in fields.iter() {
            let (arg, substitutions) = self.type_definition_type(type_name, definition, false);
            assert!(substitutions.is_empty());
            // Each method is generalized independently, so it gets a fresh kind map seeded
            // from the trait/effect's own generic annotations.
            let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);

            let mut method_type = self.from_cst_type_with_local_kinds(method_type, true, false, &mut local_kinds);

            if matches!(method_type, Type::Function(_)) {
                method_type = if is_effect {
                    self.set_effect_on_function_type(method_type, arg)
                } else {
                    self.add_implicit_arg_to_function_type(method_type, arg)
                };
            }
            self.check_name(*method_name, &method_type);
        }
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
            function_type.effects = Type::effects(vec![effect_type], None);
        })
    }

    fn map_function_type(method_type: Type, caller: &str, f: impl FnOnce(&mut types::FunctionType)) -> Type {
        match method_type {
            Type::Function(function_type) => {
                let mut function_type = Arc::unwrap_or_clone(function_type);
                f(&mut function_type);
                Type::Function(Arc::new(function_type))
            },
            other => unreachable!("{caller} expected function type, found {other:?}"),
        }
    }
}
