//! This file contains the logic for specializing generics out of the MIR. This process is called
//! monomorphization and in Ante it is a Mir -> Mir transformation.
//!
//! The monomorphizer starts from the entry point to the program and from there builds a queue
//! of functions which need to be monomorphized. This queue can be processed concurrently with
//! each individual function being handled by a single [FunctionContext] object.
use std::sync::Arc;

use dashmap::DashMap;
use inc_complete::DbGet;
use rustc_hash::FxHashMap;

mod select_largest_variant;

use crate::{
    incremental::{GetCrateGraph, GetItem, GetItemRaw, Parse, TargetPointerSize, TypeCheck},
    mir::{
        self, Definition, DefinitionId, FunctionType, GenericBindings, Instruction, Mir, Type, Value,
        builder::build_initial_mir_with_shared_map, next_definition_id,
    },
    parser::ids::TopLevelId,
};

/// Collect all items in the program.
/// This is discouraged since it limits parallelism but required for certain passes like
/// monomorphization which need access to the entire program.
fn collect_all_items<Db>(compiler: &Db) -> Vec<TopLevelId>
where
    Db: DbGet<GetCrateGraph> + DbGet<Parse>,
{
    let mut items = Vec::new();

    for crate_ in GetCrateGraph.get(compiler).values() {
        for file in crate_.source_files.values() {
            let parse = Parse(*file).get(compiler);
            for item in parse.cst.top_level_items.iter() {
                items.push(item.id);
            }
        }
    }
    items
}

/// Monomorphize the whole program, returning a MIR function if the item refers to a function.
/// If the item does not refer to a function (e.g. it is a type definition), `None` is returned.
///
/// Note that monomorphize needs access to every item to monomorphize at once - it may not be
/// called separately and combined via [Mir::extend] later as this will lead to missing generic
/// definitions which were not monomorphized. `items` must contain every item in the program.
pub(crate) fn monomorphize<Db>(compiler: &Db) -> Mir
where
    Db: DbGet<TypeCheck>
        + DbGet<GetItem>
        + DbGet<GetItemRaw>
        + DbGet<GetCrateGraph>
        + DbGet<Parse>
        + DbGet<TargetPointerSize>
        + Sync,
{
    let initial_mir = collect_all_items(compiler)
        //.into_par_iter()
        .into_iter()
        .flat_map(|item| build_initial_mir_with_shared_map(compiler, item))
        .fold(Mir::default(), Mir::extend)
        //.reduce(Mir::default, Mir::extend)
        .remove_internal_externs()
        .remove_unreachable_functions()
        .optimize_tail_resume()
        .optimize_abort_handlers()
        .lower_effects();

    let shared = SharedDefinitions::default();

    // If there are no generics this is an entry point to monomorphization.
    // If there are generics, then we'll either monomorphize this function later
    // when we find its type arguments, or never if it is unused.
    let monomorphic_definitions = initial_mir
        .definitions
        .iter()
        .filter(|(_, definition)| definition.is_monomorphic() || definition.name.as_str() == "main")
        .map(|(_, definition)| {
            shared.insert((definition.id, Arc::new(Vec::new())), definition.id);
            definition.clone()
        })
        .collect::<Vec<_>>();

    // TODO: More concrete perf testing, but this is fine for smaller programs.
    let mir = monomorphic_definitions
        //.into_par_iter()
        .into_iter()
        .fold(Mir::default(), |acc, definition| {
            let monomorphized = monomorphize_non_generic_definition(definition, &shared, &initial_mir)
                .select_largest_variants(compiler);
            acc.extend(monomorphized)
        })
        //.reduce(Mir::default, Mir::extend)
        .lower_closures();

    #[cfg(debug_assertions)]
    let mir = mir.assert_fully_linked().assert_type_checks().assert_no_unions_or_generics().assert_no_closure_types();

    mir
}

