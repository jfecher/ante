use crate::{
    incremental::{self, DbHandle, GetItem, GetType, Resolve, TypeCheck},
    name_resolution::ResolutionResult,
    parser::{
        context::TopLevelContext,
        cst::{self, Definition, Expr, Pattern, TopLevelItemKind},
    }, type_inference::types::{Type, TypeBindings},
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
            try_get_type(definition, &item_context, &resolve).unwrap_or_else(|| {
                let check = TypeCheck(context.0.top_level_item).get(compiler);
                check.result.generalized[&context.0.local_name_id].clone()
            })
        },
        _ => {
            let check = TypeCheck(context.0.top_level_item).get(compiler);
            check
                .result
                .generalized
                .get(&context.0.local_name_id)
                .unwrap_or_else(|| {
                    panic!(
                        "No generalized type entry for {} {}",
                        item_context.names[context.0.local_name_id], context.0
                    )
                })
                .clone()
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
pub(super) fn try_get_type(
    definition: &Definition, context: &TopLevelContext, resolve: &ResolutionResult,
) -> Option<Type> {
    if let Pattern::TypeAnnotation(_, typ) = &context.patterns[definition.pattern] {
        return Some(Type::from_cst_type(typ, resolve));
    }

    if let Expr::Lambda(lambda) = &context.exprs[definition.rhs] {
        let return_type = Box::new(lambda.return_type.as_ref()?.clone());

        let parameters = lambda
            .parameters
            .iter()
            .map(|parameter| match &context.patterns[parameter.pattern] {
                Pattern::TypeAnnotation(_, typ) => {
                    Some(cst::ParameterType::new(typ.clone(), parameter.is_implicit))
                }
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;

        let cst_function_type = cst::FunctionType {
            parameters,
            return_type,
            effects: lambda.effects.clone(),
        };

        // We construct a function type to convert wholesale instead of converting as we go
        // to avoid repeating logic in [Type::from_cst_type], namely handling of effect types.
        let typ = Type::from_cst_type(&cst::Type::Function(cst_function_type), resolve);
        Some(typ.generalize(&TypeBindings::default()))
    } else {
        None
    }
}
