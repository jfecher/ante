//! Rewriting calls and function values. A call resolves to one of three callees, see [Target]:
//! a definition called with the direct calling convention, a C function, or a function value.

use crate::{
    iterator_extensions::mapvec,
    mir::{
        ConstantEnvironment, Instruction, InstructionId, Type, Value,
        existentialization::{
            builder::Arg,
            resolve::{Target, known_closure},
            rewrite::Rewriter,
            types::is_dynamic,
        },
    },
};

impl Rewriter<'_> {
    pub(super) fn call(
        &mut self, id: InstructionId, function: &Value, arguments: &[Value], result_type: &Type,
    ) -> Value {
        let destination = self.destination(id);
        match self.resolve(function) {
            Target::Function(callee, bindings) => {
                assert!(!self.shared().uniform(callee), "existentialization: direct call to a uniform function");
                let args = mapvec(arguments, |argument| self.arg(argument));
                self.builder.call_direct(callee, &bindings, args, result_type, destination)
            },
            Target::Extern => self.call_extern(function, arguments, result_type),
            Target::Value => {
                if let Some(result) = self.call_known_closure(function, arguments, result_type, destination) {
                    return result;
                }
                let (function, function_type) = self.arg(function);
                let args = mapvec(arguments, |argument| self.arg(argument));
                self.builder.call_value(function, &function_type, args, result_type, destination)
            },
        }
    }

    pub(super) fn pack_closure(&mut self, function: &Value, environment: &Value, typ: &Type) -> Value {
        let Target::Function(target, bindings) = self.resolve(function) else {
            panic!("existentialization: closure of an unknown function `{function}`")
        };
        let environment = self.arg(environment);
        self.builder.function_value(target, &bindings, Some(environment), typ)
    }

    /// Call a closure read out of a constant global directly, see [known_closure]
    fn call_known_closure(
        &mut self, function: &Value, arguments: &[Value], result_type: &Type, destination: Option<Value>,
    ) -> Option<Value> {
        let shared = self.shared();
        let known = known_closure(function, self.old, shared.mir, &|id| shared.kind(id))?;
        let callee = &shared.mir.definitions[&known.function];
        let parameters = &callee.entry_block().parameter_types;
        let environment_type = (parameters.len() == arguments.len() + 1).then(|| parameters[arguments.len()].clone());

        // Checked before converting any argument so falling back to a function value call emits nothing twice
        let runtime = matches!(known.environment, ConstantEnvironment::Runtime);
        if environment_type
            .as_ref()
            .is_some_and(|typ| is_dynamic(typ) || (runtime && is_dynamic(&self.old_type(function))))
        {
            return None;
        }
        let mut args = mapvec(arguments, |argument| self.arg(argument));
        if let Some(environment_type) = environment_type {
            let environment = match known.environment {
                ConstantEnvironment::Constant(constant) => Arg::Value(constant),
                // Environments the callee never reads
                ConstantEnvironment::Transmuted(..) if environment_type == Type::POINTER => {
                    Arg::Value(self.builder.null())
                },
                ConstantEnvironment::Transmuted(constant, _) => {
                    let typ = self.lower(&environment_type);
                    Arg::Value(self.builder.emit(Instruction::Transmute(constant), typ))
                },
                ConstantEnvironment::Runtime => self.closure_environment(function, arguments.len(), &environment_type),
            };
            args.push((environment, environment_type));
        }
        Some(self.builder.call_direct(known.function, &known.bindings, args, result_type, destination))
    }

    /// The environment of `closure` of the given arity, read out of its function value
    fn closure_environment(&mut self, closure: &Value, arity: usize, environment_type: &Type) -> Arg {
        let (closure, closure_type) = self.arg(closure);
        let index = self.shared().types.reserved(arity) + 1;
        if self.builder.indirect(&closure_type) {
            let closure = self.builder.address_of(closure, &closure_type);
            let offset = self.builder.offsets(&closure_type)[index];
            Arg::Address(self.builder.offset(closure, offset))
        } else {
            let closure = self.builder.direct(closure, &closure_type);
            let typ = self.lower(environment_type);
            let index = index as u32;
            Arg::Value(self.builder.emit(Instruction::IndexTuple { tuple: closure, index }, typ))
        }
    }

    // C interop

    fn call_extern(&mut self, function: &Value, arguments: &[Value], result_type: &Type) -> Value {
        let function = self.extern_function(function);
        let arguments = mapvec(arguments, |argument| match self.old_type(argument) {
            Type::Function(_) => self.native_function(argument),
            // C takes aggregates by value
            typ if self.builder.indirect(&typ) => {
                let address = self.address(argument);
                self.builder.load(address, &typ)
            },
            _ => self.direct(argument),
        });
        let typ = self.shared().types.lower_c(result_type);
        let result = self.builder.emit(Instruction::Call { function, arguments }, typ);
        if !self.builder.indirect(result_type) {
            return result;
        }
        let slot = self.builder.slot(result_type);
        self.builder.emit(Instruction::Store { pointer: slot, value: result }, Type::UNIT);
        slot
    }

    pub(super) fn extern_symbol(&mut self, name: &str, typ: &Type) -> Value {
        let typ = self.shared().types.lower_c(typ);
        self.builder.emit(Instruction::Extern(name.to_string()), typ)
    }

    /// The C function `value` refers to
    fn extern_function(&mut self, value: &Value) -> Value {
        let old = self.old;
        match old.follow_ids(*value) {
            Value::InstructionResult(id) => match &old.instructions[id] {
                Instruction::Instantiate(id, _) => Value::Definition(*id),
                Instruction::Extern(name) => self.extern_symbol(name, old.instruction_result_type(id)),
                other => panic!("existentialization: `{other:?}` is not a C function"),
            },
            other => other,
        }
    }

    /// A function pointer C can call in place of `value`
    pub(super) fn native_function(&mut self, value: &Value) -> Value {
        match self.resolve(value) {
            Target::Function(id, _) if self.shared().needed(id).is_empty() => {
                Value::Definition(self.shared().native(id))
            },
            Target::Extern => self.extern_function(value),
            _ => panic!(
                "existentialization: `{}` passes a function to C which is not a definition or needs type information",
                self.old.name
            ),
        }
    }
}
