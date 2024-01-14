use std::path::Path;

use crate::cli::Cli;
use crate::mir::{self, Ast, Mir, Atom};
use crate::lexer::token::FloatKind;
use crate::util::{fmap, timing};

use cranelift::codegen::ir::{types as cranelift_types, Value as CraneliftValue};

mod builtin;
mod context;
mod decisiontree;
mod module;

use context::{Context, FunctionValue, Value};
use cranelift::frontend::FunctionBuilder;
use cranelift::prelude::InstBuilder;

use self::context::convert_integer_kind;

pub fn run(path: &Path, mir: Mir, args: &Cli) {
    timing::start_time("Cranelift codegen");
    Context::codegen_all(path, &mir, args);
}

pub trait CodeGen {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value;

    fn eval_all<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Vec<CraneliftValue> {
        self.codegen(context, builder).eval_all(context, builder)
    }

    fn eval_single<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> CraneliftValue {
        self.codegen(context, builder).eval_single(context, builder)
    }
}

impl CodeGen for Ast {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        dispatch_on_mir!(self, CodeGen::codegen, context, builder)
    }
}

impl CodeGen for Atom {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        dispatch_on_atom!(self, CodeGen::codegen, context, builder)
    }
}

impl CodeGen for Box<Ast> {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        self.as_ref().codegen(context, builder)
    }
}

impl CodeGen for mir::Literal {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        Value::Normal(match self {
            mir::Literal::Integer(value, kind) => {
                let typ = convert_integer_kind(*kind);
                builder.ins().iconst(typ, *value as i64)
            },
            mir::Literal::Float(float, FloatKind::F32) => builder.ins().f32const(f64::from_bits(*float) as f32),
            mir::Literal::Float(float, FloatKind::F64) => builder.ins().f64const(f64::from_bits(*float)),
            // TODO: C strings should probably be wrapped in a global value
            mir::Literal::CString(s) => context.c_string_value(s, builder),
            mir::Literal::Char(c) => builder.ins().iconst(cranelift_types::I8, *c as i64),
            mir::Literal::Bool(b) => builder.ins().iconst(cranelift_types::I8, *b as i64),
            mir::Literal::Unit => return Value::unit(),
        })
    }
}

impl CodeGen for mir::Variable {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, _: &mut FunctionBuilder) -> Value {
        match context.definitions.get(&self.definition_id) {
            Some(definition) => definition.clone(),
            None => context.definitions.get(&self.definition_id).cloned().unwrap_or_else(|| {
                unreachable!("Cranelift backend: No definition for variable '{}'", self)
            }),
        }
    }
}

impl CodeGen for mir::Lambda {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, _builder: &mut FunctionBuilder) -> Value {
        let name = match context.current_function_name.take() {
            Some(id) => format!("lambda${}", id),
            None => format!("_${}", context.next_unique_id()),
        };

        context.add_function_to_queue(self, &name)
    }
}

impl CodeGen for mir::FunctionCall {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        let f = context.codegen_function_use(&self.function, builder);
        let args = self.args.iter().flat_map(|arg| arg.eval_all(context, builder)).collect::<Vec<_>>();

        let call = match f {
            FunctionValue::Direct(function_data) => {
                let function_ref = function_data.import(builder);
                builder.ins().call(function_ref, &args)
            },
            FunctionValue::Indirect(function_pointer) => {
                let signature = context.convert_signature(&self.function_type);
                let signature = builder.import_signature(signature);
                builder.ins().call_indirect(signature, function_pointer, &args)
            },
        };

        let returns = builder.inst_results(call);
        context.array_to_value(returns, &self.function_type.return_type)
    }
}

impl CodeGen for mir::Let<Ast> {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        context.codegen_let_expr(self, builder);
        // TODO: May need immutable definitions here
        self.body.codegen(context, builder)
    }
}

impl CodeGen for mir::If {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        let cond = self.condition.eval_single(context, builder);

        let then = builder.create_block();
        let if_false = builder.create_block();
        builder.ins().brnz(cond, then, &[]);
        builder.ins().jump(if_false, &[]);

        let then_values = context.eval_all_in_block(&self.then, then, builder);

        let end = context.new_block_with_arg(&self.result_type, builder);

        if let Some(then_values) = then_values {
            builder.ins().jump(end, &then_values);
        }

        let else_values = context.eval_all_in_block(&self.otherwise, if_false, builder);

        if let Some(else_values) = else_values {
            builder.ins().jump(end, &else_values);
        }

        builder.seal_block(end);
        builder.switch_to_block(end);
        let end_values = builder.block_params(end);
        let ret = context.array_to_value(end_values, &self.result_type);

        builder.seal_block(then);
        builder.seal_block(if_false);
        ret
    }
}

impl CodeGen for mir::Match {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        context.codegen_match(self, builder)
    }
}

impl CodeGen for mir::Return {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        let value = self.expression.codegen(context, builder);
        context.create_return(value.clone(), builder);
        value
    }
}

impl CodeGen for mir::Extern {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, _builder: &mut FunctionBuilder) -> Value {
        context.codegen_extern(&self.name, &self.typ)
    }
}

impl CodeGen for mir::MemberAccess {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        let lhs = self.lhs.codegen(context, builder);
        let index = self.member_index as usize;

        match lhs {
            Value::Tuple(mut values) => values.swap_remove(index),
            other => unreachable!("MemberAccess with non-tuple value: {:?}", other),
        }
    }
}

impl CodeGen for mir::Assignment {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        let lhs = self.lhs.eval_single(context, builder);
        let rhs = self.rhs.codegen(context, builder);

        context.store_value(lhs, rhs, &mut 0, builder);
        Value::Unit
    }
}

impl CodeGen for mir::Tuple {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        Value::Tuple(fmap(&self.fields, |field| field.codegen(context, builder)))
    }
}

impl CodeGen for mir::Builtin {
    fn codegen<'ast>(&'ast self, context: &mut Context<'ast>, builder: &mut FunctionBuilder) -> Value {
        builtin::call_builtin(self, context, builder)
    }
}

impl CodeGen for mir::Handle {
    fn codegen<'ast>(&'ast self, _context: &mut Context<'ast>, _builder: &mut FunctionBuilder) -> Value {
        unreachable!("mir::Handle should be removed before cranelift codegen")
    }
}

impl CodeGen for mir::Effect {
    fn codegen<'ast>(&'ast self, _context: &mut Context<'ast>, _builder: &mut FunctionBuilder) -> Value {
        unreachable!("mir::Effect should be removed before cranelift codegen")
    }
}
