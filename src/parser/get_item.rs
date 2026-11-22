use std::sync::Arc;

use inc_complete::DbGet;
use smallbitvec::SmallBitVec;

use crate::{
    diagnostics::Location,
    incremental::{self, DbHandle, GetItem, GetItemRaw, RawItem},
    iterator_extensions::mapvec,
    lexer::token::{FloatKind, IntegerKind},
    parser::{
        cst::{
            self, Argument, Constructor, Definition, Expr, GenericParam, If, Lambda, Literal, Parameter, Path, Pattern,
            SequenceItem, TopLevelItem, TopLevelItemKind, TraitImpl, TraitOrEffectDefinition, Type, TypeDefinitionBody,
            TypeKind,
        },
        desugar_context::DesugarContext,
        ids::{ExprId, NameId, PathId, PatternId, TopLevelId, TopLevelName},
    },
    shared_arc::Shared,
};

/// Unspellable name for a trait's impl data generic
const IMPL_GENERIC_NAME: &str = "$impl";

/// The name error messages use for the impl generic, since [IMPL_GENERIC_NAME] can't be written
pub const IMPL_GENERIC_DISPLAY_NAME: &str = "i";

pub fn get_item_impl(context: &GetItem, db: &DbHandle) -> (Shared<TopLevelItem>, Shared<DesugarContext>) {
    incremental::enter_query();
    incremental::println(format_args!("Desugaring {:?}", context.0));

    let RawItem(item, context) = GetItemRaw(context.0).get(db);

    let result = match &item.kind {
        TopLevelItemKind::TraitDefinition(def) | TopLevelItemKind::EffectDefinition(def) => {
            let is_effect = matches!(&item.kind, TopLevelItemKind::EffectDefinition(_));
            let mut new_context = DesugarContext::new(context);
            let new_kind = desugar_trait_or_effect(def, is_effect, &mut new_context);
            let new_item = Shared::new(TopLevelItem {
                attributes: item.attributes.clone(),
                comments: item.comments.clone(),
                kind: new_kind,
                id: item.id,
            });
            (new_item, Shared::new(new_context))
        },
        TopLevelItemKind::TraitImpl(trait_impl) => {
            let mut new_context = DesugarContext::new(context);
            let impl_name = TopLevelName::new(item.id, trait_impl.name);
            let new_definition = desugar_trait_impl(trait_impl, impl_name, &mut new_context);
            desugar_expression(new_definition.rhs, &mut new_context);
            let kind = TopLevelItemKind::Definition(new_definition);
            let new_item = Shared::new(TopLevelItem {
                attributes: item.attributes.clone(),
                comments: item.comments.clone(),
                kind,
                id: item.id,
            });
            (new_item, Shared::new(new_context))
        },
        TopLevelItemKind::Definition(definition) => {
            let mut new_context = DesugarContext::new(context);
            desugar_expression(definition.rhs, &mut new_context);
            (item, Shared::new(new_context))
        },
        _ => {
            let new_context = DesugarContext::new(context);
            (item, Shared::new(new_context))
        },
    };

    incremental::exit_query();
    result
}

/// Desugars
/// ```ante
/// impl name {Parameter}: Trait TraitArgs with
///     method1 = ...
///     method2 = ...
/// ```
/// Into
/// ```ante
/// implicit name {Parameter}: Trait TraitArgs = Trait TraitArgs with
///     method1 = ...
///     method2 = ...
/// ```
fn desugar_trait_impl(impl_: &TraitImpl, impl_name: TopLevelName, context: &mut DesugarContext) -> Definition {
    let location = context.name_location(impl_.name).clone();
    let variable = context.push_pattern(Pattern::Variable(impl_.name), location.clone());

    let mut ability_type = Type::new(TypeKind::Named(impl_.trait_path), location.clone());

    if !impl_.trait_arguments.is_empty() {
        ability_type =
            Type::new(TypeKind::Application(Box::new(ability_type), impl_.trait_arguments.clone()), location.clone());
    }

    // If this is not a function we need to put the type annotation on the name itself rather than
    // the return type of the lambda.
    let pattern = if impl_.parameters.is_empty() {
        context.push_pattern(Pattern::TypeAnnotation(variable, ability_type.clone()), location.clone())
    } else {
        variable
    };

    let fields = impl_.body.clone();
    let constructor = Expr::Constructor(Constructor { fields, typ: ability_type.clone() });
    let constructor = context.push_expr(constructor, location.clone());

    let rhs = if impl_.parameters.is_empty() {
        constructor
    } else {
        let lambda = Expr::Lambda(Lambda {
            return_type: Some(ability_type),
            ..Lambda::new(impl_.parameters.clone(), constructor)
        });
        context.push_expr(lambda, location)
    };

    let impl_info = Some(Box::new(cst::ImplInfo { name: impl_name, constructor }));
    Definition { impl_info, ..Definition::new(true, false, pattern, rhs) }
}

