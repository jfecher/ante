use std::sync::Arc;

use inc_complete::DbGet;

use crate::{
    incremental::{GetItem, TypeCheck},
    iterator_extensions::mapvec,
    mir::{FunctionType, Type, builder::Context},
    name_resolution::{Origin, builtin::Builtin},
    type_inference::{TypeBody, top_level_types::TopLevelType, types::Type as TCType},
};

impl<'local, Db> Context<'local, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem>,
{
    pub(super) fn convert_type(&self, typ: &TCType, args: Option<&[TCType]>) -> Type {
        match typ.follow_type(&self.types.bindings) {
            TCType::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type, args),
            TCType::Generic(generic) => Type::Generic(*generic),
            // All type variables should be bound when we finish type inference
            TCType::Variable(_type_variable_id) => Type::ERROR,
            TCType::Function(function_type) => {
                // TODO: Effects
                let parameters = mapvec(&function_type.parameters, |typ| {
                    self.convert_type(&typ.typ, None)
                });
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
                Type::tuple(mapvec(fields, |(_, field)| self.convert_type(&field, None)))
            },
            TypeBody::Sum(variants) => {
                // TODO: Unify sum types and product types
                if variants.len() == 1 {
                    // Sum types with a single variant don't need a tag
                    let variant = &variants[0].1;
                    Type::tuple(mapvec(variant, |field| self.convert_type(&field, None)))
                } else {
                    let union = if let Some((_, variant_args)) = variant_index.and_then(|i| variants.get(i)) {
                        // If we want to retrieve 1 specific variant then create a tuple of each field
                        Type::tuple(mapvec(variant_args, |field| self.convert_type(&field, None)))
                    } else {
                        // Otherwise we need a raw union of the fields of all variants
                        Type::union(mapvec(variants, |(_, fields)| {
                            Type::tuple(mapvec(fields, |field| self.convert_type(&field, None)))
                        }))
                    };
                    // Then pack the result with a separate tag value.
                    Type::tuple(vec![Type::tag_type(), union])
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
            Some(args) if args.len() == 2 => Type::tuple(mapvec(args, |arg| self.convert_type(arg, None))),
            // Rely on type-checking to issue this argument-count mismatch error to the user
            _ => Type::ERROR,
        }
    }

    pub(super) fn convert_top_level_type(&self, typ: &TopLevelType, args: Option<&[TCType]>) -> Type {
        match typ {
            TopLevelType::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type, args),
            TopLevelType::Generic(generic) => Type::Generic(*generic),
            TopLevelType::Function { parameters, return_type } => {
                let parameters = mapvec(parameters, |parameter| self.convert_top_level_type(&parameter.typ, None));
                let return_type = self.convert_top_level_type(return_type, None);
                Type::Function(Arc::new(FunctionType { parameters, return_type }))
            },
            TopLevelType::Application(constructor, new_args) => {
                assert!(args.is_none());
                let new_args = mapvec(new_args.iter(), |arg| arg.as_type());
                self.convert_top_level_type(constructor, Some(&new_args))
            },
            TopLevelType::UserDefined(origin) => self.convert_type_origin(*origin, args, None),
        }
    }

    /// Returns the nth field of the tuple type, or [Type::ERROR] if there is none
    pub(super) fn tuple_field_type(tuple: &Type, n: usize) -> Type {
        match tuple {
            Type::Tuple(fields) => fields.get(n).cloned().unwrap_or(Type::ERROR),
            _ => Type::ERROR,
        }
    }
}
