use serde::{Deserialize, Serialize};

use crate::{name_resolution::Origin, type_inference::types::TypeVariableId};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Generic {
    Named(Origin),
    Inferred(TypeVariableId),
}

impl std::fmt::Display for Generic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Generic::Named(origin) => write!(f, "_{origin}"),
            Generic::Inferred(id) => write!(f, "_{id}"),
        }
    }
}
