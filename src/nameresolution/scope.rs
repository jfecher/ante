use std::collections::HashMap;
use crate::nameresolution::modulecache::{ DefinitionInfoId, TraitInfoId, ModuleCache };
use crate::types::{ TypeInfoId, TypeVariableId };
use crate::error::location::{ Location, Locatable };

#[derive(Debug, Default)]
pub struct Scope {
    pub definitions: HashMap<String, DefinitionInfoId>,
    pub types: HashMap<String, TypeInfoId>,
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
        merge_table!(types, type_infos);
    }

    pub fn check_for_unused_definitions(&self, cache: &ModuleCache) {
        macro_rules! check {
            ( $field:tt , $cache_field:tt ) => ({
                for (name, id) in &self.$field {
                    let definition = &cache.$cache_field[id.0];
                    if definition.uses == 0 && definition.name.chars().next() != Some('_') {
                        warning!(definition.location, "{} is unused (prefix name with _ to silence this warning)", name);
                    }
                }
            });
        }

        check!(definitions, definition_infos);
        check!(types, type_infos);
    }
}

/// A TypeVariableScope is an alternative to "normal" scopes that other symbols
/// live in. This is needed in general because type variables do not follow normal
/// scoping rules. Consider the following trait definition:
///
/// trait Bar a
///     bar a a -> a
///
/// In it, bar should be declared globally yet should also be able to reference
/// the type variable a that any other global shouldn't be able to access. The
/// solution to this used by this compiler is to give type variables different
/// scoping rules that follow more closely how they're defined in a lexical scope.
#[derive(Debug, Default)]
pub struct TypeVariableScope {
    type_variables: HashMap<String, TypeVariableId>,
}

impl TypeVariableScope {
    pub fn push_existing_type_variable(&mut self, key: String, id: TypeVariableId) -> TypeVariableId {
        let prev = self.type_variables.insert(key, id);
        assert!(prev.is_none());
        id
    }

    pub fn get(&self, key: &str) -> Option<&TypeVariableId> {
        self.type_variables.get(key)
    }
}
