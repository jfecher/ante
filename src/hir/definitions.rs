use std::collections::HashMap;

use crate::{cache::DefinitionInfoId, types};

use super::monomorphisation::Definition;

pub struct Definitions {
    // This is a rather inefficient representation with duplication
    // to prevent locals of one type overwriting locals of the same type
    // on different instantiations of a function, which can happen in some
    // rare instances.
    all: DefinitionMap,
    local: Vec<DefinitionMap>,
}

type DefinitionMap = HashMap<DefinitionKey, Definition>;

type DefinitionKey = (DefinitionInfoId, types::Type);

impl Definitions {
    pub fn new() -> Self {
        Self { all: HashMap::new(), local: vec![HashMap::new()] }
    }

    pub fn get(&self, id: DefinitionInfoId, typ: types::Type) -> Option<&Definition> {
        let locals = self.local.last().unwrap();
        if let Some(definition) = locals.get(&(id, typ.clone())) {
            return Some(definition);
        }

        self.all.get(&(id, typ))
    }

    pub fn insert(&mut self, id: DefinitionInfoId, typ: types::Type, definition: Definition) {
        let locals = self.local.last_mut().unwrap();
        locals.insert((id, typ.clone()), definition.clone());
        self.all.insert((id, typ), definition);
    }

    pub fn push_local_scope(&mut self) {
        self.local.push(HashMap::new());
    }

    pub fn pop_local_scope(&mut self) {
        self.local.pop();
    }
}
