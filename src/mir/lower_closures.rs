//! Lower closures to explicit struct operations. This pass simplifies the backend a bit by
//! having it worry about fewer instructions.
//!
//! The pass runs in three steps:
//!   1. Eliminate redundant closures left by monomorphization. Monomorphization can specialize
//!      generic environments on functions into `NoClosureEnv`. Go through and replace any
//!      `CallClosure`/`PackClosure` instructions on these with `Call` and `Id` instructions.
//!   2. Rewrite closure types into tuples of (f, env). Closure types get their environment
//!      appended as their last parameter type.
//!   3. Lower `PackClosure` into `MakeTuple` and `CallClosure` into `IndexTuple` & `Call`.
//!
//! After this pass there will be no remaining closure values or types.

use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::mir::{
    BlockId, Definition, DefinitionId, FunctionType, Instruction, InstructionId, Mir, PrimitiveType, Type, Value,
};

impl Mir {
    pub(crate) fn lower_closures(mut self) -> Self {
        // Closures with env `NoClosureEnv` are rewritten into raw functions.
        let definition_types: FxHashMap<DefinitionId, Type> = self
            .externals
            .iter()
            .map(|(id, ext)| (*id, ext.typ.clone()))
            .chain(self.definitions.iter().map(|(id, def)| (*id, def.typ.clone())))
            .collect();

        for definition in self.definitions.values_mut() {
            eliminate_redundant_closures(definition, &definition_types);
        }

        // Any remaining closures must be closures at runtime.
        // Replace closure types with tuple types
        for (_, definition) in self.definitions.iter_mut() {
            rewrite_types_in_definition(definition);
        }
        for (_, ext) in self.externals.iter_mut() {
            ext.typ = rewrite_definition_signature(&ext.typ);
        }

        // And closure instructions with tuple instructions
        lower_closure_instructions(&mut self);
        self
    }

    pub(crate) fn assert_no_closure_types(self) -> Self {
        for (id, definition) in &self.definitions {
            assert_no_closure_in_type(&definition.typ, &format!("definition {id} signature"));
            for (block_id, block) in definition.blocks.iter() {
                for (i, typ) in block.parameter_types.iter().enumerate() {
                    assert_no_closure_in_type(typ, &format!("definition {id} block {block_id} parameter {i}"));
                }
            }
            for (instr_id, typ) in definition.instruction_result_types.iter() {
                assert_no_closure_in_type(typ, &format!("definition {id} instruction {instr_id:?} result type"));
            }
            for (instr_id, instr) in definition.instructions.iter() {
                match instr {
                    Instruction::PackClosure { .. } => {
                        panic!("assert_no_closure_types: definition {id} still has a PackClosure ({instr_id:?})")
                    },
                    Instruction::CallClosure { .. } => {
                        panic!("assert_no_closure_types: definition {id} still has a CallClosure ({instr_id:?})")
                    },
                    Instruction::SizeOf(typ) => {
                        assert_no_closure_in_type(typ, &format!("definition {id} instruction {instr_id:?} SizeOf"));
                    },
                    Instruction::StackAllocUninit(typ) => {
                        assert_no_closure_in_type(
                            typ,
                            &format!("definition {id} instruction {instr_id:?} StackAllocUninit"),
                        );
                    },
                    Instruction::GetFieldPtr { struct_type, .. } => {
                        assert_no_closure_in_type(
                            struct_type,
                            &format!("definition {id} instruction {instr_id:?} GetFieldPtr struct_type"),
                        );
                    },
                    _ => {},
                }
            }
        }
        for (id, ext) in &self.externals {
            assert_no_closure_in_type(&ext.typ, &format!("external {id} ({})", ext.name));
        }
        self
    }
}

fn is_no_closure_env(typ: &Type) -> bool {
    matches!(typ, Type::Primitive(PrimitiveType::NoClosureEnv))
}