/// Desugars
/// ```ante
/// trait Foo args =
///     method: fn Arg1 ... ArgN -> Ret
///     constant: T
/// ```
/// Into
/// ```ante
/// type Foo args (i = _) =
///     method: fn Arg1 ... ArgN -> Ret
///     constant: T
/// ```
fn desugar_trait_or_effect(
    ability: &TraitOrEffectDefinition, is_effect: bool, context: &mut DesugarContext,
) -> TopLevelItemKind {
    let mut generics = ability.generics.clone();
    let fields = mapvec(&ability.body, |decl| (decl.name, decl.typ.clone()));

    let kind = if is_effect {
        cst::TypeDefinitionKind::Effect
    } else {
        let location = context.name_location(ability.name).clone();
        let impl_generic = context.push_name(Arc::new(IMPL_GENERIC_NAME.to_string()), location.clone());
        let default = Some(Type::new(TypeKind::Hole, location));
        generics.push(GenericParam { name: impl_generic, kind: None, default });
        cst::TypeDefinitionKind::Trait
    };

    TopLevelItemKind::TypeDefinition(super::cst::TypeDefinition {
        shared: false,
        mutable: false,
        kind,
        name: ability.name,
        generics,
        body: TypeDefinitionBody::Struct(fields),
    })
}

/// The desugared type definition of a trait, see [desugar_trait_or_effect]. Whether each of its
/// members is a method is only known once the trait is type checked, see
/// [TraitDefinition::is_method].
#[derive(Clone)]
pub struct TraitDefinition {
    item: Shared<TopLevelItem>,
    context: Shared<DesugarContext>,
}

impl TraitDefinition {
    /// Returns the trait whose type is named `name`, or `None` if `name` does not name a trait.
    /// Since a trait's constructor shares its type's name, this also recognizes the constructor.
    pub fn get(name: TopLevelName, db: &impl DbGet<GetItem>) -> Option<Self> {
        let (item, context) = GetItem(name.top_level_item).get(db);
        match &item.kind {
            TopLevelItemKind::TypeDefinition(definition)
                if definition.name == name.local_name_id && definition.kind.is_trait() =>
            {
                Some(Self { item, context })
            },
            _ => None,
        }
    }

    /// The top-level item defining this trait
    pub fn id(&self) -> TopLevelId {
        self.item.id
    }

    /// The name of this trait's type, which [Self::get] accepts
    pub fn type_name(&self) -> TopLevelName {
        TopLevelName::new(self.id(), self.definition().name)
    }

    pub fn definition(&self) -> &cst::TypeDefinition {
        match &self.item.kind {
            TopLevelItemKind::TypeDefinition(definition) => definition,
            _ => unreachable!("TraitDefinition always holds a type definition"),
        }
    }

    pub fn name(&self) -> &cst::Name {
        &self.context[self.definition().name]
    }

    /// Each member of the trait, in the order they were declared
    pub fn members(&self) -> &[(NameId, Type)] {
        match &self.definition().body {
            TypeDefinitionBody::Struct(fields) => fields,
            _ => unreachable!("TraitDefinition always holds a struct"),
        }
    }

    /// The index of the trait's impl generic, see [cst::TypeDefinitionKind::Trait]
    pub fn impl_generic_index(&self) -> usize {
        self.definition().impl_generic_index().expect("TraitDefinition always holds a trait")
    }

    /// Whether each of the trait's generics is an effect row
    pub fn effect_generics(&self) -> SmallBitVec {
        self.definition().generics.iter().map(GenericParam::is_effect).collect()
    }
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

