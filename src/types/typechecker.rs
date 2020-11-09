//! Passes over the ast, filling out the (typ: Type) field of each node.
//! This uses algorithm j extended with let polymorphism and multi-parameter
//! typeclasses (traits) with a very limited form of functional dependencies.
//! For generalization this uses let binding levels to determine if types escape
//! the current binding and should thus not be generalized.
//!
//! Note: most of this file is directly translated from:
//! https://github.com/jfecher/algorithm-j
use crate::cache::{ ModuleCache, TraitInfoId, DefinitionInfoId, DefinitionKind };
use crate::cache::{ ImplInfoId, ImplScopeId, TraitBindingId, VariableId };
use crate::error::location::{ Location, Locatable };
use crate::error::{ ErrorMessage, get_error_count };
use crate::lexer::token::IntegerKind;
use crate::parser::ast;
use crate::types::pattern;
use crate::types::{ Type, Type::*, TypeVariableId, PrimitiveType, LetBindingLevel, INITIAL_LEVEL, TypeBinding::* };
use crate::types::{ TypeBinding, STRING_TYPE };
use crate::types::typed::Typed;
use crate::types::traits::{ TraitConstraints, RequiredTrait, TraitConstraint };
use crate::util::*;

use std::collections::HashMap;
use std::sync::atomic::{ AtomicUsize, Ordering };


pub static CURRENT_LEVEL: AtomicUsize = AtomicUsize::new(INITIAL_LEVEL);

/// A sparse set of type bindings, used by try_unify
pub type TypeBindings = HashMap<TypeVariableId, Type>;

/// Replace any typevars found in typevars_to_replace with the
/// associated value in the same table, leave them otherwise
fn replace_typevars<'b>(typ: &Type, typevars_to_replace: &HashMap<TypeVariableId, TypeVariableId>, cache: &ModuleCache<'b>) -> Type {
    let typevars_to_replace = typevars_to_replace.iter()
        .map(|(key, id)| (*key, TypeVariable(*id)))
        .collect();

    bind_typevars(typ, &typevars_to_replace, cache)
}

/// Replace any typevars found with the given type bindings
pub fn bind_typevars<'b>(typ: &Type, type_bindings: &TypeBindings, cache: &ModuleCache<'b>) -> Type {
    match typ {
        Primitive(p) => Primitive(*p),
        TypeVariable(id) => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                bind_typevars(&typ.clone(), type_bindings, cache)
            } else {
                let original = TypeVariable(*id);
                let replacement = type_bindings.get(id).unwrap_or(&original);
                replacement.clone()
            }
        },
        Function(parameters, return_type) => {
            let parameters = fmap(parameters, |parameter| bind_typevars(parameter, type_bindings, cache));
            let return_type = bind_typevars(return_type, type_bindings, cache);
            Function(parameters, Box::new(return_type))
        },
        ForAll(_typevars, _typ) => {
            unreachable!("Ante does not support higher rank polymorphism");
        }
        UserDefinedType(id) => UserDefinedType(*id),

        TypeApplication(typ, args) => {
            let typ = bind_typevars(typ, type_bindings, cache);
            let args = fmap(args, |arg| bind_typevars(arg, type_bindings, cache));
            TypeApplication(Box::new(typ), args)
        },
        Tuple(elements) => {
            Tuple(fmap(elements, |element| bind_typevars(element, type_bindings, cache)))
        }
    }
}

/// Helper function for getting the next type variable at the current level
fn next_type_variable_id<'a>(cache: &mut ModuleCache<'a>) -> TypeVariableId {
    let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
    cache.next_type_variable_id(level)
}

fn next_type_variable<'a>(cache: &mut ModuleCache<'a>) -> Type {
    let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
    cache.next_type_variable(level)
}

fn to_trait_constraints(required_traits: &[RequiredTrait], scope: ImplScopeId,
    callsite_id: VariableId, callsite: Option<TraitBindingId>) -> TraitConstraints
{
    fmap(required_traits, |required_trait| required_trait.as_constraint(scope, callsite_id, callsite.unwrap()))
}

/// specializes the polytype s by copying the term and replacing the
/// bound type variables consistently by new monotype variables
/// E.g.   instantiate (forall a b. a -> b -> a) = c -> d -> c
///
/// This will also instantiate each given trait constraint, replacing
/// each free typevar of the constraint's argument types.
pub fn instantiate<'b>(s: &Type, mut constraints: TraitConstraints, cache: &mut ModuleCache<'b>) -> (Type, TraitConstraints) {
    // Note that the returned type is no longer a PolyType,
    // this means it is now monomorphic and not forall-quantified
    match s {
        TypeVariable(id) => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                instantiate(&typ.clone(), constraints, cache)
            } else {
                (TypeVariable(*id), constraints)
            }
        },
        ForAll(typevars, typ) => {
            // Must replace all typevars in typ and the required_traits list with new ones
            let mut typevars_to_replace = HashMap::new();
            for var in typevars.iter().copied() {
                typevars_to_replace.insert(var, next_type_variable_id(cache));
            }
            let typ = replace_typevars(&typ, &typevars_to_replace, cache);

            for var in find_all_typevars_in_traits(&constraints, cache).iter().copied() {
                if !typevars_to_replace.contains_key(&var) {
                    typevars_to_replace.insert(var, next_type_variable_id(cache));
                }
            }

            for constraint in constraints.iter_mut() {
                for typ in constraint.args.iter_mut() {
                    *typ = replace_typevars(typ, &typevars_to_replace, cache);
                }
            }
            (typ, constraints)
        },
        other => {
            (other.clone(), constraints)
        },
    }
}