/// The entry point to monomorphization is any non-generic definition.
/// We can't start with generic definitions since they require type bindings from their callsite(s).
///
/// `initial_mir` is the Mir pre-monomorphization and is not modified.
fn monomorphize_non_generic_definition(
    definition: Definition, definitions: &SharedDefinitions, initial_mir: &Mir,
) -> Mir {
    let mut context = FunctionContext::new(definitions, initial_mir);
    context.monomorphize_definition(definition);

    while let Some(item) = context.queue.pop() {
        let Some(original_definition) = initial_mir.get(item.old_id) else {
            panic!(
                "Monomorphization: no definition for id {}, was monomorphize not given every top-level-item in a single invocation?",
                item.old_id
            );
        };

        let mut definition = original_definition.clone_with_id(item.new_id);
        definition.generic_count = 0;
        context.generic_mapping = if original_definition.is_monomorphic() {
            // Already monomorphic: no generic substitution needed, but we still
            // must insert new_id into finished_definitions so callers can resolve it.
            Arc::new(Vec::new())
        } else {
            item.bindings.clone()
        };
        // Derive the monomorphized definition type by specializing the original's type
        // rather than item.monomorphized_type, the type from the caller.
        if !context.generic_mapping.is_empty() {
            context.specialize_type(&mut definition.typ);
        }
        context.monomorphize_definition(definition);
    }

    Mir {
        definitions: context.finished_definitions,
        externals: Default::default(),
        preserved_op_indices: Default::default(),
    }
}

struct FunctionContext<'local> {
    generic_mapping: Arc<GenericBindings>,

    queue: Vec<DefinitionToMonomorphize>,

    finished_definitions: FxHashMap<DefinitionId, Definition>,

    /// This is shared between all concurrent monomorphize calls
    definitions: &'local SharedDefinitions,

    /// The initial MIR before monomorphization, used to check whether a referenced
    /// definition is generic (needed for same-SCC mutual recursion without Instantiate).
    initial_mir: &'local Mir,
}

struct DefinitionToMonomorphize {
    /// The old id pre-monomorphization
    old_id: DefinitionId,
    /// The id referring to the monomorphized version of `old_id` with the given generic bindings
    new_id: DefinitionId,
    bindings: Arc<GenericBindings>,
}

/// Maps (old_id, generic bindings) to a new [DefinitionId] referring to the newly monomorphized
/// version of `old_id` with the given generic type bindings.
type SharedDefinitions = DashMap<(DefinitionId, Arc<GenericBindings>), DefinitionId>;

impl<'local> FunctionContext<'local> {
    fn new(definitions: &'local SharedDefinitions, initial_mir: &'local Mir) -> Self {
        Self {
            definitions,
            initial_mir,
            generic_mapping: Default::default(),
            queue: Default::default(),
            finished_definitions: Default::default(),
        }
    }

    fn monomorphize_definition(&mut self, mut definition: mir::Definition) {
        if !self.generic_mapping.is_empty() {
            self.update_value_types(&mut definition);
        }

        // We can skip the blocks and go right to the instructions themselves. There shouldn't be
        // any that aren't used in a block.
        for instruction in definition.instructions.values_mut() {
            if let Instruction::Instantiate(id, bindings) = instruction {
                assert!(!bindings.is_empty());
                if !self.generic_mapping.is_empty() {
                    self.specialize_bindings(bindings);
                }

                let new_id = *self.definitions.entry((*id, bindings.clone())).or_insert_with(|| {
                    let new_id = next_definition_id();
                    self.queue.push(DefinitionToMonomorphize { old_id: *id, new_id, bindings: bindings.clone() });
                    new_id
                });

                *instruction = Instruction::Id(Value::Definition(new_id));
            } else if !self.generic_mapping.is_empty() {
                // When a generic function directly calls another definition (e.g. a recursive
                // self-call) without going through `Instantiate`, the `Value::Definition` ID must
                // still be remapped to the monomorphized version.  We only do this when the
                // mapping already exists in `self.definitions`; if it is absent the reference is
                // already monomorphic and needs no update.
                self.remap_definition_values_in_instruction(instruction);
            }
        }

        self.finished_definitions.insert(definition.id, definition);
    }

