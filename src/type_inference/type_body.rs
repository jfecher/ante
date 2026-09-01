use inc_complete::DbGet;

use crate::{
    incremental::{GetItem, TypeCheck},
    iterator_extensions::mapvec,
    parser::{
        cst::{self, Name, TopLevelItem, TopLevelItemKind},
        desugar_context::DesugarContext,
        ids::{NameId, TopLevelId, TopLevelName},
    },
    type_inference::{
        dependency_graph::TypeCheckResult,
        types::{Type, TypeBindings},
    },
};

#[derive(Debug, PartialEq, Eq)]
pub enum TypeBody {
    Product { type_name: Name, fields: Vec<(Name, Type)> },
    Sum(Vec<(NameId, Name, Vec<(Name, Type)>)>),
}

impl TopLevelId {
    /// Returns the body of this user-defined type (the part after the `=` when declared).
    /// The given [TopLevelId] should refer to a [TypeDefinition] or something which desugars to
    /// one.
    ///
    /// If specified, `arguments` will be used to substitute any generics of the type.
    /// Panics if the arguments are specified and differ in length to the type's generics.
    ///
    /// Note that if `arguments` are not provided, the type will be instantiated and thus
    /// any fields may refer to type type variables that have not been tracked.
    ///
    /// The constructor's generalized type carries its effect ids.
    /// When `next_id` is given these are instantiated to fresh variables.
    /// Otherwise, the same effect ids are kept.
    ///
    /// - For a struct: returns each field name & type
    /// - For a union: returns each variant with its name and arguments
    pub fn type_body<Db>(self, arguments: Option<&[Type]>, compiler: &Db, next_id: Option<&mut u32>) -> TypeBody
    where
        Db: DbGet<TypeCheck> + DbGet<GetItem>,
    {
        let result = TypeCheck(self).get(compiler);
        let (item, item_context) = GetItem(self).get(compiler);
        type_body_from_item(&item, &item_context, &result, arguments, next_id)
    }
}

fn type_body_from_item(
    item: &TopLevelItem, item_context: &DesugarContext, result: &TypeCheckResult, arguments: Option<&[Type]>,
    mut next_id: Option<&mut u32>,
) -> TypeBody {
    let TopLevelItemKind::TypeDefinition(type_definition) = &item.kind else {
        panic!("type_body: passed type_id is not a type!")
    };

    match &type_definition.body {
        cst::TypeDefinitionBody::Struct(_) if type_definition.kind.is_effect() => {
            let type_name = item_context[type_definition.name].clone();
            TypeBody::Product { type_name, fields: Vec::new() }
        },
        cst::TypeDefinitionBody::Struct(fields) => {
            // This'd be easier with an explicit type data field
            let constructor_type = result.get_generalized(type_definition.name);
            let constructor = apply_type_constructor(&constructor_type, arguments, result, next_id);
            let field_types = constructor.function_parameter_types();

            assert_eq!(fields.len(), field_types.len());
            let fields = mapvec(fields.iter().zip(field_types), |((field_name, _), typ)| {
                (item_context[*field_name].clone(), typ)
            });

            let type_name = item_context[type_definition.name].clone();
            TypeBody::Product { type_name, fields }
        },
        cst::TypeDefinitionBody::Enum(variants, _) => {
            let mut variants = mapvec(variants, |(name, cst_fields)| {
                variant_name_and_fields(*name, cst_fields, arguments, result, next_id.as_deref_mut(), item_context)
            });
            if variants.len() == 1 {
                let (_name_id, type_name, fields) = variants.pop().unwrap();
                TypeBody::Product { type_name, fields }
            } else {
                TypeBody::Sum(variants)
            }
        },
        // Type/effect aliases are expanded away wherever they are referenced in name resolution, so `type_body`
        // should never be queried for one.
        cst::TypeDefinitionBody::Alias(_)
        | cst::TypeDefinitionBody::EffectAlias(_)
        | cst::TypeDefinitionBody::Error => {
            let type_name = item_context[type_definition.name].clone();
            TypeBody::Product { type_name, fields: Vec::new() }
        },
    }
}