/// Similar to instantiate but uses an explicitly passed map to map
/// the old type variables to. This version is used during trait impl
/// type inference to ensure all definitions in the trait impl are
/// mapped to the same typevars, rather than each definition instantiated
/// separately as is normal.
fn instantiate_from_map<'b>(s: &Type, typevars_to_replace: &HashMap<TypeVariableId, TypeVariableId>,
    cache: &mut ModuleCache<'b>) -> Type
{
    // Note that the returned type is no longer a PolyType,
    // this means it is now monomorphic and not forall-quantified
    match s {
        TypeVariable(id) => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                instantiate_from_map(&typ.clone(), typevars_to_replace, cache)
            } else {
                TypeVariable(*id)
            }
        },
        ForAll(_, typ) => {
            replace_typevars(&typ, typevars_to_replace, cache)
        },
        other => other.clone(),
    }
}

fn find_binding<'b>(id: TypeVariableId, map: &TypeBindings, cache: &ModuleCache<'b>) -> TypeBinding {
    match &cache.type_bindings[id.0] {
        Bound(typ) => Bound(typ.clone()),
        Unbound(level, kind) => {
            match map.get(&id) {
                Some(typ) => Bound(typ.clone()),
                None => Unbound(*level, kind.clone()),
            }
        }
    }
}

/// Can a monomorphic TypeVariable(id) be found inside this type?
/// This will mutate any typevars found to increase their LetBindingLevel.
/// Doing so increases the lifetime of the typevariable and lets us keep
/// track of which type variables to generalize later on. It also means
/// that occurs should only be called during unification however.
fn occurs<'b>(id: TypeVariableId, level: LetBindingLevel, typ: &Type, bindings: &mut TypeBindings, cache: &mut ModuleCache<'b>) -> bool {
    match typ {
        Primitive(_) => false,
        UserDefinedType(_) => false,

        TypeVariable(var_id) => {
            match find_binding(*var_id, bindings, cache) {
                Bound(binding) => occurs(id, level, &binding, bindings, cache),
                Unbound(original_level, kind) => {
                    let min_level = std::cmp::min(level, original_level);
                    cache.type_bindings[id.0] = Unbound(min_level, kind);
                    id == *var_id
                }
            }
        },
        Function(parameters, return_type) => {
            occurs(id, level, return_type, bindings, cache)
            || parameters.iter().any(|parameter| occurs(id, level, parameter, bindings, cache))
        },
        TypeApplication(typ, args) => {
            occurs(id, level, typ, bindings, cache)
            || args.iter().any(|arg| occurs(id, level, arg, bindings, cache))
        },
        Tuple(elements) => {
            elements.iter().any(|element| occurs(id, level, element, bindings, cache))
        },
        ForAll(typevars, typ) => {
            !typevars.iter().any(|typevar| *typevar == id)
            && occurs(id, level, typ, bindings, cache)
        },
    }
}

/// Returns what a given type is bound to, following all typevar links until it reaches an Unbound one.
fn follow_bindings<'b>(typ: &Type, bindings: &TypeBindings, cache: &ModuleCache<'b>) -> Type {
    match typ {
        TypeVariable(id) => {
            match find_binding(*id, bindings, cache) {
                Bound(typ) => follow_bindings(&typ, bindings, cache),
                Unbound(..) => typ.clone(),
            }
        }
        _ => typ.clone(),
    }
}

pub fn try_unify<'b>(t1: &Type, t2: &Type, bindings: &mut TypeBindings, location: Location<'b>, cache: &mut ModuleCache<'b>) -> Result<(), ErrorMessage<'b>> {
    match (t1, t2) {
        (Primitive(p1), Primitive(p2)) if p1 == p2 => Ok(()),

        (UserDefinedType(id1), UserDefinedType(id2)) if id1 == id2 => Ok(()),

        // Any type variable can be bound or unbound.
        // - If bound: unify the bound type with the other type.
        // - If unbound: 'unify' the LetBindingLevel of the type variable by setting
        //   it to the minimum scope of type variables in b. This happens within the occurs check.
        //   The unification of the LetBindingLevel here is a form of lifetime inference for the
        //   typevar and is used during generalization to determine which variables to generalize.
        (TypeVariable(id), b) => {
            match find_binding(*id, bindings, &cache) {
                Bound(a) => {
                    try_unify(&a, b, bindings, location, cache)
                },
                Unbound(a_level, _a_kind) => {
                    // Create binding for boundTy that is currently empty.
                    // Ensure not to create recursive bindings to the same variable
                    let b = follow_bindings(b, bindings, cache);
                    if *t1 != b {
                        // TODO: Can this occurs check not mutate the typevar levels until we
                        // return success?
                        if occurs(*id, a_level, &b, bindings, cache) {
                            Err(make_error!(location, "Cannot construct recursive type: {} = {}", t1.debug(cache), t2.debug(cache)))
                        } else {
                            bindings.insert(*id, b);
                            Ok(())
                        }
                    } else {
                        Ok(())
                    }
                }
            }
        },

        (a, TypeVariable(id)) => {
            match find_binding(*id, &bindings, &cache) {
                Bound(b) => {
                    try_unify(a, &b, bindings, location, cache)
                },
                Unbound(b_level, _b_kind) => {
                    // Create binding for boundTy that is currently empty.
                    // Ensure not to create recursive bindings to the same variable
                    let a = follow_bindings(a, bindings, cache);
                    if a != *t2 {
                        if occurs(*id, b_level, &a, bindings, cache) {
                            Err(make_error!(location, "Cannot construct recursive type: {} = {}", t1.debug(cache), t2.debug(cache)))
                        } else {
                            bindings.insert(*id, a);
                            Ok(())
                        }
                    } else {
                        Ok(())
                    }
                }
            }
        },

        (Function(a_args, a_ret), Function(b_args, b_ret)) => {
            if a_args.len() != b_args.len() {
                return Err(make_error!(location, "Type mismatch between {} and {}", t1.display(cache), t2.display(cache)));
            }

            for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                try_unify(a_arg, b_arg, bindings, location, cache)?;
            }

            try_unify(a_ret, b_ret, bindings, location, cache)?;
            Ok(())
        },

        (TypeApplication(a_constructor, a_args), TypeApplication(b_constructor, b_args)) => {
            if a_args.len() != b_args.len() {
                return Err(make_error!(location, "Type mismatch between {} and {}", t1.display(cache), t2.display(cache)));
            }

            try_unify(a_constructor, b_constructor, bindings, location, cache)?;

            for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                try_unify(a_arg, b_arg, bindings, location, cache)?;
            }

            Ok(())
        },

        (Tuple(a_elements), Tuple(b_elements)) => {
            if a_elements.len() != b_elements.len() {
                return Err(make_error!(location, "Type mismatch between {} and {}", t1.display(cache), t2.display(cache)));
            }

            for (a_element, b_element) in a_elements.iter().zip(b_elements.iter()) {
                try_unify(a_element, b_element, bindings, location, cache)?;
            }

            Ok(())
        },

        (ForAll(a_vars, a), ForAll(b_vars, b)) => {
            if a_vars.len() != b_vars.len() {
                return Err(make_error!(location, "Type mismatch between {} and {}", a.display(cache), b.display(cache)));
            }
            try_unify(a, b, bindings, location, cache)
        },

        (a, b) => Err(make_error!(location, "Type mismatch between {} and {}", a.display(cache), b.display(cache))),
    }
}

