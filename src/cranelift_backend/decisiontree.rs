use crate::{types::pattern::{DecisionTree, Case, VariantTag}, parser::ast::{self, Ast}, util::fmap, cache::DefinitionKind};
use cranelift::{frontend::FunctionBuilder, prelude::{InstBuilder, Block, JumpTableData, MemFlags, TrapCode}};
use cranelift::prelude::Value as CraneliftValue;

use super::{context::{Context, Value, BOXED_TYPE}, Codegen};

// Maps from Leaf(usize) -> Block where the block is the branch to take after matching the pattern.
type Branches = Vec<Block>;

pub fn codegen<'ast, 'c>(match_: &'ast ast::Match<'c>, context: &mut Context<'ast, 'c>, builder: &mut FunctionBuilder) -> Value {
    let branches = fmap(&match_.branches, |_| builder.create_block());
    store_initial_value(match_, context, builder);
    codegen_tree(match_.decision_tree.as_ref().unwrap(), context, builder, &branches);
    let ret = codegen_branches(&match_.branches, &branches, context, builder);

    for branch in branches {
        builder.seal_block(branch);
    }

    Value::Normal(ret)
}

/// Compile the expression to match on and store it in the DefinitionInfoId expected
/// by the first Case of the DecisionTree
fn store_initial_value<'ast, 'c>(match_: &'ast ast::Match<'c>, context: &mut Context<'ast, 'c>, builder: &mut FunctionBuilder) {
    let value = match_.expression.codegen(context, builder);

    if let Some(DecisionTree::Switch(id, _)) = &match_.decision_tree {
        context.definitions.insert(*id, value);
    }
}

fn codegen_branches<'ast, 'c>(branches: &'ast [(Ast<'c>, Ast<'c>)], blocks: &[Block], context: &mut Context<'ast, 'c>, builder: &mut FunctionBuilder) -> CraneliftValue {
    let end = builder.create_block();
    builder.append_block_param(end, BOXED_TYPE);

    for ((_pattern, branch), block) in branches.iter().zip(blocks) {
        builder.switch_to_block(*block);
        let value = context.codegen_eval(branch, builder);
        builder.ins().jump(end, &[value]);
    }

    builder.switch_to_block(end);
    builder.seal_block(end);
    
    let end_params = builder.block_params(end);
    assert_eq!(end_params.len(), 1);
    end_params[0]
}

fn codegen_tree<'ast, 'c>(tree: &'ast DecisionTree, context: &mut Context<'ast, 'c>, builder: &mut FunctionBuilder, branches: &Branches) {
    match tree {
        DecisionTree::Leaf(index) => {
            let target = branches[*index];
            builder.ins().jump(target, &[]);
        },
        DecisionTree::Switch(id_to_match_on, cases) => {
            let value = context.definitions[id_to_match_on].clone().eval(context, builder);
            codegen_cases(value, cases, context, builder, branches);
        },
        DecisionTree::Fail => unreachable!("Patterns should be verified to be complete before codegen"),
    }
}

fn codegen_cases<'ast, 'c>(value_to_match_on: CraneliftValue, cases: &'ast [Case], context: &mut Context<'ast, 'c>, builder: &mut FunctionBuilder, branches: &Branches) {
    // The match all case, if present, is guarenteed to be last
    let mut tag_cases = fmap(cases, |case| case);
    let match_all_case = (cases[cases.len() - 1].tag.is_none()).then(|| {
        tag_cases.pop().unwrap()
    });

    let current_block = builder.current_block().unwrap();

    let cases = if should_use_jump_table(&tag_cases, context) {
        codegen_jump_table(value_to_match_on, tag_cases, match_all_case, context, builder)
    } else if tag_cases.len() == 1 {
        // Nothing to do
        vec![(tag_cases[0], current_block)]
    } else {
        todo!()
    };

    for (case, block) in cases {
        if block != current_block {
            builder.switch_to_block(block);
        }
        bind_patterns(value_to_match_on, case, context, builder);
        codegen_tree(&case.branch, context, builder, branches);
    }
}

/// True if we should compile this DecisionTree::Switch into a jump table on the current tag value.
fn should_use_jump_table(cases: &[&Case], context: &mut Context) -> bool {
    cases.len() != 1 && match cases.get(0) {
        None => false,
        Some(Case { tag: None, .. }) => unreachable!("The match all case should already be filtered out"),
        Some(Case { tag: Some(VariantTag::True | VariantTag::False | VariantTag::Unit), .. }) => true,
        Some(Case { tag: Some(VariantTag::Literal(_)), .. }) => false,
        Some(Case { tag: Some(VariantTag::UserDefined(id)), .. }) => {
            match &context.cache.definition_infos[id.0].definition {
                Some(DefinitionKind::TypeConstructor { tag, .. }) => tag.is_some(),
                _ => unreachable!(),
            }
        }
    }
}

