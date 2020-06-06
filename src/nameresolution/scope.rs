use std::collections::HashMap;
use crate::nameresolution::modulecache::{ DefinitionInfoId, TraitInfoId, ModuleCache };
use crate::types::{ TypeInfoId, TypeVariableId };
use crate::error::location::{ Location, Locatable };

#[derive(Debug, Default)]
pub struct Scope {
    pub definitions: HashMap<String, DefinitionInfoId>,
    pub types: HashMap<String, TypeInfoId>,
    pub type_variables: HashMap<String, TypeVariableId>,
    pub traits: HashMap<String, TraitInfoId>,
}

impl Scope {
    pub fn import(&mut self, other: &Scope, cache: &mut ModuleCache, location: Location) {
        macro_rules! merge_table {
            ( $field:tt , $cache_field:tt ) => ({
                for (k, v) in other.$field.iter() {
                    if let Some(existing) = self.$field.get(k) {
                        let prev_loc = cache.$cache_field[existing.0].locate();
                        error!(location, "import shadows previous definition of {}", k);
                        note!(prev_loc, "{} was previously defined here", k);
                    } else {
                        self.$field.insert(k.clone(), *v);
                    }
                }
            });
        }

        merge_table!(definitions, definition_infos);
        merge_table!(types, type_info);
    }
}

/// A FunctionScope contains all the names visible within a function
/// at a fixed point in time. For example if we are compiling the line:
///
/// foo a b =
///     if a then
///         c = 2
///     else
///         type Tmp = i32
///         d = \x. x + 3
///         d b            // <- here
///
/// Then the FunctionScope at that point in time will be:
/// vec![
///   { definitions: a, b },
///   { definitions: d, types: Tmp },
/// ]
#[derive(Debug)]
pub struct FunctionScope {
    scopes: Vec<Scope>,
}

impl FunctionScope {
    pub fn new() -> FunctionScope {
        FunctionScope {
            scopes: vec![Scope::default()],
        }
    }

    pub fn iter(&self) -> std::slice::Iter<Scope> {
        self.scopes.iter()
    }

    pub fn push(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    pub fn top(&mut self) -> &mut Scope {
        let top = self.scopes.len() - 1;
        &mut self.scopes[top]
    }

    pub fn second(&mut self) -> &mut Scope {
        let i = self.scopes.len() - 2;
        &mut self.scopes[i]
    }

    pub fn bottom(&self) -> &Scope {
        &self.scopes[0]
    }
}
