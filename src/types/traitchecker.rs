//! traitchecker.rs -
//!
//! Trait inference is a part of type inference which determines:
//! 1. Which traits are required for a given Definition to be compiled
//! 2. When a `ast::Variable` is encountered whose Definition has some required traits
//!    whether these traits should be propagated up to be required for the current definition
//!    or whether they should be solved in place instead.
//! 3. Solving trait constraints, yielding the impl that should be used for that specific
//!    constraint and attaching this impl to the relevant callsite variable.
//!
//! The only public function of this module is `resolve_traits`, which is meant to be
//! called when an `ast::Definition` is finishes type inference on its expr rhs. This
//! function will look at all the given `TraitConstraint`s and determine whether each
//! depends on a parameter/return type and should thus be propogated up to the
//! `ast::Definition` as part of its signature, or does not depend on either and should
//! be solved in place instead. Any impl it solves in place it will attach the relevant
//! impl to the `ast::Variable` the TraitConstraint originated from, so that variable
//! has the correct definition to compile during codegen. For any impl it fails to solve,
//! a compile-time error will be issued.
use crate::cache::{DefinitionInfoId, ImplInfoId, ModuleCache, VariableId};
use crate::error::location::Location;
use crate::lexer::token::IntegerKind;
use crate::types::traits::{RequiredTrait, TraitConstraint, TraitConstraints};
use crate::types::typechecker::{self, TypeBindings, UnificationResult};
use crate::types::{PrimitiveType, Type, TypeInfoId, TypeVariableId, DEFAULT_INTEGER_TYPE};
use crate::util::{fmap, trustme};

use colored::Colorize;
use std::collections::HashMap;

/// Go through the given list of traits and determine if they should
/// be propogated upward or if an impl should be searched for now.
/// Returns the list of traits propogated upward.
/// Binds the impls that were searched for and found to the required_impls
/// in the callsite VariableInfo, and errors for any impls that couldn't be found.
pub fn resolve_traits<'a>(
    constraints: TraitConstraints, typevars_in_fn_signature: &[TypeVariableId], cache: &mut ModuleCache<'a>,
) -> Vec<RequiredTrait> {
    let (propagated_traits, int_constraints, member_access_constraints, other_constraints) =
        sort_traits(constraints, typevars_in_fn_signature, cache);

    let empty_bindings = HashMap::new();

    // Int constraints need to be searched for first since they have defaulting rules to default
    // `Int a` to `Int i32` if `a` is unbound. This can impact the remainder of the impl search
    // if, for example, there is a `Cast a string` constraint this is solveable if `a = i32` is
    // known, but not solveable otherwise (barring a user-defined impl).
    for constraint in int_constraints.iter() {
        typechecker::perform_bindings_or_print_error(
            find_int_constraint_impl(constraint, &empty_bindings, cache),
            cache,
        );
    }

    // Member access constraints don't need to be searched for before normal constraints, but
    // they're separated out anyway since searching for them is done differently since they're
    // automatically impl'd by the compiler.
    for constraint in member_access_constraints.iter() {
        typechecker::perform_bindings_or_print_error(
            find_member_access_impl(constraint, &empty_bindings, cache),
            cache,
        );
    }

    for constraint in other_constraints.iter() {
        // Searching for an impl for normal constraints may require recursively searching for
        // more impls (due to `impl A given B` constraints) before finding a matching one.
        solve_normal_constraint(constraint, &empty_bindings, cache);
    }

    propagated_traits
}

/// These just make the signature of sort_traits read better.
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
fn sort_traits<'c>(
    constraints: TraitConstraints, typevars_in_fn_signature: &[TypeVariableId], cache: &ModuleCache<'c>,
) -> (PropagatedTraits, IntTraits, MemberAccessTraits, TraitConstraints) {
    let mut propogated_traits = vec![];
    let mut int_constraints = vec![];
    let mut member_access_constraints = vec![];
    let mut other_constraints = Vec::with_capacity(constraints.len());

    for constraint in constraints {
        if should_propagate(&constraint, typevars_in_fn_signature, cache) {
            propogated_traits.push(constraint.into_required_trait());
        } else if constraint.is_int_constraint(cache) {
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
fn should_propagate<'a>(
    constraint: &TraitConstraint, typevars_in_fn_signature: &[TypeVariableId], cache: &ModuleCache<'a>,
) -> bool {
    // Don't check the fundeps since only the typeargs proper are used to find impls
    let arg_count = cache[constraint.trait_id].typeargs.len();

    constraint
        .args
        .iter()
        .take(arg_count)
        .any(|arg| typechecker::contains_any_typevars_from_list(arg, typevars_in_fn_signature, cache))
}

/// Checks if the given `Int a` constraint is satisfied. These impls don't correspond
/// to actual impls in the source code since it is a builtin trait that describes primitive
/// integer types. So instead of searching for an impl here, we simply check that the arg
/// type `a` is a primitive integer type. If `a` is an unbound type variable, this will
/// also bind `a` to `i32` by default.
fn find_int_constraint_impl<'c>(
    constraint: &TraitConstraint, bindings: &TypeBindings, cache: &mut ModuleCache<'c>,
) -> UnificationResult<'c> {
    let typ = typechecker::follow_bindings_in_cache_and_map(&constraint.args[0], bindings, cache);

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
            typechecker::try_unify(&typ, &DEFAULT_INTEGER_TYPE, constraint.locate(cache), cache)
        },
        _ => Err(make_error!(
            constraint.locate(cache),
            "Expected a primitive integer type, but found {}",
            typ.display(cache)
        )),
    }
}

