use std::sync::Arc;

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::VisibleImplicits,
    iterator_extensions::mapvec,
    name_resolution::Origin,
    parser::{
        cst::{self, Name, Pattern},
        ids::{ExprId, PatternId},
    },
    type_inference::{
        Locateable, TypeChecker,
        types::{FunctionType, ParameterType, Type},
    },
};

impl<'local, 'inner> TypeChecker<'local, 'inner> {
    /// Perform an implicit parameter coercion.
    ///
    /// Given a function `expr` which requires some implicit parameters present in the `actual`
    /// type but not the `expected` type, find values for those implicits (issuing errors for any
    /// that cannot be found) an create a new wrapper function. E.g:
    ///
    /// ```ante
    /// fn a c -> <expr> a <new-implicit> c
    /// ```
    /// where `<new-implicit>` is a new implicit that was successfully found. In the case a
    /// matching implicit value cannot be found, an error is issued and an error expression is
    /// slotted in as the argument instead. In this way, this function will always return a new
    /// closure wrapper.
    pub(super) fn implicit_parameter_coercion(
        &mut self, actual: Arc<FunctionType>, expected: Arc<FunctionType>, function: ExprId,
    ) -> Option<cst::Expr> {
        // Looking for implicit parameters that are in `actual` but not `expected`.
        // The reverse would be a type error.
        let mut new_expected = Vec::new();

        let mut actual_params = actual.parameters.iter();
        let mut expected_params = expected.parameters.iter().cloned();
        let mut current_expected = expected_params.next();

        // For each parameter, this is either `None` if no new implicit was inserted
        // at that position, or it is `Some(expr_id)` of the new expression.
        let mut implicits_added = Vec::new();

        while let Some(actual) = actual_params.next() {
            match (actual.is_implicit, current_expected.as_ref()) {
                // actual is implicit, but expected isn't, search for an implicit in scope
                (true, expected) if expected.map_or(true, |param| !param.is_implicit) => {
                    let value = self.find_implicit_value(&actual.typ, new_expected.len(), function);
                    let value = value.unwrap_or_else(|| {
                        let location = function.locate(self);
                        self.push_expr(cst::Expr::Error, Type::ERROR, location)
                    });
                    implicits_added.push(Some(value));
                    new_expected.push(ParameterType::implicit(self.expr_types[&value].clone()));
                },
                _ => {
                    let expected = current_expected.unwrap_or(ParameterType::explicit(Type::ERROR));
                    new_expected.push(expected);
                    implicits_added.push(None);
                    current_expected = expected_params.next();
                },
            }
        }
        self.create_closure_wrapper_for_implicit(function, implicits_added, new_expected)
    }

    /// If the expression is a variable, return its name
    fn try_get_name(&self, expr: ExprId) -> Option<String> {
        match &self.current_extended_context()[expr] {
            cst::Expr::Variable(path) => Some(self.current_extended_context()[*path].last_ident().to_string()),
            _ => None,
        }
    }

    /// Search for an implicit value in scope with the given type, issuing an error if no implicit
    /// is found or if multiple matching implicits are found.
    fn find_implicit_value(&mut self, target_type: &Type, parameter_index: usize, function: ExprId) -> Option<ExprId> {
        // TODO: We shouldn't commit unification bindings until we actually select a candidate

        // A Vec of (implicit name, implicit origin, implicit type, implicit arguments)
        // Non-function implicits will not have any arguments
        let mut candidates = Candidates::new();

        // TODO: Remove clone by making try_unify no longer require a mutable self
        for scope in self.implicits_in_scope.clone() {
            for name in scope {
                let name_type = self.name_types[&name].follow(&self.bindings).clone();
                let origin = Origin::Local(name);
                let name = self.current_extended_context()[name].clone();
                self.check_implicit_candidate(
                    name_type,
                    target_type,
                    name,
                    origin,
                    &mut candidates,
                    parameter_index,
                    function,
                );
            }
        }

        // Need to check globally visible implicits separately
        // TODO: Make this more efficient so we don't need to go through every single implicit
        if let Some(item) = self.current_item {
            for (name, name_id) in VisibleImplicits(item.source_file).get(self.compiler).iter() {
                let name_type = self.type_of_top_level_name(name_id);
                if self.try_unify(&name_type, target_type).is_ok() {
                    let origin = Origin::TopLevelDefinition(*name_id);
                    self.check_implicit_candidate(
                        name_type,
                        target_type,
                        name.clone(),
                        origin,
                        &mut candidates,
                        parameter_index,
                        function,
                    );
                }
            }
        }

        let (name, origin, name_type, arguments) = if candidates.is_empty() {
            self.issue_no_implicit_found_error(target_type, parameter_index, function);
            return None;
        } else if candidates.len() == 1 {
            candidates.first().unwrap().clone()
        } else {
            self.issue_multiple_matching_implicits_error(candidates, target_type, parameter_index, function);
            return None;
        };

        let location = function.locate(self);
        Some(self.create_implicit_argument_expr(name, origin, name_type, arguments, location))
    }

