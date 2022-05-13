use cranelift::prelude::Value as CraneliftValue;
use cranelift::{
    frontend::FunctionBuilder,
    prelude::{Block, InstBuilder, JumpTableData, TrapCode},
};

use crate::{hir, util::fmap};

use super::{
    context::{Context, Value},
    CodeGen,
};

impl<'ast> Context<'ast> {
    pub fn codegen_match(&mut self, match_: &'ast hir::Match, builder: &mut FunctionBuilder) -> Value {
        let branches = fmap(&match_.branches, |_| builder.create_block());
        let end_block = self.new_block_with_arg(&match_.result_type, builder);

        self.codegen_subtree(&match_.decision_tree, &branches, builder);

        for (branch, block) in match_.branches.iter().zip(branches) {
            builder.switch_to_block(block);
            builder.seal_block(block);
            let values = branch.eval_all(self, builder);
            builder.ins().jump(end_block, &values);
        }

        builder.switch_to_block(end_block);
        builder.seal_block(end_block);
        let end_values = builder.block_params(end_block);
        self.array_to_value(end_values, &match_.result_type)
    }

    fn codegen_subtree(&mut self, tree: &'ast hir::DecisionTree, branches: &[Block], builder: &mut FunctionBuilder) {
        match tree {
            hir::DecisionTree::Leaf(n) => {
                builder.ins().jump(branches[*n], &[]);
            },
            hir::DecisionTree::Definition(definition, subtree) => {
                definition.codegen(self, builder);
                self.codegen_subtree(subtree, branches, builder);
            },
            hir::DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
                let int_to_switch_on = int_to_switch_on.eval_single(self, builder);
                self.build_switch(int_to_switch_on, cases, else_case, branches, builder);
            },
        }
    }

    fn build_switch(
        &mut self, int_to_switch_on: CraneliftValue, cases: &'ast [(u32, hir::DecisionTree)],
        else_case: &'ast Option<Box<hir::DecisionTree>>, branches: &[Block], builder: &mut FunctionBuilder,
    ) {
        let mut cases = fmap(cases, |(tag, subtree)| {
            let new_block = builder.create_block();
            (*tag as i64, new_block, subtree)
        });

        let else_block = builder.create_block();
        let jump_table_data = create_jump_table_data(&mut cases, else_block);
        let jump_table = builder.create_jump_table(jump_table_data);
        builder.ins().br_table(int_to_switch_on, else_block, jump_table);

        // Fill in new blocks only after creating the jump table.
        // Cranelift enforces we cannot switch out of partially filled blocks.
        self.codegen_cases(else_block, else_case, cases, branches, builder);
    }

    fn codegen_cases(
        &mut self, else_block: Block, else_case: &'ast Option<Box<hir::DecisionTree>>,
        cases: Vec<(i64, Block, &'ast hir::DecisionTree)>, branches: &[Block], builder: &mut FunctionBuilder,
    ) {
        builder.switch_to_block(else_block);
        if let Some(subtree) = else_case {
            self.codegen_subtree(subtree, branches, builder);
        } else {
            builder.ins().trap(TrapCode::UnreachableCodeReached);
        }
        builder.seal_block(else_block);

        for (_, block, subtree) in cases {
            builder.switch_to_block(block);
            self.codegen_subtree(subtree, branches, builder);
            builder.seal_block(block);
        }
    }
}

fn create_jump_table_data(cases: &mut Vec<(i64, Block, &hir::DecisionTree)>, else_block: Block) -> JumpTableData {
    // Sorting unstably doesn't matter here, the type checker's DecisionTree generation
    // ensures there are no duplicate tag values.
    // TODO: Can we merge sorting and filling in missing cases into 1 step?
    cases.sort_unstable_by_key(|(tag, _, _)| *tag);

    let mut data = JumpTableData::new();
    for (tag, block, _) in cases {
        // Arbitrary limit, should be fixed to use if statements instead, assuming table is
        // sparse.
        if *tag > 1000 {
            eprintln!("Cranelift backend: Warning: tried to create a jump table with >1000 entries. This is inefficient and should be changed");
        }
        // Fill in any gaps with the match_all_case
        for _ in data.len()..*tag as usize {
            data.push_entry(else_block);
        }
        data.push_entry(*block);
    }

    data
}
