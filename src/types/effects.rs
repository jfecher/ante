use crate::cache::{EffectInfoId, ModuleCache};
use crate::error::location::Location;
use crate::error::TypeErrorKind as TE;
use crate::types::typechecker::TypeBindings;
use crate::types::Type;
use crate::util::fmap;

use super::typechecker::{self, OccursResult, UnificationBindings};
use super::{TypeBinding, TypeVariableId};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EffectSet {
    pub effects: Vec<Effect>,

    /// A value of `None` means this set is closed to extension.
    /// A value of `Some(unbound type variable)` means this set is
    /// open to extension.
    /// A value of `Some(bound type variable)` means this set has
    /// been extended and should be ignored entirely in favor of the
    /// type it is now bound to.
    pub replacement: Option<TypeVariableId>,
}

pub type Effect = (EffectInfoId, Vec<Type>);

impl EffectSet {
    /// Create a new, empty polymorphic effect set
    pub fn any(cache: &mut ModuleCache) -> EffectSet {
        EffectSet { effects: vec![], replacement: Some(typechecker::next_type_variable_id(cache)) }
    }

    pub fn new(effects: Vec<(EffectInfoId, Vec<Type>)>, cache: &mut ModuleCache) -> EffectSet {
        let mut set = EffectSet::any(cache);
        set.effects = effects;
        set
    }

    /// Create an effect set with only the given effects, not letting it be extended any further.
    pub fn only(effects: Vec<(EffectInfoId, Vec<Type>)>) -> EffectSet {
        EffectSet { effects, replacement: None }
    }

    /// Create an empty effect set, not letting it be extended any further.
    pub fn pure() -> EffectSet {
        EffectSet::only(Vec::new())
    }

    pub fn follow_bindings<'a>(&'a self, cache: &'a ModuleCache) -> &'a Self {
        let Some(replacement) = self.replacement else {
            return self;
        };

        match &cache.type_bindings[replacement.0] {
            TypeBinding::Bound(Type::Effects(effects)) => effects.follow_bindings(cache),
            TypeBinding::Bound(typevar @ Type::TypeVariable(_)) => match &cache.follow_typebindings_shallow(typevar) {
                Type::Effects(effects) => effects.follow_bindings(cache),
                _ => self,
            },
            _ => self,
        }
    }

