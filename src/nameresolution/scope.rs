use std::collections::HashMap;
use crate::nameresolution::modulecache::{ DefinitionInfoId, TraitInfoId };
use crate::types::TypeInfoId;

#[derive(Debug, Default)]
pub struct Scope {
    pub definitions: HashMap<String, DefinitionInfoId>,
    pub types: HashMap<String, TypeInfoId>,
    pub traits: HashMap<String, TraitInfoId>,
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
#[derive(Debug, Default)]
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
}
