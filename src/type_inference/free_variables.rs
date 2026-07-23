use std::{collections::BTreeSet, sync::Arc};

use rustc_hash::FxHashSet;

use crate::{
    name_resolution::Origin,
    parser::{
        cst,
        ids::{ExprId, NameId, PathId, PatternId},
    },
    type_inference::{
        TypeChecker,
        errors::{Locateable, TypeErrorKind},
        types::{PrimitiveType, Type, TypeBindings},
    },
};

/// True if `typ` is a pointer type
pub(super) fn is_pointer(typ: &Type, bindings: &TypeBindings) -> bool {
    match typ.follow(bindings) {
        Type::Primitive(PrimitiveType::Pointer) => true,
        Type::Application(constructor, _) => {
            matches!(constructor.follow(bindings), Type::Primitive(PrimitiveType::Pointer))
        },
        _ => false,
    }
}

impl TypeChecker<'_, '_> {
    /// Finds the environment type for the given lambda. This involves finding the free variables
    /// within the lambda. This will unify the given `expected_environment_type` with the actual
    /// environment type found but will not actually perform closure conversion. Closure conversion
    /// is instead done while building the initial `Mir`.
    pub(super) fn check_for_closure(
        &mut self, id: ExprId, expected_environment_type: &Type, self_name: Option<NameId>, is_move: bool,
    ) {
        let mut context = FreeVars::default();
        if let Some(name) = self_name {
            context.defined_in_fn.insert(name);
        }
        context.find_free_variables(id, self);

        if !is_pointer(expected_environment_type, &self.bindings) {
            let env_type = make_env_type_with_names(&context.free_vars, self, is_move);
            self.unify(&env_type, expected_environment_type, TypeErrorKind::ClosureEnv, id);
        }

        if !context.free_vars.is_empty() {
            if is_move {
                self.current_extended_context_mut().mark_move_closure(id);
            }
            self.current_extended_context_mut().insert_closure_environment(id, context.free_vars);
        }
    }

    pub(super) fn record_move_captures(&mut self, id: ExprId, self_name: Option<NameId>) {
        let mut context = FreeVars::default();
        if let Some(name) = self_name {
            context.defined_in_fn.insert(name);
        }
        context.find_free_variables(id, self);

        let location = id.locate(self);
        for name in &context.free_vars {
            let typ = self.name_types[name].clone();
            if !self.type_is_copy(&typ) {
                // Capturing a binding moves the place it denotes, same as a direct use.
                let move_path = self.binding_place(*name);
                self.move_tracker.record_move(move_path, location.clone());
            }
        }
    }

    /// True if the given branch of a Handle expression references its `resume` variable.
    pub(super) fn handler_branch_uses_resume(&self, resume_name: NameId, branch: ExprId) -> bool {
        let cst::Expr::Lambda(lambda) = &self.current_extended_context()[branch] else { unreachable!() };
        let mut context = FreeVars::default();
        context.find_free_variables(lambda.body, self);
        context.free_vars.contains(&resume_name)
    }
}

#[derive(Default)]
struct FreeVars {
    /// The free variables found
    free_vars: BTreeSet<NameId>,

    // We don't care about different scopes within the function
    defined_in_fn: FxHashSet<NameId>,
}

