use std::{
    collections::{BTreeMap, VecDeque},
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
};

use crate::{
    cache::DefinitionInfoId,
    hir::{self, IntegerKind, Type},
    parser::ast,
    types::{self, effects::Effect, typed::Typed},
    util::fmap,
};

use super::{tuple, unit_literal, unwrap_variable, Context, Definition};

impl<'c> Context<'c> {
    /// A handle expression:
    /// ```pseudocode
    /// handle expr
    /// | effect1 arg1_1 .. arg1_M -> body1
    /// ...
    /// | effectN argN_1 .. argN_M -> bodyN
    /// | return x -> return_case
    /// ```
    /// lowers into:
    /// ```pseudocode
    /// start_expr(continuation) =
    ///     env_N = continuation_pop(continuation, typeof(env_N)) // (*) Note
    ///     ...
    ///     env_1 = continuation_pop(continuation, typeof(env_1))
    ///     result = expr // `continuation` is automatically added to any function calls within expr
    ///     continuation_push(continuation, result)
    ///     ()
    ///
    /// handler(continuation) =
    ///     if continuation_suspended(continuation) then
    ///         continuation_resume(continuation)
    ///         match continuation_pop(continuation, U32)
    ///         | Effect1_Hash ->
    ///             // Note that arguments are popped in reverse order
    ///             arg1_M = continuation_pop(continuation, typeof(arg1_1))
    ///             ...
    ///             arg1_1 = continuation_pop(continuation, typeof(arg1_M))
    ///             body1
    ///         ...
    ///         | EffectN_Hash ->
    ///             argN_M = continuation_pop(continuation, typeof(argN_1))
    ///             ...
    ///             argN_1 = continuation_pop(continuation, typeof(argN_M))
    ///             bodyN
    ///     else
    ///         x = continuation_pop(continuation, typeof(x))
    ///         return_case
    ///
    /// k = continuation_init(start_expr)
    /// continuation_push(continuation, &env_1) // (*) Note
    /// ...
    /// continuation_push(continuation, &env_N)
    /// ret = handler(k)
    /// continuation_free(k)
    /// ret
    /// ```
    /// where resume functions defined in the scope of each
    /// body make recursive calls back to `handler`.
    ///
    /// (*) Note: The additional `env` variables comes from the fact we are implicitly creating
    /// functions from `Handle` expressions and thus need to handle captured variables from the
    /// environment as we would with a closure's environment. Since the type signature expected by
    /// `continuation_init` prevents us from actually passing in the environment directly we need
    /// to push and pop them to the closure's channel.
    pub(super) fn monomorphise_handle(&mut self, handle: &ast::Handle<'c>) -> hir::Ast {
        // Push a local scope, we're going to redefine our captured environment variables
        // since we're defining an implicit closure and only want to refer to the new terms.
        self.definitions.push_local_scope();
        let free_vars = handle.find_free_variables(&self.cache);

        // This should always be a subset of `free_vars`
        let expression_free_vars = handle.expression.find_free_variables(&self.cache);

        // Redefine the captured environment
        self.redefine_captured_environment(&free_vars);

        let start_expr_fn =
            self.make_start_effect_expr_function(&handle.expression, &handle.effects_handled, &expression_free_vars);

        // We need to pop the local scope before `make_handle_env_pushes` so that that function can
        // refer to the captured variables to push them to the environment.
        self.definitions.pop_local_scope();

        // We need to redefine the captured variables again to make new ids which will be function
        // parameters of the handle function.
        self.definitions.push_local_scope();
        self.redefine_captured_environment(&free_vars);
        let (handler_fn, handler_type) = self.make_handler_function(handle, &free_vars);
        self.definitions.pop_local_scope();

        // create the final inline expression
        // k = continuation_init(start_expr)
        let init = hir::Ast::Builtin(hir::Builtin::ContinuationInit(Box::new(start_expr_fn)));
        let (k_def, k) = self.fresh_definition_with_variable(init, "k".into(), Type::continuation());
        let mut statements = vec![k_def];

        // continuation_push(continuation, arg_1)
        // ...
        // continuation_push(continuation, arg_N)
        self.make_handle_env_pushes(k.clone(), &expression_free_vars, &mut statements);

        // ret = handler(k, env1, ..., envN)
        let mut args = vec![k.clone()];
        for (free_var_id, free_var_type) in free_vars {
            let variable = match self.lookup_definition(free_var_id, &free_var_type).unwrap() {
                Definition::Macro(_) => unreachable!("Macro definitions should not be captured"),
                Definition::Normal(variable) => variable,
            };
            args.push(hir::Ast::Variable(variable));
        }

        if let Some(frame) = self.effect_continuations.last() {
            args.extend(frame.iter().map(|(_, k)| hir::Ast::Variable(k.clone())));
        }

        let ret_type = self.convert_type(handle.get_type().unwrap());
        let function = Box::new(handler_fn);

        let call_handler = hir::Ast::FunctionCall(hir::FunctionCall { function, args, function_type: handler_type });
        let (ret_def, ret) = self.fresh_definition_with_variable(call_handler, "ret".into(), ret_type);
        statements.push(ret_def);

        // continuation_free(k)
        let free_k = hir::Ast::Builtin(hir::Builtin::ContinuationFree(Box::new(k)));
        statements.push(free_k);
        statements.push(ret);

        hir::Ast::Sequence(hir::Sequence { statements })
    }

