use std::sync::Arc;

use crate::{
    incremental::{DbHandle, GetItem, GetItemRaw},
    iterator_extensions::mapvec,
    parser::{
        context::TopLevelContext,
        cst::{
            Constructor, Definition, Expr, Lambda, Pattern, TopLevelItem, TopLevelItemKind, TraitDefinition, TraitImpl,
            Type, TypeDefinitionBody,
        },
    },
};

pub fn get_item_impl(context: &GetItem, db: &DbHandle) -> (Arc<TopLevelItem>, Arc<TopLevelContext>) {
    let (item, context) = GetItemRaw(context.0).get(db);

    match &item.kind {
        TopLevelItemKind::TraitDefinition(trait_definition) => {
            let new_kind = desugar_trait(trait_definition);
            let new_item = Arc::new(TopLevelItem { comments: item.comments.clone(), kind: new_kind, id: item.id });
            (new_item, context)
        },
        TopLevelItemKind::TraitImpl(trait_impl) => {
            // TODO: Reduce cloning costs for context, comments
            let mut new_context = context.as_ref().clone();
            let new_kind = desugar_impl(trait_impl, &mut new_context);
            let new_item = Arc::new(TopLevelItem { comments: item.comments.clone(), kind: new_kind, id: item.id });
            (new_item, Arc::new(new_context))
        },
        _ => (item, context),
    }
}

/// Desugars
/// ```ante
/// impl name {Parameter}: Trait TraitArgs with
///     method1 = ...
///     method2 = ...
/// ```
/// Into
/// ```ante
/// implicit name {Parameter}: Trait TraitArgs = Trait With
///     method1 = ...
///     method2 = ...
/// ```
fn desugar_impl(impl_: &TraitImpl, context: &mut TopLevelContext) -> TopLevelItemKind {
    let variable = context.patterns.push(Pattern::Variable(impl_.name));
    let location = context.name_locations[impl_.name].clone();
    assert_eq!(variable, context.pattern_locations.push(location.clone()));

    let mut trait_type = Type::Named(impl_.trait_path);
    if !impl_.trait_arguments.is_empty() {
        trait_type = Type::Application(Box::new(trait_type), impl_.trait_arguments.clone());
    }

    let pattern = context.patterns.push(Pattern::TypeAnnotation(variable, trait_type.clone()));
    assert_eq!(pattern, context.pattern_locations.push(location.clone()));

    let fields = impl_.body.clone();
    let constructor = Expr::Constructor(Constructor { fields, typ: trait_type.clone() });
    let constructor = context.exprs.push(constructor);
    assert_eq!(constructor, context.expr_locations.push(location.clone()));

    let rhs = if impl_.parameters.is_empty() {
        constructor
    } else {
        let lambda = Expr::Lambda(Lambda {
            parameters: impl_.parameters.clone(),
            return_type: Some(trait_type),
            effects: Some(Vec::new()),
            body: constructor,
        });
        let lambda = context.exprs.push(lambda);
        assert_eq!(lambda, context.expr_locations.push(location));
        lambda
    };

    TopLevelItemKind::Definition(Definition { implicit: true, mutable: false, pattern, rhs })
}

/// Desugars
/// ```ante
/// trait Foo args with
///     declaration1: Type1
///     ...
///     declarationN: TypeN
/// ```
/// Into
/// ```ante
/// type Foo args =
///     declaration1: Type1
///     ...
///     declarationN: TypeN
/// ```
fn desugar_trait(trait_: &TraitDefinition) -> TopLevelItemKind {
    // TODO: handle trait_.functional_dependencies
    let fields = mapvec(&trait_.body, |decl| (decl.name, decl.typ.clone()));

    TopLevelItemKind::TypeDefinition(super::cst::TypeDefinition {
        shared: false,
        is_trait: true,
        name: trait_.name,
        generics: trait_.generics.clone(),
        body: TypeDefinitionBody::Struct(fields),
    })
}
