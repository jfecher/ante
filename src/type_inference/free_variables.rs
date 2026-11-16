use std::collections::BTreeSet;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst,
        ids::{ExprId, NameId, PathId, PatternId},
    },
    shared_arc::Shared,
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
    /// is instead done while building the initial `Mir`.
    ///
    /// `pending` holds implicits that can only be found once this environment is decided, such as
    /// an impl for the lambda itself, along with the local implicits visible to each. Since an
    /// implicit requested within the lambda may resolve to any of those locals, the lambda captures
    /// them all.
    pub(super) fn check_for_closure(
        &mut self, id: ExprId, expected_environment_type: &Type, self_name: Option<NameId>, is_move: bool,
        pending: &[PendingImplicit],
    ) {
        let mut context = self.lambda_free_vars_context(id);
        if let Some(name) = self_name {
            context.defined_in_fn.insert(name);
        }
        context.find_free_variables(id, self);

        for implicit in pending {
            if context.exprs.contains(&implicit.source) {
                for name in &implicit.implicits_in_scope {
                    context.check_name(*name);
                }
            }
        }

        let is_handle_lambda = self.item.handle_lambdas.contains(&id);
        let moved = if is_move || is_handle_lambda || context.free_vars.is_empty() || self.is_impl_method(id) {
            BTreeSet::new()
        } else {
            self.moved_captures(id, &context.free_vars)
        };

        self.capture_drop_impls(&mut context, is_move, &moved);

        // The body or a branch of a `handle` may need to capture more once it is borrow checked, so
        // we leave its environment to [Self::bind_handle_environment]
        if !is_handle_lambda {
            let env_type = self.environment_type(id, &context.free_vars, is_move, &moved);
            self.unify(&env_type, expected_environment_type, TypeErrorKind::ClosureEnv, id);
        }

        // Only a closure which owns some of its captures can move them. Whether it does is left
        // for the borrow checker to decide.
        if !is_move
            && moved.is_empty()
            && let Some(function) = self.expr_function_type(id)
        {
            self.bind_if_unbound(&function.once, Type::MANY);
        }

        if !context.free_vars.is_empty() {
            if is_move {
                self.current_extended_context_mut().mark_move_closure(id);
            }
            if !moved.is_empty() {
                self.current_extended_context_mut().insert_moved_captures(id, moved);
            }
            self.current_extended_context_mut().insert_closure_environment(id, context.free_vars);
        }
    }

    /// Returns the environment type of the lambda `id` capturing `free_vars`. A `move` closure
    /// captures everything by value. Other closures capture what their body may move by value,
    /// a `var` by mutable reference, and everything else by read-only reference.
    fn environment_type(
        &mut self, id: ExprId, free_vars: &BTreeSet<NameId>, is_move: bool, moved: &BTreeSet<NameId>,
    ) -> Type {
        let captures = mapvec(free_vars, |name| {
            let typ = self.name_types[name].clone();
            if is_move || moved.contains(name) {
                return typ;
            }
            let kind = match self.item.mutable_definitions.contains(name) {
                true => cst::ReferenceKind::Mut,
                false => cst::ReferenceKind::Ref,
            };
            let place = self.open_place(self.binding_place(*name));
            self.reference_type(kind, place, typ)
        });

        // The body and branches of a `handle` run in place so they are never copied or dropped
        if self.item.handle_lambdas.contains(&id) {
            return make_env_type(captures.into_iter());
        }
        self.closure_environment_type(captures)
    }

    /// Returns the environment type of a closure capturing values of the types `captures`. This is
    /// either the only capture or a pair of the first capture and the environment of the rest. This
    /// way a closure is `Copy` if its environment has a `Copy` impl, and is dropped by its
    /// environment's `Drop` impl.
    fn closure_environment_type(&self, captures: Vec<Type>) -> Type {
        let mut captures = captures.into_iter().rev();
        let Some(last) = captures.next() else { return Type::NO_CLOSURE_ENV };
        let pair = Shared::new(Type::UserDefined(Origin::TopLevelDefinition(self.pair_type())));
        captures.fold(last, |rest, capture| Type::Application(pair.clone(), Shared::new(vec![capture, rest])))
    }

    /// Capture each local `drop` refers to in the lambda `id`, which is the lambda, `handle` body,
    /// or `handle` branch that `drop` was inserted into
    pub(super) fn capture_for_drop(&mut self, id: ExprId, drop: ExprId) {
        let mut used = FreeVars::default();
        used.find_free_variables(drop, self);
        let context = self.current_extended_context();
        let mut free_vars = context.get_closure_environment(id).cloned().unwrap_or_default();
        // Temporaries read by a drop are bound where the drop runs, inside the lambda
        free_vars.extend(used.free_vars.into_iter().filter(|name| !context.drops().names_temporary(*name)));
        self.current_extended_context_mut().insert_closure_environment(id, free_vars);
    }

    /// Decide the environment of the lambda `id` (or `handle` body or branch) now that everything
    /// it captures is known
    pub(super) fn bind_handle_environment(&mut self, id: ExprId) {
        let context = self.current_extended_context();
        let free_vars = context.get_closure_environment(id).cloned().unwrap_or_default();
        let is_move = context.is_move_closure(id);
        let Some(Type::Function(function)) = self.expr_types.get(&id).cloned() else { return };
        let env_type = self.environment_type(id, &free_vars, is_move, &BTreeSet::new());
        self.unify(&env_type, &function.environment, TypeErrorKind::ClosureEnv, id);
    }

    /// True if `typ` is the type of a number literal that hasn't been resolved to a number type yet
    pub(super) fn is_unresolved_literal(&self, typ: &Type) -> bool {
        match self.follow_type(typ) {
            Type::Variable(id) => self.is_integer_type_variable(*id) || self.is_float_type_variable(*id),
            _ => false,
        }
    }

    /// True if `lambda` is a method of the impl being checked. These read the impl's parameters
    /// from its data rather than capturing them.
    pub(super) fn is_impl_method(&self, lambda: ExprId) -> bool {
        self.item.impl_.as_ref().is_some_and(|impl_| impl_.methods.contains(&lambda))
    }

    /// An empty [FreeVars] context for finding the free variables of the lambda `id`. An impl's
    /// method reads the impl's parameters from the impl's data instead of capturing them.
    pub(super) fn lambda_free_vars_context(&self, id: ExprId) -> FreeVars {
        let mut context = FreeVars::default();
        if self.is_impl_method(id) {
            for parameter in self.current_impl_parameters() {
                context.declare_pattern(parameter.pattern, self);
            }
        }
        context
    }

    /// True if the given branch of a Handle expression references its `resume` variable.
    pub(super) fn handler_branch_uses_resume(&self, resume_name: NameId, branch: ExprId) -> bool {
        let cst::Expr::Lambda(lambda) = &self.current_extended_context()[branch] else { unreachable!() };
        let mut context = FreeVars::default();
        context.find_free_variables(lambda.body, self);
        context.free_vars.contains(&resume_name)
    }
}

