use std::{convert::TryInto, rc::Rc};

use crate::{
    cache::{DefinitionInfoId, DefinitionKind},
    parser::ast,
    types::pattern::{Case, DecisionTree, VariantTag},
    util::fmap,
};

use super::{monomorphisation::{Context, Definition}, Variable};
use crate::hir;

impl<'c> Context<'c> {
    pub fn monomorphise_match(&mut self, match_: &ast::Match<'c>) -> hir::Ast {
        let match_prelude = self.store_initial_value(match_);
        let decision_tree = self.monomorphise_tree(match_.decision_tree.as_ref().unwrap());
        let branches = fmap(&match_.branches, |branch| self.monomorphise(&branch.1));
        let result_type = self.convert_type(match_.typ.as_ref().unwrap());

        hir::Ast::Sequence(hir::Sequence {
            statements: vec![match_prelude, hir::Ast::Match(hir::Match { branches, decision_tree, result_type })],
        })
    }

    /// Compile the expression to match on and store it in the DefinitionInfoId expected
    /// by the first Case of the DecisionTree
    fn store_initial_value(&mut self, match_: &ast::Match<'c>) -> hir::Ast {
        let value = self.monomorphise(match_.expression.as_ref());

        if let Some(DecisionTree::Switch(id, _)) = &match_.decision_tree {
            let name = Some(self.cache[*id].name.clone());
            let typ = self.follow_all_bindings(self.cache[*id].typ.as_ref().unwrap().as_monotype());
            let monomorphised_type = self.convert_type(&typ);
            let (def, new_id) = self.fresh_definition(value, name.clone(), monomorphised_type.clone());
            let monomorphized_type = Rc::new(monomorphised_type);
            let definition = Definition::Normal(Variable::new(new_id, monomorphized_type));
            self.definitions.insert(*id, typ, definition);
            def
        } else {
            value
        }
    }

    fn monomorphise_tree(&mut self, tree: &DecisionTree) -> hir::DecisionTree {
        match tree {
            DecisionTree::Leaf(index) => hir::DecisionTree::Leaf(*index),
            DecisionTree::Switch(id_to_match_on, cases) => self.monomorphise_switch(*id_to_match_on, cases),
            DecisionTree::Fail => {
                unreachable!("Patterns should be verified to be complete before monomorphisation")
            },
        }
    }

    fn monomorphise_switch(&mut self, id_to_match_on: DefinitionInfoId, cases: &[Case]) -> hir::DecisionTree {
        let typ = self.cache[id_to_match_on].typ.as_ref().unwrap().as_monotype();

        let value = match self.lookup_definition(id_to_match_on, typ) {
            Some(Definition::Normal(variable)) => variable,
            _ => unreachable!(),
        };

        if cases.len() == 1 {
            // If there's only 1 case we must be destructuring a struct, no need to check a tag
            self.monomorphise_case_no_tag_value(&cases[0], value.definition_id)
        } else {
            let (cases, match_all_case) = self.split_cases(cases);

            let typ = typ.clone();
            let monomorphised_type = self.convert_type(&typ);

            let cases = fmap(cases, |case| self.monomorphise_case(case, value.clone()));
            let else_case =
                match_all_case.map(|case| Box::new(self.monomorphise_case_no_tag_value(case, value.definition_id)));

            let tag = Self::extract_tag(value, &monomorphised_type);
            hir::DecisionTree::Switch { int_to_switch_on: Box::new(tag), cases, else_case }
        }
    }

