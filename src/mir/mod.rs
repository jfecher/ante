use std::rc::Rc;

use self::ir::{Mir, Atom, ParameterId, FunctionId};
use self::context::Context;
use crate::hir::DecisionTree;
use crate::{hir::{self, Literal}, util::fmap};

pub mod ir;
mod context;
mod printer;
mod interpreter;

pub fn convert_to_mir(hir: hir::Ast) -> Mir {
    let mut context = Context::new();
    let ret = hir.to_atom(&mut context);

    if let Some(continuation) = context.continuation.take() {
        context.terminate_function_with_call(continuation, vec![ret]);
    }

    while let Some((_, handler, definition)) = context.definition_queue.pop_front() {
        context.current_handler = handler;
        let result = definition.to_atom(&mut context);
        assert_eq!(result, Atom::Literal(Literal::Unit));
    }

    context.mir
}

trait ToMir {
    fn to_atom(&self, context: &mut Context) -> Atom;
}

impl ToMir for hir::Ast {
    fn to_atom(&self, context: &mut Context) -> Atom {
        dispatch_on_hir!(self, ToMir::to_atom, context)
    }
}

impl ToMir for hir::Literal {
    fn to_atom(&self, _mir: &mut Context) -> Atom {
        Atom::Literal(self.clone())
    }
}

impl ToMir for hir::Variable {
    fn to_atom(&self, context: &mut Context) -> Atom {
        context.get_definition(self.definition_id, &self.typ).unwrap_or_else(|| {
            context.add_global_to_queue(self.clone())
        })
    }
}

impl ToMir for hir::Lambda {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let original_function = context.current_function_id.clone();
        let original_continuation = context.continuation.take();

        // make sure to add k parameter
        let name = Rc::new("lambda".to_owned());
        let lambda_id = context.next_fresh_function_with_name(name);

        // Add args to scope
        for (i, arg) in self.args.iter().enumerate() {
            context.insert_definition(arg.definition_id, &arg.typ, Atom::Parameter(ParameterId {
                function: lambda_id.clone(),
                parameter_index: i as u16,
            }));
        }

        // If the argument types were not already set, set them now
        let (arg_types, effects) = context.convert_function_type(&self.typ);
        let function = context.current_function_mut();
        function.argument_types = arg_types;

        // Register each effect continuation we have
        let old_handlers = context.register_handlers(&effects, &lambda_id);

        let arguments = &mut context.mir.functions.get_mut(&context.current_function_id).unwrap().argument_types;

        let mut k = ParameterId {
            function: lambda_id.clone(),
            parameter_index: arguments.len() as u16 - 1,
        };

        // If we're in a handler branch, define `resume` to be the current continuation
        // and define the handler for `effect_id` as the current function
        let end_continuation = if let Some((effect_id, variable)) = context.handler_continuation.take() {
            let k_type = arguments.pop().unwrap();
            let effect_k_type = arguments.pop().unwrap();
            let _effect_type = arguments.pop().unwrap();

            let effect_k_index = arguments.len() as u16;
            arguments.push(effect_k_type);
            arguments.push(k_type);

            k = ParameterId {
                function: lambda_id.clone(),
                parameter_index: arguments.len() as u16 - 1,
            };

            context.insert_definition(variable.definition_id, &variable.typ, Atom::Parameter(k.clone()));
            context.handlers.insert(effect_id, Atom::Function(lambda_id.clone()));
            Atom::Parameter(ParameterId { function: lambda_id.clone(), parameter_index: effect_k_index })
        } else {
            Atom::Parameter(k.clone())
        };

        let k = Atom::Parameter(k);
        context.continuation = Some(k.clone());

        let mut return_values = vec![self.body.to_atom(context)];

        for effect in effects {
            let handler_k = context.handler_ks[&effect.effect_id].clone();
            return_values.push(handler_k);
        }

        context.terminate_function_with_call(end_continuation, return_values);

        context.set_handlers(old_handlers);
        context.current_function_id = original_function;
        context.continuation = original_continuation;

        Atom::Function(lambda_id)
    }
}

impl ToMir for hir::FunctionCall {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let f = self.function.to_atom(context);
        let args = fmap(&self.args, |arg| arg.to_atom(context));

        context.with_next_function(self.function_type.return_type.as_ref(), &self.function_type.effects, |context, k| {
            let (_, effects) = context.convert_function_type(&self.function_type);
            context.terminate_function_with_call_and_effects(f, args, k.clone(), &effects);
        });

        // Now that with_next_function advanced us to the next function, the call result
        // will be the sole parameter of the current function.
        let function = context.current_function_id.clone();
        Atom::Parameter(ParameterId { function, parameter_index: 0 })
    }
}