    fn redefine_captured_environment(&mut self, free_vars: &BTreeMap<DefinitionInfoId, types::Type>) {
        for (variable, typ) in free_vars {
            let typ = self.follow_all_bindings(typ);
            let fresh_id = self.next_unique_id();
            let name = self.cache.definition_infos[variable.0].name.clone();
            let converted_type = Rc::new(self.convert_type(&typ));
            let new_variable =
                hir::Variable { definition: None, definition_id: fresh_id, typ: converted_type, name: Some(name) };
            self.definitions.insert(*variable, typ.clone(), Definition::Normal(new_variable));
        }
    }

    /// Creates the `handler` function from a handler:
    /// ```pseudocode
    /// handle expr
    /// | effect1 arg1_1 .. arg1_M -> body1
    /// ...
    /// | effectN argN_1 .. argN_M -> bodyN
    /// | return x -> return_case
    /// ```
    /// lowers into:
    /// ```pseudocode
    /// handler(continuation) =
    ///     continuation_resume(continuation)
    ///     if continuation_suspended(continuation) then
    ///         match continuation_pop(continuation, U32)
    ///         | Effect1_Hash ->
    ///             // Note that arguments are popped in reverse order
    ///             arg1_M = continuation_pop(continuation, typeof(arg1_1))
    ///             ...
    ///             arg1_1 = continuation_pop(continuation, typeof(arg1_M))
    ///             envM = continuation_pop(continuation, typeof(env1))
    ///             ...
    ///             env1 = continuation_pop(continuation, typeof(envM))
    ///             body1
    ///         ...
    ///         | EffectN_Hash ->
    ///             argN_M = continuation_pop(continuation, typeof(argN_1))
    ///             ...
    ///             argN_1 = continuation_pop(continuation, typeof(argN_M))
    ///             envM = continuation_pop(continuation, typeof(env1))
    ///             ...
    ///             env1 = continuation_pop(continuation, typeof(envM))
    ///             bodyN
    ///     else
    ///         x = continuation_pop(continuation, typeof(x))
    ///         return_case
    /// ```
    fn make_handler_function(
        &mut self, handle: &ast::Handle<'c>, free_vars: &BTreeMap<DefinitionInfoId, types::Type>,
    ) -> (hir::Ast, hir::FunctionType) {
        let continuation_var = self.fresh_variable(Type::continuation());
        let continuation = hir::Ast::Variable(continuation_var.clone());

        let mut parameters = vec![Type::continuation()];
        for (_, free_var_type) in free_vars {
            parameters.push(self.convert_type(free_var_type));
        }

        // Redefine and push effect handlers since we can't use local variables from another function
        if let Some(frame) = self.effect_continuations.last().cloned() {
            let new_frame = fmap(frame, |(effect, _old_k)| {
                parameters.push(Type::continuation());

                let new_k = self.fresh_variable(Type::continuation());
                (effect.clone(), new_k)
            });
            self.effect_continuations.push(new_frame);
        }

        let result_type = self.convert_type(handle.get_type().unwrap());
        let return_type = Box::new(result_type.clone());
        let function_type = hir::FunctionType { parameters, return_type, is_varargs: false };
        let typ = Type::Function(function_type.clone());
        let mut handle_function_var = self.fresh_variable(typ.clone());

        for old_resume_id in &handle.resumes {
            self.define_resume_function(*old_resume_id, continuation.clone(), &handle_function_var, free_vars);
        }

        let k = || Box::new(continuation.clone());

        // continuation_suspended(continuation)
        let condition = hir::Ast::Builtin(hir::Builtin::ContinuationIsSuspended(k()));

        // branch0 body
        let then = self.match_on_effect(&handle.branches, continuation.clone(), result_type.clone());

        // This is the default / `return _ -> _` case
        let otherwise = hir::Ast::Builtin(hir::Builtin::ContinuationArgPop(k(), result_type.clone()));

        let if_ = hir::Ast::If(hir::If {
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
            result_type,
        });

        let body = Box::new(hir::Ast::Sequence(hir::Sequence {
            statements: vec![hir::Ast::Builtin(hir::Builtin::ContinuationResume(k())), if_],
        }));

        let mut args = vec![continuation_var];
        for (free_var_id, free_var_type) in free_vars {
            let variable = match self.lookup_definition(*free_var_id, free_var_type).unwrap() {
                Definition::Macro(_) => unreachable!("Macro definitions should not be captured"),
                Definition::Normal(variable) => variable,
            };
            args.push(variable);
        }

        // Push any captured effect continuations too
        if let Some(frame) = self.effect_continuations.last() {
            args.extend(frame.iter().map(|(_, k)| k.clone()));
        }

        let lambda = hir::Ast::Lambda(hir::Lambda { args, body, typ: function_type.clone() });

        let definition = hir::Ast::Definition(hir::Definition {
            variable: handle_function_var.definition_id,
            name: Some("handle".into()),
            mutable: false,
            typ: typ.clone(),
            expr: Box::new(lambda),
        });

        self.pop_continuation_parameters();
        handle_function_var.definition = Some(Rc::new(definition));
        (hir::Ast::Variable(handle_function_var), function_type)
    }

