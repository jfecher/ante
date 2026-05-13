use std::{borrow::Cow, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::{Diagnostic, UnimplementedItem},
    iterator_extensions::mapvec,
    name_resolution::{Origin, ResolutionResult},
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

        let generics = mapvec(&definition.generics, |id| Generic::Named(Origin::Local(*id)));

        for (constructor_name, args) in constructors.iter() {
            let actual = self.build_constructor_type(type_name, definition, &generics, args);
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
                variants.iter().any(|(_, args)| args.iter().any(|t| Self::type_references(t, target, resolve)))
            },
            cst::TypeDefinitionBody::Struct(fields) => {
                fields.iter().any(|(_, t)| Self::type_references(t, target, resolve))
            },
            _ => false,
        }
    }

    fn type_references(typ: &cst::Type, target: Origin, resolve: &ResolutionResult) -> bool {
        match &typ.kind {
            cst::TypeKind::Named(path) => resolve.path_origins.get(path).copied() == Some(target),
            cst::TypeKind::Application(f, args) => {
                Self::type_references(f, target, resolve)
                    || args.iter().any(|a| Self::type_references(a, target, resolve))
            },
            cst::TypeKind::Tuple(elements) => elements.iter().any(|e| Self::type_references(e, target, resolve)),
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
            | cst::TypeKind::Float(_) => false,
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
                let generics = mapvec(&item.generics, |id| Type::Generic(Generic::Named(Origin::Local(*id))));
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
        variant_args: &[cst::Type],
    ) -> Type {
        let (mut result, substitutions) = self.type_definition_type(type_name, item, false);
        assert!(substitutions.is_empty());

        // For ability constructors, nested ability references (e.g. `Emit a` inside
        // `Stream.stream: fn t (Emit a) -> Unit`) need their own fresh env type variable
        // rather than erroring with "trait types can't be used here".
        let allow_implicit_type_vars = item.is_ability;
        let parameters = mapvec(variant_args, |arg| {
            let param = self.from_cst_type(arg, allow_implicit_type_vars);
            types::ParameterType::explicit(param)
        });

        if !variant_args.is_empty() {
            result = Type::Function(Arc::new(types::FunctionType {
                parameters,
                environment: Type::NO_CLOSURE_ENV,
                return_type: result,
            }));
        }

        // For ability constructors, nested ability uses (e.g. `Emit a` inside a method
        // signature) auto-insert a fresh env type variable. Promote any such variables to
        // generics in the forall so the constructor is properly polymorphic over them.
        if allow_implicit_type_vars {
            let extra: Vec<Generic> = result
                .free_vars(&self.bindings)
                .into_iter()
                .filter(|g| !generics.contains(g))
                .collect();
            if !extra.is_empty() {
                let substitutions = extra.iter().map(|var| (*var, Type::Generic(*var))).collect();
                result = result.substitute(&substitutions, &self.bindings);
                let mut all_generics = generics.to_vec();
                all_generics.extend(extra);
                if !all_generics.is_empty() {
                    result = Type::Forall(Arc::new(all_generics), Arc::new(result));
                }
                return result;
            }
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
    /// The function's closure environment is whatever the ability desugarer
    /// planted in `function.environment` (a fresh `[env]` type variable). At
    /// call sites it unifies with `NO_CLOSURE_ENV` for ordinary trait-style
    /// calls and with the handler's `Ptr Unit` for effect-style resumptions.
    fn build_method_types(
        &mut self, id: TopLevelId, definition: &cst::TypeDefinition, fields: &[(NameId, cst::Type)],
    ) {
        let type_name = TopLevelName::new(id, definition.name);

        for (method_name, method_type) in fields.iter() {
            let (implicit_arg, substitutions) = self.type_definition_type(type_name, definition, false);
            assert!(substitutions.is_empty());
            // Pass `true` so nested ability references inside method signatures (e.g. the
            // `Emit a` parameter of `Stream.stream`) automatically receive their own fresh
            // env type variable rather than being flagged as "trait types can't be used here".
            let mut method_type = self.from_cst_type(&method_type, true);

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