            if let Some(desugar) = classify_call(call, expr, context) {
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
        Expr::Do(do_) => {
            collect_expressions_to_desugar(do_.body, context, to_desugar);
            to_desugar.push(ExprDesugar::Do(expr));
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
            if and_chain_contains_is(w.condition, context) {
                let parts = flatten_and_chain(w.condition, context);
                collect_chain_sub_expressions(&parts, context, to_desugar);
                collect_expressions_to_desugar(w.body, context, to_desugar);
                to_desugar.push(ExprDesugar::WhileWithIs(expr));
            } else {
                collect_expressions_to_desugar(w.condition, context, to_desugar);
                collect_expressions_to_desugar(w.body, context, to_desugar);
            }
            to_desugar.push(ExprDesugar::While(expr));
        },
        Expr::For(fo) => {
            collect_expressions_to_desugar(fo.iterable, context, to_desugar);
            collect_expressions_to_desugar(fo.body, context, to_desugar);
            to_desugar.push(ExprDesugar::For(expr));
        },
        Expr::Break | Expr::Continue => to_desugar.push(ExprDesugar::LoopJump(expr)),
        Expr::Fallthrough => (),
        Expr::Chain(_) => unreachable!("Expr::Chain is only created by desugaring"),
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
        Expr::ArrayLiteral(elements) => {
            for element in elements {
                collect_expressions_to_desugar(*element, context, to_desugar);
            }
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
                cst::LoopParameter::Pattern(pattern) => (pattern, pattern_to_expr(pattern, context)),
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

    let lambda = push_lambda(parameters, body, cst::LambdaKind::Function, &location, context);

    let definition = cst::Expr::Definition(cst::Definition::new(false, false, recur, lambda));
    let definition = context.push_expr(definition, location.clone());

    // Create `recur args...`
    let function = push_variable(cst::Path::ident(Arc::new("recur".to_string()), location.clone()), &location, context);
    let call = cst::Expr::Call(cst::Call { function, arguments });
    let call = context.push_expr(call, location);

    let definition = SequenceItem { expr: definition, comments: Vec::new() };
    let call = SequenceItem { expr: call, comments: Vec::new() };

    let replacement_expr = cst::Expr::Sequence(vec![definition, call]);
    context.set_expr(expr, replacement_expr);
}

/// `for pattern in iterable do body`
/// gets desugared to
/// `Std.Stream.(for) iterable (fn pattern -> body)`
///
/// See [desugar_loop_call] for a `return` in the body.
fn desugar_for(expr: ExprId, context: &mut DesugarContext) {
    let Expr::For(for_) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();

    let has_return = contains_return(for_.body, context);

    let parameters = vec![Parameter::new(for_.pattern)];
    let body_location = context.expr_location(for_.body).clone();
    let body = push_lambda(parameters, for_.body, cst::LambdaKind::ForBody, &body_location, context);

    let function = std_variable(&["Std", "Stream", "for"], &location, context);
    let arguments = vec![Argument::explicit(for_.iterable), Argument::explicit(body)];
    desugar_loop_call(expr, cst::Call { function, arguments }, has_return, context);
}

/// `while condition do body`
/// gets desugared to
/// `Std.Stream.(while) (fn () -> condition) (fn () -> body)`
///
/// See [desugar_loop_call] for a `return` in the condition or body.
fn desugar_while(expr: ExprId, context: &mut DesugarContext) {
    let Expr::While(while_) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();

    let has_return = contains_return(while_.condition, context) || contains_return(while_.body, context);

    let thunk = |body, kind, context: &mut DesugarContext| {
        let location = context.expr_location(body).clone();
        let unit = context.push_pattern(Pattern::Literal(Literal::Unit), location.clone());
        push_lambda(vec![Parameter::new(unit)], body, kind, &location, context)
    };
    let condition = thunk(while_.condition, cst::LambdaKind::WhileCondition, context);
    let body = thunk(while_.body, cst::LambdaKind::WhileBody, context);

    let function = std_variable(&["Std", "Stream", "while"], &location, context);
    let arguments = vec![Argument::explicit(condition), Argument::explicit(body)];
    desugar_loop_call(expr, cst::Call { function, arguments }, has_return, context);
}

/// Sets `expr` to `loop_call`, the call a loop was desugared into.
///
/// The condition and body of a loop are closures, so a `return` inside one would only return from
/// the closure. To return from the enclosing function instead, we have the `return` perform an
/// effect: `Std.EarlyReturn.early_return e`. Type inference still checks it as a `return` first
/// before turning it into that call. `break` and `continue` don't need this since they are already
/// effects handled by the loop function. If the loop has any `return`s, we wrap it to turn the
/// effect back into a `return` from the enclosing function:
/// ```text
/// match
///     handle
///         loop_call
///         None
///     | early_return x -> Some x
/// | Some x -> return x
/// | None -> ()
/// ```
/// Loops are desugared innermost first. So a `return` in a nested loop becomes the `return x`
/// above, which is now inside the outer loop's body, causing the outer loop to be wrapped in turn.
fn desugar_loop_call(expr: ExprId, loop_call: cst::Call, has_return: bool, context: &mut DesugarContext) {
    let location = context.expr_location(expr).clone();
    let call = Expr::Call(loop_call);

    if !has_return {
        context.set_expr(expr, call);
        return;
    }

    let call = context.push_expr(call, location.clone());
    let none = std_variable(&["Std", "Prelude", "Maybe", "None"], &location, context);
    let items =
        vec![SequenceItem { expr: call, comments: Vec::new() }, SequenceItem { expr: none, comments: Vec::new() }];
    let handled = context.push_expr(Expr::Sequence(items), location.clone());
    let unit = context.push_pattern(Pattern::Literal(Literal::Unit), location.clone());
    let kind = cst::LambdaKind::LoopReturnHandler;
    let handled = push_lambda(vec![Parameter::new(unit)], handled, kind, &location, context);

    // `| early_return x -> Some x`
    let returned = Arc::new("returned".to_string());
    let parameter = context.push_name(returned.clone(), location.clone());
    let parameter = context.push_pattern(Pattern::Variable(parameter), location.clone());
    let resume_name = context.push_name(Arc::new("resume".to_string()), location.clone());
    let resume = context.push_pattern(Pattern::Variable(resume_name), location.clone());

    let some = std_variable(&["Std", "Prelude", "Maybe", "Some"], &location, context);
    let argument = push_variable(Path::ident(returned.clone(), location.clone()), &location, context);
    let branch = Expr::Call(cst::Call { function: some, arguments: vec![Argument::explicit(argument)] });
    let branch = context.push_expr(branch, location.clone());
    let parameters = vec![Parameter::new(parameter), Parameter::new(resume)];
    let branch = push_lambda(parameters, branch, cst::LambdaKind::Function, &location, context);

    let early_return = std_path(&["Std", "EarlyReturn", "early_return"], &location, context);
    let pattern = cst::HandlePattern { function: early_return, args: vec![parameter], resume_name };
    let handler_name = context.push_name(Arc::new("$handler".to_string()), location.clone());
    let handle = Expr::Handle(cst::Handle { handler_name, expression: handled, cases: vec![(pattern, branch)] });
    let handle = context.push_expr(handle, location.clone());

    // `| Some x -> return x`
    let name = context.push_name(returned.clone(), location.clone());
    let name = context.push_pattern(Pattern::Variable(name), location.clone());
    let some = std_path(&["Std", "Prelude", "Maybe", "Some"], &location, context);
    let some = context.push_pattern(Pattern::Constructor(some, vec![name]), location.clone());
    let expression = push_variable(Path::ident(returned, location.clone()), &location, context);
    let return_ = context.push_expr(Expr::Return(cst::Return { expression }), location.clone());

    // `| None -> ()`
    let none = std_path(&["Std", "Prelude", "Maybe", "None"], &location, context);
    let none = context.push_pattern(Pattern::Constructor(none, Vec::new()), location.clone());
    let unit = context.push_expr(Expr::Literal(Literal::Unit), location);

    let cases = vec![(some, return_), (none, unit)];
    context.set_expr(expr, Expr::Match(cst::Match { expression: handle, cases }));
}

fn std_path(components: &[&str], location: &Location, context: &mut DesugarContext) -> PathId {
    let components = mapvec(components, |name| (Arc::new(name.to_string()), location.clone()));
    context.push_path(Path { components }, location.clone())
}

fn std_variable(components: &[&str], location: &Location, context: &mut DesugarContext) -> ExprId {
    let path = std_path(components, location, context);
    context.push_expr(Expr::Variable(path), location.clone())
}

fn push_variable(path: Path, location: &Location, context: &mut DesugarContext) -> ExprId {
    let path = context.push_path(path, location.clone());
    context.push_expr(Expr::Variable(path), location.clone())
}

fn push_lambda(
    parameters: Vec<Parameter>, body: ExprId, kind: cst::LambdaKind, location: &Location, context: &mut DesugarContext,
) -> ExprId {
    let lambda = cst::Lambda { kind, ..cst::Lambda::new(parameters, body) };
    context.push_expr(Expr::Lambda(lambda), location.clone())
}

/// True if `expr` (the condition or body of a loop) contains a `return`.
///
/// Lambdas are not searched since a `return` in a lambda returns from that lambda. This includes
/// the body and branches of a `handle`.
fn contains_return(expr: ExprId, context: &DesugarContext) -> bool {
    match &context[expr] {
        Expr::Return(_) => true,
        Expr::Lambda(_) | Expr::Handle(_) => false,
        other => other.children().into_iter().any(|child| contains_return(child, context)),
    }
}

/// `break` and `continue` desugar to calls to the `Std.Stream.Loop` operations of the same name.
/// These are handled by each loop.
fn desugar_loop_jump(expr: ExprId, context: &mut DesugarContext) {
    let operation = match &context[expr] {
        Expr::Break => "break",
        Expr::Continue => "continue",
        _ => unreachable!(),
    };
    let location = context.expr_location(expr).clone();
    let call = loop_jump_call(operation, &location, context);
    context.set_expr(expr, call);
}

/// `Std.Stream.(operation) ()`
fn loop_jump_call(operation: &str, location: &Location, context: &mut DesugarContext) -> Expr {
    let function = std_variable(&["Std", "Stream", operation], location, context);
    let unit = context.push_expr(Expr::Literal(Literal::Unit), location.clone());
    Expr::Call(cst::Call { function, arguments: vec![Argument::explicit(unit)] })
}

/// Converts a pattern into an equivalent expression
fn pattern_to_expr(pattern: PatternId, context: &mut DesugarContext) -> ExprId {
    let location = context.pattern_location(pattern).clone();

    match context[pattern].clone() {
        cst::Pattern::Variable(name)
        | cst::Pattern::Alias(name, _)
        | cst::Pattern::ConstructorRest(_, _, Some(name)) => {
            let path = cst::Path::ident(context[name].clone(), location.clone());
            let path = context.push_path(path, location.clone());
            context.push_expr(cst::Expr::Variable(path), location)
        },
        cst::Pattern::TypeAnnotation(inner, _) => pattern_to_expr(inner, context),
        cst::Pattern::Literal(literal) => context.push_expr(cst::Expr::Literal(literal), location),
        cst::Pattern::Constructor(path, args) => {
            let function = context.push_expr(cst::Expr::Variable(path), location.clone());
            if args.is_empty() {
                return function;
            }
            let arguments = args.into_iter().map(|arg| Argument::explicit(pattern_to_expr(arg, context))).collect();
            context.push_expr(cst::Expr::Call(cst::Call { function, arguments }), location)
        },
        cst::Pattern::Or(_) | cst::Pattern::Error | cst::Pattern::ConstructorRest(_, _, None) => {
            context.push_expr(cst::Expr::Error, location)
        },
    }
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
            let name = cst::Name::new(format!("_{}", counter));
            counter += 1;

            let name_id = context.push_name(name.clone(), location.clone());

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

    context.set_expr(expr, Expr::Lambda(Lambda::new(parameters, new_call_id)));
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

    /// `a with b` desugars to `b (fn {h} -> a)`
    TildeArrow {
        call: ExprId,
    },

    /// `a; b` desugars to the sequence of `a` then `b`
    Semicolon {
        call: ExprId,
    },

    /// `do body` desugars to `fn {arg} -> <body>`
    Do(ExprId),

    /// `U8 x` desugars to `cast x : U8`
    TypeCast {
        call: ExprId,
        target_type: Type,
    },

    Loop(ExprId),

    /// `for p in e do body` desugars to `(for) e (fn p -> body)`
    For(ExprId),

    /// `while c do body` desugars to `(while) (fn () -> c) (fn () -> body)`
    While(ExprId),

    /// `break` desugars to `Std.Stream.(break) ()`, and `continue` to `Std.Stream.(continue) ()`
    LoopJump(ExprId),

    /// `"a${x}b${y}c"` desugars to `"a" ++ cast x : String ++ "b" ++ cast y : String ++ "c"`
    StringInterpolation(ExprId),

    /// `if <and-chain containing is> then T else E`: rewrite the whole If into
    /// a nested match so each `is`'s pattern bindings are in scope for any
    /// subsequent chain elements and the then branch.
    IfWithIs(ExprId),

    /// `while <and-chain containing is> do B`: rewrite to `while true do` over
    /// a nested match which runs `B` with each `is`'s bindings in scope, or breaks.
    WhileWithIs(ExprId),

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
        ";" => Some(ExprDesugar::Semicolon { call: expr }),
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
            ExprDesugar::Semicolon { call } => desugar_semicolon(call, context),
            ExprDesugar::Do(expr) => desugar_do(expr, context),
            ExprDesugar::TypeCast { call, target_type } => desugar_type_cast(call, target_type, context),
            ExprDesugar::Loop(expr) => desugar_loop(expr, context),
            ExprDesugar::For(expr) => desugar_for(expr, context),
            ExprDesugar::While(expr) => desugar_while(expr, context),
            ExprDesugar::LoopJump(expr) => desugar_loop_jump(expr, context),
            ExprDesugar::StringInterpolation(expr) => desugar_string_interpolation(expr, context),
            ExprDesugar::IfWithIs(expr) => desugar_if_with_is(expr, context),
            ExprDesugar::WhileWithIs(expr) => desugar_while_with_is(expr, context),
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
            (Arc::new("Std".to_string()), location.clone()),
            (Arc::new("Prelude".to_string()), location.clone()),
            (Arc::new("Cast".to_string()), location.clone()),
            (Arc::new("cast".to_string()), location.clone()),
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

    let string_path = std_path(&["Std", "Prelude", "String"], &location, context);
    let string_type = Type::new(TypeKind::Named(string_path), location.clone());

    // 1: create all the strings, 2: filter any known-empty strings, 3: append them together
    let mut strings = Vec::new();
    if !interpolated.fragments[0].is_empty() {
        let first_fragment = Expr::Literal(Literal::String(interpolated.fragments[0].clone()));
        strings.push(context.push_expr(first_fragment, location.clone()));
    }

    for (expr, fragment) in interpolated.exprs.iter().zip(interpolated.fragments.iter().skip(1)) {
        let cast_var = std_variable(&["Std", "Prelude", "Cast", "cast"], &location, context);

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
    let append_var = push_variable(Path::ident(Arc::new("++".to_string()), location.clone()), location, context);
    let call = Expr::Call(cst::Call {
        function: append_var,
        arguments: vec![Argument::explicit(lhs), Argument::explicit(rhs)],
    });
    context.push_expr(call, location.clone())
}

/// Desugars `x |> foo a b` into `foo x a b` and `foo a b <| x` into `foo x a b`.
/// A variable callee, as in `x |> f`, desugars to `f x`.
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

    // Prepend value as the first argument: foo b c => foo(value, b, c)
    let new_call = match &context[f] {
        Expr::Call(inner_call) => {
            let new_args = std::iter::once(x).chain(inner_call.arguments.iter().copied()).collect();
            cst::Call { function: inner_call.function, arguments: new_args }
        },
        // `x |> f` becomes `f x` so `f` is a callee with a first argument it can be resolved by
        Expr::Variable(_) => cst::Call { function: f, arguments: vec![x] },
        _ => return,
    };
    context.set_expr(expr, Expr::Call(new_call));
}

/// Desugars `a; b` into a sequence so that `b` stays in tail position. Left as a call to `;`,
/// `b` would be an argument instead and a trailing `resume` would no longer be a tail call.
fn desugar_semicolon(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Call(call) = &context[expr] else { unreachable!() };
    let items = mapvec(&call.arguments, |argument| SequenceItem { expr: argument.expr, comments: Vec::new() });
    context.set_expr(expr, Expr::Sequence(items));
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

/// Desugars `do <body>` into `fn _ -> <body>`.
fn desugar_do(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Do(do_) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();

    let name = context.push_name(Arc::new("_".to_string()), location.clone());
    let param = context.push_pattern(Pattern::Variable(name), location);

    let lambda = Expr::Lambda(cst::Lambda::new(vec![Parameter::new(param)], do_.body));
    context.set_expr(expr, lambda);
}

/// Desugars `a ~> b` into `b (fn _ -> a)`
///
/// If `b` is itself a call, `a` is prepended to its arguments directly:
/// `a ~> b c d` desugars to `b (fn _ -> a) c d`
fn desugar_tilde_arrow(expr: ExprId, context: &mut DesugarContext) {
    let Expr::Call(call) = &context[expr] else { unreachable!() };

    let a = call.arguments[0].expr;
    let b = call.arguments[1].expr;
    let location = context.expr_location(expr).clone();

    let variable_name = Arc::new("_".to_string());
    let variable_name = context.push_name(variable_name, location.clone());
    let pattern = context.push_pattern(Pattern::Variable(variable_name), location.clone());
    let lambda = Expr::Lambda(cst::Lambda::new(vec![Parameter::new(pattern)], a));
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
/// Since a chain of several elements can fail in several places, each failure is an
/// [Expr::Fallthrough] to the single `else_expr` of an [Expr::Chain].
fn compile_chain(parts: &[ChainElement], then_expr: ExprId, else_expr: ExprId, context: &mut DesugarContext) -> ExprId {
    if parts.len() <= 1 {
        return compile_chain_rest(parts, then_expr, else_expr, false, context);
    }
    let location = context.expr_location(else_expr).clone();
    let body = compile_chain_rest(parts, then_expr, else_expr, true, context);
    context.push_expr(Expr::Chain(cst::Chain { body, else_: else_expr }), location)
}

/// If `fallthrough` is true, each failing element falls through to the chain's else rather than
/// evaluating `else_expr` itself.
fn compile_chain_rest(
    parts: &[ChainElement], then_expr: ExprId, else_expr: ExprId, fallthrough: bool, context: &mut DesugarContext,
) -> ExprId {
    let Some((head, tail)) = parts.split_first() else { return then_expr };
    let inner = compile_chain_rest(tail, then_expr, else_expr, fallthrough, context);
    let else_expr = if fallthrough {
        let location = context.expr_location(else_expr).clone();
        context.push_expr(Expr::Fallthrough, location)
    } else {
        else_expr
    };
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
    let (then_branch, else_branch) = match if_.else_ {
        Some(else_) => (if_.then, else_),
        // We desugar `if _ is _ then` to a match, so if there's no else we need to make the `then`
        // branch end with a unit literal so that it returns Unit like other ifs without elses.
        None => {
            let then_unit = context.push_expr(Expr::Literal(Literal::Unit), location.clone());
            let items = [if_.then, then_unit].map(|expr| SequenceItem { expr, comments: Vec::new() });
            let then_branch = context.push_expr(Expr::Sequence(items.to_vec()), location.clone());
            (then_branch, context.push_expr(Expr::Literal(Literal::Unit), location))
        },
    };
    let parts = flatten_and_chain(if_.condition, context);
    let compiled = compile_chain(&parts, then_branch, else_branch, context);
    context.set_expr(expr, context[compiled].clone());
}

fn desugar_while_with_is(expr: ExprId, context: &mut DesugarContext) {
    let Expr::While(while_) = context[expr].clone() else { unreachable!() };
    let location = context.expr_location(expr).clone();
    let parts = flatten_and_chain(while_.condition, context);
    // `break` is `Never`, so the loop body is still checked against Unit
    let break_ = loop_jump_call("break", &location, context);
    let break_ = context.push_expr(break_, location.clone());
    let body = compile_chain(&parts, while_.body, break_, context);
    let condition = context.push_expr(Expr::Literal(Literal::Bool(true)), location);
    context.set_expr(expr, Expr::While(cst::While { condition, body }));
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