impl ToMir for hir::Definition {
    fn to_atom(&self, context: &mut Context) -> Atom {
        if let Some(expected) = context.get_definition(self.variable, &self.typ) {
            let function = match &expected {
                Atom::Function(function_id) => function_id.clone(),
                other => unreachable!("Expected Atom::Function, found {:?}", other),
            };

            let old = context.expected_function_id.take();
            context.expected_function_id = Some(function.clone());
            let rhs = self.expr.to_atom(context);

            // If rhs is an extern symbol it may define a function yet
            // not actually correspond to an Atom::Function
            if rhs != expected && !matches!(self.expr.as_ref(), hir::Ast::Effect(..)) {
                let original_function = context.current_function_id.clone();
                context.current_function_id = function;

                // The body is still empty in the case of an extern, so
                // forward all of the arguments to the extern itself
                let parameters = context.current_parameters();
                context.terminate_function_with_call(rhs, parameters);

                context.current_function_id = original_function;
            }

            context.expected_function_id = old;
        } else {
            let rhs = self.expr.to_atom(context);
            context.insert_definition(self.variable, &self.typ, rhs);
        }

        Atom::Literal(Literal::Unit)
    }
}

impl ToMir for hir::If {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let cond = self.condition.to_atom(context);
        let original_function = context.current_function_id.clone();

        // needs param
        let end_function_id = context.next_fresh_function();
        context.add_parameter(&self.result_type);
        let end_function = Atom::Function(end_function_id.clone());

        let then_fn = Atom::Function(context.next_fresh_function()) ;
        let then_value = self.then.to_atom(context);
        context.terminate_function_with_call(end_function.clone(), vec![then_value]);

        let else_fn = Atom::Function(context.next_fresh_function()) ;
        let else_value = self.otherwise.to_atom(context);
        context.terminate_function_with_call(end_function, vec![else_value]);

        context.current_function_id = original_function;
        context.terminate_function_with_call(Atom::Branch, vec![cond, then_fn, else_fn]);

        context.current_function_id = end_function_id.clone();
        Atom::Parameter(ParameterId { 
            function: end_function_id,
            parameter_index: 0,
        })
    }
}

impl ToMir for hir::Match {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let original_function = context.current_function_id.clone();
        let leaves = fmap(&self.branches, |_| context.next_fresh_function());

        // Codegen the switches first to eventually jump to each leaf
        context.current_function_id = original_function;
        decision_tree_to_mir(&self.decision_tree, &leaves, context);

        let end = context.next_fresh_function();
        context.add_parameter(&self.result_type);

        // Now codegen each leaf, all jumping to the same end continuation afterward
        for (leaf_hir, leaf_function) in self.branches.iter().zip(leaves) {
            context.current_function_id = leaf_function;
            let result = leaf_hir.to_atom(context);
            context.terminate_function_with_call(Atom::Function(end.clone()), vec![result]);
        }

        context.current_function_id = end.clone();
        Atom::Parameter(ParameterId {
            function: end,
            parameter_index: 0,
        })
    }
}

fn decision_tree_to_mir(tree: &DecisionTree, leaves: &[FunctionId], context: &mut Context) {
    match tree {
        DecisionTree::Leaf(leaf_index) => {
            let function = Atom::Function(leaves[*leaf_index].clone());
            context.terminate_function_with_call(function, vec![]);
        },
        DecisionTree::Definition(definition, rest) => {
            definition.to_atom(context);
            decision_tree_to_mir(&rest, leaves, context);
        },
        DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
            let tag = int_to_switch_on.to_atom(context);
            let original_function = context.current_function_id.clone();

            let case_functions = fmap(cases, |(tag_to_match, case_tree)| {
                let function = context.next_fresh_function();
                decision_tree_to_mir(case_tree, leaves, context);
                (*tag_to_match, function)
            });

            let else_function = else_case.as_ref().map(|else_tree| {
                let function = context.next_fresh_function();
                decision_tree_to_mir(else_tree, leaves, context);
                function
            });

            let switch = Atom::Switch(case_functions, else_function);

            context.current_function_id = original_function;
            context.terminate_function_with_call(switch, vec![tag]);
        },
    }
}

impl ToMir for hir::Return {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let continuation = context.continuation.clone().expect("No continuation for hir::Return!");
        let value = self.expression.to_atom(context);

        context.terminate_function_with_call(continuation, vec![value]);

        // This is technically not needed but we switch to a new function in case there is
        // code sequenced after a `return` as otherwise it would overwrite the call above.
        context.next_fresh_function();

        // TODO: Return some kind of unreachable/uninitialized value?
        Atom::Literal(Literal::Unit)
    }
}

impl ToMir for hir::Sequence {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let count = self.statements.len();

        // The first statements must be converted to atoms to
        // ensure we create any intermediate continuations needed
        for statement in self.statements.iter().take(count.saturating_sub(1)) {
            statement.to_atom(context);
        }

