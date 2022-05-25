use crate::types::traits::TraitConstraintId;

// This can be generalized to more types if needed.
#[derive(Debug, Default)]
pub struct TraitConstraintCounter(u32);

impl TraitConstraintCounter {
    pub fn next(&mut self) -> TraitConstraintId {
        let current = self.0;
        self.0 += 1;
        TraitConstraintId(current)
    }
}
