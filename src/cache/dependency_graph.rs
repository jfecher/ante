use std::collections::HashMap;

use super::ModuleCache;
use crate::cache::DefinitionInfoId;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Default, Debug)]
pub struct DependencyGraph {
    current_definition: Option<DefinitionInfoId>,
    graph: DiGraph<DefinitionInfoId, ()>,
    node_map: HashMap<DefinitionInfoId, NodeIndex>,
}

impl DependencyGraph {
    fn get_or_add_node(&mut self, id: DefinitionInfoId) -> NodeIndex {
        if let Some(index) = self.node_map.get(&id) {
            *index
        } else {
            let index = self.graph.add_node(id);
            self.node_map.insert(id, index);
            index
        }
    }

    // Prints the graph in graphviz format
    #[allow(unused)]
    pub fn dbg_print(&self, cache: &ModuleCache) {
        let named = self.graph.map(|_, node| cache[*node].name.clone(), |_, edge| *edge);
        let dot = Dot::with_config(&named, &[Config::EdgeNoLabel]);
        println!("{dot:?}");
    }

    pub fn set_definition(&mut self, definition: DefinitionInfoId) {
        self.current_definition = Some(definition);
    }

    // add an edge when a global is referenced
    pub fn add_edge(&mut self, dependency: DefinitionInfoId) {
        if let Some(dependent) = self.current_definition {
            let dependent = self.get_or_add_node(dependent);
            let dependency = self.get_or_add_node(dependency);
            self.graph.update_edge(dependent, dependency, ());
        }
    }

    pub fn enter_definition(&mut self) -> Option<DefinitionInfoId> {
        self.current_definition
    }

    pub fn exit_definition(&mut self, reset_definition: Option<DefinitionInfoId>) {
        self.current_definition = reset_definition;
    }
}