        // The last statement is kept as an Atom since it is directly returned
        match self.statements.last() {
            Some(statement) => statement.to_atom(context),
            None => Atom::Literal(Literal::Unit),
        }
    }
}

impl ToMir for hir::Extern {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let id = context.import_extern(&self.name, &self.typ);
        Atom::Extern(id)
    }
}

impl ToMir for hir::Assignment {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let lhs = self.lhs.to_atom(context);
        let rhs = self.rhs.to_atom(context);

        let unit = hir::Type::Primitive(hir::PrimitiveType::Unit);
        context.with_next_function(&unit, &[], |context, k| {
            context.terminate_function_with_call(Atom::Assign, vec![lhs, rhs, k]);
            Atom::Literal(Literal::Unit)
        })
    }
}

impl ToMir for hir::MemberAccess {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let lhs = Box::new(self.lhs.to_atom(context));
        let typ = context.convert_type(&self.typ);
        Atom::MemberAccess(lhs, self.member_index, typ)
    }
}

impl ToMir for hir::Tuple {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let fields = fmap(&self.fields, |field| field.to_atom(context));
        Atom::Tuple(fields)
    }
}

impl ToMir for hir::ReinterpretCast {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let value = Box::new(self.lhs.to_atom(context));
        let typ = context.convert_type(&self.target_type);
        Atom::Transmute(value, typ)
    }
}

impl ToMir for hir::Builtin {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let binary_fn = |f: fn(_, _) -> _, context: &mut Context, lhs: &hir::Ast, rhs: &hir::Ast| {
            let lhs = Box::new(lhs.to_atom(context));
            let rhs = Box::new(rhs.to_atom(context));
            f(lhs, rhs)
        };

        let unary_fn = |f: fn(_) -> _, context, lhs: &hir::Ast| {
            f(Box::new(lhs.to_atom(context)))
        };

        let unary_fn_with_type = |f: fn(_, _) -> _, context: &mut _, lhs: &hir::Ast, rhs: &hir::Type| {
            let lhs = Box::new(lhs.to_atom(context));
            let rhs = context.convert_type(rhs);
            f(lhs, rhs)
        };

        match self {
            hir::Builtin::AddInt(lhs, rhs) => binary_fn(Atom::AddInt, context, lhs, rhs),
            hir::Builtin::AddFloat(lhs, rhs) => binary_fn(Atom::AddFloat, context, lhs, rhs),
            hir::Builtin::SubInt(lhs, rhs) => binary_fn(Atom::SubInt, context, lhs, rhs),
            hir::Builtin::SubFloat(lhs, rhs) => binary_fn(Atom::SubFloat, context, lhs, rhs),
            hir::Builtin::MulInt(lhs, rhs) => binary_fn(Atom::MulInt, context, lhs, rhs),
            hir::Builtin::MulFloat(lhs, rhs) => binary_fn(Atom::MulFloat, context, lhs, rhs),
            hir::Builtin::DivSigned(lhs, rhs) => binary_fn(Atom::DivSigned, context, lhs, rhs),
            hir::Builtin::DivUnsigned(lhs, rhs) => binary_fn(Atom::DivUnsigned, context, lhs, rhs),
            hir::Builtin::DivFloat(lhs, rhs) => binary_fn(Atom::DivFloat, context, lhs, rhs),
            hir::Builtin::ModSigned(lhs, rhs) => binary_fn(Atom::ModSigned, context, lhs, rhs),
            hir::Builtin::ModUnsigned(lhs, rhs) => binary_fn(Atom::ModUnsigned, context, lhs, rhs),
            hir::Builtin::ModFloat(lhs, rhs) => binary_fn(Atom::ModFloat, context, lhs, rhs),
            hir::Builtin::LessSigned(lhs, rhs) => binary_fn(Atom::LessSigned, context, lhs, rhs),
            hir::Builtin::LessUnsigned(lhs, rhs) => binary_fn(Atom::LessUnsigned, context, lhs, rhs),
            hir::Builtin::LessFloat(lhs, rhs) => binary_fn(Atom::LessFloat, context, lhs, rhs),
            hir::Builtin::EqInt(lhs, rhs) => binary_fn(Atom::EqInt, context, lhs, rhs),
            hir::Builtin::EqFloat(lhs, rhs) => binary_fn(Atom::EqFloat, context, lhs, rhs),
            hir::Builtin::EqChar(lhs, rhs) => binary_fn(Atom::EqChar, context, lhs, rhs),
            hir::Builtin::EqBool(lhs, rhs) => binary_fn(Atom::EqBool, context, lhs, rhs),
            hir::Builtin::SignExtend(lhs, typ) => unary_fn_with_type(Atom::SignExtend, context, lhs, typ),
            hir::Builtin::ZeroExtend(lhs, typ) => unary_fn_with_type(Atom::ZeroExtend, context, lhs, typ),
            hir::Builtin::SignedToFloat(lhs, typ) => unary_fn_with_type(Atom::SignedToFloat, context, lhs, typ),
            hir::Builtin::UnsignedToFloat(lhs, typ) => unary_fn_with_type(Atom::UnsignedToFloat, context, lhs, typ),
            hir::Builtin::FloatToSigned(lhs, typ) => unary_fn_with_type(Atom::FloatToSigned, context, lhs, typ),
            hir::Builtin::FloatToUnsigned(lhs, typ) => unary_fn_with_type(Atom::FloatToUnsigned, context, lhs, typ),
            hir::Builtin::FloatPromote(value, typ) => unary_fn_with_type(Atom::FloatPromote, context, value, typ),
            hir::Builtin::FloatDemote(value, typ) => unary_fn_with_type(Atom::FloatDemote, context, value, typ),
            hir::Builtin::BitwiseAnd(lhs, rhs) => binary_fn(Atom::BitwiseAnd, context, lhs, rhs),
            hir::Builtin::BitwiseOr(lhs, rhs) => binary_fn(Atom::BitwiseOr, context, lhs, rhs),
            hir::Builtin::BitwiseXor(lhs, rhs) => binary_fn(Atom::BitwiseXor, context, lhs, rhs),
            hir::Builtin::BitwiseNot(value) => unary_fn(Atom::BitwiseNot, context, value),
            hir::Builtin::Truncate(lhs, typ) => unary_fn_with_type(Atom::Truncate, context, lhs, typ),
            hir::Builtin::Deref(lhs, typ) => unary_fn_with_type(Atom::Deref, context, lhs, typ),
            hir::Builtin::Transmute(lhs, typ) => unary_fn_with_type(Atom::Transmute, context, lhs, typ),
            hir::Builtin::StackAlloc(value) => unary_fn(Atom::StackAlloc, context, value),
            hir::Builtin::Offset(lhs, rhs, typ) => {
                let lhs = Box::new(lhs.to_atom(context));
                let rhs = Box::new(rhs.to_atom(context));
                let typ = context.convert_type(typ);
                Atom::Offset(lhs, rhs, typ)
            },
        }
    }
}

