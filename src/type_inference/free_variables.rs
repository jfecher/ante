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
        errors::TypeErrorKind,
        types::{PrimitiveType, Type},
    },
};

impl TypeChecker<'_, '_> {
    /// Finds the environment type for the given lambda. This involves finding the free variables
    /// within the lambda. This will unify the given `expected_environment_type` with the actual
    /// environment type found but will not actually perform closure conversion. Closure conversion
    /// is instead done while building the initial [crate::mir::Mir].
    pub(super) fn check_for_closure(
        &mut self, id: ExprId, expected_environment_type: &Type, self_name: Option<NameId>, is_move: bool,
    ) {
        let mut context = FreeVars::default();
        if let Some(name) = self_name {
            context.defined_in_fn.insert(name);
        }
        context.find_free_variables(id, self);

        let env_type = make_env_type_with_names(&context.free_vars, self, is_move);
        self.unify(&env_type, expected_environment_type, TypeErrorKind::ClosureEnv, id);

        if !context.free_vars.is_empty() {
            self.current_extended_context_mut().insert_closure_environment(id, context.free_vars);
            if is_move {
                self.current_extended_context_mut().mark_move_closure(id);
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
            cst::Expr::Handle(handle) => {
                self.find_free_variables(handle.expression, checker);
                for (pattern, branch) in handle.cases.iter() {
                    for argument in pattern.args.iter() {
                        self.declare_pattern(*argument, checker);
                    }
                    // The synthetic `resume` binding is introduced for each
                    // branch; declare it so it isn't counted as a free variable.
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
        // Regular closures capture mutable variables by reference (as pointers in MIR).
        // Wrap their type in a `mut` reference so the environment type matches.
        // Immutable variables are captured by value — this is safe for escaping closures
        // and observationally equivalent to by-reference since the value can't change.
        // `move` closures capture all variables by value — no wrapping needed.
        if !is_move && checker.mutable_definitions.contains(name) {
            Type::Application(Arc::new(Type::MUT), Arc::new(vec![typ]))
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