    /// Matches on each effect the handler may handle:
    ///
    /// actual_hash = continuation_pop(continuation, U32)
    /// if actual_hash == Effect1_Hash then
    ///     // Note that arguments are popped in reverse order
    ///     arg1_M = continuation_pop(continuation, typeof(arg1_1))
    ///     ...
    ///     arg1_1 = continuation_pop(continuation, typeof(arg1_M))
    ///     envM = continuation_pop(continuation, typeof(env1))
    ///     ...
    ///     env1 = continuation_pop(continuation, typeof(envM))
    ///     body1
    /// ...
    /// else if actual_hash == EffectN_Hash then
    ///     argN_M = continuation_pop(continuation, typeof(argN_1))
    ///     ...
    ///     argN_1 = continuation_pop(continuation, typeof(argN_M))
    ///     envM = continuation_pop(continuation, typeof(env1))
    ///     ...
    ///     env1 = continuation_pop(continuation, typeof(envM))
    ///     bodyN
    fn match_on_effect(
        &mut self, branches: &[(ast::Ast<'c>, ast::Ast<'c>)], k: hir::Ast, result_type: Type,
    ) -> hir::Ast {
        let branches = fmap(branches, |(pattern, branch)| {
            let effect_hash = self.hash_effect_pattern(pattern);
            let body = self.make_effect_body(pattern, branch, k.clone());
            (effect_hash, body)
        });

        // Sanity check to ensure there are no hash collisions
        for (i, (hash_i, _)) in branches.iter().enumerate() {
            for j in (i + 1)..branches.len() {
                let hash_j = branches[j].0;

                if *hash_i == hash_j {
                    panic!("Hash collision in `handle`:\n  {branches:?}");
                }
            }
        }

        let u64_type = Type::Primitive(hir::PrimitiveType::Integer(IntegerKind::U64));
        let hash_pop = hir::Ast::Builtin(hir::Builtin::ContinuationArgPop(Box::new(k), u64_type.clone()));
        let (definition, actual_hash) =
            self.fresh_definition_with_variable(hash_pop, "effect_hash".to_string(), u64_type);

        let mut ret = hir::Ast::Sequence(hir::Sequence { statements: vec![definition, unit_literal()] });

        let mut next = match &mut ret {
            hir::Ast::Sequence(seq) => &mut seq.statements[1],
            _ => unreachable!(),
        };

        for (hash, branch) in branches {
            let effect_hash = hir::Ast::Literal(hir::Literal::Integer(hash, IntegerKind::U64));
            // actual_hash == effect_hash
            let condition =
                hir::Ast::Builtin(hir::Builtin::EqInt(Box::new(actual_hash.clone()), Box::new(effect_hash)));

            *next = hir::Ast::If(hir::If {
                condition: Box::new(condition),
                then: Box::new(branch),
                otherwise: Box::new(unit_literal()),
                result_type: result_type.clone(),
            });

            match next {
                hir::Ast::If(if_) => next = if_.otherwise.as_mut(),
                _ => unreachable!(),
            }
        }

        *next = self.make_unhandled_effect_case(actual_hash, result_type);
        ret
    }

