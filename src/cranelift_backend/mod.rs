use std::path::Path;

use crate::nameresolution::builtin::BUILTIN_ID;
use crate::parser::ast;
use crate::types::typed::Typed;
use crate::util::{fmap, reinterpret_from_bits};
use crate::{args::Args, cache::ModuleCache, parser::ast::Ast};

use cranelift::codegen::ir::types as cranelift_types;
use cranelift::codegen::ir::InstBuilder;

mod builtin;
mod context;

use context::{ Context, Value, FunctionValue };

pub fn run<'c>(_path: &Path, _ast: &Ast<'c>, _cache: &mut ModuleCache<'c>, _args: &Args) {
    todo!("Cranelift backend is unfinished, avoid running it by passing the '--check' argument")
}

trait Codegen<'ast, 'c> {
    fn codegen<'local>(&'ast self, context: &mut Context<'local, 'ast, 'c>) -> Value;
}

impl<'ast, 'c> Codegen<'ast, 'c> for Ast<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        dispatch_on_expr!(self, Codegen::codegen, context)
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Literal<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        self.kind.codegen(context)
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::LiteralKind {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        let value = match self {
            ast::LiteralKind::Integer(value, kind) => {
                let typ = context.integer_kind_type(kind);
                context.builder.ins().iconst(typ, *value as i64)
            },
            ast::LiteralKind::Float(float) => {
                let ins = context.builder.ins();
                ins.f64const(reinterpret_from_bits(*float))
            },
            ast::LiteralKind::String(_) => todo!(),
            ast::LiteralKind::Char(char) => {
                context.builder.ins().iconst(cranelift_types::I64, *char as i64)
            },
            ast::LiteralKind::Bool(b) => context.builder.ins().bconst(cranelift_types::B64, *b),
            ast::LiteralKind::Unit => return context.unit_value(),
        };

        // The primitives above are unboxed but we must still cast them to a boxed type
        let cast = context.builder.ins().bitcast(cranelift_types::R64, value);
        Value::Normal(cast)
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Variable<'c> {
    fn codegen<'a>(&self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        let id = self.definition.unwrap();
        match context.definitions.get(&id) {
            Some(value) => value.clone(),
            None => context.codegen_definition(id),
        }
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Lambda<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        context.add_function_to_queue(self, "lambda")
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::FunctionCall<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        match self.function.as_ref() {
            Ast::Variable(variable) if variable.definition == Some(BUILTIN_ID) => {
                builtin::call_builtin(&self.args, context)
            },
            _ => {
                let f = self.function.codegen(context).eval_function();

                let args = fmap(&self.args, |arg| {
                    arg.codegen(context).eval(context)
                });

                let call = match f {
                    FunctionValue::Direct(function_data) => {
                        let function_ref = context.builder.import_function(function_data);
                        context.builder.ins().call(function_ref, &args)
                    }
                    FunctionValue::Indirect(function_pointer) => {
                        let signature = context.convert_signature(self.function.get_type().unwrap());
                        let signature = context.builder.import_signature(signature);
                        context.builder.ins().call_indirect(signature, function_pointer, &args)
                    }
                };

                let results = context.builder.inst_results(call);
                assert_eq!(results.len(), 1);
                Value::Normal(results[0])
            },
        }
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Definition<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        if let (Ast::Variable(variable), Ast::Lambda(_)) = (self.pattern.as_ref(), self.expr.as_ref()) {
            context.current_definition_name = variable.to_string();
        }

        let value = self.expr.codegen(context).eval(context);
        context.bind_pattern(self.pattern.as_ref(), value);
        context.unit_value()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::If<'c> {
    fn codegen<'a>(&'ast self, _context: &mut Context<'a, 'ast, 'c>) -> Value {
        todo!()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Match<'c> {
    fn codegen<'a>(&'ast self, _context: &mut Context<'a, 'ast, 'c>) -> Value {
        todo!()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::TypeDefinition<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        context.unit_value()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::TypeAnnotation<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        self.lhs.codegen(context)
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Import<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        context.unit_value()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::TraitDefinition<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        context.unit_value()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::TraitImpl<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        context.unit_value()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Return<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        let value = self.expression.codegen(context);
        context.create_return(value.clone());
        value
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Sequence<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        let mut value = None;
        for statement in &self.statements {
            value = Some(statement.codegen(context));
        }
        value.unwrap()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Extern<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        context.unit_value()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::MemberAccess<'c> {
    fn codegen<'a>(&'ast self, _context: &mut Context<'a, 'ast, 'c>) -> Value {
        todo!()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Assignment<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        let rhs = self.rhs.codegen(context).eval(context);
        let lhs = self.lhs.codegen(context).eval(context);

        let rhs_type = self.rhs.get_type().unwrap();
        let size = context.size_of_unboxed_type(rhs_type);
        let size = context.builder.ins().iconst(cranelift_types::I64, size as i64);
        context.builder.call_memcpy(context.frontend_config, lhs, rhs, size);

        context.unit_value()
    }
}