    pub fn follow_unification_bindings<'a>(
        &'a self, bindings: &'a UnificationBindings, cache: &'a ModuleCache,
    ) -> &'a Self {
        let Some(replacement) = self.replacement else {
            return self;
        };

        match &cache.type_bindings[replacement.0] {
            TypeBinding::Bound(Type::Effects(effects)) => effects.follow_unification_bindings(bindings, cache),
            _ => match bindings.bindings.get(&replacement) {
                Some(Type::Effects(effects)) => effects.follow_unification_bindings(bindings, cache),
                _ => self,
            },
        }
    }

    pub fn replace_all_typevars_with_bindings(&self, new_bindings: &mut TypeBindings, cache: &mut ModuleCache) -> Type {
        let mut new_replacement = None;

        if let Some(replacement) = self.replacement {
            if let TypeBinding::Bound(Type::Effects(effects)) = &cache.type_bindings[replacement.0] {
                return effects.clone().replace_all_typevars_with_bindings(new_bindings, cache);
            }

            new_replacement = match new_bindings.get(&replacement) {
                Some(Type::TypeVariable(new_id)) => Some(*new_id),
                Some(other) => return other.clone(),
                None => Some(typechecker::next_type_variable_id(cache)),
            };
        }

        let effects = fmap(&self.effects, |(id, args)| {
            (*id, fmap(args, |arg| typechecker::replace_all_typevars_with_bindings(arg, new_bindings, cache)))
        });

        Type::Effects(EffectSet { effects, replacement: new_replacement })
    }

    /// Replace any typevars found with the given type bindings
    ///
    /// Compared to `replace_all_typevars_with_bindings`, this function does not instantiate
    /// unbound type variables that were not in type_bindings. Thus if type_bindings is empty,
    /// this function will just clone the original EffectSet.
    pub fn bind_typevars(&self, type_bindings: &TypeBindings, cache: &ModuleCache) -> Type {
        // type_bindings is checked for bindings before the cache, see the comment
        // in typechecker::bind_typevar
        let replacement = if let Some(replacement) = self.replacement {
            match type_bindings.get(&replacement) {
                Some(Type::TypeVariable(new_id)) => Some(*new_id),
                Some(other) => return other.clone(),
                None => self.replacement,
            }
        } else {
            None
        };

        if let Some(replacement) = replacement {
            if let TypeBinding::Bound(typ) = &cache.type_bindings[replacement.0] {
                return typechecker::bind_typevars(&typ.clone(), type_bindings, cache);
            }
        }

        let effects = fmap(&self.effects, |(id, args)| {
            (*id, fmap(args, |arg| typechecker::bind_typevars(arg, type_bindings, cache)))
        });

        Type::Effects(EffectSet { effects, replacement })
    }

    pub fn try_unify_with_bindings(
        &self, other: &EffectSet, bindings: &mut UnificationBindings, cache: &mut ModuleCache,
    ) -> Result<(), ()> {
        let a = self.follow_unification_bindings(bindings, cache);
        let b = other.follow_unification_bindings(bindings, cache);

        let mut new_effects = a.effects.clone();
        new_effects.append(&mut b.effects.clone());
        new_effects.sort();
        new_effects.dedup();

        let a_id = a.replacement;
        let b_id = b.replacement;

        // Checking these now avoids having to clone them versus combining this
        // with the a_id and b_id checks later.
        if a_id.is_none() && a.effects != new_effects || b_id.is_none() && b.effects != new_effects {
            return Err(());
        }

        let new_effect = EffectSet::new(new_effects, cache);
        if let Some(a_id) = a_id {
            bindings.bindings.insert(a_id, Type::Effects(new_effect.clone()));
        }

        if let Some(b_id) = b_id {
            bindings.bindings.insert(b_id, Type::Effects(new_effect.clone()));
        }

        Ok(())
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

        let new_effect = EffectSet::new(new_effects, cache);
        if let Some(a_id) = a_id {
            cache.bind(a_id, Type::Effects(new_effect.clone()));
        }
        if let Some(b_id) = b_id {
            cache.bind(b_id, Type::Effects(new_effect.clone()));
        }
        new_effect
    }

    pub fn find_all_typevars(&self, polymorphic_only: bool, cache: &ModuleCache) -> Vec<super::TypeVariableId> {
        let this = self.follow_bindings(cache);
        let mut vars = match this.replacement {
            Some(replacement) => typechecker::find_typevars_in_typevar_binding(replacement, polymorphic_only, cache),
            None => Vec::new(),
        };

        for (_, args) in &this.effects {
            for arg in args {
                vars.append(&mut typechecker::find_all_typevars(arg, polymorphic_only, cache));
            }
        }

        vars
    }

    pub fn contains_any_typevars_from_list(&self, list: &[super::TypeVariableId], cache: &ModuleCache) -> bool {
        let this = self.follow_bindings(cache);
        this.replacement.map_or(false, |replacement| list.contains(&replacement))
            || this
                .effects
                .iter()
                .any(|(_, args)| args.iter().any(|arg| typechecker::contains_any_typevars_from_list(arg, list, cache)))
    }

    pub(super) fn occurs(
        &self, id: super::TypeVariableId, level: super::LetBindingLevel, bindings: &mut UnificationBindings, fuel: u32,
        cache: &mut ModuleCache,
    ) -> OccursResult {
        let this = self.follow_bindings(cache).clone();
        let mut result = match this.replacement {
            Some(replacement) => typechecker::typevars_match(id, level, replacement, bindings, fuel, cache),
            None => OccursResult::does_not_occur(),
        };

        for (_, args) in &this.effects {
            result = result.then_all(args, |arg| typechecker::occurs(id, level, arg, bindings, fuel, cache));
        }
        result
    }

    /// Mutates self to the set difference between self and other.
    /// Any effects that are removed are added to `handled_effects`.
    pub(super) fn handle_effects_from(
        &mut self, other: EffectSet, handled_effects: &mut Vec<Effect>, level: super::LetBindingLevel,
        cache: &mut ModuleCache,
    ) {
        let a = self.follow_bindings(cache).clone();
        let b = other.follow_bindings(cache).clone();

        let mut new_effects = Vec::with_capacity(a.effects.len());

        for a_effect in a.effects.iter() {
            match find_matching_effect(a_effect, &b.effects, cache) {
                Ok(bindings) => {
                    bindings.perform(cache);
                    handled_effects.push(a_effect.clone());
                },
                Err(()) => new_effects.push(a_effect.clone()),
            }
        }

        self.effects = new_effects;
        self.replacement = self.replacement.map(|_| cache.next_type_variable_id(level));
    }
}

fn find_matching_effect(effect: &Effect, set: &[Effect], cache: &mut ModuleCache) -> Result<UnificationBindings, ()> {
    let (effect_id, effect_args) = effect;
    for (other_id, other_args) in set {
        if effect_id == other_id {
            let bindings = UnificationBindings::empty();
            let no_loc = Location::builtin();

            if let Ok(bindings) = typechecker::try_unify_all_with_bindings(
                effect_args,
                other_args,
                bindings,
                no_loc,
                cache,
                TE::NeverShown,
            ) {
                return Ok(bindings);
            }
        }
    }
    Err(())
}
