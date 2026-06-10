use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{Diagnostic, Location},
    incremental::DbHandle,
};

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnimplementedItem {
    Comptime,
}

impl std::fmt::Display for UnimplementedItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnimplementedItem::Comptime => write!(f, "Comptime expressions"),
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
