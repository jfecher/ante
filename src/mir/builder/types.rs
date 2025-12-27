use std::sync::Arc;

use crate::{
    iterator_extensions::vecmap,
    mir::{FunctionType, Type, builder::Context},
    name_resolution::{Origin, builtin::Builtin},
    type_inference::{TypeBody, top_level_types::TopLevelType, types::Type as TCType},
};

impl<'local> Context<'local> {
    pub(super) fn convert_type(&self, typ: &TCType, args: Option<&[TCType]>) -> Type {
        match typ.follow_type(&self.types.bindings) {
            TCType::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type, args),
            TCType::Generic(generic) => Type::Generic(*generic),
            // All type variables should be bound when we finish type inference
            TCType::Variable(_type_variable_id) => Type::ERROR,
            TCType::Function(function_type) => {
                // TODO: Effects
                let parameters = vecmap(&function_type.parameters, |typ| self.convert_type(typ, None));
                let return_type = self.convert_type(&function_type.return_type, None);
                Type::Function(Arc::new(FunctionType { parameters, return_type }))
            },
            TCType::Application(constructor, new_args) => {
                assert!(args.is_none());
                self.convert_type(constructor, Some(new_args))
            },
            TCType::UserDefined(origin) => self.convert_type_origin(*origin, args, None),
        }
    }

    fn convert_type_origin(&self, origin: Origin, args: Option<&[TCType]>, variant_index: Option<usize>) -> Type {
        match origin {
            Origin::TopLevelDefinition(id) => {
                let body = id.top_level_item.type_body(args, self.compiler);
                self.convert_type_body(body, variant_index)
            },
            Origin::Local(_) => unreachable!("Types cannot be declared locally"),
            Origin::TypeResolution => unreachable!("Types should never be Origin::TypeResolution"),
            Origin::Builtin(builtin) => self.convert_builtin_type(builtin, args),
        }
    }

    /// Converts a type body to the general representation of that type.
    ///
    /// If `variant_index` is specified, the default index used to represent the sum type
    /// is overridden with the given index. In either case, sum types with multiple possible
    /// constructors will always include the tag type.
    fn convert_type_body(&self, body: TypeBody, variant_index: Option<usize>) -> Type {
        match body {
            TypeBody::Product { type_name: _, fields } => {
                Type::tuple(vecmap(fields, |(_, field)| self.convert_type(&field, None)))
            },
            TypeBody::Sum(variants) => {
                // TODO: How should we select the largest variant when it may be generic?
                // TODO: Arbitrarily select the first variant as the representation for now.
                //       Perhaps MIR should have a dedicated union type.
                let variant_index = variant_index.unwrap_or(0);
                if let Some((_, variant_args)) = variants.get(variant_index) {
                    // TODO: Unify sum types and product types
                    if variants.len() == 1 {
                        // Sum types with a single variant don't need a tag
                        Type::tuple(vecmap(variant_args, |field| self.convert_type(&field, None)))
                    } else {
                        let fields = std::iter::once(&TCType::U8).chain(variant_args);
                        Type::tuple(vecmap(fields, |field| self.convert_type(&field, None)))
                    }
                } else {
                    Type::UNIT // Void can't be constructed but a zero-sized type seems a good approximation
                }
            },
        }
    }

    fn convert_builtin_type(&self, builtin: Builtin, args: Option<&[TCType]>) -> Type {
        match builtin {
            Builtin::Unit => Type::UNIT,
            Builtin::Int => todo!(),
            Builtin::Char => Type::CHAR,
            Builtin::Float => todo!(),
            Builtin::String => Type::string(),
            Builtin::Ptr => Type::POINTER,
            Builtin::PairType => self.convert_pair_type(args),
            Builtin::PairConstructor => unreachable!("This is a constructor, not a type"),
        }
    }

    fn convert_primitive_type(
        &self, typ: crate::type_inference::types::PrimitiveType, args: Option<&[TCType]>,
    ) -> Type {
        match typ {
            crate::type_inference::types::PrimitiveType::Error => Type::ERROR,
            crate::type_inference::types::PrimitiveType::Unit => Type::UNIT,
            crate::type_inference::types::PrimitiveType::Bool => Type::BOOL,
            crate::type_inference::types::PrimitiveType::Pointer => Type::POINTER,
            crate::type_inference::types::PrimitiveType::Char => Type::CHAR,
            crate::type_inference::types::PrimitiveType::String => Type::string(),
            crate::type_inference::types::PrimitiveType::Pair => self.convert_pair_type(args),
            crate::type_inference::types::PrimitiveType::Int(kind) => Type::int(kind),
            crate::type_inference::types::PrimitiveType::Float(kind) => Type::float(kind),
            crate::type_inference::types::PrimitiveType::Reference(..) => Type::POINTER,
        }
    }

    fn convert_pair_type(&self, args: Option<&[TCType]>) -> Type {
        match args {
            Some(args) if args.len() == 2 => Type::tuple(vecmap(args, |arg| self.convert_type(arg, None))),
            // Rely on type-checking to issue this argument-count mismatch error to the user
            _ => Type::ERROR,
        }
    }

    pub(super) fn convert_top_level_type(&self, typ: &TopLevelType, args: Option<&[TCType]>) -> Type {
        match typ {
            TopLevelType::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type, args),
            TopLevelType::Generic(generic) => Type::Generic(*generic),
            TopLevelType::Function { parameters, return_type } => {
                let parameters = vecmap(parameters, |parameter| self.convert_top_level_type(parameter, None));
                let return_type = self.convert_top_level_type(return_type, None);
                Type::Function(Arc::new(FunctionType { parameters, return_type }))
            },
            TopLevelType::Application(constructor, new_args) => {
                assert!(args.is_none());
                let new_args = vecmap(new_args.iter(), |arg| arg.as_type());
                self.convert_top_level_type(constructor, Some(&new_args))
            },
            TopLevelType::UserDefined(origin) => self.convert_type_origin(*origin, args, None),
        }
    }

    /// Returns the nth field of the tuple type, or [Type::ERROR] if there is none
    pub(super) fn tuple_field_type(tuple: &Type, n: u32) -> Type {
        match tuple {
            Type::Tuple(fields) => fields.get(n as usize).cloned().unwrap_or(Type::ERROR),
            _ => Type::ERROR,
        }
    }

    /// Convert the given sum-type variant into a [Type].
    pub(super) fn convert_variant_type(&self, typ: &TCType, variant_index: usize, args: Option<&[TCType]>) -> Type {
        match typ.follow_type(&self.types.bindings) {
            TCType::Application(constructor, new_args) => {
                assert!(args.is_none());
                self.convert_variant_type(constructor, variant_index, Some(new_args))
            },
            TCType::UserDefined(origin) => self.convert_type_origin(*origin, args, Some(variant_index)),
            other => unreachable!("{other:?} should never be a variant type"),
        }
    }
}
