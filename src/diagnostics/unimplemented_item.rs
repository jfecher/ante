use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::DbHandle,
};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnimplementedItem {
    Comptime,
    /// `Circle ..c` matched through a reference to a union with `with` fields.
    /// Should probably change the representation of `with` fields to have a consistent offset.
    RestPatternThroughReference,
}

impl std::fmt::Display for UnimplementedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnimplementedItem::Comptime => write!(f, "Comptime expressions"),
            UnimplementedItem::RestPatternThroughReference => {
                write!(f, "`..` patterns through a reference to a type with `with` fields")
            },
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
