//! Various methods for validating the well-formedness of [Mir]
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashSet;

use crate::{
    lexer::token::IntegerKind,
    mir::{Definition, Instruction, InstructionId, Mir, PrimitiveType, TerminatorInstruction, Type, Value},
};

impl Mir {
    /// Ensures:
    /// - Each referenced [DefinitionId] corresponds to a [Definition] or extern item in this Mir
    pub(crate) fn assert_fully_linked(self) -> Self {
        self.definitions.par_iter().for_each(|(_, definition)| {
            definition.assert_fully_linked(&self);
        });
        self
    }

    /// Asserts the types given to and returned from each instruction are valid
    pub(crate) fn assert_type_checks(self) -> Self {
        self.definitions.par_iter().for_each(|(_, definition)| {
            definition.assert_type_checks(&self);
        });
        self
    }

    /// Union and generic types should be lowered into more explicit forms before handing off the
    /// Mir to the backend.
    pub(crate) fn assert_no_unions_or_generics(self) -> Self {
        self.definitions.par_iter().for_each(|(_, definition)| {
            definition.assert_no_unions_or_generics(&self);
        });
        self
    }
}

macro_rules! instr_panic {
    ($this: expr, $instruction_id: expr, $mir: expr, $($msg: tt)*) => {{
        $this.annotate_error($instruction_id, $mir, &format!($($msg)*));
        panic!()
    }};
}

macro_rules! instr_assert {
    ($cond: expr, $this: expr, $instruction_id: expr, $mir: expr, $($msg: tt)* ) => {{
        if !$cond {
            $this.annotate_error($instruction_id, $mir, &format!($($msg)*));
            panic!()
        }
    }};
}

macro_rules! instr_assert_eq {
    ($lhs: expr, $rhs: expr, $this: expr, $instruction_id: expr, $mir: expr, $($msg: tt)*) => {{
        if $lhs != $rhs {
            $this.annotate_error($instruction_id, $mir, &format!($($msg)*));
            panic!()
        }
    }};
}

macro_rules! instr_assert_subtype {
    ($lhs: expr, $rhs: expr, $this: expr, $instruction_id: expr, $mir: expr, $($msg: tt)*) => {{
        if $lhs != $rhs {
            $this.annotate_error($instruction_id, $mir, &format!($($msg)*));
            panic!()
        }
    }};
}

impl Definition {
    fn assert_fully_linked(&self, mir: &Mir) {
        let mut referenced_ids = FxHashSet::default();
        referenced_ids.insert(self.id);

        for instruction in self.instructions.values() {
            instruction.for_each_value(|value| {
                if let Value::Definition(definition_id) = value {
                    referenced_ids.insert(*definition_id);
                }
            });

            if let Instruction::Instantiate(id, _) = instruction {
                referenced_ids.insert(*id);
            }
        }

        for block in self.blocks.values() {
            block.terminator.as_ref().unwrap().for_each_value(|value| {
                if let Value::Definition(definition_id) = value {
                    referenced_ids.insert(*definition_id);
                }
            });
        }

        for id in referenced_ids {
            if !mir.definitions.contains_key(&id) && !mir.externals.contains_key(&id) {
                //panic!("Mir::assert_fully_linked: No definition for id {id:?}");
            }
        }
    }

    /// Asserts the argument & result types of each instruction are valid. If they are not, the Mir
    /// is not well-formed.
    fn assert_type_checks(&self, mir: &Mir) {
        self.type_check_instructions(mir);
        self.type_check_block_terminators(mir);
        self.assert_parameter_types_match_definition_type(mir);
    }

