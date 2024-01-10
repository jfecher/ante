use crate::mir::ir::{DefinitionId, Atom, Type};

#[derive(Clone, Default)]
pub struct EffectStack {
    effects: Vec<Effect>,
}

impl Eq for EffectStack {}

impl PartialEq for EffectStack {
    fn eq(&self, other: &Self) -> bool {
        if self.effects.len() != other.effects.len() {
            false
        } else if self.effects.is_empty() {
            true
        } else {
            let self_last = self.effects.last().unwrap();
            let other_last = other.effects.last().unwrap();
            self_last.time_stamp == other_last.time_stamp
        }
    }
}

impl std::hash::Hash for EffectStack {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.effects.len().hash(state);

        if let Some(last) = self.effects.last() {
            last.time_stamp.hash(state);
        }
    }
}

#[derive(Clone)]
pub struct Effect {
    pub id: DefinitionId,
    pub handler: Atom,
    pub handler_type: Type,

    /// The time stamp on an effect is an id which increases each time the
    /// EffectStack is pushed to. It is used as a quick method of determining
    /// equality and hashing of effect stacks.
    pub time_stamp: usize,
}

impl EffectStack {
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects }
    }

    /// Find the effect with the given id and panic if not found
    pub fn find(&self, id: DefinitionId) -> &Effect {
        self.effects.iter().find(|effect| effect.id == id).unwrap()
    }

    pub fn as_slice(&self) -> &[Effect] {
        &self.effects
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn push(&mut self, effect: Effect) {
        self.effects.push(effect);
    }
}

impl<'a> IntoIterator for &'a EffectStack {
    type Item = &'a Effect;

    type IntoIter = std::slice::Iter<'a, Effect>;

    fn into_iter(self) -> Self::IntoIter {
        self.effects.iter()
    }
}
