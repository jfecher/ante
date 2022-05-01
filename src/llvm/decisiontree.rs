//! llvm/decisiontree.rs - Defines how to codegen a decision tree
//! via `codegen_tree`. This decisiontree is the result of compiling
//! a match expression into a decisiontree during type inference.
use crate::llvm::{ Generator, CodeGen };
use crate::hir;
use crate::util::fmap;

use inkwell::values::BasicValueEnum;
use inkwell::basic_block::BasicBlock;

impl<'g> Generator<'g> {
    pub fn codegen_tree(&mut self, match_expr: &hir::Match) -> BasicValueEnum<'g> {
        let current_function = self.current_function();
        let ending_block = self.context.append_basic_block(current_function, "match_end");

        let branch_blocks = fmap(&match_expr.branches, |_| {
            self.context.append_basic_block(current_function, "")
        });

        self.codegen_subtree(&match_expr.decision_tree, &branch_blocks);

        let mut typ = None;
        let incoming = fmap(branch_blocks.into_iter().zip(match_expr.branches.iter()), |(block, branch)| {
            self.builder.position_at_end(block);
            let value = branch.codegen(self);
            self.builder.build_unconditional_branch(ending_block);

            typ = Some(value.get_type());
            (value, self.current_block())
        });

        self.builder.position_at_end(ending_block);
        let phi = self.builder.build_phi(typ.unwrap(), "match_result");

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
    fn codegen_subtree<'c>(&mut self, tree: &hir::DecisionTree, branches: &[BasicBlock<'g>]) {
        match tree {
            hir::DecisionTree::Leaf(n) => {
                self.builder.build_unconditional_branch(branches[*n]);
            },
            hir::DecisionTree::Definition(definition, subtree) => {
                definition.codegen(self);
                self.codegen_subtree(subtree, branches);
            },
            hir::DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
                let int_to_switch_on = int_to_switch_on.codegen(self);
                self.build_switch(int_to_switch_on, cases, else_case, branches);
            },
        }
    }

    fn build_switch(&mut self,
        int_to_switch_on: BasicValueEnum<'g>,
        cases: &[(u32, hir::DecisionTree)],
        else_case: &Option<Box<hir::DecisionTree>>,
        branches: &[BasicBlock<'g>])
    {
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
            self.builder.build_unreachable();
        }

        self.builder.position_at_end(starting_block);
        let tag = int_to_switch_on.into_int_value();
        self.builder.build_switch(tag, else_block, &cases);
    }
}
