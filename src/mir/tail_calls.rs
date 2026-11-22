//! Guarantees that a function calling itself in tail position runs in constant stack space. This is
//! done by turning each such call into a jump back to the start of the function. `loop` relies on
//! this since it is desugared into a local function which calls itself. `while` does as well since
//! it is desugared into a call to `Std.Stream.(while)`, which calls itself once per iteration.
use crate::mir::{
    Block, BlockId, Definition, Instruction, InstructionId, TerminatorInstruction, Type, Value, are_identity_bindings,
};

/// A self call in tail position: the block it ends, the call, and one argument per parameter
/// of the function. A closure's environment is its last parameter.
struct SelfTailCall {
    block: BlockId,
    call: InstructionId,
    arguments: Vec<Value>,
}

impl Definition {
    pub(crate) fn self_tail_calls_to_jumps(&mut self) {
        if self.is_global() {
            return;
        }
        let calls = self.self_tail_calls();
        if calls.is_empty() {
            return;
        }

        let entry = BlockId::ENTRY_BLOCK;
        let parameter_types = self.entry_block().parameter_types.clone();
        let parameter = |i: usize| Value::Parameter(entry, i as u32);

        // We only need slots for the parameters which are given a new value by some call
        let unchanged =
            |i: usize, argument: Value| parameter_types[i] == Type::UNIT || self.resolve(argument) == parameter(i);
        let changed: Vec<usize> = (0..parameter_types.len())
            .filter(|i| calls.iter().any(|call| !unchanged(*i, call.arguments[*i])))
            .collect();

        // Move the entire body of the function into a loop header which the entry block falls into
        let header = self.blocks.push(Block::new(Vec::new()));
        self.blocks[header].instructions = std::mem::take(&mut self.blocks[entry].instructions);
        self.blocks[header].terminator = self.blocks[entry].terminator.take();

        // Don't create the slots yet. They're initialized from the parameters in the entry block
        // and we don't want those uses of the parameters to be replaced below.
        let loads: Vec<InstructionId> = changed
            .iter()
            .map(|i| self.add_instruction(Instruction::Deref(Value::Unit), parameter_types[*i].clone()))
            .collect();

        self.for_each_used_value_mut(|value| {
            if let Value::Parameter(block, index) = *value
                && block == entry
                && let Some(position) = changed.iter().position(|i| *i == index as usize)
            {
                *value = Value::InstructionResult(loads[position]);
            }
        });

        let mut slots = Vec::with_capacity(changed.len());
        for (i, load) in changed.iter().zip(&loads) {
            let alloc = Instruction::slot(parameter(*i));
            let slot = self.push_instruction_in(entry, alloc, Type::POINTER);
            self.instructions[*load] = Instruction::Deref(slot);
            slots.push(slot);
        }
        self.blocks[header].instructions.splice(0..0, loads);
        self.blocks[entry].terminator = Some(TerminatorInstruction::Jmp((header, None)));

        for call in calls {
            let block = if call.block == entry { header } else { call.block };
            let popped = self.blocks[block].instructions.pop();
            assert_eq!(popped, Some(call.call));

            // We need to retrieve the arguments again since any parameters among them were replaced
            // by their loads above
            let arguments = self.self_call_arguments(call.call).expect("This was a self call above");
            for (i, slot) in changed.iter().zip(&slots) {
                let store = Instruction::Store { pointer: *slot, value: arguments[*i] };
                self.push_instruction_in(block, store, Type::UNIT);
            }
            self.blocks[block].terminator = Some(TerminatorInstruction::Jmp((header, None)));
        }

        // Any blocks which came after each call only returned its result, so they are unreachable
        // now
        self.remove_unreachable_blocks();
        self.compact_instructions();
    }

    fn self_tail_calls(&self) -> Vec<SelfTailCall> {
        let parameter_count = self.entry_block().parameter_types.len();
        let mut calls = Vec::new();
        for (block_id, block) in self.blocks.iter() {
            let Some(call) = block.instructions.last().copied() else { continue };
            let Some(arguments) = self.self_call_arguments(call) else { continue };
            if arguments.len() == parameter_count && self.returns_result_of(block_id, call) {
                calls.push(SelfTailCall { block: block_id, call, arguments });
            }
        }
        calls
    }

    /// If `call` is a call to this same function with the same generics, return its arguments
    fn self_call_arguments(&self, call: InstructionId) -> Option<Vec<Value>> {
        let (function, arguments, is_closure) = match &self.instructions[call] {
            Instruction::Call { function, arguments } => (*function, arguments, false),
            Instruction::CallClosure { closure, arguments } => (*closure, arguments, true),
            _ => return None,
        };
        let function = self.known_function(function)?;
        let same_generics = function.bindings.is_none_or(|bindings| are_identity_bindings(&bindings));

        if function.id != self.id || !same_generics || function.environment.is_some() != is_closure {
            return None;
        }
        let mut arguments = arguments.clone();
        arguments.extend(function.environment);
        Some(arguments)
    }

    /// True if the function returns the result of `call` without doing anything else in between.
    /// Expects `call` to be the last instruction in `block`.
    fn returns_result_of(&self, block: BlockId, call: InstructionId) -> bool {
        let mut result = Some(Value::InstructionResult(call));
        let mut block = block;
        // Limit the number of iterations since a cycle of empty blocks jumping to each other would
        // never return
        for _ in 0..self.blocks.len() {
            match &self.blocks[block].terminator {
                Some(TerminatorInstruction::Jmp((target, argument))) => {
                    if !self.blocks[*target].instructions.is_empty() {
                        return false;
                    }
                    let passes_result = argument.is_some_and(|argument| Some(self.follow_ids(argument)) == result);
                    result = passes_result.then_some(Value::Parameter(*target, 0));
                    block = *target;
                },
                // If the function returns `()` then that is the same value its self call would have
                // returned
                Some(TerminatorInstruction::Return(value)) => {
                    return *value == Value::Unit || Some(self.follow_ids(*value)) == result;
                },
                _ => return false,
            }
        }
        false
    }
}
