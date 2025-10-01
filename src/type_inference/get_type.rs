use crate::{
    incremental::{self, DbHandle, GetItem, GetType, Resolve, TypeCheck},
    name_resolution::ResolutionResult,
    parser::{
        cst::{Definition, Expr, Pattern, TopLevelItemKind},
        TopLevelContext,
    },
    type_inference::types::{GeneralizedType, TopLevelType},
};

/// Get the type of the name defined by this TopLevelId.
/// If this doesn't define a name we return the Unit type by default.
///
/// This is very similar to but separate from `type_check_impl`. We separate these
/// because `type_check_impl` will always type check the contents of a definition,
/// and we don't want other definitions to depend on the contents of another definition
/// if the other definition provides a type annotation. Without type annotations the two
/// functions should be mostly equivalent.
pub fn get_type_impl(context: &GetType, compiler: &DbHandle) -> GeneralizedType {
    incremental::enter_query();
    let (item, item_context) = compiler.get(GetItem(context.0));
    incremental::println(format!("Get type of {:?}", item.id));

    let typ = match &item.kind {
        TopLevelItemKind::Definition(definition) => {
            let resolve = Resolve(context.0).get(compiler);
            try_get_type(definition, &item_context, &resolve).unwrap_or_else(|| {
                let check = TypeCheck(context.0).get(compiler);
                check.result.typ.clone()
            })
        },
        TopLevelItemKind::TypeDefinition(_) => GeneralizedType::unit(),
        TopLevelItemKind::TraitDefinition(_) => GeneralizedType::unit(),
        TopLevelItemKind::TraitImpl(_) => todo!(),
        TopLevelItemKind::EffectDefinition(_) => GeneralizedType::unit(),
        TopLevelItemKind::Extern(item) => {
            let resolution = Resolve(context.0).get(compiler);
            GeneralizedType::from_ast_type(&item.declaration.typ, &resolution)
        },
        TopLevelItemKind::Comptime(_) => todo!(),
    };
    incremental::exit_query();
    typ
}

/// Make a best-effort attempt to get the type of a definition.
/// If the type is successfully found then this definition will not be dependent on the
/// types of its contents to get its type. Put another way, if the type is known then
/// we don't need to re-type check this definition when its contents change.
pub(super) fn try_get_type(
    definition: &Definition, context: &TopLevelContext, resolve: &ResolutionResult,
) -> Option<GeneralizedType> {
    if let Pattern::TypeAnnotation(_, typ) = &context.patterns[definition.pattern] {
        return Some(GeneralizedType::from_ast_type(typ, resolve));
    }

    if let Expr::Lambda(lambda) = &context.exprs[definition.rhs] {
        let return_type = Box::new(TopLevelType::from_ast_type(lambda.return_type.as_ref()?, resolve));

        let parameters = lambda
            .parameters
            .iter()
            .map(|parameter| match &context.patterns[parameter.pattern] {
                Pattern::TypeAnnotation(_, typ) => Some(TopLevelType::from_ast_type(typ, resolve)),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;

        // TODO: effects
        // let effects = lambda.effects.as_ref();
        let function = TopLevelType::Function { parameters, return_type };
        return Some(GeneralizedType::from_top_level_type(function));
    }

    None
}