    // Handlers translate to a match on effect hashes but in the case of a miscompilation,
    // we can have effect hashes that aren't handled by the handler. For these cases we
    // add an `else` case to the handler with the pseudocode:
    //
    // ```
    // else
    //     printf "ICE: Unhandled effect hash `%llu`\n" effect_hash
    //     exit 1
    //     deref (stack_alloc ()) : result_type
    // ```
    fn make_unhandled_effect_case(&self, effect_hash: hir::Ast, result_type: Type) -> hir::Ast {
        let message = hir::Ast::Literal(hir::Literal::CString("ICE: Unhandled effect hash `%llu`\n".to_string()));

        let int_type = |kind| Type::Primitive(hir::PrimitiveType::Integer(kind));

        let printf_type = hir::FunctionType {
            parameters: vec![Type::pointer(), int_type(IntegerKind::U64)],
            return_type: Box::new(int_type(IntegerKind::I32)),
            is_varargs: true,
        };

        let printf_definition = hir::Ast::Definition(hir::Definition {
            variable: self.printf_id,
            name: Some("printf".to_string()),
            mutable: false,
            typ: Type::Function(printf_type.clone()),
            expr: Box::new(hir::Ast::Extern(hir::Extern {
                name: "printf".to_string(),
                typ: Type::Function(printf_type.clone()),
            })),
        });

        let printf = hir::Variable {
            definition: Some(Rc::new(printf_definition)),
            definition_id: self.printf_id,
            typ: Rc::new(Type::Function(printf_type.clone())),
            name: Some("printf".to_string()),
        };

        let print_unhandled_hash_value = hir::Ast::FunctionCall(hir::FunctionCall {
            function: Box::new(hir::Ast::Variable(printf)),
            args: vec![message, effect_hash],
            function_type: printf_type,
        });

        let exit_type = hir::FunctionType {
            parameters: vec![int_type(IntegerKind::I32)],
            return_type: Box::new(Type::unit()),
            is_varargs: false,
        };

        let exit_definition = hir::Ast::Definition(hir::Definition {
            variable: self.exit_id,
            name: Some("exit".to_string()),
            mutable: false,
            typ: Type::Function(exit_type.clone()),
            expr: Box::new(hir::Ast::Extern(hir::Extern {
                name: "exit".to_string(),
                typ: Type::Function(exit_type.clone()),
            })),
        });

        let exit = hir::Variable {
            definition: Some(Rc::new(exit_definition)),
            definition_id: self.exit_id,
            typ: Rc::new(Type::Function(exit_type.clone())),
            name: Some("exit".to_string()),
        };

        let exit_call = hir::Ast::FunctionCall(hir::FunctionCall {
            function: Box::new(hir::Ast::Variable(exit)),
            args: vec![hir::Ast::Literal(hir::Literal::Integer(1, IntegerKind::I32))],
            function_type: exit_type,
        });

        // Must make the result type match still
        let alloc = hir::Ast::Builtin(hir::Builtin::StackAlloc(Box::new(unit_literal())));
        let deref = hir::Ast::Builtin(hir::Builtin::Deref(Box::new(alloc), result_type));

        hir::Ast::Sequence(hir::Sequence { statements: vec![print_unhandled_hash_value, exit_call, deref] })
    }

    fn hash_effect_pattern(&self, pattern: &ast::Ast<'c>) -> u64 {
        match pattern {
            ast::Ast::FunctionCall(call) => match call.function.as_ref() {
                ast::Ast::Variable(variable) => {
                    let variable_type = self.follow_all_bindings(variable.typ.as_ref().unwrap());
                    Self::hash_effect(&variable.kind.name(), &variable_type)
                },
                other => unreachable!("Expected function name for effect, found: `{other}`"),
            },
            other => unreachable!("Unexpected effect pattern: `{other}`"),
        }
    }

