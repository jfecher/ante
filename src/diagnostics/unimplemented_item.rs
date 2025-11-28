use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnimplementedItem {
    TypeAlias,
    Effects,
    Comptime,
}

impl std::fmt::Display for UnimplementedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The item name must be plural
            UnimplementedItem::TypeAlias => write!(f, "Type aliases"),
            UnimplementedItem::Effects => write!(f, "Effects and handle expressions"),
            UnimplementedItem::Comptime => write!(f, "Comptime expressions"),
        }
    }
}
