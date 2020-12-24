use crate::types::traits::{ RequiredImpl, RequiredTrait, TraitConstraint, TraitConstraints };
use crate::cache::{ ModuleCache, VariableId, ImplInfoId, DefinitionInfoId, TraitBindingId };
use crate::types::typechecker::{ self, TypeBindings, UnificationResult };
use crate::types::{ INITIAL_LEVEL, DEFAULT_INTEGER_TYPE, Type, PrimitiveType, TypeBinding::* };
use crate::error::location::Location;
use crate::lexer::token::IntegerKind;
use crate::util::trustme;

use std::sync::atomic::Ordering;
use std::collections::HashMap;

/// Go through the given list of traits and determine if they should
/// be propogated upward or if an impl should be searched for now.
/// Returns the list of traits propogated upward.
/// Binds the impls that were searched for and found to the required_impls
/// in the callsite VariableInfo, and errors for any impls that couldn't be found.
pub fn resolve_traits<'a>(constraints: TraitConstraints, location: Location<'a>, cache: &mut ModuleCache<'a>) -> Vec<RequiredTrait> {
    let (propogated_traits,
         int_constraints,
         member_access_constraints,
         other_constraints) = sort_traits(constraints, cache);

    for constraint in int_constraints {
        typechecker::perform_bindings_or_print_error(
            find_int_constraint_impl(&constraint, location, cache), cache
        );
    }

    for constraint in member_access_constraints {
        typechecker::perform_bindings_or_print_error(
            find_member_access_impl(&constraint, location, cache), cache
        );
    }

    for constraint in other_constraints {
        // Normal constraints require special care since searching for an impl for them may require
        // recursively searching for more impls (due to `impl A given B` constraints) before finding a matching one.
        solve_normal_constraint(&constraint, location, cache);
    }

    // NOTE: 'duplicate' trait constraints like `given Print a, Print a` are NOT separated out here
    // because they each point to different usages of the trait. They are only filtered out when
    // displaying types to the user.
    propogated_traits
}

/// These just make the signature of sort_traits read better.
///
/// PropagatedTraits is a Vec of RequiredTraits rather than TraitConstraints
/// since RequiredTraits are what are actually stored in DefinitionInfos to
/// propogate trait constraints upward. The other aliases here aren't propogated
/// so they don't need to be converted.
type PropagatedTraits = Vec<RequiredTrait>;
type IntTraits = Vec<TraitConstraint>;
type MemberAccessTraits = Vec<TraitConstraint>;

/// Sort the given list of TraitConstraints into 4 categories:
/// - Constraints that shouldn't be solved here because they contain type variables that escape
///   into an outer scope. Propagate these up as RequiredTraits.
/// - `Int a` constraints. These should be solved first since they can default their argument
///   to an i32 if it is not yet decided, which can influence subsequent trait selections.
/// - Member-access constraints e.g. `a.b`. These can be solved anytime after Int constraints
///   but are filtered out because they're required to be solved via find_member_access_impl.
/// - All other constraints. This includes all other normal trait constraints like `Print a`
///   or `Cast a b` which should have an impl searched for now. Traits like this that shouldn't
///   have an impl searched for belong to the first category of propogated traits.
fn sort_traits<'c>(constraints: TraitConstraints, cache: &ModuleCache<'c>) -> (PropagatedTraits, IntTraits, MemberAccessTraits, TraitConstraints) {
    let mut propogated_traits = vec![];
    let mut int_constraints = vec![];
    let mut member_access_constraints = vec![];
    let mut other_constraints = Vec::with_capacity(constraints.len());

    for constraint in constraints {
        if should_propagate(&constraint, cache) {
            propogated_traits.push(constraint.as_required_trait());
        } else if constraint.is_int_constraint(cache)  {
            int_constraints.push(constraint);
        } else if constraint.is_member_access(cache) {
            member_access_constraints.push(constraint);
        } else {
            other_constraints.push(constraint);
        }
    }

    (propogated_traits, int_constraints, member_access_constraints, other_constraints)
}

/// A trait should be propogated to the public signature of a Definition if any of its contained
/// type variables should be generalized. If the trait shouldn't be propogated then an impl
/// should be resolved instead.
/// For example, the trait constraint `Print i32` should never be propogated because it doesn't
/// contain any typevariables. A constraint like `Print a` may be propogated if `a` is a
/// typevariable used in the signature of the current function.
fn should_propagate<'a>(constraint: &TraitConstraint, cache: &ModuleCache<'a>) -> bool {
    // Don't check the fundeps since only the typeargs proper are used to find impls
    let arg_count = cache.trait_infos[constraint.trait_id.0].typeargs.len();
    constraint.args.iter().take(arg_count).any(|arg| !typechecker::find_all_typevars(arg, true, cache).is_empty())
        // Make sure we never propagate when we're already in top-level in main with nowhere to propagate to.
        && typechecker::CURRENT_LEVEL.load(Ordering::SeqCst) >= INITIAL_LEVEL
}

