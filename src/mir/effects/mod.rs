pub(super) mod abort_handler_optimization;
pub(super) mod effect_lowering;
pub(super) mod local_handler_optimization;
pub(super) mod resume_once_optimization;
pub(super) mod specialize_handle_lambdas;
pub(super) mod tail_resume_optimization;

use crate::mir::Mir;

impl Mir {
    /// Compile each Handle with the cheapest strategy which applies to it, lowering any left over
    /// into coroutines. See [Mir::lower_effects] for `generic_coroutine_entry`.
    pub(crate) fn optimize_and_lower_effects(self, generic_coroutine_entry: bool) -> Self {
        self.optimize_tail_resume()
            .optimize_resume_once()
            .optimize_abort_handlers()
            .lower_effects(generic_coroutine_entry)
    }
}