/// Check if the given `.` family trait constraint is satisfied.
/// A constraint `a.field: b` is satisfied iff the type `a` has a
/// field named `field` which unifies with type `b`.
/// If this is not the case, an appropriate error message is returned.
fn find_member_access_impl<'c>(
    constraint: &TraitConstraint, bindings: &TypeBindings, cache: &mut ModuleCache<'c>,
) -> UnificationResult<'c> {
    let collection = typechecker::follow_bindings_in_cache_and_map(&constraint.args[0], bindings, cache);
    let field_name = cache[constraint.trait_id].get_field_name().to_string();
    let expected_field_type = &constraint.args[1];
    let location = constraint.locate(cache);

    match &collection {
        Type::UserDefined(id) => find_field(*id, &[], &field_name, expected_field_type, location, cache),
        Type::TypeApplication(typ, args) => match typ.as_ref() {
            Type::UserDefined(id) => find_field(*id, args, &field_name, expected_field_type, location, cache),
            _ => Err(make_error!(
                location,
                "Type {} is not a struct type and has no field named {}",
                collection.display(cache),
                field_name
            )),
        },
        _ => Err(make_error!(
            location,
            "Type {} is not a struct type and has no field named {}",
            collection.display(cache),
            field_name
        )),
    }
}

fn find_field<'c>(
    id: TypeInfoId, args: &[Type], field_name: &str, expected_field_type: &Type, location: Location<'c>,
    cache: &mut ModuleCache<'c>,
) -> UnificationResult<'c> {
    let type_info = &cache[id];
    let bindings = typechecker::type_application_bindings(type_info, args);
    let mut result_bindings = bindings.clone();

    let field_type = type_info.find_field(field_name).map(|(_, field)| field.field_type.clone());

    match field_type {
        Some(field_type) => {
            typechecker::try_unify_with_bindings(
                expected_field_type,
                &field_type,
                &mut result_bindings,
                location,
                cache,
            )?;

            // Filter out only the new bindings we did not start with since we started with
            // local type bindings from the type arguments that should not be bound globally.
            Ok(result_bindings.into_iter().filter(|(id, _)| !bindings.contains_key(id)).collect())
        },
        None => Err(make_error!(location, "Type {} has no field named {}", type_info.name.blue(), field_name)),
    }
}

/// Search and bind a specific impl to the given TraitConstraint, erroring if 0
/// or >1 matching impls are found.
fn solve_normal_constraint<'c>(constraint: &TraitConstraint, bindings: &TypeBindings, cache: &mut ModuleCache<'c>) {
    let mut matching_impls = find_matching_impls(constraint, bindings, cache);

    #[allow(clippy::comparison_chain)]
    if matching_impls.len() == 1 {
        let (impls, bindings) = matching_impls.remove(0);
        typechecker::perform_type_bindings(bindings, cache);
        for (impl_id, constraint) in impls {
            bind_impl(impl_id, constraint, cache);
        }
    } else if matching_impls.len() > 1 {
        error!(
            constraint.locate(cache),
            "{} matching impls found for {}",
            matching_impls.len(),
            constraint.display(cache)
        );
        for (i, (impls, _)) in matching_impls.iter().enumerate() {
            let impl_id = impls[0].0;
            note!(cache[impl_id].location, "Candidate {}", i + 1);
        }
    } else {
        error!(constraint.locate(cache), "No impl found for {}", constraint.display(cache))
    }
}

/// Find and return (possibly multiple) matching impls for the given constraint.
/// Each matching impl will be returned along with all of its required impls from any `given`
/// constraints it may have in an element of the returned `Vec`.
///
/// For example, if our constraint is `Print i32` and we have the impls
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
fn find_matching_impls<'c>(
    constraint: &TraitConstraint, bindings: &TypeBindings, cache: &mut ModuleCache<'c>,
) -> Vec<(Vec<(ImplInfoId, TraitConstraint)>, TypeBindings)> {
    if constraint.is_int_constraint(cache) {
        match find_int_constraint_impl(constraint, bindings, cache) {
            Ok(bindings) => vec![(vec![], bindings)],
            Err(_) => vec![],
        }
    } else if constraint.is_member_access(cache) {
        match find_member_access_impl(constraint, bindings, cache) {
            Ok(bindings) => vec![(vec![], bindings)],
            Err(_) => vec![],
        }
    } else {
        find_matching_normal_impls(constraint, bindings, cache)
    }
}

