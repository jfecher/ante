use std::path::Path;

use crate::nameresolution::builtin::BUILTIN_ID;
use crate::parser::ast;
use crate::types::typed::Typed;
use crate::util::{fmap, reinterpret_from_bits, timing};
use crate::{args::Args, cache::ModuleCache, parser::ast::Ast};

use cranelift::codegen::ir::types as cranelift_types;

mod builtin;
mod context;
mod decisiontree;

use context::{ Context, Value, FunctionValue };
use cranelift::frontend::FunctionBuilder;
use cranelift::prelude::{InstBuilder, MemFlags};

use self::context::BOXED_TYPE;

pub fn run<'c>(_path: &Path, ast: &Ast<'c>, cache: &mut ModuleCache<'c>, args: &Args) {
    timing::start_time("Cranelift codegen");
    Context::codegen_all(ast, cache, args);
}

pub trait Codegen<'c> {
    fn codegen<'local>(&'local self, context: &mut Context<'local, 'c>, builder: &mut FunctionBuilder) -> Value;
}

impl<'c> Codegen<'c> for Ast<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        dispatch_on_expr!(self, Codegen::codegen, context, builder)
    }
}

impl<'c> Codegen<'c> for Box<Ast<'c>> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        self.as_ref().codegen(context, builder)
    }
}

impl<'c> Codegen<'c> for ast::Literal<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        self.kind.codegen(context, builder)
    }
}

impl<'c> Codegen<'c> for ast::LiteralKind {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        Value::Normal(match self {
            ast::LiteralKind::Integer(value, kind) => {
                let typ = context.unboxed_integer_type(kind);
                let value = builder.ins().iconst(typ, *value as i64);
                if typ == BOXED_TYPE {
                    value
                } else {
                    builder.ins().bitcast(BOXED_TYPE, value)
                }
            },
            ast::LiteralKind::Float(float) => {
                let ins = builder.ins();
                let value = ins.f64const(reinterpret_from_bits(*float));
                builder.ins().bitcast(BOXED_TYPE, value)
            },
            ast::LiteralKind::String(s) => context.string_value(s, builder),
            ast::LiteralKind::Char(char) => {
                builder.ins().iconst(cranelift_types::I64, *char as i64)
            },
            ast::LiteralKind::Bool(b) => builder.ins().iconst(BOXED_TYPE, *b as i64),
            ast::LiteralKind::Unit => return context.unit_value(builder),
        })
    }
}

impl<'c> Codegen<'c> for ast::Variable<'c> {
    fn codegen<'a>(&self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        let id = self.definition.unwrap();
        match context.definitions.get(&id) {
            Some(value) => value.clone(),
            None => context.codegen_definition(id, builder),
        }
    }
}

impl<'c> Codegen<'c> for ast::Lambda<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        let name = context.current_function_name.take()
            .unwrap_or_else(|| format!("lambda{}", context.next_unique_id()));

        context.add_lambda_to_queue(self, &name, builder)
    }
}

impl<'c> Codegen<'c> for ast::FunctionCall<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        match self.function.as_ref() {
            Ast::Variable(variable) if variable.definition == Some(BUILTIN_ID) => {
                builtin::call_builtin(&self.args, context, builder)
            },
            _ => {
                let f = self.function.codegen(context, builder).eval_function();

                let args = fmap(&self.args, |arg| {
                    context.codegen_eval(arg, builder)
                });

                let call = match f {
                    FunctionValue::Direct(function_data) => {
                        let function_ref = builder.import_function(function_data);
                        builder.ins().call(function_ref, &args)
                    }
                    FunctionValue::Indirect(function_pointer) => {
                        let signature = context.convert_signature(self.function.get_type().unwrap());
                        let signature = builder.import_signature(signature);
                        builder.ins().call_indirect(signature, function_pointer, &args)
                    }
                };

                let results = builder.inst_results(call);
                assert_eq!(results.len(), 1);
                Value::Normal(results[0])
            },
        }
    }
}

