use crate::cache::{EffectInfoId, ModuleCache};
use crate::error::location::Location;
use crate::error::TypeErrorKind as TE;
use crate::types::typechecker::{try_unify_all_with_bindings, TypeBindings};
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
    /// been extended and the full set of effects is the current
    /// effects appended to any effects in the extension.
    pub extension: Option<TypeVariableId>,
}

pub type Effect = (EffectInfoId, Vec<Type>);

impl EffectSet {
    /// Create a new, empty polymorphic effect set
    pub fn any(cache: &mut ModuleCache) -> EffectSet {
        EffectSet { effects: vec![], extension: Some(typechecker::next_type_variable_id(cache)) }
    }

    pub fn new(effects: Vec<Effect>, extension: Option<TypeVariableId>) -> EffectSet {
        EffectSet { effects, extension }
    }

    /// Create an effect set with only the given effects, not letting it be extended any further.
    pub fn only(effects: Vec<Effect>) -> EffectSet {
        EffectSet { effects, extension: None }
    }

    /// Create an empty effect set, not letting it be extended any further.
    pub fn pure() -> EffectSet {
        EffectSet::only(Vec::new())
    }

    /// Flattens this EffectSet, returning a new EffectSet containing
    /// the effects from `self` and all extensions of `self`, if any.
    pub fn flatten<'a>(&'a self, cache: &'a ModuleCache) -> Self {
        let mut effects = self.effects.clone();
        let mut extension = self.extension;

