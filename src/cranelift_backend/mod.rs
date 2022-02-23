use std::collections::HashMap;
use std::path::Path;

use crate::cache::{DefinitionInfoId, DefinitionKind};
use crate::lexer::token::IntegerKind;
use crate::nameresolution::builtin::BUILTIN_ID;
use crate::parser::ast;
use crate::types::{Type, FunctionType, TypeBinding};
use crate::types::typed::Typed;
use crate::util::{fmap, reinterpret_from_bits};
use crate::{args::Args, cache::ModuleCache, parser::ast::Ast};

use cranelift::codegen::entity::EntityRef;
use cranelift::codegen::ir::types as cranelift_types;
use cranelift::codegen::ir::{AbiParam, ExternalName, Function, InstBuilder, Signature};
use cranelift::codegen::isa::CallConv;
use cranelift::codegen::settings;
use cranelift::codegen::verifier::verify_function;
use cranelift::frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift::prelude::{ExtFuncData, Imm64, Value as CraneliftValue};
use cranelift_module::{Linkage, FuncId};

mod builtin;

pub fn run<'c>(path: &Path, ast: &Ast<'c>, cache: &mut ModuleCache<'c>, args: &Args) {
    let mut sig = Signature::new(CallConv::Fast);
    sig.returns.push(AbiParam::new(cranelift_types::I32));
    sig.params.push(AbiParam::new(cranelift_types::I32));
    let mut fn_builder_ctx = FunctionBuilderContext::new();
    let mut func = Function::with_name_signature(ExternalName::user(0, 0), sig);
    {
        let mut builder = FunctionBuilder::new(&mut func, &mut fn_builder_ctx);

        let block0 = builder.create_block();
        let block1 = builder.create_block();
        let block2 = builder.create_block();
        let block3 = builder.create_block();
        let x = Variable::new(0);
        let y = Variable::new(1);
        let z = Variable::new(2);
        builder.declare_var(x, cranelift_types::I32);
        builder.declare_var(y, cranelift_types::I32);
        builder.declare_var(z, cranelift_types::I32);
        builder.append_block_params_for_function_params(block0);

        builder.switch_to_block(block0);
        builder.seal_block(block0);
        {
            let tmp = builder.block_params(block0)[0]; // the first function parameter
            builder.def_var(x, tmp);
        }
        {
            let tmp = builder.ins().iconst(cranelift_types::I32, 2);
            builder.def_var(y, tmp);
        }
        {
            let arg1 = builder.use_var(x);
            let arg2 = builder.use_var(y);
            let tmp = builder.ins().iadd(arg1, arg2);
            builder.def_var(z, tmp);
        }
        builder.ins().jump(block1, &[]);

        builder.switch_to_block(block1);
        {
            let arg1 = builder.use_var(y);
            let arg2 = builder.use_var(z);
            let tmp = builder.ins().iadd(arg1, arg2);
            builder.def_var(z, tmp);
        }
        {
            let arg = builder.use_var(y);
            builder.ins().brnz(arg, block3, &[]);
        }
        builder.ins().jump(block2, &[]);

        builder.switch_to_block(block2);
        builder.seal_block(block2);
        {
            let arg1 = builder.use_var(z);
            let arg2 = builder.use_var(x);
            let tmp = builder.ins().isub(arg1, arg2);
            builder.def_var(z, tmp);
        }
        {
            let arg = builder.use_var(y);
            builder.ins().return_(&[arg]);
        }

        builder.switch_to_block(block3);
        builder.seal_block(block3);

        {
            let arg1 = builder.use_var(y);
            let arg2 = builder.use_var(x);
            let tmp = builder.ins().isub(arg1, arg2);
            builder.def_var(y, tmp);
        }
        builder.ins().jump(block1, &[]);
        builder.seal_block(block1);

        builder.finalize();
    }

    let flags = settings::Flags::new(settings::builder());
    let res = verify_function(&func, &flags);
    println!("{}", func.display());
    if let Err(errors) = res {
        panic!("{}", errors);
    }
}

pub struct Context<'local, 'ast, 'c> {
    cache: &'local mut ModuleCache<'c>,
    definitions: HashMap<DefinitionInfoId, Value>,
    builder_context: FunctionBuilderContext,
    builder: FunctionBuilder<'local>,
    module: &'local mut dyn cranelift_module::Module,
    module_context: cranelift::codegen::Context,
    unique_id: u32,
    current_function_parameters: Vec<CraneliftValue>,

    // TODO: Make this a threadsafe queue so we can compile functions in parallel
    function_queue: Vec<(&'ast ast::Lambda<'c>, Signature, FuncId)>,
}

