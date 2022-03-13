//! llvm/decisiontree.rs - Defines how to codegen a decision tree
//! via `codegen_tree`. This decisiontree is the result of compiling
//! a match expression into a decisiontree during type inference.
use crate::llvm::{ Generator, CodeGen };
use crate::types::pattern::{ DecisionTree, Case, VariantTag };
use crate::types::typed::Typed;
use crate::parser::ast::Match;
use crate::cache::{ ModuleCache, DefinitionInfoId, DefinitionKind };

use crate::nameresolution::builtin::PAIR_ID;

use inkwell::values::{ BasicValueEnum, IntValue, PhiValue };
use inkwell::types::BasicType;
use inkwell::basic_block::BasicBlock;

/// This type alias is used for convenience in codegen_case
/// for adding blocks and values to the switch cases
/// while compiling a given case of a pattern match.
type SwitchCases<'g> = Vec<(IntValue<'g>, BasicBlock<'g>)>;

impl<'g> Generator<'g> {
    /// Perform LLVM codegen for the given DecisionTree.
    /// This roughly translates the tree into a series of switches and phi nodes.
    pub fn codegen_tree<'c>(&mut self, tree: &DecisionTree, match_expr: &Match<'c>,
        cache: &mut ModuleCache<'c>) -> BasicValueEnum<'g>
    {
        let value_to_match = match_expr.expression.codegen(self, cache);

        // Each Switch case in the tree works by switching on a given value in a DefinitionInfoId
        // then storing each part it extracted into other DefinitionInfoIds and recursing. Thus,
        // the initial value needs to be stored in the first id here since before this there was no
        // extract and store step that would have set the value beforehand.
        if let DecisionTree::Switch(id, _) = tree {
            let typ = self.follow_bindings(match_expr.expression.get_type().unwrap(), cache);
            self.definitions.insert((*id, typ), value_to_match);
        }

        let starting_block = self.current_block();
        let ending_block = self.insert_into_new_block("match_end");

        // Create the phi value to merge the value of all the match branches
        let match_type = match_expr.typ.as_ref().unwrap();
        let llvm_type = self.convert_type(match_type, cache);
        let phi = self.builder.build_phi(llvm_type, "match_result");

        // branches may be repeated in the decision tree, so this Vec is used to store the block
        // of each branch if it was already codegen'd.
        let mut branches: Vec<_> = vec![None; match_expr.branches.len()];
        self.builder.position_at_end(starting_block);

        // Then codegen the decisiontree itself that will eventually lead to each branch.
        self.codegen_subtree(tree, &mut branches, phi, ending_block, match_expr, cache);
        self.builder.position_at_end(ending_block);
        phi.as_basic_value()
    }

