use std::{rc::Rc, hash::Hash};

/// Each function is given a globally unique ID
#[derive(Clone, Eq)]
pub struct FunctionId {
    pub id: u32,
    pub name: Rc<String>,
}

/// A parameter id is just the function it originates from and the index of the parameter
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ParameterId {
    pub function: FunctionId,
    pub parameter_index: u16,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ExternId(pub u32);

/// Each Handler node is given a unique ID for specialization of each
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct HandlerId(pub u32);

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct EffectId(pub u32);

impl Hash for FunctionId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialEq for FunctionId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ord for FunctionId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for FunctionId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Forward Debug => Display for FunctionId & ParameterId
impl std::fmt::Debug for FunctionId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::fmt::Debug for ParameterId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
