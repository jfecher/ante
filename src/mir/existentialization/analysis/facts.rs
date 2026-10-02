//! This module collects what each definition contributes to the analysis on its own, see [Facts].

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::mir::{
    Definition, DefinitionId, Instruction, Mir, TerminatorInstruction, Type, Value,
    existentialization::{
        analysis::Kind,
        resolve::{Bindings, Target, known_closure, resolve},
        types::{for_each_generic, is_key},
    },
};

/// What a single definition contributes to the analysis
pub(super) struct Facts {
    /// The generics its own values' layouts and effect keys depend on
    pub(super) layout_generics: FxHashSet<u32>,

    /// Each generic definition it references, with the bindings it references it with
    pub(super) references: Vec<(DefinitionId, Bindings)>,

    /// Each function it uses as a function value
    pub(super) function_values: Vec<DefinitionId>,

    /// Each function it calls directly or hands to C, which need the function's direct calling convention
    pub(super) direct_uses: Vec<DefinitionId>,

    /// Each global it references
    pub(super) globals: Vec<DefinitionId>,

    /// Whether it does anything a global's constant initializer cannot
    pub(super) runtime_work: bool,

    /// See [super::Info::keys]
    pub(super) keys: Vec<Type>,

    /// Whether any type ids are actually read
    pub(super) reads_ids: bool,

    pub(super) materialized: FxHashSet<Value>,
}

pub(super) fn kind_of(definition: &Definition) -> Kind {
    if !definition.is_global() {
        return Kind::Function;
    }
    let Some(TerminatorInstruction::Result(Value::InstructionResult(result))) = &definition.entry_block().terminator
    else {
        return Kind::Global;
    };
    match &definition.instructions[*result] {
        Instruction::Extern(_) if matches!(definition.typ, Type::Function(_)) => Kind::Extern,
        _ => Kind::Global,
    }
}

/// The facts of every definition in `mir`
pub(super) fn collect(mir: &Mir, kind: &(impl Fn(DefinitionId) -> Kind + Sync)) -> FxHashMap<DefinitionId, Facts> {
    mir.definitions.par_iter().map(|(id, definition)| (*id, facts(definition, mir, kind))).collect()
}

fn facts(definition: &Definition, mir: &Mir, kind: &impl Fn(DefinitionId) -> Kind) -> Facts {
    let (mut layout_generics, keys) = layout_facts(definition);

    // References to generic definitions without an `Instantiate` use the caller's own generics
    let add_reference = |value: &Value, references: &mut Vec<_>, globals: &mut Vec<_>| {
        if let Value::Definition(id) = value
            && kind(*id) != Kind::Extern
        {
            globals.push(*id);
            if mir.definitions[id].generic_count > 0 {
                references.push((*id, Bindings::Identity));
            }
        }
    };

    let mut references = Vec::new();
    let mut function_values = Vec::new();
    let mut direct_uses = Vec::new();
    let mut globals = Vec::new();
    let mut key_generics = Vec::new();
    let mut runtime_work = false;
    for instruction in definition.instructions.values() {
        runtime_work |= !matches!(
            instruction,
            Instruction::MakeTuple(_)
                | Instruction::MakeArray(_)
                | Instruction::MakeBytes(_)
                | Instruction::Id(_)
                | Instruction::Transmute(_)
                | Instruction::Extern(_)
                | Instruction::AllocShared(_)
                | Instruction::PackClosure { .. }
                | Instruction::Instantiate(..)
                | Instruction::MakeEvidence { capabilities: _, rest: None }
        );
        instruction.for_each_value(|value| add_reference(value, &mut references, &mut globals));
        match instruction {
            Instruction::LookupEvidence { key, .. } => key_generics.extend(&key.args),
            Instruction::MakeEvidence { capabilities, .. } => {
                key_generics.extend(capabilities.iter().flat_map(|(key, _)| &key.args))
            },
            Instruction::Instantiate(id, bindings) => {
                references.push((*id, Bindings::Explicit(bindings.clone())));
                globals.push(*id);
            },
            Instruction::PackClosure { function, .. } => {
                if let Target::Function(id, _) = resolve(function, definition, kind) {
                    function_values.push(id);
                }
            },
            Instruction::Call { function, arguments } | Instruction::CallClosure { closure: function, arguments } => {
                match resolve(function, definition, kind) {
                    Target::Function(id, _) => direct_uses.push(id),
                    Target::Extern => direct_uses.extend(arguments.iter().filter_map(|argument| {
                        match resolve(argument, definition, kind) {
                            Target::Function(id, _) => Some(id),
                            _ => None,
                        }
                    })),
                    Target::Value => {
                        if let Some(known) = known_closure(function, definition, mir, kind) {
                            direct_uses.push(known.function);
                        }
                    },
                }
            },
            Instruction::Transmute(value) => {
                if let Target::Function(id, _) = resolve(value, definition, kind) {
                    direct_uses.push(id);
                }
            },
            _ => (),
        }
    }
    for terminator in definition.blocks.values().filter_map(|block| block.terminator.as_ref()) {
        terminator.for_each_value(|value| add_reference(value, &mut references, &mut globals));
    }
    let mut reads_ids = false;
    for arg in key_generics {
        for_each_generic(arg, false, &mut |generic| {
            layout_generics.insert(generic);
            reads_ids = true;
        });
    }
    layout_generics.retain(|generic| *generic < definition.generic_count);

    let materialized = materialized_values(definition, kind);
    for value in &materialized {
        if let Target::Function(id, _) = resolve(value, definition, kind) {
            function_values.push(id);
        }
    }
    reads_ids |= !function_values.is_empty();

    globals.retain(|id| kind(*id) == Kind::Global);
    Facts {
        layout_generics,
        references,
        function_values,
        direct_uses,
        globals,
        runtime_work,
        materialized,
        keys,
        reads_ids,
    }
}