impl ToMir for hir::Effect {
    // Expect this hir::Effect is wrapped in its own hir::Definition
    // from monomorphisation
    fn to_atom(&self, context: &mut Context) -> Atom {
        // Monomorphization wraps effects in a definition node, which populates an
        // expected function id ahead of time (usually for Lambdas), so we have to
        // make sure to use that and not insert into the current function.
        let target_function = context.expected_function_id.take()
            .expect("Expected `expected_function_id` for hir::Effect::to_atom");

        let old_function = std::mem::replace(&mut context.current_function_id, target_function.clone());

        let effect_id = context.lookup_or_create_effect(self.id);

        let effects = match context.convert_type(&self.typ) {
            ir::Type::Function(_, effects) => effects,
            other => unreachable!("Expected type of effect to be a function, got {}", other),
        };

        let function_id = &context.current_function_id.clone();
        context.register_handlers(&effects, function_id);

        let handler = context.lookup_handler(effect_id);

        let call_parameters = context.current_parameters()
            .into_iter()
            .filter(|param| *param != handler)
            .collect();

        context.terminate_function_with_call(handler, call_parameters);
        context.current_function_id = old_function;

        Atom::Function(target_function)
    }
}

impl ToMir for hir::Handle {
    fn to_atom(&self, context: &mut Context) -> Atom {
        let current_function = context.current_function_id.clone();

        let end_function_id = context.next_fresh_function();
        context.add_parameter(&self.result_type);
        let end_function_atom = Atom::Function(end_function_id.clone());

        let effect_id = context.lookup_or_create_effect(self.effect.id);

        context.handler_continuation = Some((effect_id, self.resume.clone()));
        let handler_type = context.convert_type(&self.result_type);
        let parent_handler = context.enter_handler(effect_id, handler_type);

        let handler = self.branch_body.to_atom(context);

        // Now compile the handled expression with the new handler
        let (old_handler, old_handler_k) = context.enter_handler_expression(effect_id, handler.clone(), end_function_atom);

        context.current_function_id = current_function;
        let result = self.expression.to_atom(context);
        let handler_k = context.handler_ks[&effect_id].clone();
        context.terminate_function_with_call(handler_k, vec![result]);

        context.exit_handler_and_expression(effect_id, parent_handler, old_handler, old_handler_k);

        context.current_function_id = end_function_id.clone();
        Atom::Parameter(ParameterId { function: end_function_id, parameter_index: 0 })
    }
}