    /// Lowers the `expr` in `handle expr | ...` into:
    /// ```pseudocode
    /// start_expr(continuation) =
    ///     env_N = continuation_pop(continuation, typeof(env_N))
    ///     ...
    ///     env_1 = continuation_pop(continuation, typeof(env_1))
    ///     result = expr // `continuation` is automatically added to any function calls within expr
    ///     continuation_push(continuation, result)
    /// ```
    fn make_start_effect_expr_function(
        &mut self, expr: &ast::Ast<'c>, effects_handled: &[Effect], env: &BTreeMap<DefinitionInfoId, types::Type>,
    ) -> hir::Ast {
        let continuation_var = self.fresh_variable(Type::continuation());
        let continuation = Box::new(hir::Ast::Variable(continuation_var.clone()));

        let mut statements = self.make_handle_env_pops(continuation.clone(), env);

        // All continuations handled by this Handle expression use the same continuation variable,
        // even in the case of multiple effects being handled.
        self.effect_continuations.push(fmap(effects_handled, |(effect_id, effect_args)| {
            let effect_args = fmap(effect_args, |arg| self.follow_all_bindings(arg));
            ((*effect_id, effect_args), continuation_var.clone())
        }));

        let result = Box::new(self.monomorphise(expr));
        let push = hir::Ast::Builtin(hir::Builtin::ContinuationArgPush(continuation, result));
        statements.push(push);

        let body = Box::new(hir::Ast::Sequence(hir::Sequence { statements }));

        self.pop_continuation_parameters();

        let args = vec![continuation_var];
        let parameters = vec![Type::continuation()];
        let return_type = Box::new(Type::unit());
        let typ = hir::FunctionType { parameters, return_type, is_varargs: false };

        hir::Ast::Lambda(hir::Lambda { args, body, typ })
    }

    /// Create code to push each given variable to the continuation's channel.
    /// ```pseudocode
    /// continuation_push(continuation, env_1)
    /// ...
    /// continuation_push(continuation, env_N)
    /// ```
    /// These environment variables comes from the fact we are implicitly creating
    /// functions from `Handle` expressions and thus need to handle captured variables from the
    /// environment as we would with a closure's environment. Since the type signature expected by
    /// `continuation_init` prevents us from actually passing in the environment directly we need
    /// to push and pop them to the closure's channel.
    fn make_handle_env_pushes(
        &self, k: hir::Ast, free_vars: &BTreeMap<DefinitionInfoId, types::Type>, statements: &mut Vec<hir::Ast>,
    ) {
        use hir::{Ast::Builtin, Builtin::ContinuationArgPush};

        for (variable, variable_type) in free_vars {
            let k = Box::new(k.clone());
            let variable = match self.lookup_definition(*variable, variable_type).unwrap() {
                Definition::Macro(_) => unreachable!("Macro definitions should not be captured"),
                Definition::Normal(variable) => variable,
            };

            let variable = Box::new(hir::Ast::Variable(variable));
            statements.push(Builtin(ContinuationArgPush(k, variable)));
        }
    }

    /// Create code to pop each given variable to the continuation's channel.
    /// ```pseudocode
    /// env_N = continuation_pop(continuation, typeof(env_N))
    /// ...
    /// env_1 = continuation_pop(continuation, typeof(env_1))
    /// ```
    /// These environment variables comes from the fact we are implicitly creating
    /// functions from `Handle` expressions and thus need to handle captured variables from the
    /// environment as we would with a closure's environment. Since the type signature expected by
    /// `continuation_init` prevents us from actually passing in the environment directly we need
    /// to push and pop them to the closure's channel.
    fn make_handle_env_pops(
        &mut self, k: Box<hir::Ast>, free_vars: &BTreeMap<DefinitionInfoId, types::Type>,
    ) -> Vec<hir::Ast> {
        fmap(free_vars.iter().rev(), |(variable, variable_type)| {
            let name = self.cache.definition_infos[variable.0].name.clone();

            let variable = match self.lookup_definition(*variable, &variable_type).unwrap() {
                Definition::Macro(_) => unreachable!(),
                Definition::Normal(variable) => variable.definition_id,
            };

            let typ = self.convert_type(&variable_type);
            let pop = hir::Builtin::ContinuationArgPop(k.clone(), typ.clone());
            let expr = Box::new(hir::Ast::Builtin(pop));

            hir::Ast::Definition(hir::Definition { variable, name: Some(name), mutable: false, typ, expr })
        })
    }

