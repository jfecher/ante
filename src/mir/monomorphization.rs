//! This file contains the logic for specializing generics out of the MIR. This process is called
//! monomorphization and in Ante it is a Mir -> Mir transformation.
//!
//! The monomorphizer starts from the entry point to the program and from there builds a queue
//! of functions which need to be monomorphized. This queue can be processed concurrently with
//! each individual function being handled by a single [FunctionContext] object.

#![allow(unused)]
use std::{collections::LinkedList, sync::Arc};

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::FxHashMap;

use crate::{
    incremental::DbHandle, mir::{self, Definitions, Type, builder::build_initial_mir}, parser::{
        context::TopLevelContext,
        cst,
        ids::{NameId, TopLevelId},
    }, type_inference::dependency_graph::TypeCheckResult
};

/// Monomorphize the whole program, returning a MIR function if the item refers to a function.
/// If the item does not refer to a function (e.g. it is a type definition), `None` is returned.
#[allow(unused)]
fn monomorphize(compiler: &DbHandle, items: impl IntoParallelIterator<Item = TopLevelId>) -> Option<mir::Definition> {
    let mir = collect_all_mir(compiler, items);
    todo!()
}

/// To monomorphize we necessarily need access to all of the Mir.
fn collect_all_mir(compiler: &DbHandle, items: impl IntoParallelIterator<Item = TopLevelId>) -> LinkedList<Vec<Definitions>> {
    items.into_par_iter().flat_map(|item| {
        build_initial_mir(compiler, item).map(|mir| mir.definitions)
    }).collect_vec_list()
}

#[allow(unused)]
struct FunctionContext<'local, 'db> {
    compiler: &'local DbHandle<'db>,
    types: Arc<TypeCheckResult>,
    item_context: Arc<TopLevelContext>,
    generic_mapping: FxHashMap<NameId, Type>,

    monomorphized_functions: FxHashMap<TopLevelId, mir::Definition>,
    queue: Vec<(TopLevelId, Type)>,
}

impl<'local, 'db> FunctionContext<'local, 'db> {
    fn new(compiler: &'local DbHandle<'db>, types: Arc<TypeCheckResult>, item_context: Arc<TopLevelContext>) -> Self {
        Self {
            compiler,
            types,
            item_context,
            generic_mapping: FxHashMap::default(),
            monomorphized_functions: Default::default(),
            queue: Default::default(),
        }
    }

    fn monomorphize_function(&mut self, _definition: &cst::Definition) -> mir::Definition {
        todo!()
    }
}
