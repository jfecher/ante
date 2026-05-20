use std::{borrow::Cow, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::{Diagnostic, UnimplementedItem},
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
    /// A type definition always returns a unit value, but we must still create the
    /// types of the type constructors
    pub(super) fn check_type_definition(&mut self, definition: &cst::TypeDefinition) {
        let id = self.current_item.unwrap();

        let constructors = match &definition.body {
            cst::TypeDefinitionBody::Error => Cow::Owned(Vec::new()),
            cst::TypeDefinitionBody::Alias(_) => {
                let location = id.locate(self);
                UnimplementedItem::TypeAlias.issue(self.compiler, location);
                return;
            },
            cst::TypeDefinitionBody::Struct(fields) => {
                // If this is from an ability, each field needs to be given its own type
                // since they are publically visible, e.g. as `Eq.eq` or `Emit.emit`
                if definition.is_ability {
                    self.build_method_types(id, definition, fields);
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

    /// Checks for an unboxed recursive reference to `type_name` within the variant fields of `definition`.
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

    /// True only if `typ` uses `target` unboxed.
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
            // Function types are pointer-sized, so recursion through them does not require
            // unbounded representation.
            cst::TypeKind::Function(_)
            | cst::TypeKind::Variable(_)
            | cst::TypeKind::Reference(_)
            | cst::TypeKind::Pointer
            | cst::TypeKind::NoClosureEnv
            | cst::TypeKind::Hole
            | cst::TypeKind::Error
            | cst::TypeKind::Unit
            | cst::TypeKind::Char
            | cst::TypeKind::Integer(_)
            | cst::TypeKind::Float(_)
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

        let parameters = mapvec(variant_args, |arg| {
            let param = self.from_cst_type_with_local_kinds(arg, false, local_kinds);
            types::ParameterType::explicit(param)
        });

        if !variant_args.is_empty() {
            result = Type::Function(Arc::new(types::FunctionType {
                parameters,
                environment: Type::NO_CLOSURE_ENV,
                return_type: result,
            }));
        }

        if !generics.is_empty() {
            result = Type::Forall(Arc::new(generics.to_vec()), Arc::new(result));
        }

        // This should be prevented by the `false` flag in `from_cst_type` above but is included
        // as a sanity check to prevent things from going very wrong.
        let free_vars = result.free_vars(&self.bindings);
        if !free_vars.is_empty() {
            let location = self.current_context().name_location(type_name.local_name_id).clone();
            self.compiler.accumulate(Diagnostic::FreeVarsInTypeConstructor { location });

            for var in free_vars {
                if let Generic::Inferred(id) = var {
                    self.bindings.insert(id, Type::ERROR);
                }
            }
        }

        result
    }

    /// The name `Eq.eq` is publically visible. We want to give it the type:
    /// `Eq.eq: fn t t {Eq t} -> Bool`
    ///
    /// It should be generalized to `forall t. fn t t {Eq t} -> Bool` later.
    ///
    /// The function's closure environment is hard-coded to `Pointer` by the ability
    /// desugarer so every ability value has uniform size.
    fn build_method_types(&mut self, id: TopLevelId, definition: &cst::TypeDefinition, fields: &[(NameId, cst::Type)]) {
        let type_name = TopLevelName::new(id, definition.name);

        for (method_name, method_type) in fields.iter() {
            let (implicit_arg, substitutions) = self.type_definition_type(type_name, definition, false);
            assert!(substitutions.is_empty());
            // Each method is generalized independently, so it gets a fresh kind map seeded
            // from the trait/effect's own generic annotations.
            let mut local_kinds = Self::local_kinds_from_generics(&definition.generics);
            let mut method_type = self.from_cst_type_with_local_kinds(&method_type, false, &mut local_kinds);

            if matches!(method_type, Type::Function(_)) {
                method_type = self.add_implicit_arg_to_function_type(method_type, implicit_arg);
            }
            self.check_name(*method_name, &method_type);
        }
    }

    /// Given a function type, return a new function type with the given argument added to the end
    /// as an implicit argument.
    fn add_implicit_arg_to_function_type(&self, method_type: Type, implicit_arg: Type) -> Type {
        match method_type {
            Type::Function(function_type) => {
                let mut function_type = Arc::unwrap_or_clone(function_type);
                function_type.parameters.push(ParameterType::implicit(implicit_arg));
                Type::Function(Arc::new(function_type))
            },
            other => unreachable!("add_implicit_arg_to_function_type expected function type, found {other:?}"),
        }
    }
}
