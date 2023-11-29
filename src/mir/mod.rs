use std::rc::Rc;

use self::ir::{Mir, Atom, ParameterId};
use self::context::Context;
use crate::{hir::{self, Literal}, util::fmap};

pub mod ir;
mod context;
mod printer;

pub fn convert_to_mir(hir: hir::Ast) -> Mir {
    let mut context = Context::new();
    let ret = hir.to_atom(&mut context);

    if let Some(continuation) = context.continuation.take() {
        context.terminate_function_with_call(continuation, vec![ret]);
    }

    while let Some((_, variable)) = context.definition_queue.pop_front() {
        match &variable.definition {
            Some(definition) => {
                let result = definition.to_mir(&mut context);
                assert!(matches!(result, AtomOrCall::Atom(Atom::Literal(Literal::Unit))));
            },
            None => unreachable!("No definition for {}", variable),
        }
    }

    context.mir
}

enum AtomOrCall {
    Atom(Atom),
    Call(Atom, Vec<Atom>),
}

impl AtomOrCall {
    fn into_atom(self, context: &mut Context) -> Atom {
        match self {
            AtomOrCall::Atom(atom) => atom,
            AtomOrCall::Call(f, args) => {
                // The argument types of the continuation for the new function we're creating
                let k_types = context.continuation_types_of(&f, &args);

                let current_function_id = context.current_function_id.clone();
                let function = context.current_function_mut();

                function.body_continuation = f;
                function.body_args = args;

                // Create a new function `|rv| ...` as the continuation
                // for the call. Then resume inserting into this new function.
                // The value of the Atom is the new `rv` parameter holding the result value.
                let k = context.next_fresh_function();
                let function = context.current_function_mut();
                function.argument_types = k_types;

                // Make sure to go back to add the continuation argument
                let prev_function = context.function_mut(&current_function_id);
                prev_function.body_args.push(Atom::Function(k.clone()));

                Atom::Parameter(ParameterId {
                    function: k,
                    parameter_index: 0,
                    name: Rc::new("rv".into()),
                })
            },
        }
    }

    fn unit() -> Self {
        AtomOrCall::Atom(Atom::Literal(Literal::Unit))
    }
}

trait ToMir {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall;

    fn to_atom(&self, context: &mut Context) -> Atom {
        self.to_mir(context).into_atom(context)
    }
}

impl ToMir for hir::Ast {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        dispatch_on_hir!(self, ToMir::to_mir, context)
    }
}

impl ToMir for hir::Literal {
    fn to_mir(&self, _mir: &mut Context) -> AtomOrCall {
        AtomOrCall::Atom(Atom::Literal(self.clone()))
    }
}

impl ToMir for hir::Variable {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let atom = context.definitions.get(&self.definition_id).cloned().unwrap_or_else(|| {
            context.add_global_to_queue(self.clone())
        });
        AtomOrCall::Atom(atom)
    }
}

impl ToMir for hir::Lambda {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let original_function = context.current_function_id.clone();
        let original_continuation = context.continuation.take();

        // make sure to add k parameter
        let name = Rc::new("lambda".to_owned());
        let lambda_id = context.next_fresh_function_with_name(name);

        // Add args to scope
        for (i, arg) in self.args.iter().enumerate() {
            context.definitions.insert(arg.definition_id, Atom::Parameter(ParameterId {
                function: lambda_id.clone(),
                parameter_index: i as u16,
                name: arg.name.as_ref().map_or_else(|| Rc::new(format!("p{i}")), |name| Rc::new(name.to_string())),
            }));
        }

        // If the argument types were not already set, set them now
        let function = context.current_function_mut();
        if function.argument_types.is_empty() {
            function.argument_types.reserve_exact(self.args.len() + 1);

            for arg in &self.args {
                context.add_parameter(&arg.typ);
            }
            context.add_continuation_parameter(&self.typ.return_type);
        }

