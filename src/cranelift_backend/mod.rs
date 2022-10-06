use std::path::Path;

use crate::cli::Cli;
use crate::hir::{self, Ast};
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

pub fn run(path: &Path, hir: Ast, args: &Cli) {
    timing::start_time("Cranelift codegen");
    Context::codegen_all(path, &hir, args);
}

pub trait CodeGen {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value;

    fn eval_all<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Vec<CraneliftValue> {
        self.codegen(context, builder).eval_all(context, builder)
    }

    fn eval_single<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> CraneliftValue {
        self.codegen(context, builder).eval_single(context, builder)
    }
}

impl<'c> CodeGen for Ast {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        dispatch_on_hir!(self, CodeGen::codegen, context, builder)
    }
}

impl CodeGen for Box<Ast> {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        self.as_ref().codegen(context, builder)
    }
}

impl CodeGen for hir::Literal {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        Value::Normal(match self {
            hir::Literal::Integer(value, kind) => {
                let typ = convert_integer_kind(*kind);
                builder.ins().iconst(typ, *value as i64)
            },
            hir::Literal::Float(float) => {
                let ins = builder.ins();
                ins.f64const(f64::from_bits(*float))
            },
            // TODO: C strings should probably be wrapped in a global value
            hir::Literal::CString(s) => context.c_string_value(s, builder),
            hir::Literal::Char(c) => builder.ins().iconst(cranelift_types::I8, *c as i64),
            hir::Literal::Bool(b) => builder.ins().iconst(cranelift_types::I8, *b as i64),
            hir::Literal::Unit => return Value::unit(),
        })
    }
}

impl CodeGen for hir::Variable {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        match context.definitions.get(&self.definition_id) {
            Some(definition) => definition.clone(),
            None => {
                match self.definition.as_ref() {
                    Some(ast) => ast.codegen(context, builder),
                    None => unreachable!("Definition for {} not yet compiled", self.definition_id),
                };
                context.definitions[&self.definition_id].clone()
            },
        }
    }
}

impl CodeGen for hir::Lambda {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, _builder: &mut FunctionBuilder) -> Value {
        let name = match context.current_function_name.take() {
            Some(id) => format!("{}", id),
            None => format!("_anon{}", context.next_unique_id()),
        };

        context.add_function_to_queue(self, &name)
    }
}

impl CodeGen for hir::FunctionCall {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        let f = context.codegen_function_use(self.function.as_ref(), builder);
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

impl CodeGen for hir::Definition {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        // Cannot use entry here, need to borrow context mutably for self.expr.codegen
        #[allow(clippy::map_entry)]
        if !context.definitions.contains_key(&self.variable) {
            if matches!(self.expr.as_ref(), hir::Ast::Lambda(_)) {
                context.current_function_name = Some(self.variable);
            }

            let value = self.expr.codegen(context, builder);
            context.definitions.insert(self.variable, value);
        }
        Value::unit()
    }
}

impl CodeGen for hir::If {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        let cond = self.condition.eval_single(context, builder);

        let then = builder.create_block();
        let if_false = builder.create_block();
        builder.ins().brnz(cond, then, &[]);
        builder.ins().jump(if_false, &[]);

        let then_values = context.eval_all_in_block(&self.then, then, builder);

        let ret = if let Some(otherwise) = self.otherwise.as_ref() {
            // If we have an 'else' then the if_false branch is our else branch
            let end = context.new_block_with_arg(&self.result_type, builder);

            if let Some(then_values) = then_values {
                builder.ins().jump(end, &then_values);
            }

            let else_values = context.eval_all_in_block(otherwise, if_false, builder);

            if let Some(else_values) = else_values {
                builder.ins().jump(end, &else_values);
            }

            builder.seal_block(end);
            builder.switch_to_block(end);
            let end_values = builder.block_params(end);
            context.array_to_value(end_values, &self.result_type)
        } else {
            // If there is no 'else', then our if_false branch is the block after the if
            if then_values.is_some() {
                builder.ins().jump(if_false, &[]);
            }

            builder.switch_to_block(if_false);
            Value::unit()
        };

        builder.seal_block(then);
        builder.seal_block(if_false);
        ret
    }
}

impl CodeGen for hir::Match {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        context.codegen_match(self, builder)
    }
}

impl CodeGen for hir::Return {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        let value = self.expression.codegen(context, builder);
        context.create_return(value.clone(), builder);
        value
    }
}

impl CodeGen for hir::Sequence {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        let mut value = None;
        for statement in &self.statements {
            value = Some(statement.codegen(context, builder));
        }
        value.unwrap()
    }
}

impl CodeGen for hir::Extern {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, _builder: &mut FunctionBuilder) -> Value {
        context.codegen_extern(&self.name, &self.typ)
    }
}

impl CodeGen for hir::MemberAccess {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        let lhs = self.lhs.codegen(context, builder);
        let index = self.member_index as usize;

        match lhs {
            Value::Tuple(mut values) => values.swap_remove(index),
            other => unreachable!("MemberAccess with non-tuple value: {:?}", other),
        }
    }
}

impl CodeGen for hir::Assignment {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        let lhs = self.lhs.eval_single(context, builder);
        let rhs = self.rhs.codegen(context, builder);

        context.store_value(lhs, rhs, &mut 0, builder);
        Value::Unit
    }
}

impl CodeGen for hir::Tuple {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        Value::Tuple(fmap(&self.fields, |field| field.codegen(context, builder)))
    }
}

impl CodeGen for hir::ReinterpretCast {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        let value = self.lhs.codegen(context, builder);
        context.reinterpret_cast(value, &self.target_type, builder)
    }
}

impl CodeGen for hir::Builtin {
    fn codegen<'a>(&'a self, context: &mut Context<'a>, builder: &mut FunctionBuilder) -> Value {
        builtin::call_builtin(self, context, builder)
    }
}
