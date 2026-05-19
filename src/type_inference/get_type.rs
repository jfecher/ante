use crate::{
    incremental::{self, DbHandle, GetItem, GetType, Resolve, TypeCheck},
    iterator_extensions::mapvec,
    name_resolution::ResolutionResult,
    parser::{
        cst::{self, Definition, Expr, Pattern, TopLevelItemKind, TypeKind},
        desugar_context::DesugarContext,
    },
    type_inference::types::{Type, TypeBindings},
};

/// Get the type of the name defined by this TopLevelId.
/// If this doesn't define a name we return the Unit type by default.
///
/// This is very similar to but separate from `type_check_impl`. We separate these
/// because `type_check_impl` will always type check the contents of a definition,
/// and we don't want other definitions to depend on the contents of another definition
/// if the other definition provides a type annotation. Without type annotations the two
/// functions should be mostly equivalent.
pub fn get_type_impl(context: &GetType, compiler: &DbHandle) -> Type {
    incremental::enter_query();
    let (item, item_context) = compiler.get(GetItem(context.0.top_level_item));
    incremental::println(format!("Get type of {:?}", item.id));

    let typ = match &item.kind {
        TopLevelItemKind::Definition(definition) => {
            let resolve = Resolve(context.0.top_level_item).get(compiler);
            try_get_generalized_type(definition, item_context.as_ref(), &resolve, compiler)
                .map(|t| t.generalize(&TypeBindings::default()))
                .unwrap_or_else(|| {
                    let check = TypeCheck(context.0.top_level_item).get(compiler);
                    let typ = check.get_generalized(context.0.local_name_id);
                    typ.follow_all(&check.bindings)
                })
        },
        _ => {
            let check = TypeCheck(context.0.top_level_item).get(compiler);
            let typ = check.get_generalized(context.0.local_name_id);
            typ.follow_all(&check.bindings)
        },
    };
    incremental::exit_query();
    typ
}

/// Make a best-effort attempt to get the type of a definition.
/// If the type is successfully found then this definition will not be dependent on the
/// types of its contents to get its type. Put another way, if the type is known then
/// we don't need to re-type check this definition when its contents change.
///
/// TODO: This is used for both GetType and check_definition. It should only be used for
/// GetType because this fails if it cannot retrieve an entire type. For definitions we
/// want instead to succeed with partial types, filling in holes as needed for better type
/// errors.
pub(super) fn try_get_generalized_type(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<Type> {
    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return Some(Type::from_cst_type_generalized(typ, resolve, compiler, true));
    }

    if let Expr::Lambda(lambda) = &context[definition.rhs] {
        let return_type = Box::new(lambda.return_type.as_ref()?.clone());

        let parameters = lambda
            .parameters
            .iter()
            .map(|parameter| match &context[parameter.pattern] {
                Pattern::TypeAnnotation(_, typ) => Some(cst::ParameterType::new(typ.clone(), parameter.is_implicit)),
                Pattern::Literal(cst::Literal::Unit) => {
                    let location = context.pattern_location(parameter.pattern).clone();
                    Some(cst::ParameterType::explicit(cst::Type::new(TypeKind::Unit, location)))
                },
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;

        // Any lambda at global scope shouldn't be able to capture any local variables
        let environment = None;

        let cst_function_type = cst::FunctionType { parameters, environment, return_type, has_resume: false };

        // We construct a function type to convert wholesale instead of converting as we go
        // to avoid repeating logic in [Type::from_cst_type], namely handling of effect types.
        let lambda_location = context.expr_location(definition.rhs).clone();
        let cst_fn_type = cst::Type::new(TypeKind::Function(cst_function_type), lambda_location);

        Some(Type::from_cst_type_generalized(&cst_fn_type, resolve, compiler, true))

    // The body being a type annotation is common for `extern` declarations: `puts = extern "puts": fn ...`
    } else if let Expr::TypeAnnotation(annotation) = &context[definition.rhs] {
        Some(Type::from_cst_type_generalized(&annotation.rhs, resolve, compiler, true))
    } else {
        None
    }
}

/// Like `try_get_generalized_type` but allows the resulting type to contain fresh type variables
/// for ability closure environments. The caller passes a `next_id` counter so that
/// fresh IDs don't collide with other type variables.
/// This function always succeeds. In the case there are no annotations, a fresh type variable is returned.
pub fn get_partial_type(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
    next_id: &mut u32,
) -> Type {
    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return Type::from_cst_type(typ, resolve, compiler, next_id, true);
    }

    if let Expr::Lambda(lambda) = &context[definition.rhs] {
        let lambda_location = context.expr_location(definition.rhs).clone();
        let hole = || cst::Type::new(cst::TypeKind::Hole, lambda_location.clone());

        let return_type = Box::new(lambda.return_type.clone().unwrap_or_else(hole));

        let parameters = mapvec(&lambda.parameters, |parameter| match &context[parameter.pattern] {
            Pattern::TypeAnnotation(_, typ) => cst::ParameterType::new(typ.clone(), parameter.is_implicit),
            Pattern::Literal(cst::Literal::Unit) => {
                let location = context.pattern_location(parameter.pattern).clone();
                cst::ParameterType::explicit(cst::Type::new(TypeKind::Unit, location))
            },
            _ => cst::ParameterType::explicit(hole()),
        });

        let environment = Some(Box::new(cst::Type::new(cst::TypeKind::Hole, lambda_location.clone())));
        let cst_function_type = cst::FunctionType { parameters, environment, return_type, has_resume: false };

        let cst_fn_type = cst::Type::new(TypeKind::Function(cst_function_type), lambda_location);
        Type::from_cst_type(&cst_fn_type, resolve, compiler, next_id, true)
    } else if let Expr::TypeAnnotation(annotation) = &context[definition.rhs] {
        Type::from_cst_type(&annotation.rhs, resolve, compiler, next_id, true)
    } else {
        let lambda_location = context.expr_location(definition.rhs).clone();
        let hole = cst::Type::new(cst::TypeKind::Hole, lambda_location);
        Type::from_cst_type(&hole, resolve, compiler, next_id, true)
    }
}
