use crate::cache::{ ModuleCache, TraitInfoId, DefinitionInfoId, ImplBindingId, DefinitionNode, ImplInfoId };
use crate::parser::ast;
use crate::types::{ Type, Type::*, TypeVariableId, PrimitiveType, LetBindingLevel, TypeBinding::* };
use crate::types::{ TypeBinding, STRING_TYPE };
use crate::types::typed::Typed;
use crate::types::traits::{ TraitList, Impl };
use crate::util::*;
use crate::error::location::{ Location, Locatable };
use crate::error::ErrorMessage;

use std::collections::HashMap;
use std::sync::atomic::{ AtomicUsize, Ordering };

// Note: most of this file is directly translated from:
// https://github.com/jfecher/algorithm-j


pub static CURRENT_LEVEL: AtomicUsize = AtomicUsize::new(1);

/// A sparse set of type bindings, used by try_unify
pub type TypeBindings = HashMap<TypeVariableId, Type>;

/// Replace any typevars found in typevars_to_replace with the
/// associated value in the same table, leave them otherwise
fn replace_typevars<'b>(typ: &Type, typevars_to_replace: &HashMap<TypeVariableId, TypeVariableId>, cache: &mut ModuleCache<'b>) -> Type {
    match typ {
        Primitive(p) => Primitive(*p),
        TypeVariable(id) => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                replace_typevars(&typ.clone(), typevars_to_replace, cache)
            } else {
                let new_id = typevars_to_replace.get(id).unwrap_or(id);
                TypeVariable(*new_id)
            }
        },
        Function(parameters, return_type) => {
            let parameters = fmap(parameters, |parameter| replace_typevars(parameter, typevars_to_replace, cache));
            let return_type = replace_typevars(return_type, typevars_to_replace, cache);
            Function(parameters, Box::new(return_type))
        },
        ForAll(_typevars, _typ) => {
            unreachable!("Ante does not support higher rank polymorphism");
            // let mut table_copy = typevars_to_replace.clone();
            // for typevar in typevars.iter() {
            //     table_copy.remove(typevar);
            // }
            // ForAll(typevars.clone(), Box::new(replace_typevars(typ, &table_copy, cache)))
        }
        UserDefinedType(id) => UserDefinedType(*id),

        TypeApplication(typ, args) => {
            let typ = replace_typevars(typ, typevars_to_replace, cache);
            let args = fmap(args, |arg| replace_typevars(arg, typevars_to_replace, cache));
            TypeApplication(Box::new(typ), args)
        },
        Tuple(elements) => {
            Tuple(fmap(elements, |element| replace_typevars(element, typevars_to_replace, cache)))
        }
    }
}

/// Helper function for getting the next type variable at the current level
fn next_type_variable_id<'a>(cache: &mut ModuleCache<'a>) -> TypeVariableId {
    let level = LetBindingLevel(CURRENT_LEVEL.fetch_or(0, Ordering::SeqCst));
    cache.next_type_variable_id(level)
}

fn next_type_variable<'a>(cache: &mut ModuleCache<'a>) -> Type {
    let level = LetBindingLevel(CURRENT_LEVEL.fetch_or(0, Ordering::SeqCst));
    cache.next_type_variable(level)
}

fn collect_impl_bindings(impls: &Vec<Impl>) -> Vec<ImplBindingId> {
    fmap(impls, |trait_impl| trait_impl.binding)
}