/// Try to unify the types from all the given vectors. We only return success or failure since that
/// is all that is needed during trait resultion. If this function is ever needed outside of trait
/// resolution it can be altered to collect the appropriate bindings/error messages as well.
pub fn try_unify_all<'b>(vec1: &Vec<Type>, vec2: &Vec<Type>, location: Location<'b>, cache: &mut ModuleCache<'b>) -> Result<TypeBindings, ()> {
    if vec1.len() != vec2.len() {
        return Err(());
    }

    let mut bindings = HashMap::new();
    for (t1, t2) in vec1.iter().zip(vec2.iter()) {
        match try_unify(t1, t2, &mut bindings, location, cache) {
            Err(_) => return Err(()),
            _ => (),
        }
    }
    Ok(bindings)
}

pub fn unify<'b>(t1: &Type, t2: &Type, location: Location<'b>, cache: &mut ModuleCache<'b>) {
    let mut bindings = HashMap::new();
    match try_unify(t1, t2, &mut bindings, location, cache) {
        Ok(()) => {
            for (id, binding) in bindings.into_iter() {
                cache.type_bindings[id.0] = Bound(binding);
            }
        },
        Err(message) => {
            println!("{}", message);
        }
    }
}

fn level_is_polymorphic(level: LetBindingLevel) -> bool {
    level.0 > CURRENT_LEVEL.load(Ordering::SeqCst)
}

/// Collects all the type variables contained within typ into a Vec.
/// If polymorphic_only is true, any polymorphic type variables will be filtered out.
///
/// Since this function uses CURRENT_LEVEL when polymorphic_only = true, the function
/// should only be used with polymorphic_only = false outside of the typechecking pass.
/// Otherwise the decision of whether to propagate the variable would be incorrect.
pub fn find_all_typevars<'a>(typ: &Type, polymorphic_only: bool, cache: &ModuleCache<'a>) -> Vec<TypeVariableId> {
    match typ {
        Primitive(_) => vec![],
        UserDefinedType(_) => vec![],
        TypeVariable(id) => {
            match &cache.type_bindings[id.0] {
                Bound(t) => find_all_typevars(t, polymorphic_only, cache),
                Unbound(level, _) => {
                    if level_is_polymorphic(*level) || !polymorphic_only {
                        vec![*id]
                    } else {
                        vec![]
                    }
                }
            }
        },
        Function(parameters, return_type) => {
            let mut type_variables = vec![];
            for parameter in parameters {
                type_variables.append(&mut find_all_typevars(&parameter, polymorphic_only, cache));
            }
            type_variables.append(&mut find_all_typevars(return_type, polymorphic_only, cache));
            type_variables
        },
        TypeApplication(constructor, args) => {
            let mut type_variables = find_all_typevars(constructor, polymorphic_only, cache);
            for arg in args {
                type_variables.append(&mut find_all_typevars(&arg, polymorphic_only, cache));
            }
            type_variables
        },
        Tuple(elements) => {
            elements.iter().flat_map(|element| find_all_typevars(element, polymorphic_only, cache)).collect()
        },
        ForAll(polymorphic_typevars, typ) => {
            if polymorphic_only {
                polymorphic_typevars.clone()
            } else {
                let mut typevars = polymorphic_typevars.clone();
                typevars.append(&mut find_all_typevars(typ, false, cache));
                typevars
            }
        },
    }
}

fn find_all_typevars_in_traits<'a>(traits: &TraitConstraints, cache: &ModuleCache<'a>) -> Vec<TypeVariableId> {
    let mut typevars = vec![];
    for constraint in traits.iter() {
        for typ in constraint.args.iter() {
            typevars.append(&mut find_all_typevars(typ, true, cache));
        }
    }
    typevars
}

