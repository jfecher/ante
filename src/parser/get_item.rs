use std::sync::Arc;

use crate::{
    diagnostics::Location,
    incremental::{DbHandle, GetItem, GetItemRaw},
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    parser::{
        cst::{
            self, Argument, Constructor, Definition, EffectDefinition, Expr, If, Lambda, Literal, Parameter, Path,
            Pattern, SequenceItem, TopLevelItem, TopLevelItemKind, TraitDefinition, TraitImpl, Type,
            TypeDefinitionBody, TypeKind,
        },
        desugar_context::DesugarContext,
        ids::{ExprId, PathId, PatternId},
    },
};

pub fn get_item_impl(context: &GetItem, db: &DbHandle) -> (Arc<TopLevelItem>, Arc<DesugarContext>) {
    let (item, context) = GetItemRaw(context.0).get(db);

    match &item.kind {
        TopLevelItemKind::TraitDefinition(trait_definition) => {
            let mut new_context = DesugarContext::new(context);
            let new_kind = desugar_trait(trait_definition, &mut new_context);
            let new_item = Arc::new(TopLevelItem { comments: item.comments.clone(), kind: new_kind, id: item.id });
            (new_item, Arc::new(new_context))
        },
        TopLevelItemKind::EffectDefinition(effect_definition) => {
            let mut new_context = DesugarContext::new(context);
            let new_kind = desugar_effect(effect_definition, &mut new_context);
            let new_item = Arc::new(TopLevelItem { comments: item.comments.clone(), kind: new_kind, id: item.id });
            (new_item, Arc::new(new_context))
        },
        TopLevelItemKind::TraitImpl(trait_impl) => {
            let mut new_context = DesugarContext::new(context);
            let new_definition = desugar_impl(trait_impl, &mut new_context);
            desugar_expression(new_definition.rhs, &mut new_context);
            let kind = TopLevelItemKind::Definition(new_definition);
            let new_item = Arc::new(TopLevelItem { comments: item.comments.clone(), kind, id: item.id });
            (new_item, Arc::new(new_context))
        },
        TopLevelItemKind::Definition(definition) => {
            let mut new_context = DesugarContext::new(context);
            desugar_expression(definition.rhs, &mut new_context);
            (item, Arc::new(new_context))
        },
        _ => {
            let new_context = DesugarContext::new(context);
            (item, Arc::new(new_context))
        },
    }
}

