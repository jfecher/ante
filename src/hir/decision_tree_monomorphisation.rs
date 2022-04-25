use std::convert::TryInto;

use crate::{types::pattern::{DecisionTree, Case, VariantTag}, parser::ast, cache::{DefinitionKind, DefinitionInfoId}, util::fmap};

use super::monomorphisation::{Context, extract};
use crate::hir;

impl<'c> Context<'c> {
    pub fn monomorphise_match(&mut self, match_: &ast::Match<'c>) -> hir::Ast {
        let match_prelude = self.store_initial_value(match_);
        let decision_tree = self.monomorphise_tree(match_.decision_tree.as_ref().unwrap());
        let branches = fmap(&match_.branches, |branch| self.monomorphise(&branch.1));

        hir::Ast::Sequence(hir::Sequence {
            statements: vec![
                match_prelude,
                hir::Ast::Match(hir::Match {
                    branches,
                    decision_tree,
                }),
            ],
        })
    }

    /// Compile the expression to match on and store it in the DefinitionInfoId expected
    /// by the first Case of the DecisionTree
    fn store_initial_value(&mut self, match_: &ast::Match<'c>) -> hir::Ast {
        let value = self.monomorphise(match_.expression.as_ref());

        if let Some(DecisionTree::Switch(id, _)) = &match_.decision_tree {
            let (def, new_id) = self.fresh_definition(value, false);
            let typ = self.follow_all_bindings(self.cache[*id].typ.as_ref().unwrap());
            self.definitions.insert((*id, typ), new_id.into());
            def
        } else {
            value
        }
    }

    fn monomorphise_tree(&mut self, tree: &DecisionTree) -> hir::DecisionTree {
        match tree {
            DecisionTree::Leaf(index) => hir::DecisionTree::Leaf(*index),
            DecisionTree::Switch(id_to_match_on, cases) => self.monomorphise_switch(*id_to_match_on, cases),
            DecisionTree::Fail => unreachable!("Patterns should be verified to be complete before monomorphisation"),
        }
    }

    fn monomorphise_switch(&mut self, id_to_match_on: DefinitionInfoId, cases: &[Case]) -> hir::DecisionTree {
        let typ = self.cache[id_to_match_on].typ.as_ref().unwrap();
        let value = self.lookup_definition(id_to_match_on, typ).unwrap();

        if cases.len() == 1 {
            // If there's only 1 case we must be destructuring a struct, no need to check a tag
            self.monomorphise_case_no_tag_value(&cases[0], value.definition_id)
        } else {
            let (cases, match_all_case) = self.split_cases(cases);

            let typ = typ.clone();
            let monomorphised_type = self.convert_type(&typ);

            let cases = fmap(cases, |case| self.monomorphise_case(case, value.clone()));
            let else_case = match_all_case.map(|case| {
                Box::new(self.monomorphise_case_no_tag_value(case, value.definition_id))
            });

            let tag = self.extract_tag(value, &monomorphised_type);
            hir::DecisionTree::Switch {
                int_to_switch_on: Box::new(tag),
                cases,
                else_case,
            }
        }
    }

    fn monomorphise_case(&mut self, case: &Case, match_value: hir::DefinitionInfo) -> (u32, hir::DecisionTree) {
        let tree = if case.fields.is_empty() {
            self.monomorphise_tree(&case.branch)
        } else {
            // fresh_id = value = reinterpret match_value as variant_type
            let value = self.cast_to_variant_type(match_value, case);
            let fresh_id = self.next_unique_id();
            let field_bindings = self.bind_patterns(fresh_id, case);

            let mut tree = self.monomorphise_tree(&case.branch);

            for definition in field_bindings.into_iter().rev() {
                tree = hir::DecisionTree::Definition(definition, Box::new(tree));
            }

            let cast_definition = hir::Definition {
                variable: fresh_id,
                expr: Box::new(value),
                mutable: false,
            };

            hir::DecisionTree::Definition(cast_definition, Box::new(tree))
        };

        let expected_tag_value = self.get_tag_value(case);
        (expected_tag_value as u32, tree)
    }

    fn monomorphise_case_no_tag_value(&mut self, case: &Case, match_value: hir::DefinitionId) -> hir::DecisionTree {
        let field_bindings = self.bind_patterns(match_value, case);

        let mut tree = self.monomorphise_tree(&case.branch);

        for definition in field_bindings.into_iter().rev() {
            tree = hir::DecisionTree::Definition(definition, Box::new(tree));
        }

        tree
    }

