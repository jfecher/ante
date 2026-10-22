use crate::{
    diagnostics::Diagnostic,
    incremental::{self, DbHandle, GetItem, GetType, Resolve, TypeCheck},
    iterator_extensions::{mapvec, opt_mapvec},
    name_resolution::{Origin, ResolutionResult, trait_of_type},
    parser::{
        cst::{self, Definition, Expr, Lambda, Pattern, TopLevelItemKind, TypeKind},
        desugar_context::DesugarContext,
        get_item::TraitDefinition,
        ids::{ExprId, TopLevelName},
    },
    type_inference::{
        generics::Generic,
        types::{GenericSubstitutions, LocalKinds, Omissions, Type, TypeBindings, generic_display_name},
    },
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
    incremental::println(format_args!("Get type of {:?}", item.id));

    let typ = match &item.kind {
        TopLevelItemKind::Definition(definition) => {
            let resolve = Resolve(context.0.top_level_item).get(compiler);
            try_get_generalized_type(definition, item_context.as_ref(), &resolve, compiler)
                .map(|t| t.generalize(&TypeBindings::default()))
                .unwrap_or_else(|| {
                    let check = TypeCheck(context.0.top_level_item).get(compiler);
                    check.get_generalized(context.0.local_name_id)
                })
        },
        _ => {
            let check = TypeCheck(context.0.top_level_item).get(compiler);
            check.get_generalized(context.0.local_name_id)
        },
    };
    incremental::exit_query();
    typ
}

/// True if the signature of a definition has a hole, so its type is only known from its body. This is
/// a `_` written in one of its parameters, or any hole in its result. A parameter's other holes are
/// rigid instead, see [crate::parser::cst::TypeKind::RigidHole].
///
/// An implicit's type must be known without checking its body instead, so an omitted type argument
/// in its result defaulting to `_` is rigid as in its parameters. Omitting its trait's impl
/// argument is an error, see [implicit_result_omission].
fn signature_contains_hole(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> bool {
    if annotated_parameter_types(definition, context).into_iter().any(cst::Type::contains_inferred_hole) {
        return true;
    }
    let Some(result) = annotated_result_type(definition, context, resolve, compiler) else { return false };
    if definition.implicit {
        result.contains_hole() || implicit_result_omission(definition, result, context, resolve, compiler).is_some()
    } else {
        contains_hole(result, resolve, compiler)
    }
}

/// `typ` without any `forall`s wrapping it
fn strip_forall(mut typ: &cst::Type) -> &cst::Type {
    while let TypeKind::Forall(_, inner) = &typ.kind {
        typ = inner;
    }
    typ
}

/// The annotated type of each of a definition's parameters which has one
fn annotated_parameter_types<'a>(definition: &Definition, context: &'a DesugarContext) -> Vec<&'a cst::Type> {
    let parameters_of = |typ: &'a cst::Type| match &strip_forall(typ).kind {
        TypeKind::Function(function) => mapvec(&function.parameters, |parameter| &parameter.typ),
        _ => Vec::new(),
    };
    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return parameters_of(typ);
    }
    match &context[definition.rhs] {
        Expr::Lambda(lambda) => {
            let parameters = lambda.parameters.iter().map(|parameter| &context[parameter.pattern]);
            parameters
                .filter_map(|pattern| match pattern {
                    Pattern::TypeAnnotation(_, typ) => Some(typ),
                    _ => None,
                })
                .collect()
        },
        Expr::TypeAnnotation(annotation) => parameters_of(&annotation.rhs),
        _ => Vec::new(),
    }
}

/// The annotated result type of a definition, if any. This is the return type of a function or
/// the entire type of anything else. Returning a constructor of a fully applied type counts as
/// annotating that type.
fn annotated_result_type<'a>(
    definition: &Definition, context: &'a DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<&'a cst::Type> {
    let result_of = |typ: &'a cst::Type| {
        let typ = strip_forall(typ);
        match &typ.kind {
            TypeKind::Function(function) => &*function.return_type,
            _ => typ,
        }
    };
    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return Some(result_of(typ));
    }
    match &context[definition.rhs] {
        Expr::Lambda(lambda) => lambda
            .return_type
            .as_ref()
            .or_else(|| fully_applied_constructor_type(lambda.body, context, resolve, compiler)),
        Expr::TypeAnnotation(annotation) => Some(result_of(&annotation.rhs)),
        _ => fully_applied_constructor_type(definition.rhs, context, resolve, compiler),
    }
}

