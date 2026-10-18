//! Setting up a rewritten definition: its signature, its parameters' replacements, and the
//! storage of dynamic block parameters.

use rustc_hash::FxHashMap;

use crate::{
    iterator_extensions::mapvec,
    mir::{
        Block, BlockId, Definition, Type, Value,
        existentialization::{
            Shared,
            analysis::Kind,
            builder::FunctionBuilder,
            rewrite::{Rewriter, destinations::destination_hints},
        },
    },
};

impl<'a> Rewriter<'a> {
    /// Rewrite a definition taking the direct calling convention, see [super::super::convention]
    pub(super) fn new(shared: &'a Shared<'a>, old: &'a Definition) -> Self {
        if shared.uniform(old.id) {
            return Self::new_uniform(shared, old);
        }
        let types = &shared.types;
        let needed = shared.needed(old.id);
        let kind = shared.kind(old.id);
        let stays_global = matches!(kind, Kind::Global | Kind::Extern);
        let has_result_slot = shared.has_result_slot(old.id);

        let old_parameters = &old.entry_block().parameter_types;
        let mut parameters = Vec::with_capacity(old_parameters.len() + 2);
        if !needed.is_empty() {
            parameters.push(Type::POINTER);
        }
        parameters.extend(old_parameters.iter().filter(|typ| shared.passed(typ)).map(|typ| shared.parameter_type(typ)));
        if has_result_slot {
            parameters.push(Type::POINTER);
        }

        let typ = if kind == Kind::Extern {
            types.lower_c(&old.typ)
        } else if stays_global {
            types.lower(&old.typ)
        } else {
            let return_type = if has_result_slot { Type::POINTER } else { types.lower(&shared.return_type(old.id)) };
            Type::function(parameters.clone(), return_type)
        };

        let mut builder = FunctionBuilder::new(shared, old.name.clone(), old.id, typ, needed);
        if stays_global {
            builder.keep_aggregates_in_ssa();
        }
        let parameters = mapvec(parameters, |typ| builder.push_parameter(typ));
        let mut parameters = parameters.into_iter();
        if !needed.is_empty() {
            builder.set_metadata(parameters.next().unwrap(), shared.keys(old.id));
        }

        let mut values = FxHashMap::default();
        for (index, typ) in old_parameters.iter().enumerate() {
            let value = if shared.passed(typ) { parameters.next().unwrap() } else { Value::Unit };
            values.insert(Value::Parameter(BlockId::ENTRY_BLOCK, index as u32), value);
        }
        let result_slot = parameters.next();

        Self::finish_new(builder, old, values, result_slot, stays_global)
    }

    /// Rewrite a function that is only ever used as a function value so it takes the uniform
    /// calling convention itself and needs no value thunk, see [existential_function_type]
    fn new_uniform(shared: &'a Shared<'a>, old: &'a Definition) -> Self {
        let (mut builder, addresses, result) = FunctionBuilder::new_uniform(shared, old.name.clone(), old.id, old);
        let mut values = FxHashMap::default();
        for (index, (address, typ)) in addresses.into_iter().zip(&old.entry_block().parameter_types).enumerate() {
            let value = match typ {
                Type::Evidence(_) => address,
                _ if builder.indirect(typ) => address,
                _ => builder.load_immutable(address, typ),
            };
            values.insert(Value::Parameter(BlockId::ENTRY_BLOCK, index as u32), value);
        }
        Self::finish_new(builder, old, values, Some(result), false)
    }

    fn finish_new(
        mut builder: FunctionBuilder<'a>, old: &'a Definition, mut values: FxHashMap<Value, Value>,
        result_slot: Option<Value>, stays_global: bool,
    ) -> Self {
        let shared = builder.shared;
        let types = &shared.types;
        let mut block_slots = FxHashMap::default();
        for (id, block) in old.blocks.iter().skip(1) {
            let new_id = if block.parameter_types.iter().any(|typ| builder.indirect(typ)) {
                assert_eq!(block.parameter_types.len(), 1, "existentialization: expected one block parameter");
                let slot = builder.slot(&block.parameter_types[0]);
                block_slots.insert(id, slot);
                values.insert(Value::Parameter(id, 0), slot);
                builder.function.blocks.push(Block::new(Vec::new()))
            } else {
                let parameters = mapvec(&block.parameter_types, |typ| types.lower(typ));
                for index in 0..parameters.len() {
                    values.insert(Value::Parameter(id, index as u32), Value::Parameter(id, index as u32));
                }
                builder.function.blocks.push(Block::new(parameters))
            };
            assert_eq!(id, new_id);
        }

        let materialized = &shared.info[&old.id].materialized;

        let hints = destination_hints(old, result_slot.is_some(), &|typ| builder.indirect(typ));
        let destinations = FxHashMap::default();
        Self { builder, old, values, materialized, block_slots, result_slot, hints, destinations, stays_global }
    }
}