/// specializes the polytype s by copying the term and replacing the
/// bound type variables consistently by new monotype variables
/// E.g.   instantiate (forall a b. a -> b -> a) = c -> d -> c
fn instantiate<'b>(s: &Type, mut traits: Vec<Impl>, cache: &mut ModuleCache<'b>) -> (Type, TraitList, Vec<ImplBindingId>) {
    // Note that the returned type is no longer a PolyType,
    // this means it is now monomorphic and not forall-quantified
    match s {
        TypeVariable(id) => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                instantiate(&typ.clone(), traits, cache)
            } else {
                let bindings = collect_impl_bindings(&traits);
                (TypeVariable(*id), traits, bindings)
            }
        },
        ForAll(typevars, typ) => {
            let mut typevars_to_replace = HashMap::new();
            for var in typevars.iter().copied() {
                typevars_to_replace.insert(var, next_type_variable_id(cache));
            }
            let typ = replace_typevars(&typ, &typevars_to_replace, cache);

            for var in find_all_typevars_in_traits(&traits, true, cache).iter().copied() {
                if !typevars_to_replace.contains_key(&var) {
                    typevars_to_replace.insert(var, next_type_variable_id(cache));
                }
            }

            let mut bindings = vec![];
            for trait_impl in traits.iter_mut() {
                for typ in trait_impl.args.iter_mut() {
                    *typ = replace_typevars(typ, &typevars_to_replace, cache);
                }

                let binding = cache.push_impl_binding();
                trait_impl.binding = binding;
                bindings.push(binding);
            }
            (typ, traits, bindings)
        },
        other => {
            let bindings = collect_impl_bindings(&traits);
            (other.clone(), traits, bindings)
        },
    }
}

