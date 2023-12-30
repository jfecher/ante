use crate::hir;
use super::ir::Mir;

mod cps;

impl Mir {
    pub fn convert_to_hir(&mut self) -> hir::Ast {
        eprintln!("{}", self);

        self.evaluate();
        self.remove_unreachable_functions();

        self.cps_convert();

        eprintln!("{}", self);

        todo!("Finish convert_to_hir")
    }
}