/// If `expr` is a constructor of a fully applied type, return that type
fn fully_applied_constructor_type<'a>(
    expr: ExprId, context: &'a DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<&'a cst::Type> {
    match &context[expr] {
        Expr::Constructor(constructor) if constructor_type_is_fully_applied(&constructor.typ, resolve, compiler) => {
            Some(&constructor.typ)
        },
        _ => None,
    }
}

/// If the result type `result` of a top-level implicit other than an impl is a trait omitting its
/// impl argument, return the error for it. An implicit's type
/// must be known without checking its body so the impl argument can't be inferred as it is for other
/// definitions. Making it rigid is no help either since no dictionary has the type `forall i. Eq I32 i`,
/// unlike rigid arguments of the trait's own parameters as in `forall d. Twice I32 d i`.
fn implicit_result_omission(
    definition: &Definition, result: &cst::Type, context: &DesugarContext, resolve: &ResolutionResult,
    compiler: &DbHandle,
) -> Option<Diagnostic> {
    // An impl's own impl type argument is filled in automatically. Name resolution already reports
    // constructing a trait outside of an impl, where an impl should be used instead.
    if trait_impl_of(definition, context, resolve, compiler).is_some() || resolve.constructs_trait_outside_impl {
        return None;
    }
    let (path, given) = match &result.kind {
        TypeKind::Named(path) => (*path, 0),
        TypeKind::Application(constructor, args) => match &constructor.kind {
            TypeKind::Named(path) => (*path, args.len()),
            _ => return None,
        },
        _ => return None,
    };
    let Some(Origin::TopLevelDefinition(constructor)) = resolve.path_origins.get(&path).copied() else { return None };
    let (item, item_context) = GetItem(constructor.top_level_item).get(compiler);
    let TopLevelItemKind::TypeDefinition(definition) = &item.kind else { return None };

    // Giving fewer than the required arguments is an error rather than an omission
    if given < definition.required_generic_count() {
        return None;
    }
    let omitted = (given..definition.generics.len()).filter(|i| definition.generics[*i].defaults_to_hole());
    let missing: Vec<_> = omitted.map(|i| generic_display_name(definition, i, &item_context)).collect();
    if missing.is_empty() || trait_of_type(result, &resolve.path_origins, compiler).is_none() {
        return None;
    }
    let typ = item_context[definition.name].to_string();
    Some(Diagnostic::ImplicitOmitsHoleTypeArgument { typ, missing, location: result.location.clone() })
}

