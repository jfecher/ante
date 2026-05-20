use std::{collections::BTreeMap, sync::Arc};

use petgraph::graph::DiGraph;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::{
    incremental::{
        DbHandle, ExportedDefinitions, GetCrateGraph, GetItem, GetTypeCheckSCC, Parse, Resolve, TypeCheck,
        TypeCheckDependencyGraph, TypeCheckSCC,
    },
    iterator_extensions::mapvec,
    parser::{
        cst::TopLevelItemKind,
        ids::{NameId, TopLevelId},
    },
    type_inference::{
        IndividualTypeCheckResult, TypeMaps,
        fresh_expr::ExtendedTopLevelContext,
        get_type::try_get_generalized_type,
        types::{Type, TypeBindings},
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

    // Methods are resolved during type checking, not name resolution, so mutually recursive
    // methods (e.g. HashMap.insert <-> HashMap.resize) won't appear in each other's
    // referenced_items. Conservatively link all methods of the same object type that lack
    // known types so they end up in the same SCC.
    let crates = GetCrateGraph.get(db);
    for crate_ in crates.values() {
        for file in crate_.source_files.values() {
            let exported = ExportedDefinitions(*file).get(db);
            for (_type_id, methods) in &exported.methods {
                let mut method_ids =
                    methods.values().map(|name| &name.top_level_item).filter(|id| item_lacks_known_type(**id, db));

                if let Some(representative) = method_ids.next() {
                    let rep_index = add_node(&mut graph, *representative);

                    for other in method_ids {
                        let other_index = add_node(&mut graph, *other);
                        graph.update_edge(rep_index, other_index, ());
                        graph.update_edge(other_index, rep_index, ());
                    }
                }
            }
        }
    }

    // tarjan_scc returns SCCs in post_order, which is the order we want to analyze in.
    let sccs = petgraph::algo::tarjan_scc(&graph);
    let mut id_to_scc = BTreeMap::new();
    let order = mapvec(sccs.into_iter().enumerate(), |(scc_index, scc)| {
        let mut scc = mapvec(scc, |index| {
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
    for crate_ in crates.values() {
        for file in crate_.source_files.values() {
            let parse = Parse(*file).get(db);
            ids.extend(parse.top_level_data.keys().copied());
        }
    }
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
            try_get_generalized_type(definition, &context, &resolve, db).is_none()
        },
        TopLevelItemKind::TypeDefinition(_) => false,
        TopLevelItemKind::AbilityDefinition(_) => false,
        TopLevelItemKind::AbilityImpl(_) => false,
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
    pub bindings: TypeBindings,
}

pub fn type_check_impl(context: &TypeCheck, db: &DbHandle) -> Arc<TypeCheckResult> {
    let scc = GetTypeCheckSCC(context.0).get(db);
    let result = TypeCheckSCC(scc).get(db);

    let item_result = result.items.get(&context.0).cloned().unwrap_or_else(|| {
        // This item has no top-level names (e.g. it failed to parse). Return an empty result
        // so that callers can continue without panicking.
        let (_, item_context) = GetItem(context.0).get(db);
        IndividualTypeCheckResult {
            maps: TypeMaps::default(),
            generalized: FxHashMap::default(),
            context: ExtendedTopLevelContext::new(item_context),
        }
    });

    Arc::new(TypeCheckResult { result: item_result, bindings: result.bindings })
}

impl TypeCheckResult {
    /// Retrieves the given generalized type of the given name
    pub fn get_generalized(&self, name: NameId) -> &Type {
        self.result.generalized[&name].follow(&self.bindings)
    }
}