/// Find all typevars declared inside the current LetBindingLevel and wrap the type in a PolyType
/// e.g.  generalize (a -> b -> b) = forall a b. a -> b -> b
fn generalize<'a>(typ: &Type, cache: &ModuleCache<'a>) -> Type {
    let mut typevars = find_all_typevars(typ, true, cache);
    if typevars.is_empty() {
        typ.clone()
    } else {
        // TODO: This can be sped up, e.g. we wouldn't need to dedup at all if we didn't use a Vec
        typevars.sort();
        typevars.dedup();
        ForAll(typevars, Box::new(typ.clone()))
    }
}

fn infer_nested_definition<'a>(definition_id: DefinitionInfoId, impl_scope: ImplScopeId,
    callsite_id: VariableId, callsite: Option<TraitBindingId>, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints)
{
    let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
    let typevar = cache.next_type_variable(level);
    let info = &mut cache.definition_infos[definition_id.0];
    let definition = info.definition.as_mut().unwrap();
    // Mark the definition with a fresh typevar for recursive references
    info.typ = Some(typevar.clone());

    match definition {
        DefinitionKind::Definition(definition) => {
            let definition = trustme::extend_lifetime(*definition);
            infer(definition, cache);
        },
        DefinitionKind::TraitDefinition(definition) => {
            let definition = trustme::extend_lifetime(*definition);
            infer(definition, cache);
        },
        DefinitionKind::Extern(declaration) => {
            let definition = trustme::extend_lifetime(*declaration);
            infer(definition, cache);
        },
        DefinitionKind::Parameter => {},
        DefinitionKind::MatchPattern => {},
        DefinitionKind::TypeConstructor { .. } => {},
    };

    let info = &mut cache.definition_infos[definition_id.0];
    let constraints = to_trait_constraints(&info.required_traits, impl_scope, callsite_id, callsite);
    (info.typ.clone().unwrap(), constraints)
}

/// Binds a given type to an irrefutable pattern, recursing on the pattern and verifying
/// that it is indeed irrefutable. If should_generalize is true, this generalizes the type given
/// to any variable encountered. Appends the given required_traits list in the DefinitionInfo's
/// required_traits field.
fn bind_irrefutable_pattern<'a>(ast: &mut ast::Ast<'a>, typ: &Type,
    required_traits: &Vec<RequiredTrait>, should_generalize: bool, cache: &mut ModuleCache<'a>)
{
    use ast::Ast::*;
    use ast::LiteralKind;

    match ast {
        Literal(literal) => {
            match literal.kind {
                LiteralKind::Unit => unify(typ, &Type::Primitive(PrimitiveType::UnitType), ast.locate(), cache),
                _ => error!(ast.locate(), "Pattern is not irrefutable"),
            }
        },
        Variable(variable) => {
            let definition_id = variable.definition.unwrap();
            let info = &cache.definition_infos[definition_id.0];

            // The type may already be set (e.g. from a trait impl this definition belongs to).
            // If it is, unify the existing type and new type before generalizing them.
            if let Some(existing_type) = &info.typ {
                unify(&existing_type.clone(), &typ, variable.location, cache);
            }

            let typ = if should_generalize { generalize(typ, cache) } else { typ.clone() };

            let info = &mut cache.definition_infos[definition_id.0];
            info.required_traits.append(&mut required_traits.clone());
            variable.typ = Some(typ.clone());

            info.typ = Some(typ);
        },
        TypeAnnotation(annotation) => {
            unify(typ, annotation.typ.as_ref().unwrap(), annotation.location, cache);
            bind_irrefutable_pattern(annotation.lhs.as_mut(), typ, required_traits, should_generalize, cache);
        },
        Tuple(tuple) => {
            let tuple_type = Type::Tuple(fmap(&tuple.elements, |_| next_type_variable(cache)));
            unify(&typ, &tuple_type, tuple.location, cache);

            match tuple_type {
                Type::Tuple(elements) => {
                    for (element, element_type) in tuple.elements.iter_mut().zip(elements) {
                        bind_irrefutable_pattern(element, &element_type, required_traits, should_generalize, cache);
                    }
                },
                _ => unreachable!(),
            }
        },
        _ => {
            error!(ast.locate(), "Invalid syntax in irrefutable pattern");
        }
    }
}

fn lookup_definition_type_in_trait<'a>(name: &str, trait_id: TraitInfoId, cache: &mut ModuleCache<'a>) -> Type {
    let trait_info = &cache.trait_infos[trait_id.0];
    for definition_id in trait_info.definitions.iter() {
        let definition_info = &cache.definition_infos[definition_id.0];
        if definition_info.name == name {
            return definition_info.typ.clone().unwrap();
        }
    }
    unreachable!();
}