    fn update_value_types(&self, definition: &mut Definition) {
        for result_type in definition.instruction_result_types.values_mut() {
            self.specialize_type(result_type);
        }

        for block in definition.blocks.values_mut() {
            for parameter in block.parameter_types.iter_mut() {
                self.specialize_type(parameter);
            }
        }
    }

    /// Remap a `Value::Definition(old_id)` to its monomorphized version.
    ///
    /// If `(old_id, generic_mapping)` is already in `self.definitions`, use that mapping.
    /// Otherwise, if the definition is generic in the initial MIR (e.g. a mutual recursion
    /// partner in the same SCC that was never referenced via `Instantiate`), create a new
    /// monomorphized copy on demand using the current `generic_mapping` as bindings.
    fn remap_value(&mut self, v: &mut Value) {
        if let Value::Definition(id) = v {
            if let Some(new_id) = self.definitions.get(&(*id, self.generic_mapping.clone())) {
                *id = *new_id;
            } else if let Some(def) = self.initial_mir.get(*id)
                && !def.is_monomorphic()
            {
                let bindings = self.generic_mapping.clone();
                let new_id = *self.definitions.entry((*id, bindings.clone())).or_insert_with(|| {
                    let new_id = next_definition_id();
                    self.queue.push(DefinitionToMonomorphize { old_id: *id, new_id, bindings });
                    new_id
                });
                *id = new_id;
            }
        }
    }

    /// Remap any `Value::Definition(old_id)` inside `instruction` to its monomorphized version.
    /// This handles direct references to generic functions (e.g. recursive self-calls or
    /// mutual recursion partners in the same SCC) that bypass the `Instantiate` instruction path.
    fn remap_definition_values_in_instruction(&mut self, instruction: &mut Instruction) {
        match instruction {
            Instruction::Call { function, arguments } => {
                self.remap_value(function);
                for arg in arguments.iter_mut() {
                    self.remap_value(arg);
                }
            },
            Instruction::CallClosure { closure: function, arguments } => {
                self.remap_value(function);
                for arg in arguments.iter_mut() {
                    self.remap_value(arg);
                }
            },
            Instruction::Perform { effect_op: _, arguments } => {
                for arg in arguments.iter_mut() {
                    self.remap_value(arg);
                }
            },
            Instruction::Handle { body, cases } => {
                self.remap_value(body);
                for case in cases.iter_mut() {
                    self.remap_value(&mut case.handler);
                }
            },
            Instruction::PackClosure { function, environment } => {
                self.remap_value(function);
                self.remap_value(environment);
            },
            Instruction::IndexTuple { tuple, .. } => self.remap_value(tuple),
            Instruction::MakeTuple(elements) | Instruction::MakeArray(elements) => {
                for e in elements.iter_mut() {
                    self.remap_value(e);
                }
            },
            Instruction::StackAlloc(v)
            | Instruction::AllocShared(v)
            | Instruction::Transmute(v)
            | Instruction::Id(v) => self.remap_value(v),
            Instruction::StackAllocUninit(typ) => {
                if !self.generic_mapping.is_empty() {
                    self.specialize_type(typ);
                }
            },
            Instruction::Store { pointer, value } => {
                self.remap_value(pointer);
                self.remap_value(value);
            },
            Instruction::AddInt(a, b)
            | Instruction::AddFloat(a, b)
            | Instruction::SubInt(a, b)
            | Instruction::SubFloat(a, b)
            | Instruction::MulInt(a, b)
            | Instruction::MulFloat(a, b)
            | Instruction::DivSigned(a, b)
            | Instruction::DivUnsigned(a, b)
            | Instruction::DivFloat(a, b)
            | Instruction::ModSigned(a, b)
            | Instruction::ModUnsigned(a, b)
            | Instruction::ModFloat(a, b)
            | Instruction::LessSigned(a, b)
            | Instruction::LessUnsigned(a, b)
            | Instruction::LessFloat(a, b)
            | Instruction::EqInt(a, b)
            | Instruction::EqFloat(a, b)
            | Instruction::BitwiseAnd(a, b)
            | Instruction::BitwiseOr(a, b)
            | Instruction::BitwiseXor(a, b) => {
                self.remap_value(a);
                self.remap_value(b);
            },
            Instruction::BitwiseNot(v)
            | Instruction::SignExtend(v)
            | Instruction::ZeroExtend(v)
            | Instruction::SignedToFloat(v)
            | Instruction::UnsignedToFloat(v)
            | Instruction::FloatToSigned(v)
            | Instruction::FloatToUnsigned(v)
            | Instruction::FloatPromote(v)
            | Instruction::FloatDemote(v)
            | Instruction::Truncate(v)
            | Instruction::Deref(v) => self.remap_value(v),
            Instruction::SizeOf(typ) | Instruction::ArrayLen(typ) => self.specialize_type(typ),
            Instruction::MakeBytes(_) | Instruction::Instantiate(..) | Instruction::Extern(_) => {},
            Instruction::Capability => {},
            Instruction::GetFieldPtr { struct_ptr, struct_type, .. } => {
                self.remap_value(struct_ptr);
                if !self.generic_mapping.is_empty() {
                    self.specialize_type(struct_type);
                }
            },
        }
    }

