use std::{collections::{HashMap, HashSet}, fmt::Display};

use crate::{mir::ir::{Mir, FunctionId, Function}, util::fmap};

#[derive(Default)]
pub struct Scopes {
    scopes: HashMap<ScopeId, Scope>,
    function_scopes: HashMap<FunctionId, ScopeId>,
    next_scope_id: usize,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
struct ScopeId(usize);

pub(super) struct Scope {
    pub(super) functions: HashSet<FunctionId>,
    pub(super) entry_point: FunctionId,
}


impl Mir {
    /// Calculates the scope of each function.
    ///
    /// The scope of a function contains all other functions which directly
    /// or indirectly reference a function's parameters. The entry point of
    /// a scope is the parent function of this scope that has its parameters
    /// referenced but is not part of any other parent scope itself.
    pub fn find_scopes(&self) -> Scopes {
        let mut scopes = Scopes::default();

        for function in self.functions.values() {
            scopes.add_direct_dependencies(function)
        }

        for function in self.functions.values() {
            scopes.add_indirect_dependencies(function)
        }

        scopes
    }
}

impl Scopes {
    fn add_direct_dependencies(&mut self, function: &Function) {
        let mut scopes = HashSet::new();

        function.for_each_id(self, |_, _| (), |_, parameter| {
            if parameter.function != function.id {
                scopes.insert(parameter.function.clone());
            }
        });

        if scopes.is_empty() {
            self.add_function_to_scope(function.id.clone(), function.id.clone());
        } else {
            for parent in scopes {
                self.add_function_to_scope(parent, function.id.clone());
            }
        }
    }

    fn add_function_to_scope(&mut self, parent: FunctionId, dependent: FunctionId) {
        let scope_id = self.function_scopes.get(&parent).copied().unwrap_or_else(|| {
            self.new_scope(parent)
        });

        self.function_scopes.insert(dependent.clone(), scope_id);
        self.scopes.get_mut(&scope_id).unwrap().functions.insert(dependent);
    }

    fn add_indirect_dependencies(&mut self, function: &Function) {
        let mut scope = self.function_scopes[&function.id];

        function.for_each_id(self, |this: &mut Self, function_id: &FunctionId| {
            let new_scope_id = this.function_scopes[function_id];
            let new_scope = &this.scopes[&new_scope_id];

            if scope != new_scope_id && *function_id != new_scope.entry_point {
                let old_scope = this.scopes.remove(&scope).unwrap();
                this.merge_scopes(new_scope_id, old_scope);
                scope = new_scope_id;
            }
        }, |_, _| ());
    }

    /// Move each item in the child scope into the parent scope
    fn merge_scopes(&mut self, parent_scope_id: ScopeId, child_scope: Scope) {
        let parent_scope = self.scopes.get_mut(&parent_scope_id).unwrap();

        for function in child_scope.functions {
            self.function_scopes.insert(function.clone(), parent_scope_id);
            parent_scope.functions.insert(function);
        }
    }

    fn new_scope(&mut self, entry_point: FunctionId) -> ScopeId {
        let id = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;

        self.function_scopes.insert(entry_point.clone(), id);
        let mut functions = HashSet::new();
        functions.insert(entry_point.clone());
        self.scopes.insert(id, Scope { entry_point, functions });
        id
    }

    pub(super) fn get_scope(&self, id: &FunctionId) -> &Scope {
        let scope_id = self.function_scopes[id];
        &self.scopes[&scope_id]
    }
}

impl Scope {
    /// True if this Scope is in Control-Flow Form.
    pub fn is_cff(&self, mir: &Mir, scopes: &Scopes) -> bool {
        self.functions.iter().all(|function_id| {
            !mir.functions[&function_id].is_bad(scopes)
        })
    }
}

impl Display for Scopes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for scope in self.scopes.values() {
            writeln!(f, "{}", scope)?;
        }
        Ok(())
    }
}

impl Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items = fmap(&self.functions, ToString::to_string).join(", ");
        write!(f, "{} -> [{items}]", self.entry_point)
    }
}