        loop {
            let Some(extension_var) = extension.as_mut() else {
                Self::dedup_effects(&mut effects, cache);
                return Self::only(effects);
            };

            match &cache.type_bindings[extension_var.0] {
                TypeBinding::Bound(Type::Effects(new_set)) => {
                    extension = new_set.extension;
                    effects.extend_from_slice(&new_set.effects);
                },
                TypeBinding::Bound(Type::TypeVariable(typevar)) => {
                    *extension_var = *typevar;
                },
                _ => {
                    Self::dedup_effects(&mut effects, cache);
                    break Self { effects, extension };
                },
            }
        }
    }

    fn dedup_effects(effects: &mut Vec<Effect>, cache: &ModuleCache) {
        for (_id, args) in effects.iter_mut() {
            for arg in args {
                *arg = cache.follow_bindings(arg);
            }
        }
        effects.sort();
        effects.dedup();
    }

    fn follow_unification_bindings(&self, bindings: &UnificationBindings, cache: &ModuleCache) -> Self {
        let this = self.flatten(cache);
        let Some(extension) = this.extension else {
            return this;
        };

        let Some(typ) = bindings.bindings.get(&extension) else {
            return this;
        };

        let mut extended = typ.flatten_effects(cache);
        extended.effects.extend(this.effects);
        extended.effects.sort();
        extended.effects.dedup();
        extended
    }

    pub fn replace_all_typevars_with_bindings(&self, new_bindings: &mut TypeBindings, cache: &mut ModuleCache) -> Type {
        let effects = fmap(&self.effects, |(id, args)| {
            (*id, fmap(args, |arg| typechecker::replace_all_typevars_with_bindings(arg, new_bindings, cache)))
        });

        let extension = self.extension.map(|extension| {
            match typechecker::replace_typevar_with_binding(extension, new_bindings, cache) {
                Type::TypeVariable(id) => id,
                other => {
                    let new_id = typechecker::next_type_variable_id(cache);
                    cache.bind(new_id, other);
                    new_id
                },
            }
        });

        Type::Effects(EffectSet { effects, extension })
    }

    /// Replace any typevars found with the given type bindings
    ///
    /// Compared to `replace_all_typevars_with_bindings`, this function does not instantiate
    /// unbound type variables that were not in type_bindings. Thus if type_bindings is empty,
    /// this function will just clone the original EffectSet.
    pub fn bind_typevars(&self, type_bindings: &TypeBindings, cache: &ModuleCache) -> Type {
        let mut this = self.flatten(cache);

        this.effects = fmap(this.effects, |(id, args)| {
            (id, fmap(args, |arg| typechecker::bind_typevars(&arg, type_bindings, cache)))
        });

        // type_bindings is checked for bindings before the cache, see the comment
        // in typechecker::bind_typevar
        if let Some(extension) = this.extension {
            match type_bindings.get(&extension) {
                Some(Type::TypeVariable(new_id)) => {
                    this.extension = Some(*new_id);
                },
                Some(Type::Effects(more_effects)) => {
                    this.effects.extend(more_effects.effects.clone());
                    this.extension = more_effects.extension;
                },
                None => (),
                Some(other) => unreachable!("Cannot bind effects to {}", other.approx_to_string()),
            }
        }

        Type::Effects(this)
    }

    pub fn try_unify_with_bindings<'c>(
        &self, other: &EffectSet, bindings: &mut UnificationBindings, location: Location<'c>,
        cache: &mut ModuleCache<'c>,
    ) -> Result<(), ()> {
        let a = self.follow_unification_bindings(bindings, cache);
        let b = other.follow_unification_bindings(bindings, cache);

        let mut new_effects_in_a = Vec::new();
        let mut new_effects_in_b = Vec::new();

        // Expect effects to be sorted
        let mut check_effects = |effects_a: &[Effect], effects_b: &[Effect], new_effects: &mut Vec<Effect>| {
            for (a_id, a_args) in effects_a {
                let mut handled = false;

                for (b_id, b_args) in effects_b {
                    if a_id == b_id {
                        let new_bindings = UnificationBindings::empty();
                        let result =
                            try_unify_all_with_bindings(a_args, b_args, new_bindings, location, cache, TE::NeverShown);
                        if let Ok(new_bindings) = result {
                            bindings.extend(new_bindings);
                            handled = true;
                            break;
                        }
                    }
                }

                if !handled {
                    new_effects.push((*a_id, a_args.to_vec()));
                }
            }
        };

        check_effects(&a.effects, &b.effects, &mut new_effects_in_b);
        check_effects(&b.effects, &a.effects, &mut new_effects_in_a);

        if a.extension.is_none() && !new_effects_in_a.is_empty()
            || b.extension.is_none() && !new_effects_in_b.is_empty()
        {
            return Err(());
        }

        let fresh_extension = typechecker::next_type_variable_id(cache);

        let mut extend_effects = |new_effects: Vec<Effect>, extension| {
            if !new_effects.is_empty() {
                let extended = EffectSet::new(new_effects, Some(fresh_extension));

                if let Some(extension) = extension {
                    bindings.bindings.insert(extension, Type::Effects(extended));
                    Ok(Some(fresh_extension))
                } else {
                    Err(())
                }
            } else {
                Ok(extension)
            }
        };

        let a_extension = extend_effects(new_effects_in_a, a.extension)?;
        let b_extension = extend_effects(new_effects_in_b, b.extension)?;

        if let (Some(a), Some(b)) = (a_extension, b_extension) {
            if a != b {
                let a_type = &Type::TypeVariable(a);
                let b_type = &Type::TypeVariable(b);
                typechecker::try_unify_type_variable_with_bindings(a, a_type, b_type, true, bindings, location, cache)?;
            }
        }

        Ok(())
    }

    #[allow(unused)]
    pub fn debug<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> super::typeprinter::TypePrinter<'a, 'b> {
        Type::Effects(self.clone()).debug(cache)
    }

    #[allow(unused)]
    pub fn display<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> super::typeprinter::TypePrinter<'a, 'b> {
        Type::Effects(self.clone()).display(cache)
    }

    /// To combine two separate effects into a single new effectset containing both,
    /// it is sufficient to bind each extension to the other effect set but with a
    /// fresh extension id so that it is not infinitely recursive. This may result in
    /// duplicate effects but these should be deduplicated later during `flatten` calls.
    pub fn combine(&self, other: &EffectSet, cache: &mut ModuleCache) {
        let a = self.flatten(cache);
        let b = other.flatten(cache);

        let a_ext = a.extension;
        let b_ext = b.extension;

        let extension_var = typechecker::next_type_variable_id(cache);

        let mut new_a_effects = Vec::new();
        let mut new_b_effects = Vec::new();

        for effect in &a.effects {
            match find_matching_effect(&effect, &b.effects, cache) {
                Ok(bindings) => bindings.perform(cache),
                Err(_) => new_b_effects.push(effect.clone()),
            }
        }

        for effect in &b.effects {
            match find_matching_effect(&effect, &a.effects, cache) {
                Ok(bindings) => bindings.perform(cache),
                Err(_) => new_a_effects.push(effect.clone()),
            }
        }

        if let Some(a_id) = a_ext {
            cache.bind(a_id, Type::Effects(EffectSet::new(new_a_effects, Some(extension_var))));
        }

        if let Some(b_id) = b_ext {
            cache.bind(b_id, Type::Effects(EffectSet::new(new_b_effects, Some(extension_var))));
        }
    }

    pub fn find_all_typevars(
        &self, polymorphic_only: bool, cache: &ModuleCache, fuel: u32,
    ) -> Vec<super::TypeVariableId> {
        let this = self.flatten(cache);
        let mut vars = match this.extension {
            Some(extension) => typechecker::find_typevars_in_typevar_binding(extension, polymorphic_only, cache, fuel),
            None => Vec::new(),
        };

        for (_, args) in &this.effects {
            for arg in args {
                vars.append(&mut typechecker::find_all_typevars_helper(arg, polymorphic_only, cache, fuel));
            }
        }

        vars
    }

    pub fn contains_any_typevars_from_list(&self, list: &[super::TypeVariableId], cache: &ModuleCache) -> bool {
        let this = self.flatten(cache);
        this.extension.map_or(false, |extension| list.contains(&extension))
            || this
                .effects
                .iter()
                .any(|(_, args)| args.iter().any(|arg| typechecker::contains_any_typevars_from_list(arg, list, cache)))
    }

    pub(super) fn occurs(
        &self, id: super::TypeVariableId, level: super::LetBindingLevel, bindings: &mut UnificationBindings, fuel: u32,
        cache: &mut ModuleCache,
    ) -> OccursResult {
        let this = self.flatten(cache);
        let mut result = match this.extension {
            Some(extension) => typechecker::typevars_match(id, level, extension, bindings, fuel, cache),
            None => OccursResult::does_not_occur(),
        };

        for (_, args) in &this.effects {
            result = result.then_all(args, |arg| typechecker::occurs_helper(id, level, arg, bindings, fuel, cache));
        }
        result
    }

    /// Mutates self to the set difference between self and other.
    /// Any effects that are removed are added to `handled_effects`.
    pub(super) fn handle_effects_from(
        &mut self, other: EffectSet, handled_effects: &mut Vec<Effect>, cache: &mut ModuleCache,
    ) {
        let a = self.flatten(cache);
        let b = other.flatten(cache);
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

        self.extension = a.extension;
        self.effects = new_effects;
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
