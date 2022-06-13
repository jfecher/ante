use crate::cache::MutualRecursionId;

use crate::{
    cache::{DefinitionInfoId, DefinitionKind, ModuleCache, VariableId},
    error::location::Locatable,
    parser::ast,
    types::{
        traitchecker,
        typechecker::{bind_irrefutable_pattern, find_all_typevars},
        typed::Typed,
    },
    util::trustme,
};

use super::{
    traits::{Callsite, RequiredTrait, TraitConstraints},
    Type,
};

pub(super) fn try_generalize_definition<'c>(
    definition: &mut ast::Definition<'c>, t: Type, traits: TraitConstraints, cache: &mut ModuleCache<'c>,
) -> TraitConstraints {
    if !should_generalize(&definition.expr) {
        return traits;
    }

    let pattern = definition.pattern.as_mut();
    match is_mutually_recursive(pattern, cache) {
        MutualRecursionResult::No => {
            let typevars_in_fn = find_all_typevars(pattern.get_type().unwrap(), false, cache);
            let exposed_traits = traitchecker::resolve_traits(traits, &typevars_in_fn, cache);
            bind_irrefutable_pattern(pattern, &t, &exposed_traits, true, cache);
            vec![]
        },
        MutualRecursionResult::YesGeneralizeLater => traits, // Do nothing
        MutualRecursionResult::YesGeneralizeNow(id) => {
            // Generalize all the mutually recursive definitions at once
            for id in cache.mutual_recursion_sets[id.0].definitions.clone() {
                let info = &mut cache.definition_infos[id.0];
                info.undergoing_type_inference = false;

                let t = info.typ.as_ref().unwrap().as_monotype().clone();

                let definition = match &mut info.definition {
                    Some(DefinitionKind::Definition(definition)) => trustme::extend_lifetime(*definition),
                    _ => unreachable!(),
                };

                let pattern = &mut definition.pattern.as_mut();

                let typevars_in_fn = find_all_typevars(pattern.get_type().unwrap(), false, cache);
                let exposed_traits = traitchecker::resolve_traits(traits.clone(), &typevars_in_fn, cache);

                let callsites = &cache[id].mutually_recursive_variables;

                let exposed_traits = update_callsites(exposed_traits, callsites);
                bind_irrefutable_pattern(pattern, &t, &exposed_traits, true, cache);
            }

            let root = cache.mutual_recursion_sets[id.0].root_definition;
            cache[root].undergoing_type_inference = false;
            let typevars_in_fn = find_all_typevars(pattern.get_type().unwrap(), false, cache);
            let mut exposed_traits = traitchecker::resolve_traits(traits.clone(), &typevars_in_fn, cache);

            let callsites = &cache[root].mutually_recursive_variables;

            exposed_traits.append(&mut update_callsites(exposed_traits.clone(), callsites));
            bind_irrefutable_pattern(pattern, &t, &exposed_traits, true, cache);

            vec![]
        },
    }
}

fn update_callsites(exposed_traits: Vec<RequiredTrait>, callsites: &Vec<VariableId>) -> Vec<RequiredTrait> {
    let mut ret = Vec::with_capacity(exposed_traits.len() * callsites.len());

    for callsite in callsites {
        ret.extend(exposed_traits.iter().cloned().map(|mut exposed| {
            if exposed.callsite.id() != *callsite {
                exposed.callsite = match exposed.callsite {
                    Callsite::Direct(_) => Callsite::Indirect(*callsite, vec![exposed.signature.id]),
                    Callsite::Indirect(_, mut ids) => {
                        ids.push(exposed.signature.id);
                        Callsite::Indirect(*callsite, ids)
                    },
                };
            }
            exposed
        }));
    }

    ret
}

/// True if the expression can be generalized. Generalizing expressions
/// will cause them to be re-evaluated whenever they're used with new types,
/// so generalization should be limited to when this would be expected by
/// users (functions) or when it would not be noticeable (variables).
fn should_generalize(ast: &ast::Ast) -> bool {
    match ast {
        ast::Ast::Variable(_) => true,
        ast::Ast::Lambda(lambda) => lambda.closure_environment.is_empty(),
        _ => false,
    }
}

enum MutualRecursionResult {
    No,
    YesGeneralizeLater,
    YesGeneralizeNow(MutualRecursionId),
}

impl MutualRecursionResult {
    fn combine(self, other: Self) -> Self {
        use MutualRecursionResult::*;
        match (self, other) {
            (No, other) | (other, No) => other,

            (YesGeneralizeNow(id1), YesGeneralizeNow(id2)) => {
                assert_eq!(id1, id2);
                YesGeneralizeNow(id1)
            },
            (YesGeneralizeNow(id), _) | (_, YesGeneralizeNow(id)) => YesGeneralizeNow(id),

            (YesGeneralizeLater, YesGeneralizeLater) => YesGeneralizeLater,
        }
    }
}

pub(super) fn definition_is_mutually_recursive(definition: DefinitionInfoId, cache: &ModuleCache) -> bool {
    let info = &cache[definition];
    info.mutually_recursive_set.is_some()
}

fn is_mutually_recursive(pattern: &ast::Ast, cache: &ModuleCache) -> MutualRecursionResult {
    use ast::Ast::*;
    match pattern {
        Literal(_) => MutualRecursionResult::No,
        Variable(variable) => {
            let definition_id = variable.definition.unwrap();
            let info = &cache.definition_infos[definition_id.0];
            match info.mutually_recursive_set {
                None => MutualRecursionResult::No,
                Some(id) if cache.mutual_recursion_sets[id.0].root_definition == definition_id => {
                    MutualRecursionResult::YesGeneralizeNow(id)
                },
                Some(_) => MutualRecursionResult::YesGeneralizeLater,
            }
        },
        TypeAnnotation(annotation) => is_mutually_recursive(&annotation.lhs, cache),
        FunctionCall(call) => {
            call.args.iter().fold(MutualRecursionResult::No, |a, b| a.combine(is_mutually_recursive(b, cache)))
        },
        _ => {
            error!(pattern.locate(), "Invalid syntax in irrefutable pattern");
            MutualRecursionResult::No
        },
    }
}
