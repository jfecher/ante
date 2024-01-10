//! llvm/decisiontree.rs - Defines how to codegen a decision tree
//! via `codegen_tree`. This decisiontree is the result of compiling
//! a match expression into a decisiontree during type inference.
use crate::mir;
use crate::llvm::{CodeGen, Generator};
use crate::util::fmap;

use inkwell::basic_block::BasicBlock;
use inkwell::values::BasicValueEnum;

impl<'g> Generator<'g> {
    pub fn codegen_tree(&mut self, match_expr: &mir::Match) -> BasicValueEnum<'g> {
        let current_function = self.current_function();
        let end_block = self.context.append_basic_block(current_function, "match_end");

        let branch_blocks = fmap(&match_expr.branches, |_| self.context.append_basic_block(current_function, ""));

        self.codegen_subtree(&match_expr.decision_tree, &branch_blocks);

        let mut typ = None;
        let incoming = branch_blocks
            .into_iter()
            .zip(match_expr.branches.iter())
            .filter_map(|(block, branch)| {
                self.builder.position_at_end(block);
                let (value_type, result) = self.codegen_branch(branch, end_block);
                typ = Some(value_type);
                result
            })
            .collect::<Vec<_>>();

        self.builder.position_at_end(end_block);
        let phi = self.builder.build_phi(typ.unwrap(), "match_result")
            .expect("Could not build phi");

        // Inkwell forces us to pass a &[(&dyn BasicValue, _)] which prevents us from
        // passing an entire Vec since we'd also need to store the basic values in another
        // vec to be able to have a Vec hold references to them to begin with.
        for (value, block) in incoming {
            phi.add_incoming(&[(&value, block)]);
        }

        phi.as_basic_value()
    }

    /// Recurse on the given DecisionTree, codegening each switch and remembering
    /// all the Leaf nodes that have already been compiled, since these may be
    /// repeated in the same DecisionTree.
    fn codegen_subtree(&mut self, tree: &mir::DecisionTree, branches: &[BasicBlock<'g>]) {
        match tree {
            mir::DecisionTree::Leaf(n) => {
                self.builder.build_unconditional_branch(branches[*n])
                    .expect("Could not create br during codegen_subtree");
            },
            mir::DecisionTree::Let(let_) => {
                let_.codegen(self);
                self.codegen_subtree(&let_.body, branches);
            },
            mir::DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
                let int_to_switch_on = int_to_switch_on.codegen(self);
                self.build_switch(int_to_switch_on, cases, else_case, branches);
            },
        }
    }

    fn build_switch(
        &mut self, int_to_switch_on: BasicValueEnum<'g>, cases: &[(u32, mir::DecisionTree)],
        else_case: &Option<Box<mir::DecisionTree>>, branches: &[BasicBlock<'g>],
    ) {
        let starting_block = self.current_block();
        let cases = fmap(cases, |(tag, subtree)| {
            let tag = self.context.i8_type().const_int(*tag as u64, true);
            let new_block = self.insert_into_new_block("");
            self.codegen_subtree(subtree, branches);
            (tag, new_block)
        });

        let else_block = self.insert_into_new_block("match_else");

        if let Some(subtree) = else_case {
            self.codegen_subtree(subtree, branches);
        } else {
            self.builder.build_unreachable()
                .expect("Could not create unreachable during build_switch");
        }

        self.builder.position_at_end(starting_block);
        let tag = int_to_switch_on.into_int_value();
        self.builder.build_switch(tag, else_block, &cases)
            .expect("Could not build switch");
    }
}