    /// Defines the `resume` function for a particular handle definition.
    /// This becomes a global function and can be defined any time.
    fn define_resume_function(
        &mut self, resume_id: DefinitionInfoId, k: hir::Ast, handler_function: &hir::Variable,
        free_vars: &BTreeMap<DefinitionInfoId, types::Type>,
    ) {
        let definition_id = self.next_unique_id();
        let typ = self.cache.definition_infos[resume_id.0].typ.as_ref().unwrap().as_monotype().clone();
        let typ = self.follow_all_bindings(&typ);
        let monomorphized_type = Rc::new(self.convert_type(&typ));

        let function_effects = self.get_effects(&typ);
        let resume_function =
            self.make_resume_function(&monomorphized_type, handler_function, free_vars, function_effects);

        // `resume`'s closure environment is any free variable used by any of the effect branches
        // plus the continuation `k`.
        let mut env = VecDeque::new();
        env.push_back(k);
        for (free_var_id, free_var_type) in free_vars {
            let variable = match self.lookup_definition(*free_var_id, free_var_type).unwrap() {
                Definition::Macro(_) => unreachable!("Macro definitions should not be captured"),
                Definition::Normal(variable) => variable,
            };
            env.push_back(hir::Ast::Variable(variable));
        }

        let env = Self::make_closure_environment(env);
        let resume_closure = tuple(vec![resume_function, env]);

        let name = Some("resume".to_string());
        let definition = hir::Ast::Definition(hir::Definition {
            variable: definition_id,
            name: name.clone(),
            mutable: false,
            typ: monomorphized_type.as_ref().clone(),
            expr: Box::new(resume_closure),
        });

        let definition = Some(Rc::new(definition));
        let variable = hir::Variable { definition_id, definition, name, typ: monomorphized_type };
        let definition = Definition::Normal(variable);

        self.definitions.insert(resume_id, typ, definition);
    }

    /// A resume function `resume: Arg1 - Arg2 - ... - ArgN -> Ret` translates to:
    /// ```pseudocode
    /// Ret resume(_1: Arg1, _2: Arg2, ..., _N: ArgN, env1: Env1, env2: Env2, ..., envN: EnvN, k: Cont) {
    ///     co_push(co, &_1, sizeof(Arg1));
    ///     co_push(co, &_2, sizeof(Arg2));
    ///     ...
    ///     co_push(co, &_N, sizeof(ArgN));
    ///     return handler_function(k, env1, env2, ..., envN);
    /// }
    /// ```
    fn make_resume_function(
        &mut self, typ: &Type, handler_function: &hir::Variable, free_vars: &BTreeMap<DefinitionInfoId, types::Type>,
        mut function_effects: Vec<Effect>,
    ) -> hir::Ast {
        use hir::{Ast::Builtin, Builtin::ContinuationArgPush};
        let mut function_type = match typ {
            Type::Function(function) => function.clone(),
            Type::Tuple(elements) if elements.len() == 2 => match &elements[0] {
                Type::Function(function) => function.clone(),
                other => panic!("Expected function type, found {}", other),
            },
            other => panic!("Expected function type, found {}", other),
        };

        let environment_type =
            function_type.parameters.last_mut().expect("There should always be a closure environment parameter");
        let environment_size = free_vars.len() + 1;
        Self::fix_resume_environment_type(environment_type, environment_size);

        function_effects.reverse();
        let mut effect_continuations = Vec::with_capacity(function_effects.len());

        let lambda_args = fmap(function_type.parameters.iter().enumerate(), |(i, param)| {
            let definition_id = self.next_unique_id();
            let var = hir::DefinitionInfo { definition: None, definition_id, typ: Rc::new(param.clone()), name: None };

            // Check if this parameter is a Continuation that is not this resume's continuation.
            // If it is this resume's continuation it'll always be in the captured environment, so
            // we skip that last parameter. If it is not this resume's continuation, it should have
            // been pushed as a continuation parameter before the environment parameter.
            let is_env_param = i == function_type.parameters.len() - 1;
            if !is_env_param && *param == Type::continuation() {
                let effect = function_effects.pop().expect("Expected effect handler");
                effect_continuations.push((effect, var.clone()));
            }
            var
        });
        self.effect_continuations.push(effect_continuations);

        let environment = lambda_args.last().unwrap().clone();

        let (k_var, mut statements, mut call_args) = self.unpack_resume_environment(free_vars, environment);

        // Push any effect continuations that aren't handled by this Handle as well
        if let Some(frame) = self.effect_continuations.last() {
            call_args.extend(frame.iter().map(|(_, k)| hir::Ast::Variable(k.clone())));
        }

        let k = hir::Ast::Variable(k_var.clone());

        // Push each parameter to the continuation
        statements.extend(lambda_args.iter().map(|arg| {
            let arg = Box::new(hir::Ast::Variable(arg.clone()));
            Builtin(ContinuationArgPush(Box::new(k.clone()), arg))
        }));

        // The last arg in `lambda_args` is the environment which we don't want to push
        statements.pop();

        // Then resume the continuation
        statements.push(hir::Ast::FunctionCall(hir::FunctionCall {
            function: Box::new(hir::Ast::Variable(handler_function.clone())),
            args: call_args,
            function_type: handler_function.typ.as_ref().clone().into_function().unwrap(),
        }));

        self.pop_continuation_parameters();
        let body = Box::new(hir::Ast::Sequence(hir::Sequence { statements }));
        hir::Ast::Lambda(hir::Lambda { args: lambda_args, body, typ: function_type })
    }