    // If this definition is a:
    // - Global: it should have no block parameters
    // - Function: `self.typ` should be a function type with parameters
    //   matching the entry block's parameters.
    fn assert_parameter_types_match_definition_type(&self, mir: &Mir) {
        let entry_block = self.entry_block();
        if self.is_global() {
            let parameters = entry_block.parameter_types.len();
            assert_eq!(parameters, 0, "\n{}\n\nGlobal should have 0 parameters", self.display(Some(mir)));
        } else {
            let (parameters, env) = match &self.typ {
                Type::Function(function_type) => (&function_type.parameters, &function_type.environment),
                _ => panic!("\n{}\n\nFunction does not have a function type!", self.display(Some(mir))),
            };

            let env_adjustment = self.typ.is_closure() as usize;
            assert_eq!(
                parameters.len() + env_adjustment,
                entry_block.parameter_types.len(),
                "Entry block parameter count should match the number of parameters in the function type + 1 if the closure environment is non-empty"
            );

            assert_eq!(
                parameters,
                &entry_block.parameter_types[0..parameters.len()],
                "\n{}\n\nFunction parameters in type do not match entry block parameters",
                self.display(Some(mir))
            );

            if self.typ.is_closure() {
                assert_eq!(
                    env,
                    entry_block.parameter_types.last().unwrap(),
                    "Closure env type does not match the type of the last parameter"
                );
            }
        }
    }