#[derive(Debug, Clone)]
enum Value {
    Normal(CraneliftValue),
    Function(ExtFuncData),
    Variable(Variable),
    Tuple(Vec<Value>),
}

enum FunctionValue {
    Direct(ExtFuncData),
    Indirect(CraneliftValue), // function pointer
}

impl Value {
    /// Flattens the Value, converting each element into a cranelift value.
    fn eval<'local, 'ast, 'c>(self, context: &mut Context<'local, 'ast, 'c>) -> Vec<CraneliftValue> {
        match self {
            Value::Normal(value) => vec![value],
            Value::Variable(variable) => vec![context.builder.use_var(variable)],
            Value::Tuple(values) => values.into_iter().flat_map(|value| value.eval(context)).collect(),
            Value::Function(data) => {
                let function_ref = context.builder.import_function(data);
                let ptr_type = cranelift_types::I64;
                vec![context.builder.ins().func_addr(ptr_type, function_ref)]
            }
        }
    }

    fn eval_function<'local, 'ast, 'c>(self, context: &mut Context<'local, 'ast, 'c>) -> FunctionValue {
        match self {
            Value::Function(data) => FunctionValue::Direct(data),
            Value::Normal(value) => FunctionValue::Indirect(value),
            other => unreachable!("Expected a function value, got: {:?}", other),
        }
    }
}

impl<'local, 'ast, 'c> Context<'local, 'ast, 'c> {
    fn codegen_main(&mut self) {
        // TODO: actually codegen main

        while let Some((function, signature, id)) = self.function_queue.pop() {
            self.codegen_function(function, signature, id);
        }
    }

    fn next_unique_id(&mut self) -> u32 {
        self.unique_id += 1;
        self.unique_id
    }

    fn resolve_type(&mut self, typ: &Type) -> Type {
        match typ {
            Type::Primitive(p) => Type::Primitive(*p),
            Type::Function(f) => {
                let f = FunctionType {
                    parameters: fmap(&f.parameters, |parameter| self.resolve_type(parameter)),
                    return_type: Box::new(self.resolve_type(f.return_type.as_ref())),
                    environment: Box::new(self.resolve_type(f.environment.as_ref())),
                    is_varargs: f.is_varargs,
                };
                Type::Function(f)
            },
            Type::TypeVariable(id) => match &self.cache.type_bindings[id.0] {
                TypeBinding::Bound(t) => {
                    let t = t.clone();
                    self.resolve_type(&t)
                }
                TypeBinding::Unbound(_, _) => todo!(),
            },
            Type::UserDefinedType(id) => Type::UserDefinedType(*id),
            Type::TypeApplication(c, args) => Type::TypeApplication(Box::new(self.resolve_type(c)), fmap(args, |arg| self.resolve_type(arg))),
            Type::Ref(_) => todo!(),
            Type::ForAll(_vars, typ) => self.resolve_type(typ.as_ref()),
        }
    }

    fn convert_type(&mut self, typ: &Type) -> cranelift_types::Type {
        let _typ = self.resolve_type(typ);
        cranelift_types::I64 // TODO - is this right if we want to box everything?
    }

    fn convert_signature(&mut self, typ: &Type) -> Signature {
        let typ = self.resolve_type(typ);
        let mut sig = Signature::new(CallConv::Fast);

        match typ {
            Type::Function(f) => {
                for parameter in &f.parameters {
                    let cranelift_type = self.convert_type(parameter);
                    sig.params.push(AbiParam::new(cranelift_type));
                }

                let cranelift_type = self.convert_type(f.return_type.as_ref());
                sig.returns.push(AbiParam::new(cranelift_type));
                sig
            },
            _ => unreachable!("called convert_signature with type {}", typ.display(self.cache)),
        }
    }

