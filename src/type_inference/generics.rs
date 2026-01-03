use serde::{Deserialize, Serialize};

use crate::{
    name_resolution::Origin,
    type_inference::types::{Type, TypeVariableId},
};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Generic {
    Named(Origin),
    Inferred(TypeVariableId),
}

impl Generic {
    /// If this generic has a [TypeVariableId], return it as a [Type::Variable].
    /// Otherwise, return it as a [Type::Generic].
    pub(crate) fn as_type(&self) -> Type {
        match self {
            Generic::Named(_) => Type::Generic(*self),
            Generic::Inferred(id) => Type::Variable(*id),
        }
    }
}

impl std::fmt::Display for Generic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Generic::Named(origin) => write!(f, "_{origin}"),
            Generic::Inferred(id) => write!(f, "_{id}"),
        }
    }
}
