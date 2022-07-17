use crate::cache::{EffectInfoId, EffectBindingId, ModuleCache};
use crate::error::location::Location;
use crate::types::{ Type, TypeVariableId };
use crate::types::typechecker::TypeBindings;

use super::typechecker::UnificationBindings;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Effects {
    pub effects: Vec<Effect>,
    pub rest: EffectEnd,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Effect {
    Known(EffectInfoId, Vec<Type>),
    Variable(TypeVariableId),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum EffectEnd {
    Open(EffectBindingId),
    Closed,
}

#[derive(Debug)]
pub enum EffectBinding {
    Bound(Effects),
    Unbound
}

impl Effects {
    /// Create a new polymorphic effect set
    pub fn any(cache: &mut ModuleCache) -> Effects {
        Effects {
            effects: vec![],
            rest: EffectEnd::Open(cache.next_effect_binding_id()),
        }
    }

    pub fn none() -> Effects {
        Effects {
            effects: vec![],
            rest: EffectEnd::Closed,
        }
    }

    pub fn combine(&mut self, other: &mut Self, cache: &mut ModuleCache) {
        self.effects.append(&mut other.effects);
    }

    pub fn replace_all_typevars_with_bindings(&self, new_bindings: &mut TypeBindings, cache: &mut ModuleCache) -> Effects {
        todo!()
    }

    /// Replace any typevars found with the given type bindings
    ///
    /// Compared to `replace_all_typevars_with_bindings`, this function does not instantiate
    /// unbound type variables that were not in type_bindings. Thus if type_bindings is empty,
    /// this function will just clone the original Effects.
    pub fn bind_typevars(&self, type_bindings: &TypeBindings, cache: &ModuleCache) -> Effects {
        todo!()
    }

    pub fn try_unify_with_bindings(&self, effects: &Effects, bindings: &mut UnificationBindings, location: Location, cache: &mut ModuleCache) -> Result<(), ()> {
        todo!()
    }
}
