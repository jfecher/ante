//! Calls and function values, following the conventions in [super::super::convention].

use crate::{
    iterator_extensions::mapvec,
    mir::{
        Definition, DefinitionId, Instruction, Name, Type, Value,
        existentialization::{
            Shared,
            analysis::Kind,
            builder::{Arg, At, FunctionBuilder, Op, arithmetic::word_value},
            convention::{arity, by_address, existential_function_type, type_info_type_fields},
            resolve::Bindings,
        },
    },
};

impl<'a> FunctionBuilder<'a> {
    /// A builder for code of function values standing in for `definition`, see
    /// [existential_function_type]. Also returns the address of each of `definition`'s
    /// parameters, or the parameter itself for evidence, and the result slot.
    pub(crate) fn new_uniform(
        shared: &'a Shared<'a>, name: Name, id: DefinitionId, definition: &Definition,
    ) -> (Self, Vec<Value>, Value) {
        let Type::Function(function) = &definition.typ else {
            panic!("existentialization: `{}` is used as a function value but is not a function", definition.name)
        };
        let arity = function.parameters.len();
        let mut this = Self::new(shared, name, id, existential_function_type(arity), shared.needed(definition.id));
        let parameters = mapvec(0..arity + 2, |_| this.push_parameter(Type::POINTER));
        let (result, value) = (parameters[arity], parameters[arity + 1]);
        let mut fields = shared.types.function_value_fields(function);
        fields.truncate(shared.types.reserved(arity) + 1);

        // The type infos within a function value form a type info table without any precomputed layouts
        if !this.needed.is_empty() {
            let offset = this.offsets(&Type::tuple(fields.clone()))[1];
            let infos = this.offset_at(At::Prologue, value, offset);
            this.set_metadata(infos, &[]);
        }

        let old_parameters = &definition.entry_block().parameter_types;
        assert!(old_parameters.len() <= arity + 1, "existentialization: `{}` has extra parameters", definition.name);
        let mut addresses = parameters[..arity].to_vec();
        if let Some(environment) = old_parameters.get(arity) {
            let index = fields.len();
            fields.push(environment.clone());
            let offset = this.offsets(&Type::tuple(fields))[index];
            addresses.push(this.offset(value, offset));
        }
        (this, addresses, result)
    }
}

impl FunctionBuilder<'_> {
    /// Call `callee` with the direct calling convention, writing a dynamic result to `destination` if given
    pub(crate) fn call_direct(
        &mut self, callee: DefinitionId, bindings: &Bindings, args: Vec<(Arg, Type)>, result_type: &Type,
        destination: Option<Value>,
    ) -> Value {
        let definition = &self.shared.mir.definitions[&callee];
        let parameters = &definition.entry_block().parameter_types;
        assert_eq!(
            args.len(),
            parameters.len(),
            "existentialization: wrong number of arguments in a call to `{}`",
            definition.name
        );

        let mut arguments = Vec::with_capacity(args.len() + 2);
        arguments.extend(self.metadata_argument(callee, bindings));
        for ((arg, typ), parameter) in args.into_iter().zip(parameters) {
            if !self.shared.passed(parameter) {
                continue;
            }
            let argument = if by_address(parameter) { self.address_of(arg, &typ) } else { self.direct(arg, &typ) };
            arguments.push(argument);
        }

        let function = Value::Definition(callee);
        if self.shared.has_result_slot(callee) {
            let slot = destination.unwrap_or_else(|| self.slot(result_type));
            arguments.push(slot);
            self.emit(Instruction::Call { function, arguments }, Type::POINTER);
            self.read_result(slot, result_type)
        } else {
            let lowered = self.shared.types.lower(result_type);
            let result = self.emit(Instruction::Call { function, arguments }, lowered);
            if let Some(destination) = destination {
                self.emit(Instruction::Store { pointer: destination, value: result }, Type::UNIT);
            }
            result
        }
    }

    /// Call a function value, passing each argument but evidence by address, then the result pointer
    /// and its own address
    pub(crate) fn call_value(
        &mut self, function: Arg, function_type: &Type, args: Vec<(Arg, Type)>, result_type: &Type,
        destination: Option<Value>,
    ) -> Value {
        let this = self.address_of(function, function_type);
        let code = self.emit(Instruction::Deref(this), existential_function_type(arity(function_type)));
        let mut arguments = mapvec(args, |(arg, typ)| match typ {
            Type::Evidence(_) => self.direct(arg, &typ),
            _ => self.address_of(arg, &typ),
        });
        let slot = destination.unwrap_or_else(|| self.slot(result_type));
        arguments.push(slot);
        arguments.push(this);
        self.emit(Instruction::Call { function: code, arguments }, Type::POINTER);
        self.read_result(slot, result_type)
    }

    /// A function value of type `value_type` calling `target`
    pub(crate) fn function_value(
        &mut self, target: DefinitionId, bindings: &Bindings, environment: Option<(Arg, Type)>, value_type: &Type,
    ) -> Value {
        let Type::Function(function) = value_type else {
            panic!("existentialization: function value of non-function type `{value_type}`")
        };
        let reserved = self.shared.types.reserved(function.parameters.len());
        let code = Value::Definition(self.shared.thunk(target));
        let environment = match function.environment() {
            Some(_) => environment.expect("existentialization: closure without an environment"),
            None => (Arg::Value(Value::Unit), Type::NO_CLOSURE_ENV),
        };
        let needed = self.shared.needed(target);
        assert!(needed.len() <= reserved, "existentialization: too few type infos reserved for `{value_type}`");

        if !self.indirect(value_type) {
            let mut fields = vec![code];
            fields.extend(mapvec(needed, |generic| self.type_info(&bindings.get(*generic))));
            let zero = self.type_info(&Type::NO_CLOSURE_ENV);
            fields.resize(reserved + 1, zero);
            fields.push(self.direct(environment.0, &environment.1));
            let lowered = self.shared.types.lower(value_type);
            return self.emit(Instruction::MakeTuple(fields), lowered);
        }

        // Written a word at a time, since LLVM's FastISel rejects aggregate stores
        let slot = self.slot(value_type);
        let offsets = self.offsets(value_type);
        let info_fields = type_info_type_fields();
        let info_offsets = Type::field_offsets(&info_fields, self.shared.types.ptr_size);
        self.store_word(slot, Op::Const(0), code);
        for index in 0..reserved {
            let words = match needed.get(index) {
                Some(generic) => self.type_info_ops(&bindings.get(*generic)),
                None => self.type_info_ops(&Type::NO_CLOSURE_ENV),
            };
            for ((word, typ), info_offset) in words.into_iter().zip(&info_fields).zip(&info_offsets) {
                let offset = self.add(offsets[index + 1], Op::Const(*info_offset as u64));
                self.store_word(slot, offset, word_value(word, typ));
            }
        }
        let pointer = self.offset(slot, offsets[reserved + 1]);
        self.store(pointer, environment.0, &environment.1);
        slot
    }

    /// The value of `definition` when named on its own
    pub(crate) fn definition_value(&mut self, definition: DefinitionId, bindings: &Bindings, typ: &Type) -> Value {
        match self.shared.kind(definition) {
            Kind::Function => self.function_value(definition, bindings, None, typ),
            Kind::ComputedGlobal => self.call_direct(definition, bindings, Vec::new(), typ, None),
            Kind::Global if self.indirect(typ) => self.emit(Instruction::GlobalAddress(definition), Type::POINTER),
            Kind::Global | Kind::Extern => Value::Definition(definition),
        }
    }
}
