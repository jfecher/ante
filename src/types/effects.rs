use crate::cache::{EffectInfoId, ModuleCache};
use crate::types::typechecker::TypeBindings;
use crate::types::Type;
use crate::util::fmap;

use super::typechecker::{self, OccursResult, UnificationBindings};
use super::{TypeVariableId, TypeBinding};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EffectSet {
    pub effects: Vec<Effect>,
    pub replacement: TypeVariableId,
}

pub type Effect = (EffectInfoId, Vec<Type>);

impl EffectSet {
    /// Create a new polymorphic effect set
    pub fn any(cache: &mut ModuleCache) -> EffectSet {
        EffectSet { effects: vec![], replacement: typechecker::next_type_variable_id(cache) }
    }

    pub fn follow_bindings<'a>(&'a self, cache: &'a ModuleCache) -> &'a Self {
        match &cache.type_bindings[self.replacement.0] {
            TypeBinding::Bound(Type::Effects(effects)) => effects.follow_bindings(cache),
            _ => self,
        }
    }

    pub fn follow_unification_bindings<'a>(
        &'a self, bindings: &'a UnificationBindings, cache: &'a ModuleCache,
    ) -> &'a Self {
        match &cache.type_bindings[self.replacement.0] {
            TypeBinding::Bound(Type::Effects(effects)) => effects.follow_unification_bindings(bindings, cache),
            _ => match bindings.bindings.get(&self.replacement) {
                Some(Type::Effects(effects)) => effects.follow_unification_bindings(bindings, cache),
                _ => self,
            },
        }
    }

    pub fn replace_all_typevars_with_bindings(
        &self, new_bindings: &mut TypeBindings, cache: &mut ModuleCache,
    ) -> EffectSet {
        if let TypeBinding::Bound(Type::Effects(effects)) = &cache.type_bindings[self.replacement.0] {
            return effects.clone().replace_all_typevars_with_bindings(new_bindings, cache);
        }

        if let Some(Type::Effects(effects)) = new_bindings.get(&self.replacement) {
            return effects.clone();
        }

        let replacement = typechecker::next_type_variable_id(cache);

        let effects = fmap(&self.effects, |(id, args)| {
            (*id, fmap(args, |arg| typechecker::replace_all_typevars_with_bindings(&arg, new_bindings, cache)))
        });

        EffectSet { effects, replacement }
    }

    /// Replace any typevars found with the given type bindings
    ///
    /// Compared to `replace_all_typevars_with_bindings`, this function does not instantiate
    /// unbound type variables that were not in type_bindings. Thus if type_bindings is empty,
    /// this function will just clone the original EffectSet.
    pub fn bind_typevars(&self, type_bindings: &TypeBindings, cache: &ModuleCache) -> EffectSet {
        if let TypeBinding::Bound(Type::Effects(effects)) = &cache.type_bindings[self.replacement.0] {
            return effects.clone().bind_typevars(type_bindings, cache);
        }

        if let Some(Type::Effects(effects)) = type_bindings.get(&self.replacement) {
            return effects.clone();
        }

        let effects = fmap(&self.effects, |(id, args)| {
            (*id, fmap(args, |arg| typechecker::bind_typevars(arg, type_bindings, cache)))
        });
        EffectSet { effects, replacement: self.replacement }
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

        bindings.bindings.insert(a_id, Type::Effects(new_effect.clone()));
        bindings.bindings.insert(b_id, Type::Effects(new_effect));
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

        cache.type_bindings[a_id.0] = TypeBinding::Bound(Type::Effects(new_effect.clone()));
        cache.type_bindings[b_id.0] = TypeBinding::Bound(Type::Effects(new_effect.clone()));

        new_effect
    }

    pub fn find_all_typevars(&self, polymorphic_only: bool, cache: &ModuleCache) -> Vec<super::TypeVariableId> {
        let this = self.follow_bindings(cache);
        let mut vars = vec![this.replacement];

        for (_, args) in &this.effects {
            for arg in args {
                vars.append(&mut typechecker::find_all_typevars(arg, polymorphic_only, cache));
            }
        }
        vars
    }

    pub fn contains_any_typevars_from_list(&self, list: &[super::TypeVariableId], cache: &ModuleCache) -> bool {
        let this = self.follow_bindings(cache);
        list.contains(&this.replacement) ||
        this.effects
            .iter()
            .any(|(_, args)| args.iter().any(|arg| typechecker::contains_any_typevars_from_list(arg, list, cache)))
    }

    pub(super) fn occurs(
        &self, id: super::TypeVariableId, level: super::LetBindingLevel, bindings: &mut UnificationBindings, fuel: u32,
        cache: &mut ModuleCache,
    ) -> OccursResult {
        let this = self.follow_bindings(cache).clone();
        let mut result = typechecker::typevars_match(id, level, this.replacement, bindings, fuel, cache);

        for (_, args) in &this.effects {
            result = result.then_all(args, |arg| typechecker::occurs(id, level, arg, bindings, fuel, cache));
        }
        result
    }
}
