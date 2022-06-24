//! nameresolution/scope.rs - Defines the Scope struct which
//! represents all symbols visible in a given scope.
//!
//! This module also includes methods for importing a scope
//! into another which effectively merges them, and issuing
//! unused variable warnings which is done when a scope is
//! popped off the NameResolvers scope stack.
//!
//! This file also defines the TypeVariableScope struct which
//! is significant because a type variable's scope is different
//! than the general Scope for other symbols. See the TypeVariableScope
//! struct for more details on this.
use crate::cache::{DefinitionInfoId, ImplInfoId, ImplScopeId, ModuleCache, TraitInfoId, ModuleId};
use crate::error::location::{Locatable, Location};
use crate::parser::ast;
use crate::types::{TypeInfoId, TypeVariableId};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// A scope represents all symbols defined in a given scope.
///
/// This is not the set of all symbols visible in scope - that
/// would be determined by the stack of scopes held by a
/// NameResolver at a given point in time.
///
/// Scopes are thrown away after name resolution finishes since
/// all variables should be linked to their corresponding
/// DefinitionInfoId afterward. The main exception are the ImplScopeId
/// keys which can be used to retrieve which impls were in scope for
/// a given variable later during type inference.
#[derive(Debug, Clone)]
pub struct Scope {
    pub definitions: HashMap<String, DefinitionInfoId>,
    pub types: HashMap<String, TypeInfoId>,
    pub traits: HashMap<String, TraitInfoId>,
    pub impls: HashMap<TraitInfoId, Vec<ImplInfoId>>,
    pub impl_scope: ImplScopeId,
    pub modules: HashMap<String, ModuleId>,
}

impl Scope {
    pub fn new(cache: &mut ModuleCache) -> Scope {
        Scope {
            impl_scope: cache.push_impl_scope(),
            definitions: HashMap::new(),
            types: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            modules: HashMap::new(),
        }
    }

    /// Imports all symbols from the given scope into the current scope.
    ///
    /// This is meant to be done in the "define" pass of name resolution after which
    /// symbols are exported are determined in the "declare" pass. This is because since
    /// the other Scope's symbols are mutably added to self, they cannot be easily distinguished
    /// from definitions originating in this scope.
    pub fn import(&mut self, other: &Scope, cache: &mut ModuleCache, location: Location, symbols: &HashSet<String>) {
        self.import_definitions_types_and_traits(other, cache, location, symbols);
        self.import_impls(other, cache);
    }

    pub fn import_impls(&mut self, other: &Scope, cache: &mut ModuleCache) {
        for (k, v) in other.impls.iter() {
            if let Some(existing) = self.impls.get_mut(k) {
                existing.append(&mut v.clone());
            } else {
                self.impls.insert(*k, v.clone());
            }

            // TODO optimization: speed up propogation of impls, this shouldn't be necessary.
            cache.impl_scopes[self.impl_scope.0].append(&mut v.clone());
        }
    }

    /// Helper for `import` which imports all non-impl symbols.
    fn import_definitions_types_and_traits(&mut self, other: &Scope, cache: &mut ModuleCache, location: Location, symbols: &HashSet<String>) {
        macro_rules! merge_table {
            ( $field:tt , $cache_field:tt , $errors:tt, $symbols:expr ) => {{
                for (k, v) in other.$field.iter() {
                    if !$symbols.is_empty() && !$symbols.contains(k) {
                        continue;
                    }

                    if self.$field.get(k).is_none() {
                        self.$field.insert(k.clone(), *v);
                    }
                }
            }};
        }

        merge_table!(definitions, definition_infos, errors, symbols);
        merge_table!(types, type_infos, errors, symbols);
        merge_table!(traits, trait_infos, errors, symbols);
    }

    /// Check for any unused definitions and issue the appropriate warnings if found.
    /// This is meant to be done at the end of a scope since if we're still in the middle
    /// of name resolution for a particular scope, any currently unused symbol may become
    /// used later on.
    pub fn check_for_unused_definitions(&self, cache: &ModuleCache, id_to_ignore: Option<DefinitionInfoId>) {
        let mut warnings = vec![];

        for (name, id) in &self.definitions {
            if id_to_ignore != Some(*id) {
                let definition = &cache.definition_infos[id.0];
                if definition.uses == 0 && !definition.name.starts_with('_') {
                    warnings.push(make_warning!(
                        definition.location,
                        "{} is unused (prefix name with _ to silence this warning)",
                        name
                    ));
                }
            }
        }

        for (name, id) in &self.types {
            let definition = &cache.type_infos[id.0];
            if definition.uses == 0 && !definition.name.starts_with('_') {
                warnings.push(make_warning!(
                    definition.location,
                    "{} is unused (prefix name with _ to silence this warning)",
                    name
                ));
            }
        }

        if !warnings.is_empty() {
            warnings.sort();
            warnings.into_iter().for_each(|warning| eprintln!("{}", warning));
        }
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

#[derive(Debug)]
pub struct FunctionScopes {
    pub function: Option<*mut ast::Lambda<'static>>,
    pub function_id: Option<DefinitionInfoId>,
    pub scopes: Vec<Scope>,
}

impl FunctionScopes {
    pub fn new() -> FunctionScopes {
        FunctionScopes { function: None, function_id: None, scopes: vec![] }
    }

    pub fn from_lambda(lambda: &mut ast::Lambda, id: Option<DefinitionInfoId>) -> FunctionScopes {
        let function = Some(unsafe { std::mem::transmute(lambda) });
        FunctionScopes { function, function_id: id, scopes: vec![] }
    }

    pub fn iter(&self) -> std::slice::Iter<Scope> {
        self.scopes.iter()
    }

    pub fn last_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    pub fn first(&self) -> &Scope {
        self.scopes.first().unwrap()
    }

    pub fn first_mut(&mut self) -> &mut Scope {
        self.scopes.first_mut().unwrap()
    }

    pub fn pop(&mut self) {
        self.scopes.pop();
    }

    pub fn push_new_scope<'c>(&mut self, cache: &mut ModuleCache<'c>) {
        self.scopes.push(Scope::new(cache));
    }

    /// Within the current function, map an existing variable to a parameter variable
    /// that is part of the closure's environment. This mapping is remembered for codegen
    /// so we can store the existing variable along with the closure as part of its environment.
    pub fn add_closure_environment_variable_mapping<'c>(
        &mut self, existing: DefinitionInfoId, parameter: DefinitionInfoId, location: Location<'c>,
        cache: &mut ModuleCache<'c>,
    ) {
        let function =
            self.function.expect("Internal compiler error: attempted to create a closure without a current function");
        let function = unsafe { function.as_mut().unwrap() };

        let name = cache[existing].name.clone();
        let fake_var = cache.push_variable(name, location);
        function.closure_environment.insert(existing, (fake_var, parameter, Rc::new(HashMap::new())));
    }
}