/// Both this function and bind_irrefutable_pattern traverse an irrefutable pattern.
/// The former traverses the pattern along with a type and unifies them. This one traverses
/// the pattern and unifies any names it finds with matching names in the given TraitInfo.
/// Additionally, instead of instantiating every definition separately this function receives the
/// already-instantiated type variables from the trait impl.
///
/// Note: This function needs to be called before type inference on the trait impl definition
/// for two reasons:
///     1. Inference on Definitions performs generalization which would mean we'd otherwise need to
///        forcibly remove the forall without instantiating it to unify with trait_type here.
///     2. Binding the pattern to the definintion type from the parent trait here improves error
///        messages! Binding it beforehand leads to error messages inside the function body where
///        the e.g. return type conflicts. Binding it afterward would produce error messages with
///        the location of the ast in this function, which would just be the entire Definition.
///        Additionally, it would give the entire function type instead of just the return
///        type or parameter type that was incorrect.
fn bind_irrefutable_pattern_in_impl<'a>(ast: &ast::Ast<'a>, trait_id: TraitInfoId, typevars_to_replace: &HashMap<TypeVariableId, TypeVariableId>, cache: &mut ModuleCache<'a>) {
    use ast::Ast::*;
    match ast {
        Variable(variable) => {
            let name = variable.to_string();
            let trait_type = lookup_definition_type_in_trait(&name, trait_id, cache);
            let trait_type = instantiate_from_map(&trait_type, typevars_to_replace, cache);

            let definition_id = variable.definition.unwrap();
            let info = &mut cache.definition_infos[definition_id.0];
            info.typ = Some(trait_type);
        },
        TypeAnnotation(annotation) => {
            bind_irrefutable_pattern_in_impl(annotation.lhs.as_ref(), trait_id, typevars_to_replace, cache);
        },
        _ => {
            error!(ast.locate(), "Invalid syntax in irrefutable pattern in trait impl, expected a name or a tuple of names");
        }
    }
}

/// A trait should be propogated to the public signature of a Definition if any of its contained
/// type variables should be generalized. If the trait shouldn't be propogated then an impl
/// should be resolved instead.
fn should_propagate<'a>(constraint: &TraitConstraint, cache: &ModuleCache<'a>) -> bool {
    // Don't check the fundeps since only the typeargs proper are used to find impls
    let arg_count = cache.trait_infos[constraint.trait_id.0].typeargs.len();
    constraint.args.iter().take(arg_count).any(|arg| !find_all_typevars(arg, true, cache).is_empty())
        // Make sure we never propagate when we're already in top-level in main with nowhere to propagate to.
        && CURRENT_LEVEL.load(Ordering::SeqCst) >= INITIAL_LEVEL
}

fn check_member_access<'a>(constraint: &TraitConstraint, location: Location<'a>, cache: &mut ModuleCache<'a>) -> Vec<ErrorMessage<'a>> {
    let empty_bindings = HashMap::new();
    let collection = follow_bindings(&constraint.args[0], &empty_bindings, cache);

    let field_name = &cache.trait_infos[constraint.trait_id.0].name[1..];

    match collection {
        Type::UserDefinedType(id) => {
            let field_type = cache.type_infos[id.0].find_field(field_name)
                .map(|(_, field)| field.field_type.clone());

            match field_type {
                Some(field_type) => {
                    // FIXME: this unifies the type variables from the definition of field_type
                    // rather than the types it was instantiated to. This will be incorrect if
                    // the user ever uses a generic field with two different types!
                    unify(&constraint.args[1], &field_type, location, cache);
                    vec![]
                },
                _ => vec![make_error!(location, "Type {} has no field named {}", collection.display(cache), field_name)],
            }

        },
        _ => vec![make_error!(location, "Type {} is not a struct type and has no field named {}", collection.display(cache), field_name)],
    }
}

fn check_int_trait<'a>(constraint: &TraitConstraint, location: Location<'a>, cache: &mut ModuleCache<'a>) -> Vec<ErrorMessage<'a>> {
    let empty_bindings = HashMap::new();
    let typ = follow_bindings(&constraint.args[0], &empty_bindings, cache);
    
    match &typ {
        Type::Primitive(PrimitiveType::IntegerType(kind)) => {
            // Any integer literal impls Int by default, though assert that none should
            // be Unknown or Inferred at this point in type inference. Any Unknown literal
            // is translated to Inferred in LiteralKind::infer_impl and the type of such
            // a literal is always a TypeVariable rather than remaining an Inferred IntegerType.
            match kind {
                IntegerKind::Unknown => unreachable!(),
                IntegerKind::Inferred(_) => unreachable!(),
                _ => vec![],
            }
        },
        Type::TypeVariable(_) => {
            // If this is an inferred integer literal and we need to assign a type, just choose i32 by default
            unify(&typ, &Type::Primitive(PrimitiveType::IntegerType(IntegerKind::I32)), location, cache);
            vec![]
        },
        _ => vec![make_error!(location, "Expected a primitive integer type, but found {}", typ.display(cache))],
    }
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

/// Search for an impl for the given TraitConstraint or error if 0
/// or >1 matching impls are found.
fn find_impl<'c>(constraint: &TraitConstraint, location: Location<'c>, cache: &mut ModuleCache<'c>) -> Vec<ErrorMessage<'c>> {
    if cache.trait_infos[constraint.trait_id.0].is_member_access() {
        return check_member_access(&constraint, location, cache);
    } else if constraint.trait_id == cache.int_trait {
        return check_int_trait(&constraint, location, cache);
    }

    let scope = cache.impl_scopes[constraint.scope.0].clone();
    let mut found = vec![];
    let mut bindings = HashMap::new();

    for impl_id in scope.iter().copied() {
        let info = &cache.impl_infos[impl_id.0];

        if info.trait_id == constraint.trait_id {
            // TODO: remove excess cloning
            if let Ok(map) = try_unify_all(&constraint.args, &info.typeargs.clone(), location, cache) {
                found.push(impl_id);
                bindings = map;
            }
        }
    }

    if found.len() == 1 {
        // Actually bind the types from the impl.
        // This lets us infer (e.g.) types from fundeps in an impl
        for (id, binding) in bindings.into_iter() {
            cache.type_bindings[id.0] = Bound(binding);
        }

        infer_trait_impl(found[0], cache);

        // Now attach the RequiredImpl to the callsite variable it is used in
        let binding = find_definition_in_impl(constraint.origin, found[0], cache);
        let callsite = constraint.callsite;
        let required_impl = constraint.as_required_impl(binding);

        let callsite_info = &mut cache.trait_bindings[callsite.0];
        callsite_info.required_impls.push(required_impl);
        vec![]
    } else if found.len() > 1 {
        let mut errors = vec![make_error!(location, "{} matching impls found for {}", found.len(), constraint.display(cache))];
        for (i, id) in found.iter().enumerate() {
            let info = &cache.impl_infos[id.0];
            errors.push(make_note!(info.location, "Candidate {} ({})", i + 1, id.0));
        }
        errors
    } else {
        vec![make_error!(location, "No impl found for {}", constraint.display(cache))]
    }
}

/// Go through the given list of traits and determine if they should
/// be propogated upward or if an impl should be searched for now.
/// Returns the list of traits propogated upward.
/// Binds the impls that were searched for and found to the required_impls
/// in the callsite VariableInfo, and errors for any impls that couldn't be found.
fn resolve_traits<'a>(constraints: TraitConstraints, location: Location<'a>, cache: &mut ModuleCache<'a>) -> Vec<RequiredTrait> {
    let mut results = Vec::with_capacity(constraints.len());
    let mut erroring_constraints = vec![];

    for constraint in constraints {
        if should_propagate(&constraint, cache) {
            results.push(constraint.as_required_trait());
        } else {
            let errors = find_impl(&constraint, location, cache);
            if !errors.is_empty() {
                erroring_constraints.push(constraint);
            }
        }
    }

    for constraint in erroring_constraints {
        // Try to find the impl again. If any `Int a` constraints were automatically resolved
        // to `i32` there's a chance an impl can be found now and no errors will be returned.
        for error in find_impl(&constraint, location, cache) {
            println!("{}", error);
        }
    }

    // NOTE: 'duplicate' trait constraints like `given Print a, Print a` are NOT separated out here
    // because they each point to different usages of the trait. They are only filtered out when
    // displaying types to the user.
    results
}


