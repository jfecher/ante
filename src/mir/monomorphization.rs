//! This file contains the logic for translating the type-checked CST into a monomorphized MIR
//! where any functions with generics are specialized by compiling a separate version of them for
//! each combination of generics that it is called with.
//!
//! The monomorphizer starts from the entry point to the program and from there builds a queue
//! of functions which need to be monomorphized. This queue can be processed concurrently with
//! each individual function being handled by a single [FunctionContext] object.

use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::{
    incremental::{DbHandle, GetItem, TypeCheck},
    mir::{self, Type},
    parser::{
        context::TopLevelContext,
        cst,
        ids::{NameId, TopLevelId},
    },
    type_inference::dependency_graph::TypeCheckResult,
};

/// Monomorphize the item, returning a MIR function if the item refers to a function.
/// If the item does not refer to a function (e.g. it is a type definition), `None` is returned.
#[allow(unused)]
fn monomorphize(compiler: &DbHandle, item: TopLevelId) -> Option<mir::Definition> {
    let types = TypeCheck(item).get(compiler);
    let (item, item_context) = GetItem(item).get(compiler);

    match &item.kind {
        cst::TopLevelItemKind::Definition(definition) => {
            let mut context = FunctionContext::new(compiler, types, item_context);
            Some(context.monomorphize_function(definition))
        },
        cst::TopLevelItemKind::TypeDefinition(_) => None,
        cst::TopLevelItemKind::TraitDefinition(_) => None,
        cst::TopLevelItemKind::TraitImpl(_) => None,
        cst::TopLevelItemKind::EffectDefinition(_) => None,
        cst::TopLevelItemKind::Extern(_) => None, // TODO
        cst::TopLevelItemKind::Comptime(_) => None,
    }
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