    // The macro calls here are too long so rustfmt puts every argument on a different line, ruining readability
    #[rustfmt::skip]
    fn type_check_instructions(&self, mir: &Mir) {
        for (id, instruction) in self.instructions.iter() {
            let result_type = &self.instruction_result_types[id];

            match instruction {
                Instruction::Call { function, arguments } | Instruction::CallClosure { closure: function, arguments } => {
                    self.type_check_call(function, arguments, id, result_type, mir);
                },
                Instruction::Perform { effect_op, arguments } => {
                    let op_value = Value::Definition(*effect_op);
                    let op_type = mir.type_of_value(&op_value, self);

                    let Type::Function(op_fn) = op_type else {
                        instr_panic!(self, id, mir, "effect_op is not a function");
                    };

                    instr_assert_eq!(
                        op_fn.parameters.len(), arguments.len(),
                        self, id, mir,
                        "Perform arg count does not match effect op arg count"
                    );
                    for (i, (param, arg)) in op_fn.parameters.iter().zip(arguments).enumerate() {
                        let arg_type = mir.type_of_value(arg, self);
                        if *param != Type::ERROR && arg_type != Type::ERROR {
                            instr_assert_subtype!(*param, arg_type, self, id, mir, "Type mismatch in arg {i} of perform");
                        }
                    }
                    instr_assert_subtype!(op_fn.return_type, *result_type, self, id, mir, "Perform result type does not match effect op return type");
                },
                Instruction::Handle { body, cases } => {
                    let Type::Function(body) = mir.type_of_value(body, self) else {
                        instr_panic!(self, id, mir, "Handle body is not a function");
                    };

                    instr_assert_subtype!(body.return_type, *result_type, self, id, mir, "Handle body return type does not match Handle result type");

                    for case in cases {
                        let Type::Function(handler) = mir.type_of_value(&case.handler, self) else {
                            instr_panic!(self, id, mir, "Handle branch is not a function");
                        };
                        instr_assert_subtype!(handler.return_type, *result_type, self, id, mir, "Handle branch return type does not match Handle result type");
                    }
                },
                Instruction::PackClosure { function, environment } => {
                    let function_type = mir.type_of_value(function, self);
                    let environment_type = mir.type_of_value(environment, self);

                    let Type::Function(function_type) = function_type else {
                        instr_panic!(self, id, mir, "PackClosure function is not a function type, it is a(n) `{function_type}`")
                    };

                    instr_assert_subtype!(function_type.environment, environment_type, self, id, mir, "Closure env type doesn't match the environment value packed here");

                    let Type::Function(closure_type) = result_type else {
                        instr_panic!(self, id, mir, "PackClosure result is not a function type, it is a(n) `{result_type}`")
                    };
                    instr_assert!(closure_type.is_closure(), self, id, mir, "PackClosure result is not a closure");
                }
                Instruction::IndexTuple { tuple, index } => {
                    let tuple_type = mir.type_of_value(tuple, self);
                    let Type::Tuple(tuple_type) = tuple_type else {
                        instr_panic!(self, id, mir, "IndexTuple value is not a tuple, it is a(n) `{tuple_type}`")
                    };

                    instr_assert!((*index as usize) < tuple_type.len(), self, id, mir, "Index OOB");
                    instr_assert_subtype!(tuple_type[*index as usize], *result_type, self, id, mir, "Element type from tuple != result type");
                },
                Instruction::MakeBytes(_) => {
                    instr_assert_subtype!(*result_type, Type::POINTER, self, id, mir, "MakeBytes returns a non-pointer, it is `{result_type}`");
                },
                Instruction::MakeTuple(elements) => {
                    let Type::Tuple(tuple_type) = result_type else {
                        instr_panic!(self, id, mir, "MakeTuple result is not a tuple, it is a(n) `{result_type}`")
                    };
                    instr_assert_eq!(tuple_type.len(), elements.len(), self, id, mir, "Tuple type element length mismatch vs the actual elements length");
                    for (result, element) in tuple_type.iter().zip(elements) {
                        let element_type = mir.type_of_value(element, self);
                        instr_assert_subtype!(*result, element_type, self, id, mir, "Tuple elem `{result}` != `{element_type}`");
                    }
                },
                Instruction::MakeArray(elements) => {
                    let Type::Array { length, element: array_element_type } = result_type else {
                        instr_panic!(self, id, mir, "MakeArray result is not an array, it is a(n) `{result_type}`")
                    };
                    let length_value = match length.as_ref() {
                        Type::U32(n) => *n as usize,
                        other => instr_panic!(self, id, mir, "MakeArray length is not a constant: `{other}`"),
                    };
                    instr_assert_eq!(length_value, elements.len(), self, id, mir, "Array type length mismatch vs the actual elements length");
                    for element in elements {
                        let element_type = mir.type_of_value(element, self);
                        instr_assert_subtype!(**array_element_type, element_type, self, id, mir, "Array elem `{array_element_type}` != `{element_type}`");
                    }
                },
                Instruction::StackAlloc(_) => {
                    instr_assert_subtype!(*result_type, Type::POINTER, self, id, mir, "Result type is not a pointer, it is `{result_type}`");
                },
                Instruction::Transmute(_) => (),
                Instruction::Instantiate(def_id, generic_args) => {
                    let target_type = &mir.type_of_value(&Value::Definition(*def_id), self).substitute(generic_args);
                    instr_assert_subtype!(result_type, target_type, self, id, mir, "Result type `{result_type}` does not match manually substited type `{target_type}`");
                },
                Instruction::Id(value) => {
                    let value_type = mir.type_of_value(value, self);
                    instr_assert_subtype!(*result_type, value_type, self, id, mir, "Value type `{value_type}` != result type `{result_type}`");
                },
                Instruction::AddInt(a, b)
                | Instruction::SubInt(a, b)
                | Instruction::MulInt(a, b)
                | Instruction::BitwiseAnd(a, b)
                | Instruction::BitwiseOr(a, b)
                | Instruction::BitwiseXor(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    instr_assert!(a_type.is_int(), self, id, mir, "Argument type is not an integer");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(a_type, *result_type, self, id, mir, "Argument type does not match result type `{a_type}` != `{result_type}`");
                },

                Instruction::AddFloat(a, b)
                | Instruction::SubFloat(a, b)
                | Instruction::MulFloat(a, b)
                | Instruction::DivFloat(a, b)
                | Instruction::ModFloat(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    instr_assert!(a_type.is_float(), self, id, mir, "Argument type is not a float");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(a_type, *result_type, self, id, mir, "Argument type does not match result type `{a_type}` != `{result_type}`");
                },

                Instruction::DivSigned(a, b) | Instruction::ModSigned(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    instr_assert!(a_type.is_signed_int(), self, id, mir, "Argument type is not a signed int");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(a_type, *result_type, self, id, mir, "Argument type does not match result type `{a_type}` != `{result_type}`");
                },

                Instruction::DivUnsigned(a, b) | Instruction::ModUnsigned(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    instr_assert!(a_type.is_unsigned_int(), self, id, mir, "Argument type is not an unsigned int");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(a_type, *result_type, self, id, mir, "Argument type does not match result type `{a_type}` != `{result_type}`");
                }

                Instruction::LessSigned(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    instr_assert!(a_type.is_signed_int(), self, id, mir, "Argument type is not a signed int");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(*result_type, Type::BOOL, self, id, mir, "Result type `{result_type}` is not a Bool");
                },

                Instruction::LessUnsigned(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    instr_assert!(a_type.is_unsigned_int() || a_type == Type::CHAR, self, id, mir, "Argument type is not an unsigned int or Char");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(*result_type, Type::BOOL, self, id, mir, "Result type `{result_type}` is not a Bool");
                },

                Instruction::LessFloat(a, b) | Instruction::EqFloat(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    instr_assert!(a_type.is_float(), self, id, mir, "Argument type is not a float");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(*result_type, Type::BOOL, self, id, mir, "Result type `{result_type}` is not a Bool");
                },

                Instruction::EqInt(a, b) => {
                    let a_type = mir.type_of_value(a, self);
                    let b_type = mir.type_of_value(b, self);
                    let valid = a_type.is_int() || a_type == Type::BOOL || a_type == Type::CHAR;
                    instr_assert!(valid, self, id, mir, "Argument type is not an integer, bool, or char");
                    instr_assert_subtype!(a_type, b_type, self, id, mir, "Argument types do not match: {a_type} != {b_type}");
                    instr_assert_subtype!(*result_type, Type::BOOL, self, id, mir, "Result type `{result_type}` is not a Bool");
                },

                Instruction::BitwiseNot(value) => {
                    let value_type = mir.type_of_value(value, self);
                    instr_assert!(value_type.is_int(), self, id, mir, "Argument type is not an integer");
                    instr_assert_subtype!(value_type, *result_type, self, id, mir, "Argument type does not match result type `{value_type}` != `{result_type}`");
                },

                Instruction::SignExtend(value) => {
                    let value_type = mir.type_of_value(value, self);
                    instr_assert!(value_type.is_signed_int(), self, id, mir, "Argument type is not a signed integer");
                    instr_assert!(result_type.is_int(), self, id, mir, "Result type is not an integer");
                },
                Instruction::ZeroExtend(value) => {
                    let value_type = mir.type_of_value(value, self);
                    instr_assert!(value_type.is_unsigned_int() || value_type == Type::BOOL || value_type == Type::CHAR, self, id, mir, "Argument type is not an unsigned integer");
                    instr_assert!(result_type.is_int(), self, id, mir, "Result type is not an integer");
                },
                Instruction::SignedToFloat(value) => {
                    instr_assert!(mir.type_of_value(value, self).is_signed_int(), self, id, mir, "Argument type is not a signed integer");
                    instr_assert!(result_type.is_float(), self, id, mir, "Result type is not a float");
                },
                Instruction::UnsignedToFloat(value) => {
                    instr_assert!(mir.type_of_value(value, self).is_unsigned_int(), self, id, mir, "Argument type is not an unsigned integer");
                    instr_assert!(result_type.is_float(), self, id, mir, "Result type is not a float");
                },
                Instruction::FloatToSigned(value) => {
                    instr_assert!(mir.type_of_value(value, self).is_float(), self, id, mir, "Argument type is not a float");
                    instr_assert!(result_type.is_signed_int(), self, id, mir, "Result type is not a signed integer");
                },
                Instruction::FloatToUnsigned(value) => {
                    instr_assert!(mir.type_of_value(value, self).is_float(), self, id, mir, "Argument type is not a float");
                    instr_assert!(result_type.is_unsigned_int(), self, id, mir, "Result type is not an unsigned integer");
                },
                Instruction::FloatPromote(value) => {
                    instr_assert!(mir.type_of_value(value, self).is_float(), self, id, mir, "Argument type is not a float");
                    instr_assert!(result_type.is_float(), self, id, mir, "Result type is not a float");
                },
                Instruction::FloatDemote(value) => {
                    instr_assert!(mir.type_of_value(value, self).is_float(), self, id, mir, "Argument type is not a float");
                    instr_assert!(result_type.is_float(), self, id, mir, "Result type is not a float");
                },
                Instruction::Truncate(value) => {
                    let typ = mir.type_of_value(value, self);
                    instr_assert!(typ.can_be_used_as_integer(), self, id, mir, "Argument type is not an integer");
                    instr_assert!(result_type.can_be_used_as_integer(), self, id, mir, "Result type is not an integer");
                },
                Instruction::Deref(value) => {
                    let value_type = mir.type_of_value(value, self);
                    instr_assert!(matches!(value_type, Type::POINTER), self, id, mir, "Argument type is not a pointer");
                },
                Instruction::Store { pointer, value: _ } => {
                    let pointer_type = mir.type_of_value(pointer, self);
                    instr_assert_subtype!(pointer_type, Type::POINTER, self, id, mir, "Store pointer must be a pointer type, got `{pointer_type}`");
                    instr_assert_subtype!(*result_type, Type::UNIT, self, id, mir, "Store result must be unit");
                },
                Instruction::SizeOf(_) => {
                    instr_assert_subtype!(*result_type, Type::int(IntegerKind::Usz), self, id, mir, "SizeOf result must be Usz");
                },
                Instruction::ArrayLen(_) => {
                    instr_assert_subtype!(*result_type, Type::int(IntegerKind::Usz), self, id, mir, "ArrayLen result must be Usz");
                },
                Instruction::StackAllocUninit(_) => {
                    instr_assert_subtype!(*result_type, Type::POINTER, self, id, mir, "StackAllocUninit result must be a pointer");
                },
                Instruction::AllocShared(_) => {
                    instr_assert_subtype!(*result_type, Type::POINTER, self, id, mir, "AllocShared result must be a pointer");
                },
                Instruction::GetFieldPtr { struct_ptr, .. } => {
                    let ptr_type = mir.type_of_value(struct_ptr, self);
                    instr_assert!(matches!(ptr_type, Type::POINTER), self, id, mir, "GetFieldPtr struct_ptr must be a pointer");
                    instr_assert_subtype!(*result_type, Type::POINTER, self, id, mir, "GetFieldPtr result must be a pointer");
                },
                Instruction::Extern(_) => (),
                Instruction::Capability => {
                    instr_assert!(
                        matches!(result_type, Type::Tuple(_)) || *result_type == Type::ERROR,
                        self, id, mir,
                        "Capability result type should be a Tuple (capability), got `{result_type}`"
                    );
                },
            }
        }
    }