/// Expands a trait-typed parameter from e.g. `Print a` → `Print a [env_i]`.
/// This ensures `from_cst_type_no_type_variables` sees a named generic for the env
/// rather than auto-inserting a fresh type variable (which would cause it to return None).
fn add_env_to_trait_type(typ: &Type, env_var: crate::parser::ids::NameId, location: &Location) -> Type {
    let env_type = Type::new(TypeKind::Variable(env_var), location.clone());
    match &typ.kind {
        TypeKind::Named(_) => {
            Type::new(TypeKind::Application(Box::new(typ.clone()), vec![env_type]), typ.location.clone())
        },
        TypeKind::Application(f, args) => {
            let mut new_args = args.clone();
            new_args.push(env_type);
            Type::new(TypeKind::Application(f.clone(), new_args), typ.location.clone())
        },
        _ => typ.clone(),
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
/// implicit name {Parameter}: Trait TraitArgs Parameter = Trait With
///     method1 = ...
///     method2 = ...
/// ```
/// Note that this assumes the returned trait will capture each parameter used.
fn desugar_impl(impl_: &TraitImpl, context: &mut DesugarContext) -> Definition {
    let location = context.name_location(impl_.name).clone();
    let variable = context.push_pattern(Pattern::Variable(impl_.name), location.clone());

    let mut trait_type = Type::new(TypeKind::Named(impl_.trait_path), location.clone());

    // Collect existing parameter info before mutating context.
    let param_infos: Vec<(bool, crate::parser::ids::PatternId, Type)> = impl_
        .parameters
        .iter()
        .map(|param| match &context[param.pattern] {
            Pattern::TypeAnnotation(inner, typ) => (param.is_implicit, *inner, typ.clone()),
            _ => unreachable!("impl parameters are expected to have type annotations"),
        })
        .collect();

    // Build new parameters with expanded env types (e.g. `Print a` -> `Print a [env_0]`).
    // This prevents `from_cst_type_no_type_variables` from auto-inserting fresh type variables.
    let expanded_parameters = mapvec(param_infos.iter().enumerate(), |(i, (is_implicit, inner, typ))| {
        let env_name = context.push_name(Arc::new(format!("[env_{}]", i)), location.clone());
        let expanded_type = add_env_to_trait_type(typ, env_name, &location);
        let new_pattern = context.push_pattern(Pattern::TypeAnnotation(*inner, expanded_type), location.clone());
        cst::Parameter::with_implicit(new_pattern, *is_implicit)
    });

    if !impl_.trait_arguments.is_empty() || !impl_.parameters.is_empty() {
        let app_location = location.clone();
        let mut arguments = impl_.trait_arguments.clone();

        // Assume the returned trait captures each parameter.
        let parameter_types = expanded_parameters.iter().map(|param| match &context[param.pattern] {
            Pattern::TypeAnnotation(_, typ) => typ.clone(),
            _ => unreachable!("impl parameters are expected to have type annotations"),
        });
        arguments.push(make_tuple_type(&location, parameter_types));

        trait_type = Type::new(TypeKind::Application(Box::new(trait_type), arguments), app_location);
    }

    // If this is not a function we need to put the type annotation on the name itself rather than
    // the return type of the lambda.
    let pattern = if impl_.parameters.is_empty() {
        context.push_pattern(Pattern::TypeAnnotation(variable, trait_type.clone()), location.clone())
    } else {
        variable
    };

    let fields = impl_.body.clone();
    let constructor = Expr::Constructor(Constructor { fields, typ: trait_type.clone() });
    let constructor = context.push_expr(constructor, location.clone());

    let rhs = if impl_.parameters.is_empty() {
        constructor
    } else {
        let lambda = Expr::Lambda(Lambda {
            parameters: expanded_parameters,
            return_type: Some(trait_type),
            body: constructor,
            is_move: false,
        });
        context.push_expr(lambda, location)
    };

    Definition { implicit: true, mutable: false, pattern, rhs }
}

fn make_tuple_type(location: &Location, types: impl ExactSizeIterator<Item = Type>) -> Type {
    if types.len() == 0 {
        return Type::new(TypeKind::NoClosureEnv, location.clone());
    }
    Type::new(TypeKind::Tuple(types.collect()), location.clone())
}

/// Desugars
/// ```ante
/// trait Foo args with
///     declaration1: fn Arg1_1 ... ArgN_1 -> Ret_1
///     ...
///     declarationN: fn Arg1_N ... ArgN_N -> Ret_N
///     field1: SomeTrait args
/// ```
/// Into
/// ```ante
/// type Foo args env =
///     declaration1: fn Arg1_1 ... ArgN_1 [env] -> Ret_1
///     ...
///     declarationN: fn Arg1_N ... ArgN_N [env] -> Ret_N
///     field1: SomeTrait args [env]
/// ```
fn desugar_trait(trait_: &TraitDefinition, context: &mut DesugarContext) -> TopLevelItemKind {
    let name_location = context.name_location(trait_.name).clone();

    // TODO: Can this be done more cleanly without resorting to strings users cannot type?
    let env = context.push_name(Arc::new("[env]".into()), name_location.clone());

    // Add the `env` generic to the trait type itself
    let mut generics = trait_.generics.clone();
    generics.push(env);

    // Add `[env]` to each field type: for function types this is set as the closure environment,
    // for non-function types (e.g. sub-trait fields like `Add a`) it is appended as a type argument
    // so that the env is properly substituted when the trait is instantiated.
    let fields = mapvec(&trait_.body, |decl| {
        let typ = match &decl.typ.kind {
            cst::TypeKind::Function(f) => {
                let mut f = f.clone();
                f.environment = Some(Box::new(Type::new(TypeKind::Variable(env), name_location.clone())));
                Type::new(cst::TypeKind::Function(f), decl.typ.location.clone())
            },
            _ => add_env_to_trait_type(&decl.typ, env, &name_location),
        };
        (decl.name, typ)
    });

    TopLevelItemKind::TypeDefinition(super::cst::TypeDefinition {
        shared: false,
        is_trait: true,
        is_effect: false,
        name: trait_.name,
        generics,
        body: TypeDefinitionBody::Struct(fields),
    })
}

/// Desugars
/// ```ante
/// effect Add with
///     add: fn U32 -> Unit
/// ```
/// Into
/// ```ante
/// type Add =
///     add: fn U32 [Ptr Unit] -> Unit
/// ```
/// Each effect operation becomes a struct field whose type is a closure capturing the
/// local scope of the handler. These capability objects are second class to prevent them
/// from escaping, so capturing the entire environment by reference should be fine.
fn desugar_effect(effect: &EffectDefinition, context: &mut DesugarContext) -> TopLevelItemKind {
    let name_location = context.name_location(effect.name).clone();
    let generics = effect.generics.clone();

    let fields = mapvec(&effect.body, |decl| {
        let typ = match &decl.typ.kind {
            cst::TypeKind::Function(f) => {
                let mut f = f.clone();
                let ptr = Type::new(cst::TypeKind::Pointer, name_location.clone());
                let unit = Type::new(TypeKind::Unit, name_location.clone());
                let ptr_unit = Type::new(TypeKind::Application(Box::new(ptr), vec![unit]), name_location.clone());
                f.environment = Some(Box::new(ptr_unit));
                Type::new(cst::TypeKind::Function(f), decl.typ.location.clone())
            },
            _ => decl.typ.clone(),
        };
        (decl.name, typ)
    });

    TopLevelItemKind::TypeDefinition(super::cst::TypeDefinition {
        shared: false,
        is_trait: false,
        is_effect: true,
        name: effect.name,
        generics,
        body: TypeDefinitionBody::Struct(fields),
    })
}

/// Traverse the expression recursively, desugaring along the way looking for
/// any desugaring in [ExprDesugar].
fn desugar_expression(expr: ExprId, context: &mut DesugarContext) {
    let mut desugars = Vec::new();
    collect_expressions_to_desugar(expr, context, &mut desugars);

    for desugar in desugars {
        desugar.apply(context);
    }
}

fn collect_expressions_to_desugar(expr: ExprId, context: &DesugarContext, to_desugar: &mut Vec<ExprDesugar>) {
    match &context[expr] {
        Expr::Error => (),
        Expr::Literal(_) => (),
        Expr::Variable(_) => (),
        Expr::Quoted(_) => (),
        Expr::Definition(definition) => collect_expressions_to_desugar(definition.rhs, context, to_desugar),
        Expr::MemberAccess(access) => collect_expressions_to_desugar(access.object, context, to_desugar),
        Expr::Lambda(lambda) => collect_expressions_to_desugar(lambda.body, context, to_desugar),
        Expr::Reference(reference) => collect_expressions_to_desugar(reference.rhs, context, to_desugar),
        Expr::TypeAnnotation(annotation) => collect_expressions_to_desugar(annotation.lhs, context, to_desugar),
        Expr::Return(return_) => collect_expressions_to_desugar(return_.expression, context, to_desugar),
        Expr::Sequence(sequence) => {
            for item in sequence {
                collect_expressions_to_desugar(item.expr, context, to_desugar);
            }
        },
        Expr::Call(call) => {
            if is_and_call(expr, context) && and_chain_contains_is(expr, context) {
                let parts = flatten_and_chain(expr, context);
                collect_chain_sub_expressions(&parts, context, to_desugar);
                to_desugar.push(ExprDesugar::AndWithIs(expr));
                return;
            }

            collect_expressions_to_desugar(call.function, context, to_desugar);
            for arg in call.arguments.iter() {
                collect_expressions_to_desugar(arg.expr, context, to_desugar);
            }

            if call.arguments.iter().any(|arg| is_wildcard(arg.expr, context)) {
                to_desugar.push(ExprDesugar::CallWildcards(expr));
            }

            if let Some(desugar) = classify_call(&call, expr, context) {
                to_desugar.push(desugar);
            }
        },
        Expr::If(if_) => {
            if and_chain_contains_is(if_.condition, context) {
                let parts = flatten_and_chain(if_.condition, context);
                collect_chain_sub_expressions(&parts, context, to_desugar);
                collect_expressions_to_desugar(if_.then, context, to_desugar);
                if let Some(else_) = if_.else_ {
                    collect_expressions_to_desugar(else_, context, to_desugar);
                }
                to_desugar.push(ExprDesugar::IfWithIs(expr));
            } else {
                collect_expressions_to_desugar(if_.condition, context, to_desugar);
                collect_expressions_to_desugar(if_.then, context, to_desugar);
                if let Some(else_) = if_.else_ {
                    collect_expressions_to_desugar(else_, context, to_desugar);
                }
            }
        },
        Expr::Is(is_) => {
            collect_expressions_to_desugar(is_.lhs, context, to_desugar);
            to_desugar.push(ExprDesugar::BareIs(expr));
        },
        Expr::Bind(bind) => {
            collect_expressions_to_desugar(bind.rhs, context, to_desugar);
            collect_expressions_to_desugar(bind.body, context, to_desugar);
            to_desugar.push(ExprDesugar::Bind(expr));
        },
        Expr::Match(match_) => {
            collect_expressions_to_desugar(match_.expression, context, to_desugar);
            for case in match_.cases.iter() {
                collect_expressions_to_desugar(case.1, context, to_desugar);
            }
        },
        Expr::Handle(handle) => {
            collect_expressions_to_desugar(handle.expression, context, to_desugar);
            for case in handle.cases.iter() {
                collect_expressions_to_desugar(case.1, context, to_desugar);
            }
        },
        Expr::Constructor(constructor) => {
            for field in constructor.fields.iter() {
                collect_expressions_to_desugar(field.1, context, to_desugar);
            }
        },
        Expr::Loop(loop_) => {
            collect_expressions_to_desugar(loop_.body, context, to_desugar);
            to_desugar.push(ExprDesugar::Loop(expr));
        },
        Expr::While(w) => {
            collect_expressions_to_desugar(w.condition, context, to_desugar);
            collect_expressions_to_desugar(w.body, context, to_desugar);
        },
        Expr::For(fo) => {
            collect_expressions_to_desugar(fo.start, context, to_desugar);
            collect_expressions_to_desugar(fo.end, context, to_desugar);
            collect_expressions_to_desugar(fo.body, context, to_desugar);
        },
        Expr::Break | Expr::Continue => (),
        Expr::Assignment(assignment) => {
            let rhs = assignment.rhs;
            collect_expressions_to_desugar(assignment.lhs, context, to_desugar);
            collect_expressions_to_desugar(rhs, context, to_desugar);
            if let Some((_, op_expr)) = assignment.op {
                collect_expressions_to_desugar(op_expr, context, to_desugar);
            }
        },
        Expr::Extern(_) => (),
        Expr::InterpolatedString(interpolated) => {
            for expr in &interpolated.exprs {
                collect_expressions_to_desugar(*expr, context, to_desugar);
            }
            to_desugar.push(ExprDesugar::StringInterpolation(expr));
        },
    }
}

/// `loop (p1 = e1) ... -> body`
/// gets desugared to
/// `{ recur p1 ... = body; recur e1 ... }`
fn desugar_loop(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Loop(loop_) = context[expr].clone() else { unreachable!() };

    let location = context.expr_location(expr).clone();
    let body = loop_.body;
    let parameters = loop_.parameters.into_iter();

    // parameters and arg list
    let (parameters, arguments) = parameters
        .map(|parameter| {
            let (pattern, expr) = match parameter {
                cst::LoopParameter::Variable(name) => {
                    let pattern = cst::Pattern::Variable(name);
                    let pattern = context.push_pattern(pattern, location.clone());

                    let name_string = context[name].clone();
                    let path = cst::Path::ident(name_string.to_string(), location.clone());
                    let path = context.push_path(path, location.clone());
                    let expr = cst::Expr::Variable(path);
                    let expr = context.push_expr(expr, location.clone());
                    (pattern, expr)
                },
                cst::LoopParameter::PatternAndExpr(pattern, expr) => (pattern, expr),
                cst::LoopParameter::UnitLiteral(location) => {
                    let pattern = cst::Pattern::Literal(cst::Literal::Unit);
                    let pattern = context.push_pattern(pattern, location.clone());
                    let expr = cst::Expr::Literal(cst::Literal::Unit);
                    let expr = context.push_expr(expr, location.clone());
                    (pattern, expr)
                },
            };
            (Parameter::new(pattern), Argument::explicit(expr))
        })
        .unzip();

    // Create `recur = fn params... -> body`
    let name_id = context.push_name(cst::Name::new("recur".to_string()), location.clone());

    let recur = cst::Pattern::Variable(name_id);
    let recur = context.push_pattern(recur, location.clone());

    let lambda = cst::Expr::Lambda(cst::Lambda { parameters, return_type: None, body, is_move: false });
    let lambda = context.push_expr(lambda, location.clone());

    let definition =
        cst::Expr::Definition(cst::Definition { implicit: false, mutable: false, pattern: recur, rhs: lambda });
    let definition = context.push_expr(definition, location.clone());

    // Create `recur args...`
    let function_path = cst::Path::ident("recur".to_string(), location.clone());
    let function_path = context.push_path(function_path, location.clone());
    let function = cst::Expr::Variable(function_path);
    let function = context.push_expr(function, location.clone());
    let call = cst::Expr::Call(cst::Call { function, arguments });
    let call = context.push_expr(call, location);

    let definition = SequenceItem { expr: definition, comments: Vec::new() };
    let call = SequenceItem { expr: call, comments: Vec::new() };

    let replacement_expr = cst::Expr::Sequence(vec![definition, call]);
    context.set_expr(expr, replacement_expr);
}

fn is_wildcard(expr_id: ExprId, context: &DesugarContext) -> bool {
    if let Expr::Variable(path_id) = &context[expr_id] { context[*path_id].last_ident() == "_" } else { false }
}

/// Desugars `foo _ x _` into `fn _1 _2 -> foo _1 x _2`
fn desugar_call_wildcards(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Call(call) = context[expr].clone() else { unreachable!() };

    let location = context.expr_location(expr).clone();
    let mut parameters = Vec::new();
    let mut counter = 1u32;

    let new_arguments = mapvec(&call.arguments, |arg| {
        if is_wildcard(arg.expr, context) {
            let name = format!("_{}", counter);
            counter += 1;

            let name_id = context.push_name(cst::Name::new(name.clone()), location.clone());

            let pattern = context.push_pattern(Pattern::Variable(name_id), location.clone());
            parameters.push(Parameter::with_implicit(pattern, arg.is_implicit));

            let path = cst::Path::ident(name, location.clone());
            let path_id = context.push_path(path, location.clone());
            let var_expr = context.push_expr(Expr::Variable(path_id), location.clone());

            Argument { is_implicit: arg.is_implicit, expr: var_expr }
        } else {
            *arg
        }
    });

    let new_call = Expr::Call(cst::Call { function: call.function, arguments: new_arguments });
    let new_call_id = context.push_expr(new_call, location.clone());

    context.set_expr(expr, Expr::Lambda(Lambda { parameters, return_type: None, body: new_call_id, is_move: false }));
}

enum ExprDesugar {
    CallWildcards(ExprId),

    /// true = `|>`, false = `<|`
    Pipe {
        call: ExprId,
        pipe_right: bool,
    },

    /// true = `and`, false = `or`
    LogicalOperator {
        call: ExprId,
        is_or: bool,
    },

    /// `a with b` desugars to `b (fn () -> a)`
    TildeArrow {
        call: ExprId,
    },

    /// `<pattern> <- <rhs>` followed by statements desugars to `<rhs> (fn <pattern> -> <body>)`,
    /// prepending the lambda to `<rhs>`'s arguments if `<rhs>` is itself a call.
    Bind(ExprId),

    /// `U8 x` desugars to `cast x : U8`
    TypeCast {
        call: ExprId,
        target_type: Type,
    },

    Loop(ExprId),

    /// `"a${x}b${y}c"` desugars to `"a" ++ cast x : String ++ "b" ++ cast y : String ++ "c"`
    StringInterpolation(ExprId),

    /// `if <and-chain containing is> then T else E`: rewrite the whole If into
    /// a nested match so each `is`'s pattern bindings are in scope for any
    /// subsequent chain elements and the then branch.
    IfWithIs(ExprId),

    /// Value-position `and`-chain containing at least one `is`: rewrite the
    /// whole chain into a nested match.
    AndWithIs(ExprId),

    /// A bare `x is P` not in a parent if/and expression.
    BareIs(ExprId),
}

/// Classifies this call based on the called function.
/// Returns the desugaring to perform on this call, or `None` if there is nothing to do.
fn classify_call(call: &cst::Call, expr: ExprId, context: &DesugarContext) -> Option<ExprDesugar> {
    let Expr::Variable(path_id) = &context[call.function] else { return None };

    let path = &context[*path_id];
    if path.components.len() > 1 {
        return None;
    }

    let name = path.last_ident();

    if call.arguments.len() == 1 {
        let location = context.expr_location(call.function);
        if let Some(target_type) = type_name_to_type(name, location, *path_id) {
            return Some(ExprDesugar::TypeCast { call: expr, target_type });
        }
    }

    if call.arguments.len() != 2 {
        return None;
    }

    match name {
        "|>" => Some(ExprDesugar::Pipe { call: expr, pipe_right: true }),
        "<|" => Some(ExprDesugar::Pipe { call: expr, pipe_right: false }),
        "or" => Some(ExprDesugar::LogicalOperator { call: expr, is_or: true }),
        "and" => Some(ExprDesugar::LogicalOperator { call: expr, is_or: false }),
        "~>" => Some(ExprDesugar::TildeArrow { call: expr }),
        _ => None,
    }
}

fn type_name_to_type(name: &str, location: &Location, path_id: PathId) -> Option<Type> {
    let kind = match name {
        "I8" => TypeKind::Integer(IntegerKind::I8),
        "I16" => TypeKind::Integer(IntegerKind::I16),
        "I32" => TypeKind::Integer(IntegerKind::I32),
        "I64" => TypeKind::Integer(IntegerKind::I64),
        "Isz" => TypeKind::Integer(IntegerKind::Isz),
        "U8" => TypeKind::Integer(IntegerKind::U8),
        "U16" => TypeKind::Integer(IntegerKind::U16),
        "U32" => TypeKind::Integer(IntegerKind::U32),
        "U64" => TypeKind::Integer(IntegerKind::U64),
        "Usz" => TypeKind::Integer(IntegerKind::Usz),
        "F32" => TypeKind::Float(FloatKind::F32),
        "F64" => TypeKind::Float(FloatKind::F64),
        "Char" => TypeKind::Char,
        "Bool" => TypeKind::Named(path_id),
        _ => return None,
    };
    Some(Type::new(kind, location.clone()))
}

impl ExprDesugar {
    /// Apply the desugaring, mutating the DesugarContext with new expressions
    fn apply(self, context: &mut DesugarContext) {
        match self {
            ExprDesugar::CallWildcards(expr) => desugar_call_wildcards(expr, context),
            ExprDesugar::Pipe { call, pipe_right } => desugar_pipeline(call, context, pipe_right),
            ExprDesugar::LogicalOperator { call, is_or } => desugar_logical_operators(call, context, is_or),
            ExprDesugar::TildeArrow { call } => desugar_tilde_arrow(call, context),
            ExprDesugar::Bind(expr) => desugar_bind(expr, context),
            ExprDesugar::TypeCast { call, target_type } => desugar_type_cast(call, target_type, context),
            ExprDesugar::Loop(expr) => desugar_loop(expr, context),
            ExprDesugar::StringInterpolation(expr) => desugar_string_interpolation(expr, context),
            ExprDesugar::IfWithIs(expr) => desugar_if_with_is(expr, context),
            ExprDesugar::AndWithIs(expr) => desugar_and_with_is(expr, context),
            ExprDesugar::BareIs(expr) => desugar_bare_is(expr, context),
        }
    }
}

/// Desugars `U8 x` into `(Std.Prelude.Cast.cast x) : U8`
fn desugar_type_cast(expr: ExprId, target_type: Type, context: &mut DesugarContext) {
    let Expr::Call(call) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();

    let cast_path = Path {
        components: vec![
            ("Std".to_string(), location.clone()),
            ("Prelude".to_string(), location.clone()),
            ("Cast".to_string(), location.clone()),
            ("cast".to_string(), location.clone()),
        ],
    };
    let cast_path = context.push_path(cast_path, location.clone());
    let cast_var = context.push_expr(Expr::Variable(cast_path), location.clone());

    let cast_call = Expr::Call(cst::Call { function: cast_var, arguments: call.arguments });
    let cast_call = context.push_expr(cast_call, location);

    context.set_expr(expr, Expr::TypeAnnotation(cst::TypeAnnotation { lhs: cast_call, rhs: target_type }));
}

/// Desugars `"a${x}b${y}c"` into
/// `"a" ++ cast x : String ++ "b" ++ cast y : String ++ "c"`.
fn desugar_string_interpolation(expr: ExprId, context: &mut DesugarContext) {
    let Expr::InterpolatedString(interpolated) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();
    let item = |name: &str| (name.to_string(), location.clone());

    let string_type = {
        let string_path = Path { components: vec![item("Std"), item("Prelude"), item("String")] };
        let string_path = context.push_path(string_path, location.clone());
        Type::new(TypeKind::Named(string_path), location.clone())
    };

    // 1: create all the strings, 2: filter any known-empty strings, 3: append them together
    let mut strings = Vec::new();
    if !interpolated.fragments[0].is_empty() {
        let first_fragment = Expr::Literal(Literal::String(interpolated.fragments[0].clone()));
        strings.push(context.push_expr(first_fragment, location.clone()));
    }

    for (expr, fragment) in interpolated.exprs.iter().zip(interpolated.fragments.iter().skip(1)) {
        // Std.Prelude.Cast.cast
        let cast_path = Path { components: vec![item("Std"), item("Prelude"), item("Cast"), item("cast")] };
        let cast_path = context.push_path(cast_path, location.clone());
        let cast_var = context.push_expr(Expr::Variable(cast_path), location.clone());

        // Std.Prelude.Cast.cast expr : String
        let cast_call = Expr::Call(cst::Call { function: cast_var, arguments: vec![Argument::explicit(*expr)] });
        let cast_call = context.push_expr(cast_call, location.clone());
        let annotation = cst::TypeAnnotation { lhs: cast_call, rhs: string_type.clone() };
        let casted = context.push_expr(Expr::TypeAnnotation(annotation), location.clone());

        strings.push(casted);
        if !fragment.is_empty() {
            strings.push(context.push_expr(Expr::Literal(Literal::String(fragment.clone())), location.clone()));
        }
    }

    let appended = strings.into_iter().reduce(|acc, string| push_append(acc, string, &location, context)).unwrap();

    // Replace `expr`'s slot in-place with the root of the `++` chain so downstream
    // lookups of `expr` return the desugared form.
    context.set_expr(expr, context[appended].clone());
}

// Returns `lhs ++ rhs`
fn push_append(lhs: ExprId, rhs: ExprId, location: &Location, context: &mut DesugarContext) -> ExprId {
    let append_path = Path::ident("++".to_string(), location.clone());
    let append_path = context.push_path(append_path, location.clone());
    let append_var = context.push_expr(Expr::Variable(append_path), location.clone());
    let call = Expr::Call(cst::Call {
        function: append_var,
        arguments: vec![Argument::explicit(lhs), Argument::explicit(rhs)],
    });
    context.push_expr(call, location.clone())
}

/// Desugars `x |> foo a b` into `foo x a b` and `foo a b <| x` into `foo x a b`
///
/// Although `|>` and `<|` always slot into the first argument, this can be combined with
/// explicit currying via `_` to slot into the underscore's position:
/// `x |> foo a _ b` => `x |> (fn _1 -> foo a _1 b)` => `(fn _1 -> foo a _1 b) x`
fn desugar_pipeline(expr: ExprId, context: &mut DesugarContext, is_pipe_right: bool) {
    let Expr::Call(call) = &context[expr] else { unreachable!() };

    // TODO: This check bypasses name resolution of these operators. If the user shadows
    // the prelude's definitions of the pipeline operators they'll still get this behavior.
    // `x |> f`, `f <| x`
    let (x, f) = if is_pipe_right {
        (call.arguments[0], call.arguments[1].expr)
    } else {
        (call.arguments[1], call.arguments[0].expr)
    };

    if let Expr::Call(inner_call) = &context[f] {
        // Prepend value as the first argument: foo b c => foo(value, b, c)
        let new_args = std::iter::once(x).chain(inner_call.arguments.iter().copied()).collect();
        let new_call = cst::Call { function: inner_call.function, arguments: new_args };
        context.set_expr(expr, Expr::Call(new_call));
    }
}

/// Desugars:
/// - `a and b` into `if a then b else false`
/// - `a or b` into `if a then true else b`
fn desugar_logical_operators(expr: ExprId, context: &mut DesugarContext, is_or: bool) {
    let Expr::Call(call) = &context[expr] else { unreachable!() };

    let a = call.arguments[0].expr;
    let b = call.arguments[1].expr;
    let location = context.expr_location(expr).clone();
    // We need `false` in the `and` case and `true` in the `or` case
    let boolean = Expr::Literal(Literal::Bool(is_or));
    let boolean = context.push_expr(boolean, location);

    context.set_expr(
        expr,
        if is_or {
            Expr::If(If { condition: a, then: boolean, else_: Some(b) })
        } else {
            Expr::If(If { condition: a, then: b, else_: Some(boolean) })
        },
    );
}

/// Desugars `<pattern> <- <rhs> <newline> <body...>` into `<rhs> (fn <pattern> -> <body>)`.
///
/// If `<rhs>` is itself a call, the lambda is appended as its final argument.
/// e.g. `x <- f a b` (with body c) desugars to `f a b (fn x -> c)`.
fn desugar_bind(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Bind(bind) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();

    let lambda = Expr::Lambda(cst::Lambda {
        parameters: vec![Parameter::new(bind.pattern)],
        return_type: None,
        body: bind.body,
        is_move: false,
    });
    let lambda = context.push_expr(lambda, location);

    let new_call = if let Expr::Call(inner_call) = &context[bind.rhs] {
        let mut new_args = inner_call.arguments.clone();
        new_args.push(Argument::explicit(lambda));
        cst::Call { function: inner_call.function, arguments: new_args }
    } else {
        cst::Call { function: bind.rhs, arguments: vec![Argument::explicit(lambda)] }
    };
    context.set_expr(expr, Expr::Call(new_call));
}

/// Desugars `a ~> b` into `b (fn () -> a)`
///
/// If `b` is itself a call, `a` is prepended to its arguments directly:
/// `a ~> b c d` desugars to `b (fn () -> a) c d`
fn desugar_tilde_arrow(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Call(call) = &context[expr] else { unreachable!() };

    let a = call.arguments[0].expr;
    let b = call.arguments[1].expr;
    let location = context.expr_location(expr).clone();

    let pattern = context.push_pattern(cst::Pattern::Literal(Literal::Unit), location.clone());
    let lambda = Expr::Lambda(cst::Lambda {
        parameters: vec![cst::Parameter::new(pattern)],
        return_type: None,
        body: a,
        is_move: false,
    });
    let lambda = context.push_expr(lambda, location);

    let new_call = if let Expr::Call(inner_call) = &context[b] {
        // Prepend value as the first argument: foo b c => foo(value, b, c)
        let new_args =
            std::iter::once(Argument::explicit(lambda)).chain(inner_call.arguments.iter().copied()).collect();
        cst::Call { function: inner_call.function, arguments: new_args }
    } else {
        // Create a new call
        cst::Call { function: b, arguments: vec![Argument::explicit(lambda)] }
    };
    context.set_expr(expr, Expr::Call(new_call));
}

/// Element of a flattened `and`-chain.
enum ChainElement {
    /// No pattern bindings to propagate.
    Cond(ExprId),
    /// `lhs is pattern`: pattern bindings scope over subsequent chain elements.
    Is(cst::Is),
}

/// Is this Call-node `foo and bar`?
fn is_and_call(expr: ExprId, context: &DesugarContext) -> bool {
    let Expr::Call(c) = &context[expr] else { return false };
    if c.arguments.len() != 2 {
        return false;
    }
    let Expr::Variable(p) = &context[c.function] else { return false };
    let path = &context[*p];
    path.components.len() == 1 && path.last_ident() == "and"
}

/// True if this `and`-call chain contains an `is` expression
fn and_chain_contains_is(expr: ExprId, context: &DesugarContext) -> bool {
    match &context[expr] {
        Expr::Is(_) => true,
        _ if is_and_call(expr, context) => {
            let Expr::Call(c) = &context[expr] else { unreachable!() };
            let l = c.arguments[0].expr;
            let r = c.arguments[1].expr;
            and_chain_contains_is(l, context) || and_chain_contains_is(r, context)
        },
        _ => false,
    }
}

/// Flatten an expression tree along its `and` joins.
fn flatten_and_chain(expr: ExprId, context: &DesugarContext) -> Vec<ChainElement> {
    match &context[expr] {
        Expr::Is(is_) => vec![ChainElement::Is(is_.clone())],
        _ if is_and_call(expr, context) => {
            let Expr::Call(c) = &context[expr] else { unreachable!() };
            let l = c.arguments[0].expr;
            let r = c.arguments[1].expr;
            let mut parts = flatten_and_chain(l, context);
            parts.extend(flatten_and_chain(r, context));
            parts
        },
        _ => vec![ChainElement::Cond(expr)],
    }
}

/// Recurse into the non-structural sub-expressions of a flattened chain:
/// `Is.lhs` for each is-element, and the full expression for each cond-leaf.
fn collect_chain_sub_expressions(parts: &[ChainElement], context: &DesugarContext, to_desugar: &mut Vec<ExprDesugar>) {
    for part in parts {
        match part {
            ChainElement::Is(is_) => collect_expressions_to_desugar(is_.lhs, context, to_desugar),
            ChainElement::Cond(e) => collect_expressions_to_desugar(*e, context, to_desugar),
        }
    }
}

/// Build `match scrutinee | pattern -> then_branch | _ -> else_branch`.
fn build_match_from_is(
    scrutinee: ExprId, pattern: PatternId, then_branch: ExprId, else_branch: ExprId, location: Location,
    context: &mut DesugarContext,
) -> Expr {
    let underscore = context.push_name(Arc::new("_".to_string()), location.clone());
    let wildcard = context.push_pattern(Pattern::Variable(underscore), location);
    Expr::Match(cst::Match { expression: scrutinee, cases: vec![(pattern, then_branch), (wildcard, else_branch)] })
}

/// Compile a flat and-chain into an expression that evaluates to `then_expr`
/// iff every element succeeds (with each `Is`'s bindings in scope for every
/// element after it), else `else_expr`.
///
/// `else_expr` is shared across all fallthrough positions. Only one fallthrough
/// ever runs, so side effects are executed at most once.
fn compile_chain(parts: &[ChainElement], then_expr: ExprId, else_expr: ExprId, context: &mut DesugarContext) -> ExprId {
    let Some((head, tail)) = parts.split_first() else { return then_expr };
    let inner = compile_chain(tail, then_expr, else_expr, context);
    match head {
        ChainElement::Is(is_) => {
            let location = context.expr_location(is_.lhs).clone();
            let match_expr = build_match_from_is(is_.lhs, is_.pattern, inner, else_expr, location.clone(), context);
            context.push_expr(match_expr, location)
        },
        ChainElement::Cond(cond) => {
            let location = context.expr_location(*cond).clone();
            let if_expr = Expr::If(If { condition: *cond, then: inner, else_: Some(else_expr) });
            context.push_expr(if_expr, location)
        },
    }
}

fn desugar_if_with_is(expr: ExprId, context: &mut DesugarContext) {
    let Expr::If(if_) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();
    let else_branch = if_.else_.unwrap_or_else(|| context.push_expr(Expr::Literal(Literal::Unit), location));
    let parts = flatten_and_chain(if_.condition, context);
    let compiled = compile_chain(&parts, if_.then, else_branch, context);
    context.set_expr(expr, context[compiled].clone());
}

fn desugar_and_with_is(expr: ExprId, context: &mut DesugarContext) {
    let location = context.expr_location(expr).clone();
    let true_expr = context.push_expr(Expr::Literal(Literal::Bool(true)), location.clone());
    let false_expr = context.push_expr(Expr::Literal(Literal::Bool(false)), location);
    let parts = flatten_and_chain(expr, context);
    let compiled = compile_chain(&parts, true_expr, false_expr, context);
    context.set_expr(expr, context[compiled].clone());
}

fn desugar_bare_is(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Is(is_) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();
    let true_expr = context.push_expr(Expr::Literal(Literal::Bool(true)), location.clone());
    let false_expr = context.push_expr(Expr::Literal(Literal::Bool(false)), location.clone());
    let replacement = build_match_from_is(is_.lhs, is_.pattern, true_expr, false_expr, location, context);
    context.set_expr(expr, replacement);
}
