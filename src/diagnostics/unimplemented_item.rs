use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::DbHandle,
};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnimplementedItem {
    TypeAlias,
    Effects,
    Comptime,
    PolymorphicIntegers,
    PolymorphicFloats,
    Strings,
}

impl std::fmt::Display for UnimplementedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The item name must be plural
            UnimplementedItem::TypeAlias => write!(f, "Type aliases"),
            UnimplementedItem::Effects => write!(f, "Effects and handle expressions"),
            UnimplementedItem::Comptime => write!(f, "Comptime expressions"),
            UnimplementedItem::PolymorphicIntegers => write!(f, "Polymorphic Integers"),
            UnimplementedItem::PolymorphicFloats => write!(f, "Polymorphic Floats"),
            UnimplementedItem::Strings => write!(f, "Strings"),
        }
    }
}

impl UnimplementedItem {
    /// Issue an unimplemented item error
    ///
    /// Internally this translates to `db.accumulate(Diagnostic::Unimplemented { ... })`
    pub(crate) fn issue(self, db: &DbHandle, location: Location) {
        db.accumulate(Diagnostic::Unimplemented { item: self, location });
    }
}
