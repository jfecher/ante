use serde::{Deserialize, Serialize};

use crate::{name_resolution::Origin, type_inference::types::TypeVariableId};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Generic {
    Named(Origin),
    Inferred(TypeVariableId),
}