impl<'c> Codegen<'c> for ast::Definition<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        if let (Ast::Variable(variable), Ast::Lambda(_)) = (self.pattern.as_ref(), self.expr.as_ref()) {
            context.current_function_name = Some(variable.to_string());
        }

        let value = context.codegen_eval(&self.expr, builder);
        context.bind_pattern(self.pattern.as_ref(), value, builder);
        context.unit_value(builder)
    }
}

impl<'c> Codegen<'c> for ast::If<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        let cond = context.codegen_eval(&self.condition, builder);

        let then = builder.create_block();
        let if_false = builder.create_block();
        builder.ins().brnz(cond, then, &[]);
        builder.ins().jump(if_false, &[]);

        builder.switch_to_block(then);
        let then_value = context.codegen_eval(&self.then, builder);

        let ret = if let Some(otherwise) = self.otherwise.as_ref() {
            // If we have an 'else' then the if_false branch is our else branch
            let end = builder.create_block();
            builder.append_block_param(end, BOXED_TYPE);
            builder.ins().jump(end, &[then_value]);

            builder.switch_to_block(if_false);
            let else_value = context.codegen_eval(otherwise, builder);
            builder.ins().jump(end, &[else_value]);

            builder.seal_block(end);
            builder.switch_to_block(end);
            let block_params = builder.block_params(end);
            assert_eq!(block_params.len(), 1);
            Value::Normal(block_params[0])
        } else {
            // If there is no 'else', then our if_false branch is the block after the if
            builder.ins().jump(if_false, &[]);
            builder.switch_to_block(if_false);
            context.unit_value(builder)
        };

        builder.seal_block(then);
        builder.seal_block(if_false);
        ret
    }
}

impl<'c> Codegen<'c> for ast::Match<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        decisiontree::codegen(self, context, builder)
    }
}

impl<'c> Codegen<'c> for ast::TypeDefinition<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        context.unit_value(builder)
    }
}

impl<'c> Codegen<'c> for ast::TypeAnnotation<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        self.lhs.codegen(context, builder)
    }
}

impl<'c> Codegen<'c> for ast::Import<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        context.unit_value(builder)
    }
}

impl<'c> Codegen<'c> for ast::TraitDefinition<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        context.unit_value(builder)
    }
}

impl<'c> Codegen<'c> for ast::TraitImpl<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        context.unit_value(builder)
    }
}

impl<'c> Codegen<'c> for ast::Return<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        let value = self.expression.codegen(context, builder);
        context.create_return(value.clone(), builder);
        value
    }
}

impl<'c> Codegen<'c> for ast::Sequence<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        let mut value = None;
        for statement in &self.statements {
            value = Some(statement.codegen(context, builder));
        }
        value.unwrap()
    }
}

impl<'c> Codegen<'c> for ast::Extern<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        context.unit_value(builder)
    }
}

impl<'c> Codegen<'c> for ast::MemberAccess<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        let lhs = context.codegen_eval(&self.lhs, builder);
        let index = context.get_field_index(&self.field, self.lhs.get_type().unwrap());
        Value::Normal(builder.ins().load(BOXED_TYPE, MemFlags::new(), lhs, index as i32))
    }
}

impl<'c> Codegen<'c> for ast::Assignment<'c> {
    fn codegen<'a>(&'a self, context: &mut Context<'a, 'c>, builder: &mut FunctionBuilder) -> Value {
        let rhs = context.codegen_eval(&self.rhs, builder);
        let lhs = context.codegen_eval(&self.lhs, builder);

        let rhs_type = self.rhs.get_type().unwrap();
        let size = context.size_of_unboxed_type(rhs_type);
        let size = builder.ins().iconst(cranelift_types::I64, size as i64);
        builder.call_memcpy(context.frontend_config, lhs, rhs, size);

        context.unit_value(builder)
    }
}