pub trait Inferable<'a> {
    fn infer_impl(&mut self, checker: &mut ModuleCache<'a>) -> (Type, TraitConstraints);
}

/// Compile an entire program, starting from main then lazily compiling
/// each used function as it is called.
pub fn infer_ast<'a>(ast: &mut ast::Ast<'a>, cache: &mut ModuleCache<'a>) {
    CURRENT_LEVEL.store(INITIAL_LEVEL, Ordering::SeqCst);
    let (_, traits) = infer(ast, cache);
    CURRENT_LEVEL.store(INITIAL_LEVEL - 1, Ordering::SeqCst);

    let exposed_traits = resolve_traits(traits, ast.locate(), cache);
    // No traits should be propogated above the top-level main function
    assert!(exposed_traits.is_empty());
}

pub fn infer<'a, T>(ast: &mut T, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints)
    where T: Inferable<'a> + Typed + std::fmt::Display
{
    let (typ, traits) = ast.infer_impl(cache);
    ast.set_type(typ.clone());
    (typ, traits)
}

fn infer_trait_impl<'a>(id: ImplInfoId, cache: &mut ModuleCache<'a>) {
    let info = &mut cache.impl_infos[id.0];
    let trait_impl = trustme::extend_lifetime(info.trait_impl);
    infer(trait_impl, cache);
}

/// Note: each Ast's inference rule is given above the impl if available.
impl<'a> Inferable<'a> for ast::Ast<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        dispatch_on_expr!(self, Inferable::infer_impl, cache)
    }
}

impl<'a> Inferable<'a> for ast::Literal<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        use ast::LiteralKind::*;
        match self.kind {
            Integer(x, kind) => {
                if kind == IntegerKind::Unknown {
                    // Mutate this unknown integer literal to an IntegerKind::Inferred(int_type).
                    // Also add `Int int_type` constraint to restrict this type variable to one
                    // of the native integer types.
                    let int_type = next_type_variable_id(cache);
                    let trait_impl = TraitConstraint::int_constraint(int_type.clone(), cache);
                    self.kind = Integer(x, IntegerKind::Inferred(int_type));
                    (Type::TypeVariable(int_type), vec![trait_impl])
                } else {
                    (Type::Primitive(PrimitiveType::IntegerType(kind)), vec![])
                }
            },
            Float(_) => (Type::Primitive(PrimitiveType::FloatType), vec![]),
            String(_) => (Type::UserDefinedType(STRING_TYPE), vec![]),
            Char(_) => (Type::Primitive(PrimitiveType::CharType), vec![]),
            Bool(_) => (Type::Primitive(PrimitiveType::BooleanType), vec![]),
            Unit => (Type::Primitive(PrimitiveType::UnitType), vec![]),
        }
    }
}

/* Var
 *   x : s ∊ cache
 *   t = instantiate s
 *   -----------
 *   infer cache x = t
 */
impl<'a> Inferable<'a> for ast::Variable<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let info = &cache.definition_infos[self.definition.unwrap().0];

        let impl_scope = self.impl_scope.unwrap();
        let id = self.id.unwrap();
        let trait_binding = self.trait_binding;

        // Lookup the type of the definition.
        // We'll need to recursively infer the type if it is not found
        let (s, traits) = match &info.typ {
            Some(typ) => {
                let constraints = to_trait_constraints(&info.required_traits, impl_scope, id, trait_binding);
                (typ.clone(), constraints)
            },
            None => {
                // If the variable has a definition we can infer from then use that
                // to determine the type, otherwise fill in a type variable for it.
                let (typ, traits) = if info.definition.is_some() {
                    infer_nested_definition(self.definition.unwrap(), impl_scope, id, trait_binding, cache)
                } else {
                    (next_type_variable(cache), vec![])
                };

                let info = &mut cache.definition_infos[self.definition.unwrap().0];
                info.typ = Some(typ.clone());
                (typ, traits)
            },
        };

        instantiate(&s, traits, cache)
    }
}