/// Searches for a non-Int, non-member-access impl for the given constraint.
/// Returns each matching impl found in a Vec. Since each matching impl may have n
/// required `given` constraints, these impls in the given constraints are also returned.
/// Thus, each element of the returned Vec will contain a set of the original impl found
/// and all impls it depends on (in practice this number is small, usually < 2).
fn find_matching_normal_impls<'c>(
    constraint: &TraitConstraint, bindings: &TypeBindings, cache: &mut ModuleCache<'c>,
) -> Vec<(Vec<(ImplInfoId, TraitConstraint)>, TypeBindings)> {
    let scope = cache[constraint.scope].clone();

    scope
        .iter()
        .filter_map(|&impl_id| {
            // First, filter all the impls whose arguments typecheck against our constraint's arguments
            if cache[impl_id].trait_id != constraint.trait_id {
                return None;
            }

            // Replace all the type variables in the `impl Foo a` so when we unify later we don't
            // bind to the original `a`, just one instantiation of it.
            let (impl_typeargs, impl_bindings) =
                typechecker::replace_all_typevars(&cache[impl_id].typeargs.clone(), cache);

            let location = constraint.locate(cache);
            let type_bindings = typechecker::try_unify_all_with_bindings(
                &impl_typeargs,
                &constraint.args,
                bindings.clone(),
                location,
                cache,
            )
            .ok()?;

            // Then, check any `given Trait2 a ...` clauses for our impls to further narrow them down
            check_given_constraints(constraint, impl_id, type_bindings, impl_bindings, cache)
        })
        .collect()
}

/// Check whether the given constraint has any required `given` constraints for the impl to be
/// valid. For example, the impl `impl Print a given Cast a string` has the given constraint
/// `Cast a string` and is thus only valid if that impl can be found as well.
/// If any of these given constraints cannot be solved then None is returned. Otherwise, the Vec
/// of the original constraint and all its required given constraints are returned.
fn check_given_constraints<'c>(
    constraint: &TraitConstraint, impl_id: ImplInfoId, mut type_bindings: TypeBindings,
    mut impl_bindings: TypeBindings, cache: &mut ModuleCache<'c>,
) -> Option<(Vec<(ImplInfoId, TraitConstraint)>, TypeBindings)> {
    let mut required_impls = vec![(impl_id, constraint.clone())];

    // TODO: Remove need for cloning here.
    // Needed because cache is borrowed mutably below.
    for required_trait in cache[impl_id].given.clone() {
        let mut constraint = required_trait.as_constraint(constraint.scope, constraint.origin, constraint.callsite);

        // Must carry forward the impl_bindings we got from find_matching_normal_impls
        // manually since we don't want to insert them into the catch if this impl doesn't
        // get selected to be used for the TraitConstraint.
        constraint.args = fmap(&constraint.args, |typ| {
            typechecker::replace_all_typevars_with_bindings(typ, &mut impl_bindings, cache)
        });

        let mut matching_impls = find_matching_impls(&constraint, &type_bindings, cache);

        if matching_impls.len() == 1 {
            let (mut impls, bindings) = matching_impls.remove(0);
            type_bindings.extend(bindings);
            required_impls.append(&mut impls);
        } else {
            return None;
        }
    }

    Some((required_impls, type_bindings))
}

/// Binds a selected impl to its callsite. This attaches the relevant impl definition to the
/// callsite variable so that static dispatch may occur during codegen.
fn bind_impl(impl_id: ImplInfoId, constraint: TraitConstraint, cache: &mut ModuleCache) {
    // Make sure the definition of this impl undergoes type inference if it hasn't already
    infer_trait_impl(impl_id, cache);

    // Now attach the RequiredImpl to the callsite variable it is used in
    let binding = find_definition_in_impl(constraint.origin, impl_id, cache);
    let callsite = constraint.callsite;
    let required_impl = constraint.into_required_impl(binding);

    let callsite_info = &mut cache[callsite];
    callsite_info.required_impls.push(required_impl);
}

/// Returns the given definition from the given impl.
/// This should always succeed (and thus panics if it does not) since all impls are validated
/// during name resolution that they have definitions for every declaration in the corresponding trait.
fn find_definition_in_impl(origin: VariableId, impl_id: ImplInfoId, cache: &ModuleCache) -> DefinitionInfoId {
    let name = &cache.variable_nodes[origin.0];

    let impl_info = &cache[impl_id];
    for definition in impl_info.definitions.iter().copied() {
        let definition_name = &cache[definition].name;
        if definition_name == name {
            return definition;
        }
    }
    unreachable!("Could not find definition for {} in impl at {}", name, impl_info.location);
}

/// Once an impl is selected, recur type inference on the impl's definitions to make
/// sure it is well typed. This follows the recursion scheme used by the rest of the type
/// inference pass: Definitions are lazily type inferenced when a variable using that defintion
/// is found in the program.
fn infer_trait_impl(id: ImplInfoId, cache: &mut ModuleCache) {
    let trait_impl = trustme::extend_lifetime(cache[id].trait_impl);
    typechecker::infer(trait_impl, cache);
}
