use crate::{
    incremental::{self, DbHandle, GetItem, GetType, Resolve, TypeCheck},
    iterator_extensions::opt_mapvec,
    name_resolution::{Origin, ResolutionResult},
    parser::{
        cst::{self, Definition, Expr, Lambda, Pattern, TopLevelItemKind, TypeKind},
        desugar_context::DesugarContext,
        ids::TopLevelName,
    },
    type_inference::types::{LocalKinds, Type, TypeBindings},
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

/// True if the result type of a definition has a hole
fn result_annotation_contains_hole(
    definition: &Definition, context: &DesugarContext, resolve: &ResolutionResult, compiler: &DbHandle,
) -> bool {
    fn result_contains_hole(typ: &cst::Type, resolve: &ResolutionResult, compiler: &DbHandle) -> bool {
        match &typ.kind {
            TypeKind::Function(function) => contains_hole(&function.return_type, resolve, compiler),
            TypeKind::Forall(_, typ) => result_contains_hole(typ, resolve, compiler),
            _ => contains_hole(typ, resolve, compiler),
        }
    }
    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return result_contains_hole(typ, resolve, compiler);
    }
    match &context[definition.rhs] {
        Expr::Lambda(lambda) => match (&lambda.return_type, &context[lambda.body]) {
            (Some(typ), _) | (None, Expr::Constructor(cst::Constructor { typ, .. })) => {
                contains_hole(typ, resolve, compiler)
            },
            _ => false,
        },
        Expr::TypeAnnotation(annotation) => result_contains_hole(&annotation.rhs, resolve, compiler),
        Expr::Constructor(constructor) => contains_hole(&constructor.typ, resolve, compiler),
        _ => false,
    }
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
    // A hole in the result, such as a returned `=>` function's environment, is only known from the body
    if result_annotation_contains_hole(definition, context, resolve, compiler) {
        return None;
    }

    if let Pattern::TypeAnnotation(_, typ) = &context[definition.pattern] {
        return Some(Type::generalized(|next_id| Type::from_cst_annotation(typ, resolve, compiler, next_id, false)));
    }

    if let Expr::Lambda(lambda) = &context[definition.rhs] {
        let signature = lambda_signature(lambda, None, true, context, resolve, compiler)?;
        Some(Type::generalized(|next_id| Type::from_cst_function(&signature, resolve, compiler, next_id, false, true)))

    // The body being a type annotation is common for `extern` declarations: `puts = extern "puts": fn ...`
    } else if let Expr::TypeAnnotation(annotation) = &context[definition.rhs] {
        let typ = &annotation.rhs;
        Some(Type::generalized(|next_id| Type::from_cst_annotation(typ, resolve, compiler, next_id, false)))
    } else if let Expr::Constructor(constructor) = &context[definition.rhs]
        && constructor_type_is_fully_applied(&constructor.typ, resolve, compiler)
    {
        Some(Type::from_cst_type_generalized(&constructor.typ, resolve, compiler, true, false))
    } else {
        None
    }
}

/// A lambda's function type with `missing` as its environment and omitted types, if given
fn lambda_signature(
    lambda: &Lambda, missing: Option<cst::Type>, is_top_level: bool, context: &DesugarContext,
    resolve: &ResolutionResult, compiler: &DbHandle,
) -> Option<cst::FunctionType> {
    let return_type = match (&lambda.return_type, &context[lambda.body]) {
        (Some(typ), _) => typ.clone(),
        // Treat returning a constructor as an explicit return type to allow
        // implicits to avoid annotating repetitive return types.
        (None, Expr::Constructor(c)) if constructor_type_is_fully_applied(&c.typ, resolve, compiler) => c.typ.clone(),
        (None, _) => missing.clone()?,
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
    // Only a top-level annotation's elided parameter places are rigid
    let convert_annotation = |typ: &cst::Type, next_id: &mut u32| {
        if is_top_level {
            Type::from_cst_annotation(typ, resolve, compiler, next_id, true)
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
        Type::from_cst_function(&signature, resolve, compiler, next_id, true, is_top_level)
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