/// Checks if the given `Int a` constraint is satisfied. These impls don't correspond
/// to actual impls in the source code since it is a builtin trait that describes primitive
/// integer types. So instead of searching for an impl here, we simply check that the arg
/// type `a` is a primitive integer type. If `a` is an unbound type variable, this will
/// also bind `a` to `i32` by default.
fn find_int_constraint_impl<'c>(constraint: &TraitConstraint, location: Location<'c>, cache: &mut ModuleCache<'c>) -> UnificationResult<'c> {
    let typ = typechecker::follow_bindings_in_cache(&constraint.args[0], cache);

    match &typ {
        Type::Primitive(PrimitiveType::IntegerType(kind)) => {
            // Any integer literal impl Int by default, though none should
            // be Unknown or Inferred at this point in type inference. Any Unknown literal
            // is translated to Inferred in LiteralKind::infer_impl and the type of such
            // a literal is always a TypeVariable rather than remaining an Inferred IntegerType.
            match kind {
                IntegerKind::Unknown => unreachable!(),
                IntegerKind::Inferred(_) => unreachable!(),
                _ => Ok(HashMap::new()),
            }
        },
        Type::TypeVariable(_) => {
            // The `Int a` constraint has special defaulting rules - since we know this typevar is
            // unbound, bind it to the default integer type (i32) here.
            // try_unify is used here to avoid performing the binding in case this impl isn't
            // selected to be used.
            typechecker::try_unify(&typ, &DEFAULT_INTEGER_TYPE, location, cache)
        },
        _ => Err(make_error!(location, "Expected a primitive integer type, but found {}", typ.display(cache))),
    }
}

/// Check if the given `.` family trait constraint is satisfied.
/// A constraint `a.field: b` is satisfied iff the type `a` has a
/// field named `field` which unifies with type `b`.
/// If this is not the case, an appropriate error message is returned.
fn find_member_access_impl<'c>(constraint: &TraitConstraint, location: Location<'c>, cache: &mut ModuleCache<'c>) -> UnificationResult<'c> {
    let collection = typechecker::follow_bindings_in_cache(&constraint.args[0], cache);

    let field_name = cache.trait_infos[constraint.trait_id.0].get_field_name();

    match collection {
        Type::UserDefinedType(id) => {
            let field_type = cache.type_infos[id.0].find_field(field_name)
                .map(|(_, field)| field.field_type.clone());

            match field_type {
                Some(field_type) => {
                    // FIXME: this unifies the type variables from the definition of field_type
                    // rather than the types it was instantiated to. This will be incorrect if
                    // the user ever uses a generic field with two different types!
                    typechecker::try_unify(&constraint.args[1], &field_type, location, cache)
                },
                None => Err(make_error!(location, "Type {} has no field named {}", collection.display(cache), field_name)),
            }
        },
        _ => Err(make_error!(location, "Type {} is not a struct type and has no field named {}", collection.display(cache), field_name)),
    }
}

/// Search and bind a specific impl to the given TraitConstraint, erroring if 0
/// or >1 matching impls are found.
fn solve_normal_constraint<'c>(constraint: &TraitConstraint, location: Location<'c>, cache: &mut ModuleCache<'c>) {
    let mut matching_impls = find_matching_impls(constraint, location, cache);

    if matching_impls.len() == 1 {
        let (impls, bindings) = matching_impls.remove(0);
        typechecker::perform_type_bindings(bindings, cache);
        for (impl_id, constraint) in impls {
            bind_impl(impl_id, constraint, cache);
        }
    } else if matching_impls.len() > 1 {
        error!(location, "{} matching impls found for {}", matching_impls.len(), constraint.display(cache));
        for (i, (impls, _)) in matching_impls.iter().enumerate() {
            let impl_id = impls[0].0;
            note!(cache.impl_infos[impl_id.0].location, "Candidate {}", i + 1);
        }
    } else {
        error!(location, "No impl found for {}", constraint.display(cache))
    }
}

