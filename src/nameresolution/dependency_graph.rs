use std::collections::HashMap;

use crate::cache::{ModuleCache, DefinitionInfoId};

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

    pub fn set_definition<'c>(&mut self, definition: DefinitionInfoId) {
        self.current_definition = Some(definition);
    }

    // add an edge when a global is referenced
    pub fn add_edge<'c>(&mut self, dependency: DefinitionInfoId, cache: &ModuleCache<'c>) {
        if let Some(dependent) = self.current_definition {
            let cdef = &cache[dependent].name;
            let new = &cache[dependency].name;
            println!("{} references {}", cdef, new);

            let dependent = self.get_or_add_node(dependent);
            let dependency = self.get_or_add_node(dependency);

            self.graph.add_edge(dependency, dependent, ());
        } else {
            println!("Tried to add edge but current_definition = None");
        }
    }

    pub fn enter_definition(&mut self) -> bool {
        self.current_definition.is_none()
    }

    pub fn exit_definition(&mut self, reset_definition: bool) {
        if reset_definition {
            self.current_definition = None;
        }
    }
}