        let k = Atom::Parameter(ParameterId {
            function: lambda_id.clone(),
            parameter_index: self.args.len() as u16,
            name: context.continuation_name.clone(),
        });

        context.continuation = Some(k.clone());

        let lambda_body = self.body.to_atom(context);
        context.terminate_function_with_call(k, vec![lambda_body]);

        context.current_function_id = original_function;
        context.continuation = original_continuation;

        AtomOrCall::Atom(Atom::Function(lambda_id))
    }
}

impl ToMir for hir::FunctionCall {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let f = self.function.to_atom(context);
        let args = fmap(&self.args, |arg| arg.to_atom(context));
        AtomOrCall::Call(f, args)
    }
}

impl ToMir for hir::Definition {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        if let Some(expected) = context.definitions.get(&self.variable).cloned() {
            let function = match &expected {
                Atom::Function(function_id) => function_id.clone(),
                other => unreachable!("Expected Atom::Function, found {:?}", other),
            };

            let old = context.expected_function_id.take();
            context.expected_function_id = Some(function);
            let rhs = self.expr.to_atom(context);
            assert_eq!(rhs, expected);
            context.expected_function_id = old;
        } else {
            let rhs = self.expr.to_atom(context);
            context.definitions.insert(self.variable, rhs);
        }

        AtomOrCall::unit()
    }
}

impl ToMir for hir::If {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
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
        AtomOrCall::Atom(Atom::Parameter(ParameterId { 
            function: end_function_id,
            parameter_index: 0,
            name: context.intermediate_result_name.clone(),
        }))
    }
}

impl ToMir for hir::Match {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::Return {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let continuation = context.continuation.clone().expect("No continuation for hir::Return!");
        let value = self.expression.to_atom(context);
        AtomOrCall::Call(continuation, vec![value])
    }
}

impl ToMir for hir::Sequence {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let count = self.statements.len();

        // The first statements must be converted to atoms to
        // ensure we create any intermediate continuations needed
        for statement in self.statements.iter().take(count.saturating_sub(1)) {
            statement.to_atom(context);
        }

        // The last statement is kept as an AtomOrCall since it is directly returned
        match self.statements.last() {
            Some(statement) => statement.to_mir(context),
            None => AtomOrCall::unit(),
        }
    }
}

impl ToMir for hir::Extern {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::Assignment {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::MemberAccess {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        todo!()
    }
}

impl ToMir for hir::Tuple {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let fields = fmap(&self.fields, |field| field.to_atom(context));
        AtomOrCall::Atom(Atom::Tuple(fields))
    }
}

impl ToMir for hir::ReinterpretCast {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let value = Box::new(self.lhs.to_atom(context));
        let typ = Context::convert_type(&self.target_type);
        AtomOrCall::Atom(Atom::Transmute(value, typ))
    }
}

impl ToMir for hir::Builtin {
    fn to_mir(&self, context: &mut Context) -> AtomOrCall {
        let binary_fn = |f: fn(_, _) -> _, context: &mut Context, lhs: &hir::Ast, rhs: &hir::Ast| {
            let lhs = Box::new(lhs.to_atom(context));
            let rhs = Box::new(rhs.to_atom(context));
            AtomOrCall::Atom(f(lhs, rhs))
        };

        let unary_fn = |f: fn(_) -> _, context, lhs: &hir::Ast| {
            let lhs = Box::new(lhs.to_atom(context));
            AtomOrCall::Atom(f(lhs))
        };

        let unary_fn_with_type = |f: fn(_, _) -> _, context, lhs: &hir::Ast, rhs: &hir::Type| {
            let lhs = Box::new(lhs.to_atom(context));
            let rhs = Context::convert_type(rhs);
            AtomOrCall::Atom(f(lhs, rhs))
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
                let typ = Context::convert_type(typ);
                AtomOrCall::Atom(Atom::Offset(lhs, rhs, typ))
            },
        }
    }
}