    fn specialize_bindings(&self, bindings: &mut Arc<Vec<Type>>) {
        // There shouldn't be any external refs to Instruction::Instantiate bindings so this should
        // always succeed.
        let bindings = Arc::make_mut(bindings);
        bindings.iter_mut().for_each(|typ| self.specialize_type(typ));
    }

    /// Replace any instances of generics in `self.generic_mapping` of the given type with their mapping.
    /// The resulting type should be guaranteed free of [Type::Generic].
    fn specialize_type(&self, typ: &mut Type) {
        if let Some(new_type) = self.specialize_type_opt(typ) {
            *typ = new_type;
        }
    }

    /// Returns `Some(new_type)` when a generic was specialized somewhere within `typ`, and `None`
    /// when the subtree contains no generic so that the caller can reuse the original `Arc`s.
    fn specialize_type_opt(&self, typ: &Type) -> Option<Type> {
        match typ {
            Type::Primitive(_) | Type::U32(_) => None,
            Type::Generic(generic) => {
                let Some(mapping) = self.generic_mapping.get(generic.0 as usize) else {
                    unreachable!("Unmapped generic found in monomorphization: {generic:?}")
                };
                Some(mapping.clone())
            },
            Type::Tuple(fields) => self.specialize_each(fields).map(|v| Type::Tuple(Arc::new(v))),
            Type::Union(variants) => self.specialize_each(variants).map(|v| Type::Union(Arc::new(v))),
            Type::Function(function) => {
                let parameters = self.specialize_each(&function.parameters);
                let environment = self.specialize_type_opt(&function.environment);
                let return_type = self.specialize_type_opt(&function.return_type);
                if parameters.is_none() && environment.is_none() && return_type.is_none() {
                    return None;
                }
                Some(Type::Function(Arc::new(FunctionType {
                    parameters: parameters.unwrap_or_else(|| function.parameters.to_vec()),
                    environment: environment.unwrap_or_else(|| function.environment.clone()),
                    return_type: return_type.unwrap_or_else(|| function.return_type.clone()),
                })))
            },
            Type::Array { length, element } => {
                let new_length = self.specialize_type_opt(length);
                let new_element = self.specialize_type_opt(element);
                if new_length.is_none() && new_element.is_none() {
                    return None;
                }
                let length = new_length.map(Arc::new).unwrap_or_else(|| length.clone());
                let element = new_element.map(Arc::new).unwrap_or_else(|| element.clone());
                Some(Type::Array { length, element })
            },
        }
    }

    /// Specialize each element of a type list, returning `Some(new_vec)` if any element changed.
    fn specialize_each(&self, items: &[Type]) -> Option<Vec<Type>> {
        let mut result: Option<Vec<Type>> = None;
        for (i, item) in items.iter().enumerate() {
            match self.specialize_type_opt(item) {
                Some(new) => result.get_or_insert_with(|| items[..i].to_vec()).push(new),
                None => {
                    if let Some(new_items) = result.as_mut() {
                        new_items.push(item.clone());
                    }
                },
            }
        }
        result
    }
}