    #[rustfmt::skip]
    fn type_check_call(&self, function: &Value, arguments: &[Value], id: InstructionId, result_type: &Type, mir: &Mir) {
        let function_type = mir.type_of_value(function, self);
        let Type::Function(function_type) = function_type else {
            instr_panic!(self, id, mir, "Called value is not a function, it is a(n) `{function_type}`")
        };

        instr_assert_eq!(function_type.parameters.len(), arguments.len(), self, id, mir, "parameter type len does not match arg type len");
        for (i, (parameter, argument)) in function_type.parameters.iter().zip(arguments).enumerate() {
            let arg_type = mir.type_of_value(argument, self);
            // Skip type mismatch checks involving Error types. Error occurs when a
            // value's type is unknown (e.g., captured env params not yet converted).
            if *parameter != Type::ERROR && arg_type != Type::ERROR {
                instr_assert_subtype!(*parameter, arg_type, self, id, mir, "Type mismatch in arg {i} of call");
            }
        }
        instr_assert_subtype!(function_type.return_type, *result_type, self, id, mir, "Function type result type does not match result type of call instruction");
    }

    fn type_check_block_terminators(&self, mir: &Mir) {
        let block_arg_type_checks = |(target, arg): &(_, Option<Value>)| {
            let target_block = &self.blocks[*target];
            match arg {
                Some(arg) => {
                    assert_eq!(target_block.parameter_types.len(), 1);
                    let arg_type = mir.type_of_value(arg, self);
                    assert!(
                        target_block.parameter_types[0] == arg_type,
                        "Block parameter type `{}` does not match jmp argument type `{}`",
                        target_block.parameter_types[0],
                        arg_type
                    );
                },
                None => {
                    assert_eq!(target_block.parameter_types.len(), 0);
                },
            }
        };

        for (block_id, block) in self.blocks.iter() {
            match block.terminator.as_ref() {
                Some(TerminatorInstruction::Jmp(target)) => block_arg_type_checks(target),
                Some(TerminatorInstruction::If { condition, then, else_, end: _ }) => {
                    let cond_type = mir.type_of_value(condition, self);
                    assert!(cond_type == Type::BOOL, "If condition type is not Bool, got `{cond_type}`");
                    block_arg_type_checks(then);
                    block_arg_type_checks(else_);
                },
                Some(TerminatorInstruction::Switch { int_value, cases, else_, end: _ }) => {
                    let int_type = mir.type_of_value(int_value, self);
                    assert!(
                        matches!(int_type, Type::Primitive(PrimitiveType::Int(_))),
                        "Switch value type is not an integer, got `{int_type}`"
                    );

                    for (_, jmp) in cases {
                        block_arg_type_checks(jmp);
                    }

                    block_arg_type_checks(else_);
                },
                Some(TerminatorInstruction::Unreachable) => (),
                Some(TerminatorInstruction::Return(value)) => {
                    let return_type = match self.typ.function_return_type() {
                        Some(return_type) => return_type,
                        None => &self.typ,
                    };
                    let value_type = mir.type_of_value(value, self);
                    assert!(
                        value_type == *return_type,
                        "Returned value's type `{value_type}` does not match function return type `{return_type}`:\n{}",
                        self.display(Some(mir))
                    );
                },
                Some(TerminatorInstruction::Result(value)) => {
                    let value_type = mir.type_of_value(value, self);
                    assert!(
                        value_type == self.typ,
                        "Result value's type `{value_type}` does not match the type of the global `{}`:\n{}",
                        self.typ,
                        self.display(Some(mir))
                    );
                },
                None => panic!("type_check_block_terminators: {block_id} has no terminators!"),
            }
        }
    }