/// Convert `CallClosure` to `Call` and `PackClosure` to `Id` for closures whose env
/// was specialized to `NoClosureEnv` by monomorphization, and drop references
/// to orphan `NoClosureEnv` parameters from instruction argument lists.
fn eliminate_redundant_closures(definition: &mut Definition, definition_types: &FxHashMap<DefinitionId, Type>) {
    remove_no_closure_env_parameter_references(definition);

    for (instruction_id, instruction) in definition.instructions.iter_mut() {
        match instruction {
            Instruction::CallClosure { closure, arguments } => {
                let closure_is_real = match closure {
                    Value::InstructionResult(id) => definition.instruction_result_types[*id].is_closure(),
                    Value::Parameter(block_id, idx) => {
                        definition.blocks[*block_id].parameter_types[*idx as usize].is_closure()
                    },
                    Value::Definition(id) => definition_types[id].is_closure(),
                    _ => true,
                };
                if !closure_is_real {
                    let function = *closure;
                    let args = std::mem::take(arguments);
                    *instruction = Instruction::Call { function, arguments: args };
                }
            },
            Instruction::PackClosure { function, .. } => {
                let result_type = &definition.instruction_result_types[instruction_id];
                if !result_type.is_closure() {
                    *instruction = Instruction::Id(*function);
                }
            },
            _ => (),
        }
    }
}

/// Remove references to `NoClosureEnv` entry block parameters from argument lists
/// in `Call` / `CallClosure` / `MakeTuple` instructions.
fn remove_no_closure_env_parameter_references(definition: &mut Definition) {
    let mut to_remove = Vec::new();
    for (i, parameter_type) in definition.entry_block().parameter_types.iter().enumerate() {
        if *parameter_type == Type::NO_CLOSURE_ENV {
            to_remove.push(Value::Parameter(BlockId::ENTRY_BLOCK, i as u32));
        }
    }

    if to_remove.is_empty() {
        return;
    }

    for instruction in definition.instructions.values_mut() {
        match instruction {
            Instruction::Call { arguments, .. } | Instruction::CallClosure { arguments, .. } => {
                arguments.retain(|value| !to_remove.contains(value));
            },
            _ => (),
        }
    }
}

/// Rewrite closure types into: `Tuple(fn, env)`
fn rewrite_value_type(typ: &Type) -> Type {
    match typ {
        Type::Primitive(_) | Type::Generic(_) => typ.clone(),
        Type::Tuple(fields) => Type::Tuple(Arc::new(fields.iter().map(rewrite_value_type).collect())),
        Type::Union(fields) => Type::Union(Arc::new(fields.iter().map(rewrite_value_type).collect())),
        Type::Function(f) => {
            let return_type = rewrite_value_type(&f.return_type);
            let env_type = rewrite_value_type(&f.environment);
            let parameters: Vec<Type> = f.parameters.iter().map(rewrite_value_type).collect();

            if is_no_closure_env(&f.environment) {
                Type::Function(Arc::new(FunctionType { parameters, environment: env_type, return_type }))
            } else {
                let folded_parameters =
                    parameters.into_iter().chain(std::iter::once(env_type.clone())).collect::<Vec<_>>();
                let normalized_fn = Type::Function(Arc::new(FunctionType {
                    parameters: folded_parameters,
                    environment: Type::NO_CLOSURE_ENV,
                    return_type,
                }));
                Type::Tuple(Arc::new(vec![normalized_fn, env_type]))
            }
        },
    }
}

fn rewrite_definition_signature(typ: &Type) -> Type {
    let Type::Function(f) = typ else {
        return rewrite_value_type(typ);
    };
    let return_type = rewrite_value_type(&f.return_type);
    let env_type = rewrite_value_type(&f.environment);
    let mut parameters: Vec<Type> = f.parameters.iter().map(rewrite_value_type).collect();
    if !is_no_closure_env(&f.environment) {
        parameters.push(env_type);
    }
    Type::Function(Arc::new(FunctionType { parameters, environment: Type::NO_CLOSURE_ENV, return_type }))
}

fn rewrite_types_in_definition(definition: &mut Definition) {
    definition.typ = rewrite_definition_signature(&definition.typ);

    for (_, block) in definition.blocks.iter_mut() {
        for typ in &mut block.parameter_types {
            *typ = rewrite_value_type(typ);
        }
    }

    for (_, typ) in definition.instruction_result_types.iter_mut() {
        *typ = rewrite_value_type(typ);
    }

    for (_, instr) in definition.instructions.iter_mut() {
        match instr {
            Instruction::SizeOf(typ) => {
                *typ = rewrite_value_type(typ);
            },
            Instruction::StackAllocUninit(typ) => {
                *typ = rewrite_value_type(typ);
            },
            Instruction::GetFieldPtr { struct_type, .. } => {
                *struct_type = rewrite_value_type(struct_type);
            },
            _ => {},
        }
    }

    fix_fn_ptr_id_chains(definition);
}