/* Abs
 *   arg_type1 = newvar ()
 *   arg_type2 = newvar ()
 *   ...
 *   arg_typeN = newvar ()
 *   infer body (x1:arg_type1 x2:arg_type2 ... xN:arg_typeN :: cache) = return_type
 *   -------------
 *   infer (\arg1 arg2 ... argN . body) cache = arg_type1 arg_type2 ... arg_typeN -> return_type
 */
impl<'a> Inferable<'a> for ast::Lambda<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        // The newvars for the parameters are filled out during name resolution
        let arg_types = fmap(&self.args, |_| next_type_variable(cache));

        for (arg, arg_type) in self.args.iter_mut().zip(arg_types.iter()) {
            bind_irrefutable_pattern(arg, arg_type, &vec![], false, cache);
        }

        let (return_type, traits) = infer(self.body.as_mut(), cache);
        (Function(arg_types, Box::new(return_type)), traits)
    }
}

/* App
 *   infer cache function = f
 *   infer cache arg1 = t1
 *   infer cache arg2 = t2
 *   ...
 *   infer cache argN = tN
 *   return_type = newvar ()
 *   unify f (t1 t2 ... tN -> return_type)
 *   ---------------
 *   infer cache (function args) = return_type
 */
impl<'a> Inferable<'a> for ast::FunctionCall<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let (f, mut traits) = infer(self.function.as_mut(), cache);
        let (args, mut arg_traits) = fmap_mut_pair_flatten_second(&mut self.args, |arg| infer(arg, cache));

        let return_type = next_type_variable(cache);
        traits.append(&mut arg_traits);

        unify(&f, &Function(args, Box::new(return_type.clone())), self.location, cache);
        (return_type, traits)
    }
}

/* Let
 *   infer cache expr = t
 *   infer (pattern:(generalize t) :: cache) rest = t'
 *   -----------------
 *   infer cache (let pattern = expr in rest) = t'
 */
impl<'a> Inferable<'a> for ast::Definition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let unit = Type::Primitive(PrimitiveType::UnitType);

        if self.typ.is_some() {
            return (unit, vec![]);
        } else {
            // Without this self.typ wouldn't be set yet while inferring the type of self.expr
            // if this definition is recursive. If this is removed we would recursively infer
            // this definition repeatedly until eventually reaching an error when the previous type
            // is generalized but the new one is not.
            self.typ = Some(unit.clone());
        }

        let level = self.level.unwrap();
        let previous_level = CURRENT_LEVEL.swap(level.0, Ordering::SeqCst);

        // The rhs of a Definition must be inferred at a greater LetBindingLevel than
        // the lhs below. Here we use level for the rhs and level - 1 for the lhs
        let (t, traits) = infer(self.expr.as_mut(), cache);

        CURRENT_LEVEL.store(level.0 - 1, Ordering::SeqCst);

        // Now infer the traits + type of the lhs
        let exposed_traits = resolve_traits(traits, self.location, cache);
        bind_irrefutable_pattern(self.pattern.as_mut(), &t, &exposed_traits, true, cache);

        // And restore the previous LetBindingLevel.
        // TODO: Can these operations on the LetBindingLevel be simplified?
        CURRENT_LEVEL.store(previous_level, Ordering::SeqCst);

        (unit, vec![])
    }
}

impl<'a> Inferable<'a> for ast::If<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let (condition, mut traits) = infer(self.condition.as_mut(), cache);
        let bool_type = Type::Primitive(PrimitiveType::BooleanType);
        unify(&condition, &bool_type, self.condition.locate(), cache);

        let (then, mut then_traits) = infer(self.then.as_mut(), cache);
        traits.append(&mut then_traits);

        if let Some(otherwise) = &mut self.otherwise {
            let (otherwise, mut otherwise_traits) = infer(otherwise.as_mut(), cache);
            traits.append(&mut otherwise_traits);

            unify(&then, &otherwise, self.location, cache);
            (then, traits)
        } else {
            (Type::Primitive(PrimitiveType::UnitType), traits)
        }
    }
}

impl<'a> Inferable<'a> for ast::Match<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let error_count = get_error_count();

        let (expression, mut traits) = infer(self.expression.as_mut(), cache);
        let mut return_type = Type::Primitive(PrimitiveType::UnitType);

        if self.branches.len() >= 1 {
            // Unroll the first iteration of inferring (pattern, branch) types so each
            // subsequent (pattern, branch) types can be unified against the first.
            let (pattern_type, mut pattern_traits) = infer(&mut self.branches[0].0, cache);
            traits.append(&mut pattern_traits);
            unify(&expression, &pattern_type, self.branches[0].0.locate(), cache);

            let (branch, mut branch_traits) = infer(&mut self.branches[0].1, cache);
            return_type = branch;
            traits.append(&mut branch_traits);

            for (pattern, branch) in self.branches.iter_mut().skip(1) {
                let (pattern_type, mut pattern_traits) = infer(pattern, cache);
                let (branch_type, mut branch_traits) = infer(branch, cache);
                unify(&expression, &pattern_type, pattern.locate(), cache);
                unify(&return_type, &branch_type, branch.locate(), cache);
                traits.append(&mut pattern_traits);
                traits.append(&mut branch_traits);
            }
        }

        // Compiling the decision tree for this pattern requires each pattern is well-typed.
        // So skip this step if there was an error in inferring types for this match expression.
        if get_error_count() == error_count {
            let mut tree = pattern::compile(self, cache);
            // TODO: Infer new variables created by a decision tree within pattern::compile.
            //       It is done separately currently only for convenience/ease of implementation.
            tree.infer(self.expression.get_type().unwrap(), self.location, cache);
            self.decision_tree = Some(tree);
        }

        (return_type, traits)
    }
}

