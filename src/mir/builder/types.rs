use std::{cell::RefCell, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    incremental::{GetItem, TypeCheck},
    iterator_extensions::mapvec,
    mir::{FunctionType, Type, builder::Context},
    name_resolution::{Origin, builtin::Builtin},
    parser::ids::{ExprId, PathId, PatternId},
    type_inference::{
        TypeBody,
        types::{Type as TCType, TypeBindings, TypeVariableId},
    },
};

impl<'local, Db> Context<'local, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem>,
{
    pub(super) fn convert_expr_type(&self, expr: ExprId) -> Type {
        let typ = &self.types.result.maps.expr_types[&expr];
        self.convert_type(typ, None)
    }

    pub(super) fn convert_path_type(&self, path: PathId) -> Type {
        let typ = &self.types.result.maps.path_types[&path];
        self.convert_type(typ, None)
    }

    pub(super) fn convert_pattern_type(&self, pattern: PatternId) -> Type {
        let typ = &self.types.result.maps.pattern_types[&pattern];
        self.convert_type(typ, None)
    }

    pub(super) fn convert_type(&self, typ: &TCType, args: Option<&[TCType]>) -> Type {
        let ctx = ConvertTypeContext {
            compiler: self.compiler,
            type_bindings: &self.types.bindings,
            generics_in_scope: &self.generics_in_scope,
            in_progress: RefCell::new(FxHashSet::default()),
        };
        ctx.convert_type(typ, args)
    }

    /// Returns the nth field of the tuple type, or [Type::ERROR] if there is none
    pub(super) fn tuple_field_type(tuple: &Type, n: usize) -> Type {
        match tuple {
            Type::Tuple(fields) => fields.get(n).cloned().unwrap_or(Type::ERROR),
            _ => Type::ERROR,
        }
    }
}

/// Maps type inference generics to Mir generics
type GenericsInScope = FxHashMap<crate::type_inference::generics::Generic, crate::mir::Generic>;

struct ConvertTypeContext<'a, Db> {
    compiler: &'a Db,
    type_bindings: &'a TypeBindings,
    generics_in_scope: &'a GenericsInScope,

    /// Tracks the (Origin, args) pairs whose user-defined type bodies are currently
    /// being expanded. Without this, recursive ADTs like `Nat = | Zero | Succ Nat`
    /// cause unbounded recursion. This does not guard against polymorphic recursion.
    in_progress: RefCell<FxHashSet<(Origin, Arc<Vec<TCType>>)>>,
}

impl<Db> ConvertTypeContext<'_, Db>
where
    Db: DbGet<TypeCheck> + DbGet<GetItem>,
{
    /// TODO: The split of this from [Context::convert_type] ended up being unnecessary.
    pub(super) fn convert_type(&self, typ: &TCType, args: Option<&[TCType]>) -> Type {
        match typ.follow(self.type_bindings) {
            TCType::Primitive(primitive_type) => self.convert_primitive_type(*primitive_type),
            TCType::Generic(generic) => self.generics_in_scope.get(&generic).map_or(Type::ERROR, |g| Type::Generic(*g)),
            TCType::Variable(id) => {
                // Any unbound variables at this point should be defaultable to Unit with only
                // slight changes in behavior. Implicits should already be found so this won't affect
                // impl search. A case where this triggers is `(transmute function) args` where even
                // if the argument & return types of the function are constrained, the environment
                // type will be left unbound.
                self.convert_type_variable(*id, Type::UNIT)
            },
            TCType::Function(function_type) => {
                // Effects on the function type are dropped in MIR
                let parameters = mapvec(&function_type.parameters, |typ| self.convert_type(&typ.typ, None));

                // Default to NoClosureEnv instead of Unit
                let environment = match function_type.environment.follow(self.type_bindings) {
                    TCType::Variable(id) => self.convert_type_variable(*id, Type::NO_CLOSURE_ENV),
                    other => self.convert_type(other, None),
                };

                let return_type = self.convert_type(&function_type.return_type, None);
                Type::Function(Arc::new(FunctionType { parameters, environment, return_type }))
            },
            TCType::Application(constructor, new_args) => {
                assert!(args.is_none());
                self.convert_type(constructor, Some(new_args))
            },
            TCType::UserDefined(origin) => self.convert_type_origin(*origin, args, None),
            TCType::Forall(_, typ) => self.convert_type(typ, args),
            TCType::Tuple(elements) => {
                let elements = mapvec(elements.iter(), |t| self.convert_type(t, None));
                Type::Tuple(Arc::new(elements))
            },
        }
    }

    fn convert_type_variable(&self, id: TypeVariableId, default: Type) -> Type {
        let generic = crate::type_inference::generics::Generic::Inferred(id);
        self.generics_in_scope.get(&generic).map_or(default, |g| Type::Generic(*g))
    }

    fn convert_type_origin(&self, origin: Origin, args: Option<&[TCType]>, variant_index: Option<usize>) -> Type {
        match origin {
            Origin::TopLevelDefinition(id) => {
                let key = (origin, Arc::new(args.unwrap_or(&[]).to_vec()));
                if !self.in_progress.borrow_mut().insert(key.clone()) {
                    // The type recursively references itself in a non-pointer position.
                    return Type::ERROR;
                }
                let body = id.top_level_item.type_body(args, self.compiler);
                let result = self.convert_type_body(body, variant_index);
                self.in_progress.borrow_mut().remove(&key);
                result
            },
            Origin::Local(_) => unreachable!("Types cannot be declared locally"),
            Origin::TypeResolution => unreachable!("Types should never be Origin::TypeResolution"),
            Origin::Builtin(builtin) => self.convert_builtin_type(builtin),
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
            },
        }
    }

    fn convert_builtin_type(&self, builtin: Builtin) -> Type {
        match builtin {
            Builtin::Unit => Type::UNIT,
            Builtin::Char => Type::CHAR,
            Builtin::Bool => Type::BOOL,
            Builtin::Ptr => Type::POINTER,
            // LLVM has no bottom type. The builder pairs every divergent call with an
            // `Unreachable` terminator, so the erased Unit is dead at runtime.
            Builtin::Never => Type::UNIT,
            Builtin::Intrinsic => unreachable!("Builtin::Intrinsic is not a type"),
        }
    }

    fn convert_primitive_type(&self, typ: crate::type_inference::types::PrimitiveType) -> Type {
        match typ {
            crate::type_inference::types::PrimitiveType::Error => Type::ERROR,
            crate::type_inference::types::PrimitiveType::Unit => Type::UNIT,
            crate::type_inference::types::PrimitiveType::Bool => Type::BOOL,
            crate::type_inference::types::PrimitiveType::Pointer => Type::POINTER,
            crate::type_inference::types::PrimitiveType::Char => Type::CHAR,
            // See `Builtin::Never` above.
            crate::type_inference::types::PrimitiveType::Never => Type::UNIT,
            crate::type_inference::types::PrimitiveType::Int(kind) => Type::int(kind),
            crate::type_inference::types::PrimitiveType::Float(kind) => Type::float(kind),
            crate::type_inference::types::PrimitiveType::Reference(..) => Type::POINTER,
            crate::type_inference::types::PrimitiveType::NoClosureEnv => Type::NO_CLOSURE_ENV,
        }
    }
}