/// Codegen a DecisionTree::Switch into a jump table on the tag of value_to_match_on.
/// Expects tag_cases to be each non-matchall case.
fn codegen_jump_table<'a, 'ast, 'c>(
    value_to_match_on: CraneliftValue,
    mut tag_cases: Vec<&'a Case>,
    match_all_case: Option<&'a Case>,
    context: &mut Context<'ast, 'c>,
    builder: &mut FunctionBuilder,
) -> Vec<(&'a Case, Block)>
{
    // Sorting unstably doesn't matter here, the type checker's DecisionTree generation
    // ensures there are no duplicate tag values.
    // TODO: Should we merge sorting and filling in missing cases into 1 step?
    tag_cases.sort_unstable_by_key(|case| get_tag_value(case, context));

    let mut cases = fmap(tag_cases, |case| (case, builder.create_block()));
    let match_all = match_all_case.map(|case| (case, builder.create_block()));

    let tag_value = builder.ins().load(BOXED_TYPE, MemFlags::new(), value_to_match_on, 0);

    let data = make_jump_table_data(&cases, match_all, context);
    let jump_table = builder.create_jump_table(data);
    let trap_block = builder.create_block();

    builder.ins().br_table(tag_value, trap_block, jump_table);

    fill_trap_block(trap_block, builder);

    if let Some(case) = match_all {
        cases.push(case);
    }

    for (_, block) in &cases {
        builder.seal_block(*block);
    }

    cases
}

fn get_tag_value(case: &Case, context: &Context) -> u8 {
    match case.tag.as_ref().unwrap() {
        VariantTag::True => 1,
        VariantTag::False => 0,
        VariantTag::Unit => 0,
        VariantTag::Literal(_) => unreachable!(),
        VariantTag::UserDefined(id) => {
            match &context.cache.definition_infos[id.0].definition {
                Some(DefinitionKind::TypeConstructor { tag: Some(tag), .. }) => *tag,
                _ => unreachable!(),
            }
        },
    }
}

/// Fill in any gaps in tag_cases with the match_all_case
fn make_jump_table_data<'a, 'ast, 'c>(cases: &[(&'a Case, Block)], match_all: Option<(&'a Case, Block)>, context: &Context<'ast, 'c>) -> JumpTableData {
    let variants_in_type = num_variants(cases[0].0, context);
    let mut data = JumpTableData::new();

    for (case, block) in cases {
        let tag = get_tag_value(case, context);

        // Fill in any gaps with the match_all_case
        for _ in data.len() .. tag as usize {
            data.push_entry(match_all.unwrap().1);
        }

        data.push_entry(*block);
    }

    for _ in data.len() .. variants_in_type {
        data.push_entry(match_all.unwrap().1);
    }

    data
}

fn num_variants(case: &Case, context: &Context) -> usize {
    match case.tag.as_ref().unwrap() {
        VariantTag::True => 2,
        VariantTag::False => 2,
        VariantTag::Unit => 1,
        VariantTag::Literal(_) => unreachable!(),
        VariantTag::UserDefined(id) => {
            let info = &context.cache.definition_infos[id.0];
            let variants = info.typ.as_ref().unwrap().union_constructor_variants(context.cache);
            variants.unwrap().len()
        },
    }
}

/// Finish the given block with a trap instruction.
fn fill_trap_block(block: Block, builder: &mut FunctionBuilder) {
    builder.switch_to_block(block);
    builder.ins().trap(TrapCode::UnreachableCodeReached);
    builder.seal_block(block);
}

fn bind_patterns<'ast, 'c>(value_to_match_on: CraneliftValue, case: &Case, context: &mut Context<'ast, 'c>, builder: &mut FunctionBuilder) {
    match &case.tag {
        Some(VariantTag::UserDefined(id)) => {
            let info_type = &context.cache.definition_infos[id.0].typ.as_ref().unwrap();
            // Skip the tag value for unions when extracting fields
            let start_index = if info_type.is_union_constructor(context.cache) { 1 } else { 0 };

            for (i, field_aliases) in case.fields.iter().enumerate() {
                let field_index = (start_index + i as i32) * Context::pointer_size();
                let field = builder.ins().load(BOXED_TYPE, MemFlags::new(), value_to_match_on, field_index);

                for field_alias in field_aliases {
                    context.definitions.insert(*field_alias, Value::Normal(field));
                }
            }
        },
        None
        | Some(VariantTag::True
        | VariantTag::False
        | VariantTag::Unit
        | VariantTag::Literal(_)) => (), // No fields to bind
    }
}