/// Find and return (possibly multiple) matching impls for the given constraint.
/// Each matching impl will be returned along with all of its required impls from any `given`
/// constraints it may have in an element of the returned `Vec`.
///
/// For example, if our constraint is `Print i32` and we have he impls
/// `impl Print a given Cast a string` and
/// `impl Print i32` in scope then our returned set of matching impls will be
/// ```
/// vec![
///     (vec![(43, Print a), (123, Cast a string)], { a => i32 }),
///     (vec![(21, Print i32)], {})
/// ]
/// ```
///
/// Note that any impls that are automatically impld by the compiler will not have their
/// ImplInfoIds within the returned Vec (since they don't have any).
fn find_matching_impls<'c>(constraint: &TraitConstraint, location: Location<'c>, cache: &mut ModuleCache<'c>) -> Vec<(Vec<(ImplInfoId, TraitConstraint)>, TypeBindings)> {
    if constraint.is_int_constraint(cache) {
        match find_int_constraint_impl(constraint, location, cache) {
            Ok(bindings) => vec![(vec![], bindings)],
            Err(_) => vec![],
        }
    } else if constraint.is_member_access(cache) {
        match find_member_access_impl(constraint, location, cache) {
            Ok(bindings) => vec![(vec![], bindings)],
            Err(_) => vec![],
        }
    } else {
        find_matching_normal_impls(constraint, location, cache)
    }
}

fn find_matching_normal_impls<'c>(constraint: &TraitConstraint, location: Location<'c>, cache: &mut ModuleCache<'c>) -> Vec<(Vec<(ImplInfoId, TraitConstraint)>, TypeBindings)> {
    let scope = cache.impl_scopes[constraint.scope.0].clone();

    // First, filter all the impls whose arguments typecheck against our constraint's arguments
    let impls = scope.iter()
        .filter_map(|&impl_id| {
            if cache.impl_infos[impl_id.0].trait_id != constraint.trait_id {
                return None;
            }

            let impl_typeargs = cache.impl_infos[impl_id.0].typeargs.clone();

            match typechecker::try_unify_all(&impl_typeargs, &constraint.args, location, cache) {
                Ok(type_bindings) => Some((impl_id, type_bindings)),
                Err(_) => None,
            }
        }).collect::<Vec<_>>();

    // Then, check any `given Trait2 a ...` clauses for our impls to further narrow them down
    impls.into_iter().filter_map(|(impl_id, mut type_bindings)| {
        let impl_info = &cache.impl_infos[impl_id.0];
        let mut required_impls = vec![(impl_id, constraint.clone())];

        // TODO: Remove need for cloning here.
        // Needed because cache is borrowed mutably below.
        for required_trait in impl_info.given.clone() {
            let constraint = required_trait.as_constraint(constraint.scope, constraint.origin, constraint.callsite);
            let mut matching_impls = find_matching_impls(&constraint, location, cache);

            if matching_impls.len() == 1 {
                let (mut impls, bindings) = matching_impls.remove(0);
                type_bindings.extend(bindings);
                required_impls.append(&mut impls);
            } else {
                return None;
            }
        }

        Some((required_impls, type_bindings))
    }).collect()
}

fn bind_impl<'c>(impl_id: ImplInfoId, constraint: TraitConstraint, cache: &mut ModuleCache<'c>) {
    // Make sure the definition of this impl undergoes type inference if it hasn't already
    infer_trait_impl(impl_id, cache);

    // Now attach the RequiredImpl to the callsite variable it is used in
    let binding = find_definition_in_impl(constraint.origin, impl_id, cache);
    let callsite = constraint.callsite;
    let required_impl = constraint.as_required_impl(binding);

    let callsite_info = &mut cache.trait_bindings[callsite.0];
    callsite_info.required_impls.push(required_impl);
}

fn find_definition_in_impl<'c>(origin: VariableId, impl_id: ImplInfoId, cache: &ModuleCache<'c>) -> DefinitionInfoId {
    let name = &cache.variable_nodes[origin.0];

    let impl_info = &cache.impl_infos[impl_id.0];
    for definition in impl_info.definitions.iter().copied() {
        let definition_name = &cache.definition_infos[definition.0].name;
        if definition_name == name {
            return definition;
        }
    }
    unreachable!("Could not find definition for {} in impl at {}", name, impl_info.location);
}

fn infer_trait_impl<'a>(id: ImplInfoId, cache: &mut ModuleCache<'a>) {
    let info = &mut cache.impl_infos[id.0];
    let trait_impl = trustme::extend_lifetime(info.trait_impl);
    typechecker::infer(trait_impl, cache);
}