    /// Check if the given `implicit_type` matches the `target_type` directly, or if it can be
    /// called as a function to produce the target type. If either are true, push the candidate to
    /// the candidates list.
    fn check_implicit_candidate(
        &mut self, implicit_type: Type, target_type: &Type, name: Name, origin: Origin, candidates: &mut Candidates,
        parameter_index: usize, function: ExprId,
    ) {
        match self.implicit_type_matches(&implicit_type, target_type) {
            ImplicitMatch::NoMatch => (),
            ImplicitMatch::MatchedAsIs => {
                candidates.push((name, origin, implicit_type, Vec::new()));
            },
            ImplicitMatch::Call(function_type) => {
                // TODO: Make this algorithm iterative instead of recursive
                let mut arguments = Vec::new();
                for parameter in &function_type.parameters {
                    if parameter.is_implicit {
                        if let Some(argument) = self.find_implicit_value(&parameter.typ, parameter_index, function) {
                            arguments.push(cst::Argument::implicit(argument));
                        }
                    }
                }

                if arguments.len() == function_type.parameters.len() {
                    candidates.push((name, origin, implicit_type, arguments));
                }
            },
        }
    }

    /// Given the type of an implicit value, and the target type to search for, return whether the
    /// given implicit is a match for the target type, whether it can produce such a type by
    /// calling it as a function, or whether there is no match.
    fn implicit_type_matches(&mut self, implicit_type: &Type, target_type: &Type) -> ImplicitMatch {
        if self.try_unify(implicit_type, target_type).is_ok() {
            ImplicitMatch::MatchedAsIs
        } else if let Type::Function(f) = implicit_type {
            if self.try_unify(&f.return_type, target_type).is_ok() {
                ImplicitMatch::Call(f.clone())
            } else {
                ImplicitMatch::NoMatch
            }
        } else {
            ImplicitMatch::NoMatch
        }
    }

    // error: No implicit found for parameter N of type T
    fn issue_no_implicit_found_error(&self, implicit_type: &Type, parameter_index: usize, function: ExprId) {
        let type_string = self.type_to_string(&implicit_type);
        let function_name = self.try_get_name(function);
        let location = function.locate(self);
        self.compiler.accumulate(Diagnostic::NoImplicitFound { type_string, function_name, parameter_index, location });
    }

    // error: No implicit found for parameter N of type T
    fn issue_multiple_matching_implicits_error(
        &self, matching: Candidates, implicit_type: &Type, parameter_index: usize, function: ExprId,
    ) {
        let type_string = self.type_to_string(&implicit_type);
        let function_name = self.try_get_name(function);
        let location = function.locate(self);
        let matches = mapvec(matching, |(name, _, _, _)| name);
        self.compiler.accumulate(Diagnostic::MultipleImplicitsFound {
            matches,
            type_string,
            function_name,
            parameter_index,
            location,
        });
    }