    fn monomorphise_case(&mut self, case: &Case, match_value: hir::DefinitionInfo) -> (u32, hir::DecisionTree) {
        let tree = if case.fields.is_empty() {
            self.monomorphise_tree(&case.branch)
        } else {
            // variable = value = reinterpret match_value as variant_type
            let (value, typ) = self.cast_to_variant_type(match_value, case);
            let variable = self.next_unique_id();
            let field_bindings = self.bind_patterns(variable, case);

            let mut tree = self.monomorphise_tree(&case.branch);

            for definition in field_bindings.into_iter().rev() {
                tree = hir::DecisionTree::Definition(definition, Box::new(tree));
            }

            let expr = Box::new(value);
            let cast_definition = hir::Definition { variable, expr, typ, name: None };

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

    fn extract_tag(value: hir::DefinitionInfo, typ: &hir::Type) -> hir::Ast {
        use hir::types::*;
        match typ {
            Type::Primitive(PrimitiveType::Integer(_)) => value.into(),
            Type::Tuple(fields) => Self::extract(value.into(), 0, fields[0].clone()),
            _ => unreachable!(),
        }
    }

    /// Groups the given cases into an optional match-all case and a list of the other cases.
    fn split_cases<'a>(&self, cases: &'a [Case]) -> (&'a [Case], Option<&'a Case>) {
        let last = cases.last().unwrap();
        if last.tag.is_none() {
            (&cases[0..cases.len() - 1], Some(last))
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
                ast::LiteralKind::Float(_, _) => todo!(),
                ast::LiteralKind::String(_) => todo!(),
                ast::LiteralKind::Char(x) => {
                    let codepoint: u32 = (*x).into();
                    codepoint.try_into().unwrap()
                },
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
                let start_index = u32::from(info_type.is_union_constructor(&self.cache));

                let info_type = info_type.clone();

                // Note: should not use function_type for any bindings, it is from a generalized
                // info_type that makes it only useful for checking if it is a function or not.
                let function_type = self.convert_type(info_type.remove_forall()).into_function();

                if let Some(function_type) = function_type {
                    let variant_type = Rc::new(hir::Type::Function(function_type));
                    let variant_variable = hir::Variable::new(variant, variant_type);

                    fmap(case.fields.iter().enumerate(), |(i, field_aliases)| {
                        let field_index = start_index + i as u32;
                        let field_variable_id = self.next_unique_id();
                        let mut monomorphized_field_type = None;

                        for field_alias in field_aliases {
                            let alias_type = self.cache[*field_alias].typ.as_ref().unwrap().as_monotype();
                            let field_type = self.follow_all_bindings(alias_type);

                            if monomorphized_field_type.is_none() {
                                monomorphized_field_type = Some(self.convert_type(&field_type));
                            }

                            let monomorphized_field_type = Rc::new(self.convert_type(&field_type));
                            let field_variable = hir::Variable::new(field_variable_id, monomorphized_field_type);

                            let field_definition = Definition::Normal(field_variable);
                            self.definitions.insert(*field_alias, field_type, field_definition);
                        }

                        let typ = monomorphized_field_type.unwrap();
                        hir::Definition {
                            variable: field_variable_id,
                            expr: Box::new(Self::extract(variant_variable.clone().into(), field_index, typ.clone())),
                            typ,
                            name: None,
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
                        let alias_type = self.cache[*field_alias].typ.as_ref().unwrap().as_monotype();
                        let field_type = self.follow_all_bindings(alias_type);

                        let monomorphized_field_type = Rc::new(self.convert_type(&field_type));
                        let variant_variable = hir::Variable::new(variant, monomorphized_field_type);
                        let definition = Definition::Normal(variant_variable);
                        self.definitions.insert(*field_alias, field_type, definition);
                    }
                }
                // We've aliased everything this pattern was bound to and did not
                // need to create any new Extract instructions to do so, so there is
                // no need to return any new definitions to insert.
                vec![]
            },
            Some(VariantTag::True | VariantTag::False | VariantTag::Unit | VariantTag::Literal(_)) => vec![], // No fields to bind
        }
    }

    fn cast_to_variant_type(&mut self, value: hir::DefinitionInfo, case: &Case) -> (hir::Ast, hir::Type) {
        let value = value.into();
        match &case.tag {
            Some(VariantTag::UserDefined(id)) => {
                let mut elems = Vec::with_capacity(case.fields.len() + 1);

                let constructor = self.follow_all_bindings(self.cache[*id].typ.as_ref().unwrap().remove_forall());
                if constructor.is_union_constructor(&self.cache) {
                    elems.push(Self::tag_type());
                }

                for field_aliases in &case.fields {
                    let typ = self.cache[field_aliases[0]].typ.as_ref().unwrap().clone().into_monotype();
                    elems.push(self.convert_type(&typ));
                }

                let target_type = hir::Type::Tuple(elems);
                let cast = hir::ReinterpretCast { lhs: Box::new(value), target_type: target_type.clone() };

                // TODO: Add padding to cast to smaller type in case some backends need it
                (hir::Ast::ReinterpretCast(cast), target_type)
            },
            other => unreachable!("Expected cast to Some(user defined type), found cast to: {:?}", other),
        }
    }
}
