use crate::cache::{EffectBindingId, EffectInfoId, ModuleCache};
use crate::types::typechecker::TypeBindings;
use crate::types::Type;
use crate::util::fmap;

use super::typechecker::{self, UnificationBindings};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EffectSet {
    pub effects: Vec<Effect>,
    pub replacement: EffectBindingId,
}

pub type Effect = (EffectInfoId, Vec<Type>);

#[derive(Debug)]
pub enum EffectBinding {
    Bound(EffectSet),
    Unbound,
}

impl EffectSet {
    /// Create a new polymorphic effect set
    pub fn any(cache: &mut ModuleCache) -> EffectSet {
        EffectSet { effects: vec![], replacement: cache.next_effect_binding_id() }
    }

    pub fn follow_bindings<'a>(&'a self, cache: &'a ModuleCache) -> &'a Self {
        match &cache.effect_bindings[self.replacement.0] {
            EffectBinding::Bound(effects) => effects.follow_bindings(cache),
            EffectBinding::Unbound => self,
        }
    }

    pub fn follow_unification_bindings<'a>(
        &'a self, bindings: &'a UnificationBindings, cache: &'a ModuleCache,
    ) -> &'a Self {
        match &cache.effect_bindings[self.replacement.0] {
            EffectBinding::Bound(effects) => effects.follow_unification_bindings(bindings, cache),
            EffectBinding::Unbound => match bindings.effect_bindings.get(&self.replacement) {
                Some(effects) => effects.follow_unification_bindings(bindings, cache),
                None => self,
            },
        }
    }

    /// Since we map only from TypeVariableId to Type, this will always
    /// instantiate the EffectEnd with a new type variable, is this desired?
    pub fn replace_all_typevars_with_bindings(
        &self, new_bindings: &mut TypeBindings, cache: &mut ModuleCache,
    ) -> EffectSet {
        let new_id = cache.next_effect_binding_id();
        let this = self.follow_bindings(cache);

        let replacement = new_id;

        let effects = fmap(this.effects.clone(), |(id, args)| {
            (id, fmap(args, |arg| typechecker::replace_all_typevars_with_bindings(&arg, new_bindings, cache)))
        });

        EffectSet { effects, replacement }
    }

    /// Replace any typevars found with the given type bindings
    ///
    /// Compared to `replace_all_typevars_with_bindings`, this function does not instantiate
    /// unbound type variables that were not in type_bindings. Thus if type_bindings is empty,
    /// this function will just clone the original EffectSet.
    pub fn bind_typevars(&self, type_bindings: &TypeBindings, cache: &ModuleCache) -> EffectSet {
        let this = self.follow_bindings(cache);
        let effects = fmap(&this.effects, |(id, args)| {
            (*id, fmap(args, |arg| typechecker::bind_typevars(arg, type_bindings, cache)))
        });
        EffectSet { effects, replacement: this.replacement }
    }

    pub fn try_unify_with_bindings(
        &self, other: &EffectSet, bindings: &mut UnificationBindings, cache: &mut ModuleCache,
    ) {
        let a = self.follow_unification_bindings(bindings, cache);
        let b = other.follow_unification_bindings(bindings, cache);

        let mut new_effects = a.effects.clone();
        new_effects.append(&mut b.effects.clone());
        new_effects.sort();
        new_effects.dedup();

        let a_id = a.replacement;
        let b_id = b.replacement;

        let mut new_effect = EffectSet::any(cache);
        new_effect.effects = new_effects;

        bindings.effect_bindings.insert(a_id, new_effect.clone());
        bindings.effect_bindings.insert(b_id, new_effect);
    }

    pub fn combine(&self, other: &EffectSet, cache: &mut ModuleCache) -> EffectSet {
        let a = self.follow_bindings(cache);
        let b = other.follow_bindings(cache);

        let mut new_effects = a.effects.clone();
        new_effects.append(&mut b.effects.clone());
        new_effects.sort();
        new_effects.dedup();

        let a_id = a.replacement;
        let b_id = b.replacement;

        let mut new_effect = EffectSet::any(cache);
        new_effect.effects = new_effects;

        cache.effect_bindings[a_id.0] = EffectBinding::Bound(new_effect.clone());
        cache.effect_bindings[b_id.0] = EffectBinding::Bound(new_effect.clone());

        new_effect
    }
}