/// An implicit that can't be found until a lambda's environment is decided. See
/// [TypeChecker::check_for_closure]
pub(super) struct PendingImplicit {
    /// The expression requesting the implicit
    pub(super) source: ExprId,
    /// The local implicits in scope where it is requested
    pub(super) implicits_in_scope: Vec<NameId>,
}

#[derive(Default)]
pub(super) struct FreeVars {
    /// The free variables found
    pub(super) free_vars: BTreeSet<NameId>,

    /// The first expression each free variable is used in
    pub(super) use_sites: FxHashMap<NameId, ExprId>,

    // We don't care about different scopes within the function
    pub(super) defined_in_fn: FxHashSet<NameId>,

    /// Every expression searched
    pub(super) exprs: Vec<ExprId>,
}

impl FreeVars {
    pub(super) fn find_free_variables(&mut self, expr: ExprId, checker: &TypeChecker) {
        self.exprs.push(expr);
        match &checker.current_extended_context()[expr] {
            cst::Expr::Error => (),
            cst::Expr::Literal(_) => (),
            cst::Expr::Extern(_) => (),
            cst::Expr::Variable(path) => self.find_free_variable(*path, expr, checker),
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
            cst::Expr::Chain(chain) => {
                self.find_free_variables(chain.body, checker);
                self.find_free_variables(chain.else_, checker);
            },
            cst::Expr::Fallthrough => (),
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
            cst::Pattern::ConstructorRest(_, args, name) => {
                for arg in args {
                    self.declare_pattern(*arg, checker);
                }
                if let Some(name) = name {
                    self.defined_in_fn.insert(*name);
                }
            },
            cst::Pattern::TypeAnnotation(pattern, _) => self.declare_pattern(*pattern, checker),
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

    fn find_free_variable(&mut self, path: PathId, expr: ExprId, checker: &TypeChecker) {
        if let Some(Origin::Local(name)) = checker.path_origin(path)
            && self.check_name(name)
        {
            self.use_sites.entry(name).or_insert(expr);
        }
    }

    /// Returns true if `name` is free
    pub(super) fn check_name(&mut self, name: NameId) -> bool {
        let free = !self.defined_in_fn.contains(&name);
        if free {
            self.free_vars.insert(name);
        }
        free
    }
}

fn make_env_type(free_vars: impl ExactSizeIterator<Item = Type>) -> Type {
    if free_vars.len() == 0 {
        return Type::Primitive(PrimitiveType::NoClosureEnv);
    }
    Type::Tuple(Shared::new(free_vars.collect()))
}