/// The generics the layouts of the types in `definition` depend on, and its [Facts::keys]
fn layout_facts(definition: &Definition) -> (FxHashSet<u32>, Vec<Type>) {
    let mut layout_generics = FxHashSet::default();
    let mut keys = Vec::new();
    let mut seen_keys = FxHashSet::default();
    let mut add = |typ: &Type| {
        for_each_generic(typ, true, &mut |generic| _ = layout_generics.insert(generic));
        if is_key(typ) && seen_keys.insert(typ.clone()) {
            keys.push(typ.clone());
        }
    };
    definition.blocks.values().flat_map(|block| &block.parameter_types).for_each(&mut add);
    definition.instruction_result_types.values().for_each(&mut add);
    if definition.is_global() {
        add(&definition.typ);
    }
    for instruction in definition.instructions.values() {
        match instruction {
            Instruction::StackAllocUninit(typ) | Instruction::SizeOf(typ) | Instruction::ArrayLen(typ) => add(typ),
            Instruction::GetFieldPtr { struct_type, .. } => add(struct_type),
            _ => (),
        }
    }
    (layout_generics, keys)
}

/// The values of `definition` which must exist at runtime rather than only being called
pub(super) fn materialized_values(definition: &Definition, kind: &impl Fn(DefinitionId) -> Kind) -> FxHashSet<Value> {
    let mut values = FxHashSet::default();
    for instruction in definition.instructions.values() {
        let callee = match instruction {
            Instruction::Call { function, .. } | Instruction::PackClosure { function, .. } => Some(function),
            Instruction::CallClosure { closure, .. } => Some(closure),
            _ => None,
        };
        let callee = callee.filter(|callee| !matches!(resolve(callee, definition, kind), Target::Value));
        instruction.for_each_value(|value| {
            if Some(value) != callee {
                values.insert(*value);
            }
        });
    }
    for block in definition.blocks.values() {
        if let Some(terminator) = &block.terminator {
            terminator.for_each_value(|value| _ = values.insert(*value));
        }
    }
    while {
        let mut changed = false;
        // Reversed since an `Id` refers to an earlier instruction
        for (id, instruction) in definition.instructions.iter().rev() {
            if let Instruction::Id(inner) = instruction
                && values.contains(&Value::InstructionResult(id))
            {
                changed |= values.insert(*inner);
            }
        }
        changed
    }{}
    values
}
