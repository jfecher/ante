use std::{borrow::Cow, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    diagnostics::UnimplementedItem,
    iterator_extensions::mapvec,
    name_resolution::Origin,
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
                // If this is from a trait, each field needs to be given its own type
                // since they are publically visible, e.g. as `Eq.eq`
                if definition.is_trait {
                    self.build_trait_method_types(id, definition, fields);
                }
                let fields = mapvec(fields, |(_, field_type)| field_type.clone());
                Cow::Owned(vec![(definition.name, fields)])
            },
            cst::TypeDefinitionBody::Enum(variants) => Cow::Borrowed(variants),
        };

        let type_name = TopLevelName::new(id, definition.name);

        for (constructor_name, args) in constructors.iter() {
            let actual = self.build_constructor_type(type_name, definition, args);
            self.check_name(*constructor_name, &actual);
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
    fn build_constructor_type<'a>(
        &mut self, type_name: TopLevelName, item: &cst::TypeDefinition, variant_args: &[cst::Type],
    ) -> Type {
        let (data_type, substitutions) = self.type_definition_type(type_name, item, true);

        // If there are no variant args, the result is not a function.
        // Returning early here also lets us avoid a dependency on `Resolve(id)` since
        // we do not need to resolve any types in `item`'s context when the variant has no arguments.
        if variant_args.len() == 0 {
            return data_type;
        }

        let parameters = mapvec(variant_args, |arg| {
            let mut param = self.from_cst_type(arg);

            if !substitutions.is_empty() {
                param = param.substitute(&substitutions, &self.bindings);
            }
            types::ParameterType::explicit(param)
        });

        Type::Function(Arc::new(types::FunctionType { parameters, return_type: data_type, effects: Type::UNIT }))
    }

    /// Given a desugared trait definition like:
    /// ```ante
    /// type Eq t =
    ///     eq: fn t t -> Bool
    /// ```
    /// The name `Eq.eq` is publically visible. We want to give it the type:
    /// `Eq.eq: fn t t {Eq t} -> Bool`
    fn build_trait_method_types(
        &mut self, id: TopLevelId, definition: &cst::TypeDefinition, fields: &[(NameId, cst::Type)],
    ) {
        let type_name = TopLevelName::new(id, definition.name);

        for (method_name, method_type) in fields.iter() {
            let (implicit_arg, substitutions) = self.type_definition_type(type_name, definition, false);
            assert!(substitutions.is_empty());
            let method_type = self.from_cst_type(&method_type);
            let modified_type = self.add_implicit_arg_to_function_type(method_type, implicit_arg);
            self.check_name(*method_name, &modified_type);
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
