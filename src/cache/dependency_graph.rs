use std::collections::HashMap;

use crate::cache::DefinitionInfoId;

use super::ModuleCache;

#[derive(Default, Debug)]
pub struct DependencyGraph {
    current_definition: Option<DefinitionInfoId>,
    graph: petgraph::graph::DiGraph<DefinitionInfoId, ()>,
    node_map: HashMap<DefinitionInfoId, petgraph::graph::NodeIndex>,
}

impl DependencyGraph {
    fn get_or_add_node(&mut self, id: DefinitionInfoId) -> petgraph::graph::NodeIndex {
        if let Some(index) = self.node_map.get(&id) {
            *index
        } else {
            let index = self.graph.add_node(id);
            self.node_map.insert(id, index);
            index
        }
    }

    pub fn into_dbg_petgraph(&self, cache: &ModuleCache) -> petgraph::graph::DiGraph<String, ()> {
        self.graph.map(|_, node| cache[*node].name.clone(), |_, edge| *edge)
    }

    pub fn set_definition<'c>(&mut self, definition: DefinitionInfoId) {
        self.current_definition = Some(definition);
    }

    // add an edge when a global is referenced
    pub fn add_edge<'c>(&mut self, dependency: DefinitionInfoId) {
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