impl FreeVars {
    fn find_free_variables(&mut self, expr: ExprId, checker: &TypeChecker) {
        match &checker.current_extended_context()[expr] {
            cst::Expr::Error => (),
            cst::Expr::Literal(_) => (),
            cst::Expr::Extern(_) => (),
            cst::Expr::Variable(path) => self.find_free_variable(*path, checker),
            cst::Expr::Sequence(items) => {
                for item in items {
                    self.find_free_variables(item.expr, checker);
                }
            },
            cst::Expr::Definition(definition) => {
                self.declare_pattern(definition.pattern, checker);
                self.find_free_variables(definition.rhs, checker);
            },
            cst::Expr::MemberAccess(access) => self.find_free_variables(access.object, checker),
            cst::Expr::Call(call) => {
                self.find_free_variables(call.function, checker);
                for argument in call.arguments.iter() {
                    self.find_free_variables(argument.expr, checker);
                }
            },
            cst::Expr::Lambda(lambda) => {
                for parameter in lambda.parameters.iter() {
                    self.declare_pattern(parameter.pattern, checker);
                }
                self.find_free_variables(lambda.body, checker);
            },
            cst::Expr::If(if_) => {
                self.find_free_variables(if_.condition, checker);
                self.find_free_variables(if_.then, checker);
                if let Some(else_) = if_.else_ {
                    self.find_free_variables(else_, checker);
                }
            },
            cst::Expr::Match(match_) => {
                self.find_free_variables(match_.expression, checker);
                for (pattern, branch) in match_.cases.iter() {
                    self.declare_pattern(*pattern, checker);
                    self.find_free_variables(*branch, checker);
                }
            },
            cst::Expr::Is(_) => unreachable!("Expr::Is should be desugared during GetItem"),
            cst::Expr::Do(_) => unreachable!("Expr::Do should be desugared during GetItem"),
            cst::Expr::Handle(handle) => {
                // Declared by the `handle` itself so refs inside the body lambda aren't reported as captures.
                self.defined_in_fn.insert(handle.handler_name);
                self.find_free_variables(handle.expression, checker);
                for (pattern, branch) in handle.cases.iter() {
                    for argument in pattern.args.iter() {
                        self.declare_pattern(*argument, checker);
                    }
                    // Declare resume so it isn't counted as free.
                    self.defined_in_fn.insert(pattern.resume_name);
                    self.find_free_variables(*branch, checker);
                }
            },
            cst::Expr::Reference(reference) => self.find_free_variables(reference.rhs, checker),
            cst::Expr::TypeAnnotation(annotation) => self.find_free_variables(annotation.lhs, checker),
            cst::Expr::Constructor(constructor) => {
                for (_name, expr) in constructor.fields.iter() {
                    self.find_free_variables(*expr, checker);
                }
            },
            cst::Expr::Loop(_) => unreachable!("Loops should be desugared before finding free variables"),
            cst::Expr::While(w) => {
                self.find_free_variables(w.condition, checker);
                self.find_free_variables(w.body, checker);
            },
            cst::Expr::For(fo) => {
                self.find_free_variables(fo.start, checker);
                self.find_free_variables(fo.end, checker);
                self.defined_in_fn.insert(fo.variable);
                self.find_free_variables(fo.body, checker);
            },
            cst::Expr::Break | cst::Expr::Continue => (),
            cst::Expr::Quoted(_) => (),
            cst::Expr::Return(return_) => self.find_free_variables(return_.expression, checker),
            cst::Expr::Assignment(assignment) => {
                self.find_free_variables(assignment.lhs, checker);
                self.find_free_variables(assignment.rhs, checker);
                if let Some((_, op_expr)) = assignment.op {
                    self.find_free_variables(op_expr, checker);
                }
            },
            cst::Expr::InterpolatedString(_) => {
                unreachable!("InterpolatedString should be desugared before finding free variables")
            },
            cst::Expr::ArrayLiteral(elements) => {
                for element in elements.clone() {
                    self.find_free_variables(element, checker);
                }
            },
        }
    }

    /// Inserts any [NameId]s of values within this [PatternId] into `self.defined_in_fn`
    fn declare_pattern(&mut self, pattern: PatternId, checker: &TypeChecker) {
        match &checker.current_extended_context()[pattern] {
            cst::Pattern::Error => (),
            cst::Pattern::Variable(name) => {
                self.defined_in_fn.insert(*name);
            },
            cst::Pattern::Literal(_) => (),
            cst::Pattern::Constructor(_, fields) => {
                for field in fields {
                    self.declare_pattern(*field, checker);
                }
            },
            cst::Pattern::TypeAnnotation(pattern, _) => self.declare_pattern(*pattern, checker),
            cst::Pattern::MethodName { type_name: _, item_name } => {
                self.defined_in_fn.insert(*item_name);
            },
            cst::Pattern::Or(alts) => {
                // Each alt binds the same names, so we only need to walk the first.
                if let Some(alt) = alts.first() {
                    self.declare_pattern(*alt, checker);
                }
            },
            cst::Pattern::Alias(name, inner) => {
                self.defined_in_fn.insert(*name);
                self.declare_pattern(*inner, checker);
            },
        }
    }

    fn find_free_variable(&mut self, path: PathId, checker: &TypeChecker) {
        if let Some(Origin::Local(name)) = checker.path_origin(path) {
            self.check_name(name);
        }
    }

    fn check_name(&mut self, name: NameId) {
        if !self.defined_in_fn.contains(&name) {
            self.free_vars.insert(name);
        }
    }
}

fn make_env_type_with_names(free_vars: &BTreeSet<NameId>, checker: &TypeChecker, is_move: bool) -> Type {
    let free_vars = free_vars.iter().map(|name| {
        let typ = checker.name_types[name].clone();

        // Closures:
        // - Capture mutable variables by reference (so we wrap in a Mut ref here)
        // - Capture immutable variables by value (FIXME)
        // - Capture everything by move if it is a `move` closure
        if !is_move && checker.mutable_definitions.contains(name) {
            let lifetime = checker.next_type_variable();
            Type::Application(Arc::new(Type::MUT), Arc::new(vec![lifetime, typ]))
        } else {
            typ
        }
    });
    make_env_type(free_vars)
}

fn make_env_type(free_vars: impl ExactSizeIterator<Item = Type>) -> Type {
    if free_vars.len() == 0 {
        return Type::Primitive(PrimitiveType::NoClosureEnv);
    }
    Type::Tuple(Arc::new(free_vars.collect()))
}