/// If the result type of a top-level implicit other than an impl is a trait omitting its impl
/// argument, return the error for it. See [implicit_result_omission].
pub(crate) fn implicit_result_omitted_type_args(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<Diagnostic> {
    let result = annotated_result_type(definition, context, resolve, compiler)?;
    implicit_result_omission(definition, result, context, resolve, compiler)
}

/// True if `typ` has a hole, including one from an omitted type argument defaulting to `_`
fn contains_hole(typ: &cst::Type, resolve: &ResolutionResult, compiler: &DbHandle) -> bool {
    contains_hole_helper(typ, resolve, compiler, &mut Vec::new())
}

fn contains_hole_helper(
    typ: &cst::Type, resolve: &ResolutionResult, compiler: &DbHandle, visited: &mut Vec<TopLevelName>,
) -> bool {
    typ.contains_hole_with(&mut |path, arg_count| {
        let Some(Origin::TopLevelDefinition(name)) = resolve.path_origins.get(&path).copied() else {
            return false;
        };
        let (item, _) = compiler.get(GetItem(name.top_level_item));
        let TopLevelItemKind::TypeDefinition(definition) = &item.kind else {
            return false;
        };

        // Omitted arguments are only filled in if they all have defaults
        let omitted = definition.generics.get(arg_count..).unwrap_or_default();
        if omitted.is_empty() || arg_count < definition.required_generic_count() || visited.contains(&name) {
            return false;
        }

        visited.push(name);
        let resolve = Resolve(name.top_level_item).get(compiler);
        let mut defaults = omitted.iter().filter_map(|param| param.default.as_ref());
        let result = defaults.any(|default| contains_hole_helper(default, &resolve, compiler, visited));
        visited.pop();
        result
    })
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
pub fn try_get_generalized_type(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<Type> {
    // A hole such as a returned `=>` function's environment is only known from the body
    if signature_contains_hole(definition, context, resolve, compiler) {
        return None;
    }

    let typ = convert_declared_type(definition, context, resolve, compiler)?;
    Some(apply_impl_data(typ, definition, context, resolve, compiler))
}

/// Convert the declared type of a definition, if it is fully annotated. The parts left out of it
/// are rigid, see [Omissions::Rigid].
fn convert_declared_type(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<Type> {
    let rigid = Omissions::Rigid;
    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return Some(Type::generalized(|next_id| {
            Type::from_cst_annotation(typ, resolve, compiler, next_id, false, rigid)
        }));
    }

    if let Expr::Lambda(lambda) = &context[definition.rhs] {
        let signature = lambda_signature(lambda, None, true, context, resolve, compiler)?;
        Some(Type::generalized(|next_id| {
            Type::from_cst_function(&signature, resolve, compiler, next_id, false, true, rigid)
        }))

    // The body being a type annotation is common for `extern` declarations: `puts = extern "puts": fn ...`
    } else if let Expr::TypeAnnotation(annotation) = &context[definition.rhs] {
        let typ = &annotation.rhs;
        Some(Type::generalized(|next_id| Type::from_cst_annotation(typ, resolve, compiler, next_id, false, rigid)))
    } else {
        let typ = fully_applied_constructor_type(definition.rhs, context, resolve, compiler)?;
        Some(Type::from_cst_type_generalized(typ, resolve, compiler, true, false))
    }
}

/// If this definition was desugared from an `impl` of a trait, return the impl and the trait.
/// An `impl` of any other type is an ordinary definition with no impl data. `context` and `resolve`
/// are the definition's context and name resolution result.
///
/// This is the only place deciding whether a definition is an impl of a trait.
pub(crate) fn trait_impl_of<'a>(
    definition: &'a Definition, context: &DesugarContext, resolve: &ResolutionResult, db: &DbHandle,
) -> Option<(&'a cst::ImplInfo, TraitDefinition)> {
    let impl_info = definition.impl_info.as_deref()?;
    let Expr::Constructor(constructor) = &context[impl_info.constructor] else { return None };
    let trait_ = trait_of_type(&constructor.typ, &resolve.path_origins, db)?;
    Some((impl_info, trait_))
}

/// An impl's declared trait type omits the trait's impl argument, which then converts to a rigid
/// [Generic::Elided] like any other argument defaulting to `_` omitted from a signature. For an impl,
/// that generic is the impl's own data type applied to the impl's parameter types:
/// `fn {Eq t e i} -> Eq (Vec t) e _` becomes `fn {Eq t e i} -> Eq (Vec t) e (eq_vec (Eq t e i))`.
/// This is done once the type is converted since the trait may be named through an alias, in which
/// case which omitted argument is the impl argument is only known once the alias is expanded.
/// Does nothing if `definition` is not an impl of a trait.
///
/// An impl argument given explicitly is left as-is. It is checked against the impl's data type along
/// with the impl's constructor, see `start_impl`.
fn apply_impl_data(
    typ: Type, definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, db: &DbHandle,
) -> Type {
    let Some((impl_info, trait_)) = trait_impl_of(definition, context, resolve, db) else { return typ };
    let (parameter_types, trait_type) = match &typ {
        Type::Function(function) => {
            (mapvec(&function.parameters, |parameter| parameter.typ.clone()), &function.return_type)
        },
        other => (Vec::new(), other),
    };
    let Type::Application(_, args) = trait_type else { return typ };
    let Some(Type::Generic(omitted @ Generic::Elided(_))) = args.get(trait_.impl_generic_index()) else {
        return typ;
    };
    let impl_data = Type::impl_data(impl_info.name, parameter_types);
    let substitutions = GenericSubstitutions::from_iter([(*omitted, impl_data)]);
    typ.substitute(&substitutions, &TypeBindings::default())
}

/// A lambda's function type with `missing` as its environment and omitted types, if given
fn lambda_signature(
    lambda: &Lambda, missing: Option<cst::Type>, is_top_level: bool, context: &DesugarContext,
    resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<cst::FunctionType> {
    // Treat returning a constructor as an explicit return type to allow
    // implicits to avoid annotating repetitive return types.
    let return_type =
        lambda.return_type.as_ref().or_else(|| fully_applied_constructor_type(lambda.body, context, resolve, compiler));
    let return_type = match return_type {
        Some(typ) => typ.clone(),
        None => missing.clone()?,
    };

    let parameters = opt_mapvec(&lambda.parameters, |parameter| {
        let (typ, name) = match &context[parameter.pattern] {
            Pattern::TypeAnnotation(inner, typ) => {
                let name = if let Pattern::Variable(name) = &context[*inner] { Some(*name) } else { None };
                (typ.clone(), name)
            },
            Pattern::Literal(cst::Literal::Unit) => {
                let location = context.pattern_location(parameter.pattern).clone();
                (cst::Type::new(TypeKind::Unit, location), None)
            },
            _ => (missing.clone()?, None),
        };
        Some(cst::ParameterType { typ, is_implicit: parameter.is_implicit, name })
    })?;

    // A top-level function with a return type and no `can` clause is pure
    let effects = match &lambda.effects {
        None if is_top_level && lambda.return_type.is_some() => Some(Vec::new()),
        effects => effects.clone(),
    };

    let environment = missing.map(Box::new);
    Some(cst::FunctionType { parameters, environment, return_type: Box::new(return_type), effects })
}

fn constructor_type_is_fully_applied(typ: &cst::Type, resolve: &ResolutionResult, compiler: &DbHandle) -> bool {
    match &typ.kind {
        TypeKind::Named(path) => match resolve.path_origins.get(path) {
            Some(Origin::TopLevelDefinition(name)) => {
                let (item, _) = compiler.get(GetItem(name.top_level_item));
                match &item.kind {
                    TopLevelItemKind::TypeDefinition(definition) => definition.generics.is_empty(),
                    _ => true,
                }
            },
            _ => true,
        },
        _ => true,
    }
}

/// If a definition is fully annotated with no type holes, this returns `(seeded, declared)`
/// where `declared` is the declared type and `seeded` is `declared` but with all
/// `Type::Generic(Generic::Inferred(id))` replaced with `Type::Variable(id)`.
pub fn try_get_seeded_and_declared_type(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
    next_id: &mut u32,
) -> Option<(Type, Type)> {
    let declared = try_get_generalized_type(definition, context, resolve, compiler)?;
    let declared = declared.generalize(&TypeBindings::default());
    let (seeded, declared) = declared.open_inferred_generics(next_id);
    Some((seeded, declared))
}

/// Like `try_get_generalized_type` but allows the resulting type to contain fresh type variables
/// for ability closure environments.
/// In the case there are no annotations, a fresh type variable is returned.
pub fn get_partial_type(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
    next_id: &mut u32, is_top_level: bool,
) -> Type {
    // The type is inferred from the body, so its omitted parts are fresh variables. See [Omissions].
    let convert_annotation = |typ: &cst::Type, next_id: &mut u32| {
        if is_top_level {
            Type::from_cst_annotation(typ, resolve, compiler, next_id, true, Omissions::Fresh)
        } else {
            Type::from_cst_type(typ, resolve, compiler, next_id, &mut LocalKinds::default(), true, true)
        }
    };

    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return convert_annotation(typ, next_id);
    }

    if let Expr::Lambda(lambda) = &context[definition.rhs] {
        let hole = cst::Type::new(cst::TypeKind::Hole, context.expr_location(definition.rhs).clone());
        let signature = lambda_signature(lambda, Some(hole), is_top_level, context, resolve, compiler)
            .expect("omitted types are holes");
        Type::from_cst_function(&signature, resolve, compiler, next_id, true, is_top_level, Omissions::Fresh)
    } else if let Expr::TypeAnnotation(annotation) = &context[definition.rhs] {
        convert_annotation(&annotation.rhs, next_id)
    } else if let Expr::Constructor(constructor) = &context[definition.rhs]
        && constructor_type_is_fully_applied(&constructor.typ, resolve, compiler)
    {
        let mut local_kinds = LocalKinds::default();
        Type::from_cst_type(&constructor.typ, resolve, compiler, next_id, &mut local_kinds, true, true)
    } else {
        let lambda_location = context.expr_location(definition.rhs).clone();
        let hole = cst::Type::new(cst::TypeKind::Hole, lambda_location);
        let mut local_kinds = LocalKinds::default();
        Type::from_cst_type(&hole, resolve, compiler, next_id, &mut local_kinds, true, true)
    }
}