/// The block above indiscriminately rewrote recorded result types from `Type::Function(env)` to
/// `Tuple [raw_fn_ptr, env]` but the initial Id/definition instructions should just be
/// functions, not tuples.
fn fix_fn_ptr_id_chains(definition: &mut Definition) {
    let mut fn_ptr_results: FxHashSet<InstructionId> = FxHashSet::default();
    for (id, instr) in definition.instructions.iter() {
        let is_fn_ptr = match instr {
            Instruction::Instantiate(_, _) => true,
            Instruction::Id(Value::Definition(_)) => true,
            Instruction::Id(Value::InstructionResult(prev)) => fn_ptr_results.contains(prev),
            _ => false,
        };
        if is_fn_ptr {
            fn_ptr_results.insert(id);
        }
    }
    for id in fn_ptr_results {
        // If the original type was already a raw fn ptr, it was left untouched
        // by the rewrite, leave it alone here too.
        let typ = &definition.instruction_result_types[id];
        if let Type::Tuple(fields) = typ {
            if fields.len() == 2 {
                if matches!(&fields[0], Type::Function(_)) && !matches!(&fields[1], Type::Function(_)) {
                    definition.instruction_result_types[id] = fields[0].clone();
                }
            }
        }
    }
}

fn lower_closure_instructions(mir: &mut Mir) {
    // TODO: Refactor
    let mut call_sites: Vec<(DefinitionId, BlockId, InstructionId, Type, Type, Value, Vec<Value>)> = Vec::new();

    for (def_id, definition) in &mir.definitions {
        for (block_id, block) in definition.blocks.iter() {
            for instr_id in &block.instructions {
                if let Instruction::CallClosure { closure, arguments } = &definition.instructions[*instr_id] {
                    let closure_type = mir.type_of_value(closure, definition);
                    let Type::Tuple(fields) = closure_type else {
                        panic!(
                            "lower_closures: closure value in CallClosure is not Tuple-typed after type rewrite, got `{}`",
                            closure_type
                        );
                    };
                    assert_eq!(fields.len(), 2, "lower_closures: closure tuple must have 2 fields");
                    call_sites.push((
                        *def_id,
                        block_id,
                        *instr_id,
                        fields[0].clone(),
                        fields[1].clone(),
                        *closure,
                        arguments.clone(),
                    ));
                }
            }
        }
    }

    for definition in mir.definitions.values_mut() {
        for (_, instr) in definition.instructions.iter_mut() {
            if let Instruction::PackClosure { function, environment } = instr {
                let function = *function;
                let environment = *environment;
                *instr = Instruction::MakeTuple(vec![function, environment]);
            }
        }
    }

    // Expand each CallClosure into IndexTuple + IndexTuple + Call.
    for (def_id, block_id, call_id, fn_type, env_type, closure, arguments) in call_sites {
        let definition = mir.definitions.get_mut(&def_id).expect("lower_closures: def disappeared");
        lower_one_call_closure(definition, block_id, call_id, fn_type, env_type, closure, arguments);
    }
}

fn lower_one_call_closure(
    definition: &mut Definition, block_id: BlockId, call_id: InstructionId, fn_type: Type, env_type: Type,
    closure: Value, arguments: Vec<Value>,
) {
    let extract_fn = definition.instructions.push(Instruction::IndexTuple { tuple: closure, index: 0 });
    definition.instruction_result_types.push_existing(extract_fn, fn_type);
    let extract_env = definition.instructions.push(Instruction::IndexTuple { tuple: closure, index: 1 });
    definition.instruction_result_types.push_existing(extract_env, env_type);

    let mut new_args = arguments;
    new_args.push(Value::InstructionResult(extract_env));
    definition.instructions[call_id] =
        Instruction::Call { function: Value::InstructionResult(extract_fn), arguments: new_args };

    let block = &mut definition.blocks[block_id];
    let pos = block
        .instructions
        .iter()
        .position(|id| *id == call_id)
        .expect("lower_closures: CallClosure disappeared from its block");

    block.instructions.splice(pos..pos, [extract_fn, extract_env]);
}

fn assert_no_closure_in_type(typ: &Type, where_: &str) {
    match typ {
        Type::Primitive(_) | Type::Generic(_) => {},
        Type::Tuple(fields) | Type::Union(fields) => {
            for f in fields.iter() {
                assert_no_closure_in_type(f, where_);
            }
        },
        Type::Function(f) => {
            assert!(is_no_closure_env(&f.environment), "Closure remaining at {where_}: `{typ}`");
            assert_no_closure_in_type(&f.return_type, where_);
            for p in &f.parameters {
                assert_no_closure_in_type(p, where_);
            }
        },
    }
}