/// Similar to instantiate but uses an explicitly passed map to map
/// the old type variables to. This version is used during trait impl
/// type inference to ensure all definitions in the trait impl are
/// mapped to the same typevars, rather than each definition instantiated
/// separately as is normal.
fn instantiate_from_map<'b>(s: &Type, typevars_to_replace: &HashMap<TypeVariableId, TypeVariableId>, cache: &mut ModuleCache<'b>) -> Type {
    // Note that the returned type is no longer a PolyType,
    // this means it is now monomorphic and not forall-quantified
    match s {
        TypeVariable(id) => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                instantiate(&typ.clone(), vec![], cache).0
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
fn try_unify_all<'b>(vec1: &Vec<Type>, vec2: &Vec<Type>, location: Location<'b>, cache: &mut ModuleCache<'b>) -> Result<TypeBindings, ()> {
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

fn unify<'b>(t1: &Type, t2: &Type, location: Location<'b>, cache: &mut ModuleCache<'b>) {
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

/// Collects all the type variables contained within typ into a Vec.
/// If monomorphic_only is true, any polymorphic type variables will be filtered out.
pub fn find_all_typevars<'a>(typ: &Type, monomorphic_only: bool, cache: &ModuleCache<'a>) -> Vec<TypeVariableId> {
    match typ {
        Primitive(_) => vec![],
        UserDefinedType(_) => vec![],
        TypeVariable(id) => {
            match &cache.type_bindings[id.0] {
                Bound(t) => find_all_typevars(t, monomorphic_only, cache),
                Unbound(level, _) => {
                    if level.0 >= CURRENT_LEVEL.fetch_or(1, Ordering::SeqCst) || !monomorphic_only {
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
                type_variables.append(&mut find_all_typevars(&parameter, monomorphic_only, cache));
            }
            type_variables.append(&mut find_all_typevars(return_type, monomorphic_only, cache));
            type_variables
        },
        TypeApplication(constructor, args) => {
            let mut type_variables = find_all_typevars(constructor, monomorphic_only, cache);
            for arg in args {
                type_variables.append(&mut find_all_typevars(&arg, monomorphic_only, cache));
            }
            type_variables
        },
        Tuple(elements) => {
            elements.iter().flat_map(|element| find_all_typevars(element, monomorphic_only, cache)).collect()
        },
        ForAll(polymorphic_typevars, typ) => {
            if !monomorphic_only {
                let mut typevars = polymorphic_typevars.clone();
                typevars.append(&mut find_all_typevars(typ, true, cache));
                typevars
            } else {
                // Remove all of tvs from find_all_typevars typ, this could be faster
                let mut monomorphic_typevars = find_all_typevars(typ, monomorphic_only, cache);
                monomorphic_typevars.retain(|typevar| !contains(polymorphic_typevars, typevar));
                monomorphic_typevars
            }
        },
    }
}

fn find_all_typevars_in_traits<'a>(traits: &Vec<Impl>, monomorphic_only: bool, cache: &ModuleCache<'a>) -> Vec<TypeVariableId> {
    let mut typevars = vec![];
    for trait_impl in traits.iter() {
        for typ in trait_impl.args.iter() {
            typevars.append(&mut find_all_typevars(typ, monomorphic_only, cache));
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

fn infer_nested_definition<'a>(definition_id: DefinitionInfoId, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
    let level = LetBindingLevel(CURRENT_LEVEL.fetch_or(0, Ordering::SeqCst));
    let typevar = cache.next_type_variable(level);
    let info = &mut cache.definition_infos[definition_id.0];
    let definition = info.definition.as_mut().unwrap();
    // Mark the definition with a fresh typevar for recursive references
    info.typ = Some(typevar.clone());

    match definition {
        DefinitionNode::Definition(definition) => {
            let definition = trustme::extend_lifetime(*definition);
            infer(definition, cache);
        },
        DefinitionNode::TraitDefinition(definition) => {
            let definition = trustme::extend_lifetime(*definition);
            infer(definition, cache);
        },
        DefinitionNode::Extern(declaration) => {
            let definition = trustme::extend_lifetime(*declaration);
            infer(definition, cache);
        },
        DefinitionNode::Impl => unreachable!("DefinitionNode::Impl shouldn't be reachable when inferring nested definitions. Only the TraitDefinition should be visible."),
        DefinitionNode::Parameter => {},
        DefinitionNode::TypeConstructor { .. } => {},
    };

    let info = &mut cache.definition_infos[definition_id.0];
    (info.typ.clone().unwrap(), info.required_impls.clone())
}

/// Binds a given type to an irrefutable pattern, recursing on the pattern and verifying
/// that it is indeed irrefutable. If should_generalize is true, this generalizes the type given
/// to any variable encountered.
fn bind_irrefutable_pattern<'a>(ast: &mut ast::Ast<'a>, typ: &Type, traits: &Vec<Impl>, should_generalize: bool, cache: &mut ModuleCache<'a>) {
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
            let info = &mut cache.definition_infos[variable.definition.unwrap().0];

            // The type may already be set (e.g. from a trait impl this definition belongs to).
            // If it is, unify the existing type and new type before generalizing them.
            if let Some(existing_type) = &info.typ {
                unify(&existing_type.clone(), &typ, variable.location, cache);
            }

            let typ = if should_generalize { generalize(typ, cache) } else { typ.clone() };
            let info = &mut cache.definition_infos[variable.definition.unwrap().0];
            info.required_impls.append(&mut traits.clone());
            variable.typ = Some(typ.clone());
            info.typ = Some(typ);
        },
        TypeAnnotation(annotation) => {
            unify(typ, annotation.typ.as_ref().unwrap(), annotation.location, cache);
            bind_irrefutable_pattern(annotation.lhs.as_mut(), typ, traits, should_generalize, cache);
        },
        Tuple(tuple) => {
            let tuple_type = Type::Tuple(fmap(&tuple.elements, |_| next_type_variable(cache)));
            unify(&typ, &tuple_type, tuple.location, cache);

            match tuple_type {
                Type::Tuple(elements) => {
                    for (element, element_type) in tuple.elements.iter_mut().zip(elements) {
                        bind_irrefutable_pattern(element, &element_type, traits, should_generalize, cache);
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

            let info = &mut cache.definition_infos[variable.definition.unwrap().0];
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

// A trait should be propogated to the public signature of a Definition if any of its contained
// type variables should be generalized. If the trait shouldn't be propogated then an impl
// should be resolved instead.
fn should_propagate<'a>(trait_impl: &Impl, cache: &ModuleCache<'a>) -> bool {
    // Don't check the fundeps since only the typeargs proper are used to find impls
    let arg_count = cache.trait_infos[trait_impl.trait_id.0].typeargs.len();
    trait_impl.args.iter().take(arg_count).any(|arg| !find_all_typevars(arg, true, cache).is_empty())
}

fn check_member_access<'a>(trait_impl: &Impl, location: Location<'a>, cache: &mut ModuleCache<'a>) {
    let empty_bindings = HashMap::new();
    let collection = follow_bindings(&trait_impl.args[0], &empty_bindings, cache);

    let field_name = &cache.trait_infos[trait_impl.trait_id.0].name[1..];

    match collection {
        Type::UserDefinedType(id) => {
            let field_type = cache.type_infos[id.0].find_field(field_name)
                .map(|(_, field)| field.field_type.clone());

            match field_type {
                Some(field_type) => {
                    // FIXME: this unifies the type variables from the definition of field_type
                    // rather than the types it was instantiated to. This will be incorrect if
                    // the user ever uses a generic field with two different types!
                    unify(&trait_impl.args[1], &field_type, location, cache);
                },
                _ => error!(location, "Type {} has no field named {}", collection.display(cache), field_name),
            }

        },
        _ => error!(location, "Type {} is not a struct type and has no field named {}", collection.display(cache), field_name),
    }
}

fn find_impl<'a>(trait_impl: &mut Impl, location: Location<'a>, cache: &mut ModuleCache<'a>) {
    if cache.trait_infos[trait_impl.trait_id.0].is_member_access() {
        check_member_access(trait_impl, location, cache);
        return;
    }

    let scope = cache.impl_scopes[trait_impl.scope.0].clone();
    let mut found = vec![];
    let mut bindings = HashMap::new();

    for impl_id in scope.iter().copied() {
        let info = &cache.impl_infos[impl_id.0];

        if info.trait_id == trait_impl.trait_id {
            // TODO: remove excess cloning
            if let Ok(map) = try_unify_all(&trait_impl.args, &info.typeargs.clone(), location, cache) {
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
        let binding = &mut cache.impl_bindings[trait_impl.binding.0];
        // TODO: the 'binding == Some(found[0])' clause is likely indicative of another bug
        //       since ImplBindings should be unique
        assert!(binding.is_none() || *binding == Some(found[0]), "Binding {} for impl {} is not none", trait_impl.binding.0, trait_impl.debug(cache));
        *binding = Some(found[0]);
    } else if found.len() > 1 {
        error!(location, "{} matching impls found for {}", found.len(), trait_impl.display(cache));
        for (i, id) in found.iter().enumerate() {
            let info = &cache.impl_infos[id.0];
            note!(info.location, "Candidate {} ({})", i + 1, id.0);
        }
    } else {
        error!(location, "No impl found for {}", trait_impl.display(cache));
    }
}

/// Go through the given list of traits and determine if they should
/// be propogated upward or if an impl should be searched for now.
/// Returns the list of traits propogated upward.
fn resolve_traits<'a>(traits: Vec<Impl>, location: Location<'a>, cache: &mut ModuleCache<'a>) -> Vec<Impl> {
    let mut results = Vec::with_capacity(traits.len());
    for mut trait_impl in traits {
        if should_propagate(&trait_impl, cache) {
            results.push(trait_impl);
        } else {
            find_impl(&mut trait_impl, location, cache);
        }
    }
    results
}


pub trait Inferable<'a> {
    fn infer_impl(&mut self, checker: &mut ModuleCache<'a>) -> (Type, TraitList);
}

pub fn infer_ast<'a>(ast: &mut ast::Ast<'a>, cache: &mut ModuleCache<'a>) {
    let (_, traits) = infer(ast, cache);
    let exposed_traits = resolve_traits(traits, ast.locate(), cache);
    for exposed in exposed_traits {
        error!(ast.locate(), "Trait {} has not been resolved", exposed.display(cache));
    }
}

pub fn infer<'a, T>(ast: &mut T, cache: &mut ModuleCache<'a>) -> (Type, TraitList)
    where T: Inferable<'a> + Typed
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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        dispatch_on_expr!(self, Inferable::infer_impl, cache)
    }
}

impl<'a> Inferable<'a> for ast::Literal<'a> {
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        use ast::LiteralKind::*;
        match self.kind {
            Integer(_) => (Type::Primitive(PrimitiveType::IntegerType), vec![]),
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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let info = &cache.definition_infos[self.definition.unwrap().0];

        // Lookup the type of the definition.
        // We'll need to recursively infer the type if it is not found
        let (s, traits) = match &info.typ {
            Some(typ) => (typ.clone(), info.required_impls.clone()),
            None => {
                // If the variable has a definition we can infer from then use that
                // to determine the type, otherwise fill in a type variable for it.
                let (typ, traits) = if info.definition.is_some() {
                    infer_nested_definition(self.definition.unwrap(), cache)
                } else {
                    (next_type_variable(cache), vec![])
                };
                let info = &mut cache.definition_infos[self.definition.unwrap().0];
                info.typ = Some(typ.clone());
                (typ, traits)
            },
        };

        let (t, mut traits, impl_bindings) = instantiate(&s, traits, cache);
        for trait_impl in traits.iter_mut() {
            trait_impl.scope = self.impl_scope.unwrap();
        }

        self.impl_bindings = impl_bindings;
        (t, traits)
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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (f, mut traits) = infer(self.function.as_mut(), cache);
        let (args, mut arg_traits) = fmap_mut_pair_merge_second(&mut self.args, |arg| infer(arg, cache));

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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
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

        let previous_level = CURRENT_LEVEL.fetch_or(1, Ordering::SeqCst);

        CURRENT_LEVEL.swap(self.level.unwrap().0, Ordering::SeqCst);
        let (t, traits) = infer(self.expr.as_mut(), cache);
        CURRENT_LEVEL.swap(previous_level, Ordering::SeqCst);

        let exposed_traits = resolve_traits(traits, self.location, cache);
        bind_irrefutable_pattern(self.pattern.as_mut(), &t, &exposed_traits, true, cache);

        (unit, vec![])
    }
}