impl<'a> Inferable<'a> for ast::TypeDefinition<'a> {
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TypeAnnotation<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let (typ, traits) = infer(self.lhs.as_mut(), cache);
        unify(&typ, self.typ.as_mut().unwrap(), self.location, cache);
        (typ, traits)
    }
}

impl<'a> Inferable<'a> for ast::Import<'a> {
    /// Type checker doesn't need to follow imports.
    /// It typechecks definitions as-needed when it finds a variable whose type is still unknown.
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TraitDefinition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let previous_level = CURRENT_LEVEL.swap(self.level.unwrap().0, Ordering::SeqCst);
        
        for declaration in self.declarations.iter_mut() {
            let rhs = declaration.typ.as_ref().unwrap();

            bind_irrefutable_pattern(declaration.lhs.as_mut(), rhs, &vec![], true, cache);
        }

        CURRENT_LEVEL.store(previous_level, Ordering::SeqCst);
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TraitImpl<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let trait_info = &cache.trait_infos[self.trait_info.unwrap().0];

        let mut typevars_to_replace = trait_info.typeargs.clone();
        typevars_to_replace.append(&mut trait_info.fundeps.clone());

        let typevar_bindings = fmap(&typevars_to_replace, |_| next_type_variable_id(cache));

        // Bind each impl type argument to the corresponding trait type variable
        for (type_variable, binding) in typevar_bindings.iter().copied().zip(self.trait_arg_types.iter()) {
            // These bindings are all new type variables so this unification should never fail
            unify(&TypeVariable(type_variable), binding, self.location, cache);
        }

        // Instantiate the typevars in the parent trait to bind their definition
        // types against the types in this trait impl. This needs to be done once
        // at the trait level rather than at each definition so that each definition
        // refers to the same type variable instances/bindings.
        //
        // This is because only these bindings in trait_to_impl are unified against
        // the types declared in self.typeargs
        let mut trait_to_impl = HashMap::new();
        for (trait_type_variable, impl_type_variable) in typevars_to_replace.into_iter().zip(typevar_bindings) {
            trait_to_impl.insert(trait_type_variable, impl_type_variable);
        }

        for definition in self.definitions.iter_mut() {
            bind_irrefutable_pattern_in_impl(definition.pattern.as_ref(), self.trait_info.unwrap(), &trait_to_impl, cache);

            // TODO: Ensure no traits are propogated up that aren't required by the impl
            infer(definition, cache);
        }

        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::Return<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let traits = infer(self.expression.as_mut(), cache).1;
        (next_type_variable(cache), traits)
    }
}

impl<'a> Inferable<'a> for ast::Sequence<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let ignore_len = self.statements.len() - 1;
        let mut traits = vec![];

        for statement in self.statements.iter_mut().take(ignore_len) {
            let (_, mut statement_traits) = infer(statement, cache);
            traits.append(&mut statement_traits);
        }

        let (last_statement_type, mut statement_traits) = infer(self.statements.last_mut().unwrap(), cache);
        traits.append(&mut statement_traits);
        (last_statement_type, traits)
    }
}

impl<'a> Inferable<'a> for ast::Extern<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let previous_level = CURRENT_LEVEL.swap(self.level.unwrap().0, Ordering::SeqCst);
        for declaration in self.declarations.iter_mut() {
            bind_irrefutable_pattern(declaration.lhs.as_mut(), declaration.typ.as_ref().unwrap(), &vec![], true, cache);
        }
        CURRENT_LEVEL.store(previous_level, Ordering::SeqCst);
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::MemberAccess<'a> {
    /// Member access (e.g. foo.bar) in ante implies a corresponding trait constraint
    /// that is automatically implemented by the compiler. This is to allow multiple
    /// conflicting field names in a scope. For example a function:
    ///
    /// foo bar =
    ///    bar.x + 2
    /// 
    /// Has the type
    ///
    /// bar : a -> int
    ///   given .x a int
    ///
    /// This given trait constraint is a member access constraint denoting that
    /// type a must have a field x of type int.
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let (collection_type, mut traits) = infer(self.lhs.as_mut(), cache);

        let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
        let trait_id = cache.get_member_access_trait(&self.field, level);

        let field_type = cache.next_type_variable(level);

        let typeargs = vec![collection_type, field_type.clone()];
        let trait_impl = TraitConstraint::member_access_constraint(trait_id, typeargs);
        traits.push(trait_impl);

        (field_type, traits)
    }
}

impl<'a> Inferable<'a> for ast::Tuple<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let mut elements = vec![];
        let mut traits = vec![];

        for element in self.elements.iter_mut() {
            let (element_type, mut element_traits) = infer(element, cache);
            elements.push(element_type);
            traits.append(&mut element_traits);
        }

        (Tuple(elements), traits)
    }
}

impl<'a> Inferable<'a> for ast::Assignment<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitConstraints) {
        let mut traits = infer(self.lhs.as_mut(), cache).1;
        traits.append(&mut infer(self.rhs.as_mut(), cache).1);
        (Type::Primitive(PrimitiveType::UnitType), traits)
    }
}
