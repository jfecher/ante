use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnimplementedItem {
    TypeAlias,
}

impl std::fmt::Display for UnimplementedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnimplementedItem::TypeAlias => write!(f, "Type aliases"),
        }
    }
}