impl<'a> Inferable<'a> for ast::If<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (expression, mut traits) = infer(self.expression.as_mut(), cache);
        let mut return_type = Type::Primitive(PrimitiveType::UnitType);

        if self.branches.len() >= 1 {
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
        (return_type, traits)
    }
}

impl<'a> Inferable<'a> for ast::TypeDefinition<'a> {
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TypeAnnotation<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (typ, traits) = infer(self.lhs.as_mut(), cache);
        unify(&typ, self.typ.as_mut().unwrap(), self.location, cache);
        (typ, traits)
    }
}

impl<'a> Inferable<'a> for ast::Import<'a> {
    /// Type checker doesn't need to follow imports.
    /// It typechecks definitions as-needed when it finds a variable whose type is still unknown.
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TraitDefinition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        for declaration in self.declarations.iter_mut() {
            let rhs = declaration.typ.as_ref().unwrap();

            bind_irrefutable_pattern(declaration.lhs.as_mut(), rhs, &vec![], true, cache);
        }
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TraitImpl<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        infer(self.expression.as_mut(), cache)
    }
}

impl<'a> Inferable<'a> for ast::Sequence<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
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
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        for declaration in self.declarations.iter_mut() {
            bind_irrefutable_pattern(declaration.lhs.as_mut(), declaration.typ.as_ref().unwrap(), &vec![], true, cache);
        }
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::MemberAccess<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (collection_type, mut traits) = infer(self.lhs.as_mut(), cache);

        let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
        let trait_id = cache.get_member_access_trait(&self.field, level);

        let field_type = cache.next_type_variable(level);

        use crate::cache::ImplScopeId;
        let trait_impl = Impl::new(trait_id, ImplScopeId(0), ImplBindingId(0), vec![collection_type, field_type.clone()]);
        traits.push(trait_impl);

        (field_type, traits)
    }
}

impl<'a> Inferable<'a> for ast::Tuple<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
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