    /// Unpack `resume`'s closure environment, adding each definition to the returned
    /// Vec of statements. This also returns a Vec of each defined variable (k being first)
    /// so that these can later be forwarded to the handle function since k + the environment
    /// are the arguments it takes as well.
    fn unpack_resume_environment(
        &mut self, free_vars: &BTreeMap<DefinitionInfoId, types::Type>, mut environment: hir::Variable,
    ) -> (hir::Variable, Vec<hir::Ast>, Vec<hir::Ast>) {
        let environment_vars = std::iter::once(Type::continuation())
            .chain(free_vars.iter().map(|(_, typ)| self.convert_type(typ)))
            .collect::<Vec<_>>();

        let mut env_type = environment.typ.as_ref().clone();
        let total_vars = environment_vars.len();

        let mut statements = Vec::new();
        let mut defined_vars = Vec::new();

        for (i, environment_var_type) in environment_vars.into_iter().enumerate() {
            if i == total_vars - 1 {
                defined_vars.push(environment.clone());
            } else {
                let env_ast = hir::Ast::Variable(environment);

                let variable_extract = Self::extract(env_ast.clone(), 0, environment_var_type.clone());
                env_type = Self::extract_second_type(env_type);
                let env_ast = Self::extract(env_ast, 1, env_type.clone());

                let (var_def, var) = self.fresh_definition(variable_extract, None, environment_var_type.clone());
                let (env_def, env_var) = self.fresh_definition(env_ast, None, env_type.clone());

                statements.push(var_def);
                statements.push(env_def);
                defined_vars.push(hir::Variable::new(var, Rc::new(environment_var_type)));
                environment = hir::Variable::new(env_var, Rc::new(env_type.clone()));
            }
        }

        let k = defined_vars.first().unwrap().clone();
        let defined_vars = fmap(defined_vars, hir::Ast::Variable);
        (k, statements, defined_vars)
    }

    /// The frontend does not have a continuation type so it encodes it as a `Ptr Unit` instead.
    /// Here we take an environment tuple and mutate the very first element to be a continuation
    /// type.
    fn fix_resume_environment_type(typ: &mut Type, environment_size: usize) {
        if environment_size > 1 {
            match typ {
                Type::Tuple(items) => {
                    assert_eq!(items.len(), 2, "environment tuples should always be nested pairs");
                    items[0] = Type::continuation();
                },
                other => unreachable!(
                    "Expected an environment tuple of size {environment_size} but found a non-tuple {other}"
                ),
            }
        } else {
            *typ = Type::continuation();
        }
    }