impl TopLevelName {
    /// Like [TopLevelId::type_body], but scoped to a single name within the type definition.
    ///
    /// If `self.local_name_id` is the type's own name, this delegates directly to
    /// [TopLevelId::type_body]. Otherwise the name may refer to a variant type of an enum.
    pub fn type_body<Db>(self, arguments: Option<&[Type]>, compiler: &Db, mut next_id: Option<&mut u32>) -> TypeBody
    where
        Db: DbGet<TypeCheck> + DbGet<GetItem>,
    {
        let (item, item_context) = GetItem(self.top_level_item).get(compiler);
        let TopLevelItemKind::TypeDefinition(type_definition) = &item.kind else {
            panic!("TopLevelName::type_body: passed id is not a type!")
        };

        if self.local_name_id == type_definition.name {
            let result = TypeCheck(self.top_level_item).get(compiler);
            return type_body_from_item(&item, &item_context, &result, arguments, next_id);
        }

        let (_, (_, cst_fields)) = type_definition
            .body
            .find_variant(self.local_name_id)
            .expect("TopLevelName::type_body: local_name_id names neither the type nor one of its variants");

        let result = TypeCheck(self.top_level_item).get(compiler);
        let (_name_id, type_name, fields) = variant_name_and_fields(
            self.local_name_id,
            cst_fields,
            arguments,
            &result,
            next_id.as_deref_mut(),
            &item_context,
        );
        TypeBody::Product { type_name, fields }
    }

    /// Number of `with`-clause common fields on the union type `self`
    pub fn common_field_count<Db: DbGet<GetItem>>(self, compiler: &Db) -> usize {
        let (item, _) = GetItem(self.top_level_item).get(compiler);
        let TopLevelItemKind::TypeDefinition(definition) = &item.kind else { return 0 };
        if self.local_name_id == definition.name { definition.body.common_fields().len() } else { 0 }
    }
}

fn variant_name_and_fields(
    name: NameId, cst_fields: &[(Option<NameId>, cst::Type)], arguments: Option<&[Type]>, result: &TypeCheckResult,
    next_id: Option<&mut u32>, item_context: &DesugarContext,
) -> (NameId, Name, Vec<(Name, Type)>) {
    let constructor_type = result.get_generalized(name);
    let constructor = apply_type_constructor(&constructor_type, arguments, result, next_id);
    let field_types = constructor.function_parameter_types();
    let fields = mapvec(field_types.enumerate(), |(i, field_type)| {
        let field_name = item_context.field_name_or_index(cst_fields.get(i).and_then(|(n, _)| *n), i);
        (field_name, field_type)
    });
    (name, item_context[name].clone(), fields)
}

/// Try to apply the given type to the given type arguments. Note that this assumes there are no
/// bound type variables within `typ`!
///
/// `next_id`, if given, instantiates the constructor's effect ids.
///
// This assumes constructor args are in the same order as the type args.
// This should be guaranteed by [TypeChecker::build_constructor_type].
pub(crate) fn apply_type_constructor(
    typ: &Type, args: Option<&[Type]>, types: &TypeCheckResult, next_id: Option<&mut u32>,
) -> Type {
    let expected_generic_count = match typ.follow(&types.bindings) {
        Type::Forall(generics, _) => generics.len(),
        _ => 0,
    };

    let arg_len = args.map_or(0, |args| args.len());
    if arg_len != expected_generic_count {
        // TODO: We should be issuing an error either here or above somewhere
    }

    let no_type_var_bindings = TypeBindings::default();

    let applied = match args {
        Some(args) => {
            if args.len() < expected_generic_count {
                let mut new_args = args.to_vec();
                for _ in args.len()..expected_generic_count {
                    new_args.push(Type::ERROR);
                }
                typ.apply_type(&new_args, &no_type_var_bindings)
            } else {
                typ.apply_type(args, &no_type_var_bindings)
            }
        },
        None if expected_generic_count == 0 => typ.clone(),
        None => {
            // TODO: This should be an error in the future
            let Type::Forall(generics, _) = typ.follow(&types.bindings) else { unreachable!() };
            let args = mapvec(generics.iter(), |_| Type::ERROR);
            typ.apply_type(&args, &no_type_var_bindings)
        },
    };

    match next_id.and_then(|next_id| applied.instantiate_effect_ids(next_id, &no_type_var_bindings)) {
        Some(instantiated) => instantiated,
        None => applied,
    }
}