    /// Recurse on the given DecisionTree, codegening each switch and remembering
    /// all the Leaf nodes that have already been compiled, since these may be
    /// repeated in the same DecisionTree.
    fn codegen_subtree<'c>(&mut self, tree: &DecisionTree, branches: &mut [Option<BasicBlock<'g>>],
        phi: PhiValue<'g>, match_end: BasicBlock<'g>, match_expr: &Match<'c>, cache: &mut ModuleCache<'c>)
    {
        match tree {
            DecisionTree::Leaf(n) => {
                // If this leaf has been codegen'd already, branches[n] was already set to Some in codegen_case
                match branches[*n] {
                    Some(_block) => (),
                    _ => {
                        self.codegen_branch(&match_expr.branches[*n].1, match_end, cache)
                            .map(|(branch, value)| phi.add_incoming(&[(&value, branch)]));
                    }
                }
            },
            DecisionTree::Fail => {
                unreachable!("DecisionTree::Fail encountered during DecisionTree codegen. This should have been caught during completeness checking.");
            },
            DecisionTree::Switch(id, cases) => {
                if !cases.is_empty() {
                    let type_to_switch_on = cache.definition_infos[id.0].typ.as_ref().unwrap();
                    let type_to_switch_on = self.follow_bindings(type_to_switch_on, cache);

                    let value_to_switch_on = self.definitions[&(*id, type_to_switch_on)];

                    let starting_block = self.current_block();

                    // All llvm switches require an else block, even if this pattern doesn't
                    // include one. In that case we insert an unreachable instruction.
                    let else_block = self.codegen_match_else_block(value_to_switch_on,
                        cases, branches, phi, match_end, match_expr, cache);

                    let mut switch_cases = vec![];
                    for case in cases.iter() {
                        self.codegen_case(case, value_to_switch_on, &mut switch_cases,
                            branches, phi, match_end, match_expr, cache);
                    }

                    self.builder.position_at_end(starting_block);

                    if cases.len() > 1 {
                        self.build_switch(value_to_switch_on, else_block, switch_cases);
                    } else if cases.len() == 1 {
                        // If we only have 1 case we don't need to test anything, just forcibly
                        // br to that case. This optimization is necessary for structs since structs
                        // have no tag to check against.
                        self.builder.build_unconditional_branch(switch_cases[0].1);
                    }
                }
            },
        }
    }

    fn build_switch<'c>(&self,
        value_to_switch_on: BasicValueEnum<'g>,
        else_block: BasicBlock<'g>,
        switch_cases: SwitchCases<'g>)
    {
        // TODO: Switch to if-else chains over a single switch block.
        //       Currently this will fail at runtime when attempting to match
        //       a constructor with a string value after trying to convert it into an
        //       integer tag value.
        let tag = self.extract_tag(value_to_switch_on);
        self.builder.build_switch(tag, else_block, &switch_cases);
    }

    fn codegen_case<'c>(&mut self,
        case: &Case,
        matched_value: BasicValueEnum<'g>,
        switch_cases: &mut SwitchCases<'g>,
        branches: &mut [Option<BasicBlock<'g>>],
        phi: PhiValue<'g>,
        match_end: BasicBlock<'g>,
        match_expr: &Match<'c>,
        cache: &mut ModuleCache<'c>)
    {
        // Early out if this is a match-all case. Those should be handled by codegen_match_else_block
        let tag = match &case.tag {
            Some(tag) => tag,
            None => return,
        };

        // Bind each pattern then codegen the rest of the tree.
        // If the rest of the tree is a Leaf that has already been codegen'd we shouldn't compile
        // it twice, instead we take its starting block and jump straight to that in the switch case.
        let block = match &case.branch {
            DecisionTree::Leaf(n) => {
                match &branches[*n] {
                    Some(block) => *block,
                    None => {
                        // Codegening the branch also stores its starting_block in branches,
                        // so we can retrieve it here.
                        let branch_start = self.codegen_case_in_new_block(case,
                            matched_value, branches, phi, match_end, match_expr, cache);

                        branches[*n] = Some(branch_start);
                        branch_start
                    }
                }
            },
            _ => self.codegen_case_in_new_block(case,
                matched_value, branches, phi, match_end, match_expr, cache)
        };

        let constructor_tag = self.get_constructor_tag(tag, cache).unwrap();
        switch_cases.push((constructor_tag.into_int_value(), block));
    }

    /// Creates a new llvm::BasicBlock to insert into, then binds the union downcast
    /// from the current case, then compiles the rest of the subtree.
    fn codegen_case_in_new_block<'c>(&mut self,
        case: &Case,
        matched_value: BasicValueEnum<'g>,
        branches: &mut [Option<BasicBlock<'g>>],
        phi: PhiValue<'g>,
        match_end: BasicBlock<'g>,
        match_expr: &Match<'c>,
        cache: &mut ModuleCache<'c>) -> BasicBlock<'g>
    {
        let branch_start = self.insert_into_new_block("match_branch");
        self.bind_pattern_fields(case, matched_value, cache);
        self.codegen_subtree(&case.branch, branches, phi, match_end, match_expr, cache);
        branch_start
    }

    /// Given a tagged union (either { tag: u8, ... } or just (tag: u8)), extract the
    /// integer tag component to compare which constructor this value was constructed from.
    fn extract_tag(&self, variant: BasicValueEnum<'g>) -> IntValue<'g> {
        if variant.is_struct_value() {
            self.builder.build_extract_value(variant.into_struct_value(), 0, "tag").unwrap().into_int_value()
        } else {
            assert!(variant.is_int_value());
            variant.into_int_value()
        }
    }

    /// Get the tag value that identifies which constructor this is.
    fn get_constructor_tag<'c>(&mut self, tag: &VariantTag, cache: &mut ModuleCache<'c>) -> Option<BasicValueEnum<'g>> {
        match tag {
            VariantTag::True => Some(self.bool_value(true)),
            VariantTag::False => Some(self.bool_value(false)),
            VariantTag::Unit => Some(self.unit_value()),

            // TODO: Remove pair tag, it shouldn't need one
            VariantTag::UserDefined(PAIR_ID) => Some(self.unit_value()),

            VariantTag::UserDefined(id) => {
                match &cache.definition_infos[id.0].definition {
                    Some(DefinitionKind::TypeConstructor { tag: Some(tag), .. }) => {
                        Some(self.tag_value(*tag as u8))
                    },
                    _ => None,
                }
            },
            VariantTag::Literal(literal) => Some(literal.codegen(self, cache)),
        }
    }


    /// Cast the given value to the given tagged-union variant. Returns None if
    /// the given VariantTag is not a tagged-union tag.
    fn cast_to_variant_type<'c>(&mut self, value: BasicValueEnum<'g>, case: &Case,
        cache: &mut ModuleCache<'c>) -> BasicValueEnum<'g>
    {
        match &case.tag {
            Some(VariantTag::UserDefined(id)) => {
                let mut field_types = vec![];

                let constructor = &cache.definition_infos[id.0];
                if constructor.typ.as_ref().unwrap().is_union_constructor(cache) {
                    field_types.push(self.tag_type());
                }

                for field_ids in case.fields.iter() {
                    let typ = cache.definition_infos[field_ids[0].0].typ.as_ref().unwrap();
                    field_types.push(self.convert_type(typ, cache));
                }

                let cast_type = self.context.struct_type(&field_types, false).as_basic_type_enum();
                self.reinterpret_cast_llvm_type(value, cast_type)
            },
            _ => value,
        }
    }

    /// When creating a decision tree, any match all case is always last in the case list.
    fn has_match_all_case(&self, cases: &[Case]) -> bool {
        cases.last().unwrap().tag == None
    }

    /// codegen an else/match-all case of a particular constructor in a DecisionTree.
    /// If there is no MatchAll case (represented by a None value for case.tag) then
    /// a block is created with an llvm unreachable assertion.
    fn codegen_match_else_block<'c>(&mut self,
        value_to_switch_on: BasicValueEnum<'g>,
        cases: &[Case],
        branches: &mut [Option<BasicBlock<'g>>],
        phi: PhiValue<'g>,
        match_end: BasicBlock<'g>,
        match_expr: &Match<'c>,
        cache: &mut ModuleCache<'c>) -> BasicBlock<'g>
    {
        let block = self.insert_into_new_block("match_all");
        let last_case = cases.last().unwrap();

        // If there's a catch-all case we can codegen the code there. Otherwise if this
        // constructor has no catchall the resulting code should be unreachable.
        if self.has_match_all_case(cases) {
            self.bind_pattern_field(value_to_switch_on, &last_case.fields[0], cache);
            self.codegen_subtree(&last_case.branch, branches, phi, match_end, match_expr, cache);
        } else {
            self.builder.build_unreachable();
        }

        block
    }

    /// Each Case in a DecisionTree::Switch contains { tag, fields, branch } where tag
    /// is the matched constructor tag and fields contains a Vec of Vec<DefinitionInfoId>
    /// where the outer Vec contains an inner Vec for each field of the tagged-union variant,
    /// and each inner Vec contains the variables to bind the result of that field to. There
    /// can be multiple ids for a single field as a result of combining multiple cases into one,
    /// see the DecisionTree type and its completeness checking for more information.
    fn bind_pattern_field<'c>(&mut self, value: BasicValueEnum<'g>, field: &[DefinitionInfoId], cache: &mut ModuleCache<'c>) {
        for id in field {
            let typ = self.follow_bindings(cache.definition_infos[id.0].typ.as_ref().unwrap(), cache);
            self.definitions.insert((*id, typ), value);
        }
    }

    /// Performs the union downcast, binding each field of the downcasted variant
    /// the the appropriate DefinitionInfoIds held within the given Case.
    fn bind_pattern_fields<'c>(&mut self, case: &Case, matched_value: BasicValueEnum<'g>, cache: &mut ModuleCache<'c>) {
        let variant = self.cast_to_variant_type(matched_value, &case, cache);

        // There are three cases here:
        // 1. The tag is a tagged union tag. In this case, the value is a tuple of (tag, fields...)
        //    so bind each nth field to the n+1 value in this tuple.
        // 2. The tag is a tuple. In this case, bind each nth tuple field to each nth field.
        // 3. The tag is a primitive like true/false. In this case there is only 1 "field" and we
        //    bind it to the entire value.
        match &case.tag {
            Some(VariantTag::UserDefined(constructor)) => {
                let variant = variant.into_struct_value();

                // TODO: Stop special casing pairs and allow a 0 offset
                // for every product type
                let offset = if *constructor == PAIR_ID { 0 } else { 1 };

                for (field_no, ids) in case.fields.iter().enumerate() {
                    let field = self.builder.build_extract_value(variant, offset + field_no as u32, "pattern_extract").unwrap();
                    self.bind_pattern_field(field, ids, cache);
                }
            },
            _ => {
                assert!(case.fields.len() <= 1);
                if case.fields.len() == 1 {
                    self.bind_pattern_field(variant, &case.fields[0], cache);
                }
            }
        }
    }
}