    fn integer_kind_type(&mut self, kind: &IntegerKind) -> cranelift_types::Type {
        match kind {
            IntegerKind::Unknown => unreachable!("Unknown IntegerKind encountered during codegen"),
            IntegerKind::Inferred(id) => {
                self.convert_type(&Type::TypeVariable(*id))
            },
            IntegerKind::I8 | IntegerKind::U8 => cranelift_types::I8,
            IntegerKind::I16 | IntegerKind::U16 => cranelift_types::I16,
            IntegerKind::I32 | IntegerKind::U32 => cranelift_types::I32,
            IntegerKind::I64 | IntegerKind::Isz | IntegerKind::U64 | IntegerKind::Usz => cranelift_types::I64,
        }
    }

    fn codegen_definition(&mut self, id: DefinitionInfoId) -> Value {
        match &self.cache.definition_infos[id.0].definition {
            Some(DefinitionKind::Definition(definition)) => todo!(),
            Some(DefinitionKind::Extern(annotation)) => todo!(),
            Some(DefinitionKind::TypeConstructor { name, tag }) => todo!(),
            Some(DefinitionKind::TraitDefinition(annotation)) => todo!(),
            Some(DefinitionKind::Parameter) => unreachable!("Parameter definitions should already be codegen'd"),
            Some(DefinitionKind::MatchPattern) => unreachable!("Pattern definitions should already be codegen'd"),
            None => todo!(),
        }
    }

    fn create_return(&mut self, value: Value) {
        // TODO: Check for pre-existing branch instruction
        let values = value.eval(self);
        self.builder.ins().return_(&values);
    }

    fn codegen_function(&mut self, function: &'ast ast::Lambda<'c>, signature: Signature, function_id: FuncId) {
        let mut func = Function::with_name_signature(ExternalName::user(0, 0), signature);

        let mut builder = FunctionBuilder::new(&mut func, &mut self.builder_context);
        let entry = builder.create_block();

        for parameter in &function.args {
            let x = Variable::new(0);
            builder.declare_var(x, cranelift_types::I32);
        }

        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let body = function.body.codegen(self);
        self.create_return(body);
        self.builder.finalize();

        self
            .module
            .define_function(function_id, &mut self.module_context)
            .unwrap();

        let flags = settings::Flags::new(settings::builder());
        let res = verify_function(&func, &flags);
        println!("{}", func.display());
        if let Err(errors) = res {
            panic!("{}", errors);
        }
    }

    fn add_function_to_queue(&mut self, function: &'ast ast::Lambda<'c>, name: &'ast str) -> Value {
        let signature = self.convert_signature(function.get_type().unwrap());
        let function_id = self.module.declare_function(name, Linkage::Export, &signature).unwrap();
        self.function_queue.push((function, signature.clone(), function_id));

        let signature = self.builder.import_signature(signature);

        Value::Function(ExtFuncData {
            name: ExternalName::user(0, self.next_unique_id()),
            signature,
            colocated: true,
        })
    }

    fn unit_value(&mut self) -> Value {
        Value::Normal(self.builder.ins().bconst(cranelift_types::B1, false))
    }
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
        Value::Normal(match self {
            ast::LiteralKind::Integer(value, kind) => {
                let typ = context.integer_kind_type(kind);
                context.builder.ins().iconst(typ, Imm64::new(*value as i64))
            },
            ast::LiteralKind::Float(float) => {
                let ins = context.builder.ins();
                ins.f64const(reinterpret_from_bits(*float))
            },
            ast::LiteralKind::String(_) => todo!(),
            ast::LiteralKind::Char(char) => {
                context.builder.ins().iconst(cranelift_types::I8, Imm64::new(*char as i64))
            },
            ast::LiteralKind::Bool(b) => context.builder.ins().bconst(cranelift_types::B1, *b),
            ast::LiteralKind::Unit => return context.unit_value(),
        })
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
                let f = self.function.codegen(context).eval_function(context);

                let args = self.args.iter().flat_map(|arg| {
                    arg.codegen(context).eval(context)
                }).collect::<Vec<_>>();

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
                if results.len() == 1 {
                    Value::Normal(results[0])
                } else {
                    Value::Tuple(fmap(results, |result| Value::Normal(*result)))
                }
            },
        }
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Definition<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        todo!()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::If<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        todo!()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Match<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
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
        context.create_return(value);
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
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        todo!()
    }
}

impl<'ast, 'c> Codegen<'ast, 'c> for ast::Assignment<'c> {
    fn codegen<'a>(&'ast self, context: &mut Context<'a, 'ast, 'c>) -> Value {
        todo!()
    }
}
