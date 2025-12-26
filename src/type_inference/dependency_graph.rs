use std::{collections::BTreeMap, sync::Arc};

use petgraph::graph::DiGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    incremental::{
        DbHandle, GetCrateGraph, GetItem, GetTypeCheckSCC, Parse, Resolve, TypeCheck, TypeCheckDependencyGraph,
        TypeCheckSCC,
    },
    iterator_extensions::vecmap,
    name_resolution::namespace::LOCAL_CRATE,
    parser::{cst::TopLevelItemKind, ids::TopLevelId},
    type_inference::{
        IndividualTypeCheckResult, get_type::try_get_type, type_context::TypeContext, type_id::TypeId, types::{Type, TypeBindings}
    },
};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct TypeCheckDependencyGraphResult {
    /// Maps each top-level id to the index of its SCC in the outer `sccs` Vec.
    id_to_scc: BTreeMap<TopLevelId, u32>,

    /// TODO: Explore using small vecs for the inner Vec with a size of 1.
    /// The majority of all SCCs should be a single item
    sccs: Vec<SCC>,
}

pub type SCC = Arc<Vec<TopLevelId>>;

// DependencyGraph
//   |
//   V
// GetSCC
//   |
//   V
// TypeCheck <-> TypeCheckSCC

/// Build a type inference dependency graph for the entire local crate, finding the
/// SCCs in the graph, and deferring to TypeCheckSCC.
pub fn get_type_check_graph_impl(_: &TypeCheckDependencyGraph, db: &DbHandle) -> Arc<TypeCheckDependencyGraphResult> {
    let mut graph = DiGraph::new();
    let mut item_to_index = FxHashMap::default();
    let mut index_to_item = FxHashMap::default();

    let mut add_node = |graph: &mut DiGraph<_, _>, item| {
        if let Some(index) = item_to_index.get(&item) {
            *index
        } else {
            let index = graph.add_node(());
            item_to_index.insert(item, index);
            index_to_item.insert(index, item);
            index
        }
    };

    let mut queue = get_all_top_level_ids(db);
    let mut visited = FxHashSet::default();

    while let Some(item) = queue.pop() {
        if !visited.insert(item) {
            continue;
        }

        let resolution = Resolve(item).get(db);
        let item_index = add_node(&mut graph, item);

        for dependency_id in resolution.referenced_items {
            let dependency_index = add_node(&mut graph, dependency_id);

            if item_lacks_known_type(dependency_id, db) {
                graph.update_edge(item_index, dependency_index, ());
            }
        }
    }

    // tarjan_scc returns SCCs in post_order, which is the order we want to analyze in.
    let sccs = petgraph::algo::tarjan_scc(&graph);
    let mut id_to_scc = BTreeMap::new();
    let order = vecmap(sccs.into_iter().enumerate(), |(scc_index, scc)| {
        let mut scc = vecmap(scc, |index| {
            let item = index_to_item[&index];
            id_to_scc.insert(item, scc_index as u32);
            item
        });
        scc.sort_unstable();
        Arc::new(scc)
    });
    Arc::new(TypeCheckDependencyGraphResult { sccs: order, id_to_scc })
}

/// Retrieves all top-level ids in the program (including dependencies)
fn get_all_top_level_ids(db: &DbHandle) -> Vec<TopLevelId> {
    let crates = GetCrateGraph.get(db);
    let mut ids = Vec::new();
    // The Stdlib still has many errors preventing it from type-checking so we skip it until
    // the compiler has more implemented.
    //for crate_ in crates.values() {
    let crate_ = &crates[&LOCAL_CRATE];
    for file in crate_.source_files.values() {
        let parse = Parse(*file).get(db);
        ids.extend(parse.top_level_data.keys().copied());
    }
    //}
    ids
}

/// If the dependency in question lacks a known type it means we must infer its
/// type before we infer the type of the item that refers to it. These edges
/// are used to build a dependency tree for type inference where cycles represent
/// mutually recursive functions without type annotations.
fn item_lacks_known_type(dependency_id: TopLevelId, db: &DbHandle) -> bool {
    let (item, context) = GetItem(dependency_id).get(db);

    // Only Definitions matter here but the full match is written in case more
    // TopLevelItemKinds are added in the future which do matter for this analysis.
    match &item.kind {
        TopLevelItemKind::Definition(definition) => {
            let resolve = Resolve(dependency_id).get(db);
            try_get_type(definition, &context, &resolve).is_none()
        },
        TopLevelItemKind::TypeDefinition(_) => false,
        TopLevelItemKind::TraitDefinition(_) => false,
        TopLevelItemKind::TraitImpl(_) => false,
        TopLevelItemKind::EffectDefinition(_) => false,
        TopLevelItemKind::Extern(_) => false,
        // Comptime items shouldn't be possible to be referred to in this way
        TopLevelItemKind::Comptime(_) => false,
    }
}

pub fn get_type_check_scc_impl(context: &GetTypeCheckSCC, db: &DbHandle) -> SCC {
    let graph = TypeCheckDependencyGraph.get(db);

    match graph.id_to_scc.get(&context.0) {
        Some(index) => graph.sccs[*index as usize].clone(),
        // Ids in the stdlib currently are excluded from the dependency graph.
        // We assume these are mostly types currently and return them in their own SCC.
        // This should be replaced with an unwrap when the stdlib type checks.
        None => Arc::new(vec![context.0]),
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeCheckResult {
    pub result: IndividualTypeCheckResult,
    pub types: TypeContext,
    pub bindings: TypeBindings,
}

impl TypeCheckResult {
    /// Retrieve a Type then follow all its type variable bindings so that we only return
    /// `Type::Variable` if the type variable is unbound. Note that this may still return
    /// a composite type such as `Type::Application` with bound type variables within.
    pub fn follow_type(&self, type_id: TypeId) -> &Type {
        self.types.follow_type(type_id, &self.bindings)
    }
}

pub fn type_check_impl(context: &TypeCheck, db: &DbHandle) -> Arc<TypeCheckResult> {
    let scc = GetTypeCheckSCC(context.0).get(db);
    let result = TypeCheckSCC(scc).get(db);

    Arc::new(TypeCheckResult {
        result: result.items[&context.0].clone(),
        types: result.types,
        bindings: result.bindings,
    })
}