    fn extract_tag(&mut self, value: hir::DefinitionInfo, typ: &hir::Type) -> hir::Ast {
        match typ {
            hir::types::Type::Primitive(p) => {
                match p {
                    hir::types::PrimitiveType::IntegerType(_) => value.into(),
                    _ => unreachable!(),
                }
            },
            hir::types::Type::Tuple(_, _) => extract(value.into(), 0),
            _ => unreachable!(),
        }
    }

    /// Groups the given cases into an optional match-all case and a list of the other cases.
    fn split_cases<'a>(&self, cases: &'a [Case]) -> (&'a [Case], Option<&'a Case>) {
        let last = cases.last().unwrap();
        if last.tag == None {
            (&cases[0 .. cases.len() - 1], Some(last))
        } else {
            (cases, None)
        }
    }

    fn get_tag_value(&self, case: &Case) -> u8 {
        match case.tag.as_ref().unwrap() {
            VariantTag::True => 1,
            VariantTag::False => 0,
            VariantTag::Unit => 0,
            VariantTag::Literal(literal) => match literal {
                ast::LiteralKind::Integer(x, _) => (*x).try_into().unwrap(), // TODO: larger tags
                ast::LiteralKind::Float(_) => todo!(),
                ast::LiteralKind::String(_) => todo!(),
                ast::LiteralKind::Char(x) => (*x).try_into().unwrap(),
                ast::LiteralKind::Bool(_) => unreachable!(),
                ast::LiteralKind::Unit => unreachable!(),
            },
            VariantTag::UserDefined(id) => {
                match &self.cache[*id].definition {
                    Some(DefinitionKind::TypeConstructor { tag: Some(tag), .. }) => *tag,
                    _ => dbg!(0), //unreachable!(),
                }
            },
        }
    }

    fn bind_patterns(&mut self, variant: hir::DefinitionId, case: &Case) -> Vec<hir::Definition> {
        match &case.tag {
            Some(VariantTag::UserDefined(id)) => {
                let info_type = self.cache.definition_infos[id.0].typ.as_ref().unwrap();
                // Skip the tag value for unions when extracting fields
                let start_index = if info_type.is_union_constructor(&self.cache) { 1 } else { 0 };

                let info_type = info_type.clone();
                let function_type = self.convert_type(&info_type).into_function();

                if function_type.is_some() {
                    fmap(case.fields.iter().enumerate(), |(i, field_aliases)| {
                        let field_index = start_index + i as u32;
                        let variant_variable: hir::Variable = variant.into();
                        let field_variable = self.next_unique_id();

                        for field_alias in field_aliases {
                            let field_type = self.follow_all_bindings(self.cache[*field_alias].typ.as_ref().unwrap());
                            self.definitions.insert((*field_alias, field_type), field_variable.into());
                        }

                        hir::Definition {
                            variable: field_variable,
                            expr: Box::new(extract(variant_variable.into(), field_index)),
                            mutable: false,
                        }
                    })
                } else {
                    vec![]
                }
            },
            None => {
                assert!(case.fields.len() <= 1);
                for field_aliases in &case.fields {
                    for field_alias in field_aliases {
                        let field_type = self.follow_all_bindings(self.cache[*field_alias].typ.as_ref().unwrap());
                        self.definitions.insert((*field_alias, field_type), variant.into());
                    }
                }
                // We've aliased everything this pattern was bound to and did not
                // need to create any new Extract instructions to do so, so there is
                // no need to return any new definitions to insert.
                vec![]
            }
            Some(VariantTag::True
            | VariantTag::False
            | VariantTag::Unit
            | VariantTag::Literal(_)) => vec![], // No fields to bind
        }
    }

    fn cast_to_variant_type(&mut self, value: hir::DefinitionInfo, case: &Case) -> hir::Ast {
        let value = value.into();
        match &case.tag {
            Some(VariantTag::UserDefined(id)) => {
                let constructor = self.follow_all_bindings(self.cache[*id].typ.as_ref().unwrap());
                let mut elems = Vec::with_capacity(case.fields.len() + 1);

                if constructor.is_union_constructor(&self.cache) {
                    elems.push(Self::tag_type());
                }

                for field_aliases in &case.fields {
                    let typ = self.cache[field_aliases[0]].typ.as_ref().unwrap().clone();
                    elems.push(self.convert_type(&typ));
                }

                // TODO: Add padding to cast to smaller type in case some backends need it
                hir::Ast::ReinterpretCast(hir::ReinterpretCast {
                    lhs: Box::new(value),
                    target_type: hir::Type::Tuple(None, elems),
                })
            },
            _ => value,
        }
    }
}