    /// When matching on an effect in a handler:
    /// ```pseudocode
    /// | effect1 arg1_1 .. arg1_M -> body1
    /// ```
    /// Generate the code:
    /// ```pseudocode
    /// // Pop arguments in reverse order
    /// arg1_M = continuation_pop(continuation, typeof(arg1_1))
    /// ...
    /// arg1_1 = continuation_pop(continuation, typeof(arg1_M))
    /// body1
    /// ```
    fn make_effect_body(&mut self, pattern: &ast::Ast<'c>, branch: &ast::Ast<'c>, continuation: hir::Ast) -> hir::Ast {
        use hir::{Ast::Builtin, Builtin::ContinuationArgPop};

        let arguments = self.effect_function_parameters(pattern);
        let mut statements = Vec::with_capacity(arguments.len() + 1);

        for argument in arguments.into_iter().rev() {
            let k = Box::new(continuation.clone());
            let argument_type = argument.typ.as_ref().clone();

            // arg = continuation_pop(continuation, typeof(arg))
            let arg_def = hir::Ast::Definition(hir::Definition {
                variable: argument.definition_id,
                name: argument.name.clone(),
                mutable: false,
                typ: argument_type.clone(),
                expr: Box::new(Builtin(ContinuationArgPop(k, argument_type))),
            });
            statements.push(arg_def);
        }

        statements.push(self.monomorphise(branch));
        hir::Ast::Sequence(hir::Sequence { statements })
    }

    /// Define and return each parameter from this effect function pattern
    fn effect_function_parameters(&mut self, pattern: &ast::Ast<'c>) -> Vec<hir::Variable> {
        match pattern {
            ast::Ast::FunctionCall(call) => {
                // TODO: use function to get the right function hash
                let _function = unwrap_variable(&call.function);

                fmap(&call.args, |arg| {
                    let var = unwrap_variable(arg);
                    let definition_id = self.next_unique_id();
                    let typ = self.follow_all_bindings(var.get_type().unwrap());
                    let monomorphized_type = Rc::new(self.convert_type(&typ));
                    let name = Some(var.to_string());
                    let variable = hir::Variable { definition_id, definition: None, name, typ: monomorphized_type };
                    let definition = Definition::Normal(variable.clone());
                    self.definitions.insert(var.definition.unwrap(), typ, definition);
                    variable
                })
            },
            other => {
                unreachable!("Expected a function call in a handle pattern. Found: {:?}", other)
            },
        }
    }

    /// This is a simple function but it is useful to have a centralized place that decides
    /// how an effect should be hashed.
    /// Expects `frontend_type` to equal `frontend_type.follow_bindings()`.
    fn hash_effect(name: &str, frontend_type: &types::Type) -> u64 {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        frontend_type.hash(&mut hasher);
        hasher.finish()
    }

    /// Compile an effect `eff : A - ... - Z -> Ret` as:
    /// ```pseudocode
    /// eff(a, ..., z, k) =
    ///     continuation_push(k, a)
    ///     ...
    ///     continuation_push(k, z)
    ///     continuation_push(k, hash_effect(eff))  // `hash_effect(eff)` is a known constant
    ///     continuation_suspend(k)
    ///     continuation_pop(k, Ret)
    /// ```
    pub(super) fn make_effect_function(&mut self, typ: Type, frontend_type: &types::Type, name: &str) -> hir::Ast {
        use hir::{Ast, Builtin};
        let hir::types::Type::Function(function_type) = typ else { unreachable!("All effects should be functions") };

        let mut args = fmap(&function_type.parameters, |param| self.fresh_variable(param.clone()));
        let continuation = args.pop().unwrap();

        let k = || Box::new(Ast::Variable(continuation.clone()));

        // continuation_push(k, arg)
        let mut statements = fmap(&args, |arg| {
            let arg = Box::new(Ast::Variable(arg.clone()));
            Ast::Builtin(Builtin::ContinuationArgPush(k(), arg))
        });

        // Finally push the hash of the effect we're performing so the handler knows which
        // case to branch to
        let effect_hash = Self::hash_effect(name, frontend_type);
        let effect_hash = hir::Ast::Literal(hir::Literal::Integer(effect_hash, IntegerKind::U64));
        statements.push(Ast::Builtin(Builtin::ContinuationArgPush(k(), Box::new(effect_hash))));

        let return_type = function_type.return_type.as_ref().clone();

        statements.push(Ast::Builtin(Builtin::ContinuationSuspend(k())));
        statements.push(Ast::Builtin(Builtin::ContinuationArgPop(k(), return_type)));

        args.push(continuation);
        let body = Box::new(Ast::Sequence(hir::Sequence { statements }));
        Ast::Lambda(hir::Lambda { args, body, typ: function_type })
    }
}