    /// Try to add the given implicit into scope
    pub(super) fn add_implicit(&mut self, id: PatternId) {
        let name = match &self.current_extended_context()[id] {
            Pattern::Error => return,
            Pattern::Variable(name) => *name,
            Pattern::TypeAnnotation(inner_id, _) => return self.add_implicit(*inner_id),
            _ => {
                let location = id.locate(self);
                self.compiler.accumulate(Diagnostic::ImplicitNotAVariable { location });
                return;
            },
        };
        self.implicits_in_scope.last_mut().unwrap().push(name);
    }

    /// Given:
    /// - A function `f`
    /// - `implicits_added = [None, Some(i), None]` (e.g.)
    /// - `argument_types = [t, u, v]`
    ///
    /// Create:
    /// `fn (a: t) (c: v) -> f a {i} c`
    fn create_closure_wrapper_for_implicit(
        &mut self, function: ExprId, implicits_added: Vec<Option<ExprId>>, argument_types: Vec<ParameterType>,
    ) -> Option<cst::Expr> {
        // We should always have at least 1 added implicit parameter
        let implicit_added = implicits_added.iter().any(|param| param.is_some());

        // A type-error is expected when type checking this call
        if !implicit_added || implicits_added.len() != argument_types.len() {
            return None;
        }

        let mut parameters = Vec::new();
        let mut arguments = Vec::new();

        for (implicit, arg_type) in implicits_added.into_iter().zip(argument_types) {
            match implicit {
                // We want new implicit arguments to be in the call but not the lambda parameters
                Some(arg) => {
                    arguments.push(cst::Argument::implicit(arg));
                },
                None => {
                    let location = function.locate(self);
                    let (var_path, var_name) = self.fresh_variable("p", arg_type.typ.clone(), location.clone());

                    let pattern = self.push_pattern(cst::Pattern::Variable(var_name), location.clone());
                    let expr = self.push_expr(cst::Expr::Variable(var_path), arg_type.typ, location);

                    arguments.push(cst::Argument { is_implicit: arg_type.is_implicit, expr });
                    parameters.push(cst::Parameter { is_implicit: arg_type.is_implicit, pattern });
                },
            }
        }

        // Since `function` is the ExprId we'll be replacing, we can't use it directly here. We
        // have to copy it to a new id.
        let location = function.locate(self);
        let expr = self.current_extended_context()[function].clone();

        // This type should be overwritten later when cst_traversal traverses this new expr
        let function = self.push_expr(expr, Type::ERROR, location.clone());

        let body = cst::Expr::Call(cst::Call { function, arguments });
        let body_type = Type::ERROR;
        let body = self.push_expr(body, body_type, location);

        Some(cst::Expr::Lambda(cst::Lambda { parameters, body, return_type: None, effects: None }))
    }

    /// Creates a new expression referring to the given implicit value.
    /// - 0 arguments: The expression is a variable
    /// - 1+ arguments: The expression is a function call to the given name, using the given arguments.
    fn create_implicit_argument_expr(
        &mut self, name: Name, origin: Origin, name_type: Type, arguments: Vec<cst::Argument>, location: Location,
    ) -> ExprId {
        let name = name.as_ref().clone();
        let path = self.push_path(cst::Path::ident(name, location.clone()), name_type.clone(), location.clone());
        self.current_extended_context_mut().insert_path_origin(path, origin);
        let variable = cst::Expr::Variable(path);

        if arguments.is_empty() {
            self.push_expr(variable, name_type, location)
        } else {
            let return_type = name_type.return_type().unwrap().clone();
            let function = self.push_expr(variable, name_type, location.clone());
            let call = cst::Expr::Call(cst::Call { function, arguments });
            self.push_expr(call, return_type, location)
        }
    }
}

/// Candidates when searching for an implicit value.
/// Contains the name, origin, type for the implicit, along with any arguments to call it with (if
/// any) if we should call this implicit for its return value.
type Candidates = Vec<(Name, Origin, Type, Vec<cst::Argument>)>;

enum ImplicitMatch {
    NoMatch,
    MatchedAsIs,
    Call(Arc<FunctionType>),
}