    /// Helper to show the error annotated under the actual failing instruction
    #[track_caller]
    fn annotate_error(&self, instruction_id: InstructionId, mir: &Mir, message: &str) {
        let mir_string = self.display(Some(mir)).to_string();
        let instruction_string = instruction_id.display(self, Some(mir)).to_string();

        // For an instruction string `    foo bar baz`
        // Construct:                `    ^^^^^^^^^^^`
        let trimmed = instruction_string.trim_start();
        let first_non_space = instruction_string.len() - trimmed.len();
        let spaces = " ".repeat(first_non_space);
        let arrows = "^".repeat(trimmed.len() - 1);
        let error_message = format!("{spaces}{arrows} {message}");

        let mut result_string = String::with_capacity(mir_string.len() + instruction_string.len() + 1);

        for (i, s) in mir_string.split(&instruction_string).enumerate() {
            if i != 0 {
                result_string += &instruction_string;
                result_string += &error_message;
                result_string += "\n\n";
            }
            result_string += s;
        }

        panic!("{}", result_string);
    }

    #[rustfmt::skip]
    fn assert_no_unions_or_generics(&self, mir: &Mir) {
        if self.typ.contains_union_or_generic() {
            panic!("{}\nDefinition type contains a union or generic", self.display(Some(mir)));
        }

        for (instruction_id, typ) in self.instruction_result_types.iter() {
            instr_assert!(!typ.contains_union_or_generic(), self, instruction_id, mir, "Result type contains union or generic");
        }

        for (block_id, block) in self.blocks.iter() {
            for parameter in block.parameter_types.iter() {
                if parameter.contains_union_or_generic() {
                    panic!("{}\nParameter to {block_id} contains a union or generic", self.display(Some(mir)));
                }
            }
        }
    }
}

impl Type {
    fn contains_union_or_generic(&self) -> bool {
        match self {
            Type::Primitive(_) | Type::U32(_) => false,
            Type::Union(_) | Type::Generic(_) => true,
            Type::Tuple(fields) => fields.iter().any(Type::contains_union_or_generic),
            Type::Array { length, element } => {
                length.contains_union_or_generic() || element.contains_union_or_generic()
            },
            Type::Function(function) => {
                function.parameters.iter().any(Type::contains_union_or_generic)
                    || function.return_type.contains_union_or_generic()
            },
        }
    }
}
