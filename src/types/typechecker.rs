//! typechecker.rs - Defines the type inference pass used by the compiler.
//! This pass comes after name resolution and is followed by the lifetime inference.
//!
//! This pass traverses over the ast, filling out the (typ: Option<Type>) field of each node.
//! When this pass is finished, all such fields are guarenteed to be filled out. The formatting
//! of this file begins with helper functions for type inference at the type, and ends with
//! the actual AST pass defined in the `Inferable` trait. Note that this AST pass starts
//! in the first module, and whenever it finds a variable using a definition that hasn't yet
//! been typechecked, it delves into that definition to typecheck it. This means any variables
//! that are unused are not typechecked by default.
//!
//! This uses algorithm j extended with let polymorphism and multi-parameter
//! typeclasses (traits) with a very limited form of functional dependencies.
//! For generalization this uses let binding levels to determine if types escape
//! the current binding and should thus not be generalized.
//!
//! Most of this file is translated from: https://github.com/jfecher/algorithm-j
//! That repository may be a good starting place for those new to type inference.
//! For those already familiar with type inference or more interested in ante's
//! internals, the reccomended starting place while reading this file is the
//! `Inferable` trait and its impls for each node. From there, you can see what
//! type inference does for each node type and inspect any helpers that are used.
//!
//! Note that as a result of type inference, the following Optional fields in the
//! Ast will be filled out:
//! - `typ: Option<Type>` for all nodes,
//! - `trait_binding: Option<TraitBindingId>` for `ast::Variable`s,
//! - `decision_tree: Option<DecisionTree>` for `ast::Match`s
use crate::cache::{DefinitionInfoId, DefinitionKind, EffectInfoId, ModuleCache, TraitInfoId};
use crate::cache::{ImplScopeId, VariableId};
use crate::error::location::{Locatable, Location};
use crate::error::{get_error_count, ErrorMessage};
use crate::lexer::token::IntegerKind;
use crate::parser::ast::{self, ClosureEnvironment};
use crate::types::traits::{RequiredTrait, TraitConstraint, TraitConstraints};
use crate::types::typed::Typed;
use crate::types::EffectSet;
use crate::types::{
    pattern, traitchecker, FunctionType, LetBindingLevel, PrimitiveType, Type, Type::*, TypeBinding, TypeBinding::*,
    TypeInfo, TypeVariableId, INITIAL_LEVEL, PAIR_TYPE, STRING_TYPE,
};
use crate::util::*;

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::mutual_recursion::{definition_is_mutually_recursive, try_generalize_definition};
use super::traits::{Callsite, ConstraintSignature, TraitConstraintId};
use super::{error, GeneralizedType, TypeInfoBody};

/// The current LetBindingLevel we are at.
/// This increases by 1 whenever we enter the rhs of a `ast::Definition` and decreases
/// by 1 whenever we exit this rhs. This helps keep track of which scope type variables
/// arose from and whether they should be generalized or not. See
/// http://okmij.org/ftp/ML/generalization.html for more information on let binding levels.
pub static CURRENT_LEVEL: AtomicUsize = AtomicUsize::new(INITIAL_LEVEL);

/// A sparse set of type bindings, used by try_unify
pub type TypeBindings = HashMap<TypeVariableId, Type>;

/// The result of `try_unify`: either a set of type bindings to perform,
/// or an error message of which types failed to unify.
pub type UnificationResult<'c> = Result<UnificationBindings, ErrorMessage<'c>>;

type LevelBindings = Vec<(TypeVariableId, LetBindingLevel)>;

/// Arbitrary limit of maximum recursive calls to functions like find_binding.
/// Expected not to happen but leads to better errors than a stack overflow when it does.
const RECURSION_LIMIT: u32 = 15;

#[derive(Debug, Clone)]
pub struct UnificationBindings {
    pub bindings: TypeBindings,
    level_bindings: LevelBindings,
}

impl UnificationBindings {
    pub fn empty() -> UnificationBindings {
        UnificationBindings { bindings: HashMap::new(), level_bindings: vec![] }
    }

    pub fn perform(self, cache: &mut ModuleCache) {
        perform_type_bindings(self.bindings, cache);

        for (id, level) in self.level_bindings {
            match &cache.type_bindings[id.0] {
                Bound(_) => (), // The binding changed from under us. Is this an issue?
                Unbound(original_level, kind) => {
                    let min_level = std::cmp::min(level, *original_level);
                    cache.type_bindings[id.0] = Unbound(min_level, kind.clone());
                },
            }
        }
    }

    pub fn extend(&mut self, mut other: UnificationBindings) {
        self.bindings.extend(other.bindings);
        self.level_bindings.append(&mut other.level_bindings);
    }
}

pub struct TypeResult {
    typ: Type,
    traits: TraitConstraints,
    effects: EffectSet,
}

impl TypeResult {
    fn new(typ: Type, traits: TraitConstraints, cache: &mut ModuleCache) -> TypeResult {
        Self { typ, traits, effects: EffectSet::any(cache) }
    }

    fn of(typ: Type, cache: &mut ModuleCache) -> TypeResult {
        Self { typ, traits: vec![], effects: EffectSet::any(cache) }
    }

    fn with_type(mut self, typ: Type) -> TypeResult {
        self.typ = typ;
        self
    }

    fn combine(&mut self, other: &mut Self, cache: &mut ModuleCache) {
        self.traits.append(&mut other.traits);
        self.effects = self.effects.combine(&other.effects, cache);
    }
}

/// Convert a TypeApplication(UserDefinedType(id), args) into the set of TypeBindings
/// so that each mapping in the bindings is in the form `var -> arg` where each variable
/// was one of the variables given in the definition of the user-defined-type:
/// `type Foo var1 var2 ... varN = ...` and each `arg` corresponds to the generic argument
/// of the type somewhere in the program, e.g: `foo : Foo arg1 arg2 ... argN`
pub fn type_application_bindings<'c>(info: &TypeInfo<'c>, typeargs: &[Type], cache: &ModuleCache) -> TypeBindings {
    info.args
        .iter()
        .copied()
        .zip(typeargs.iter().cloned())
        .filter_map(|(a, b)| {
            let b = follow_bindings_in_cache(&b, cache);
            if TypeVariable(a) != b {
                Some((a, b))
            } else {
                None
            }
        })
        .collect()
}

/// Given `a` returns `ref a`
fn ref_of(typ: Type, cache: &mut ModuleCache) -> Type {
    let new_var = next_type_variable_id(cache);
    let constructor = Box::new(Type::Ref(new_var));
    TypeApplication(constructor, vec![typ])
}

/// Replace any typevars found in typevars_to_replace with the
/// associated value in the same table, leave them otherwise
fn replace_typevars<'c>(
    typ: &Type, typevars_to_replace: &HashMap<TypeVariableId, TypeVariableId>, cache: &ModuleCache<'c>,
) -> Type {
    let typevars_to_replace = typevars_to_replace.iter().map(|(key, id)| (*key, TypeVariable(*id))).collect();

    bind_typevars(typ, &typevars_to_replace, cache)
}

/// Return a new type with all typevars found in the given type
/// replaced with fresh ones, along with the type bindings used.
///
/// Note that unlike `generalize(typ).instantiate(..)`, this will
/// replace all type variables rather than only type variables
/// that have not originated from an outer scope.
pub fn replace_all_typevars<'c>(types: &[Type], cache: &mut ModuleCache<'c>) -> (Vec<Type>, TypeBindings) {
    let mut bindings = HashMap::new();
    let types = fmap(types, |typ| replace_all_typevars_with_bindings(typ, &mut bindings, cache));
    (types, bindings)
}

/// Replace all type variables in the given type, using new_bindings
/// to lookup what each variable should be bound to, inserting a
/// fresh type variable into new_bindings if that type variable was not present.
pub fn replace_all_typevars_with_bindings<'c>(
    typ: &Type, new_bindings: &mut TypeBindings, cache: &mut ModuleCache<'c>,
) -> Type {
    match typ {
        Primitive(p) => Primitive(*p),

        TypeVariable(id) => replace_typevar_with_binding(*id, new_bindings, TypeVariable, cache),

        Function(function) => {
            let parameters = fmap(&function.parameters, |parameter| {
                replace_all_typevars_with_bindings(parameter, new_bindings, cache)
            });
            let return_type = Box::new(replace_all_typevars_with_bindings(&function.return_type, new_bindings, cache));
            let environment = Box::new(replace_all_typevars_with_bindings(&function.environment, new_bindings, cache));
            let is_varargs = function.is_varargs;
            let effects = function.effects.replace_all_typevars_with_bindings(new_bindings, cache);
            Function(FunctionType { parameters, return_type, environment, is_varargs, effects })
        },
        UserDefined(id) => UserDefined(*id),

        // We must recurse on the lifetime variable since they are unified as normal type variables
        Ref(lifetime) => match replace_typevar_with_binding(*lifetime, new_bindings, Ref, cache) {
            TypeVariable(new_lifetime) => Ref(new_lifetime),
            Ref(new_lifetime) => Ref(new_lifetime),
            _ => unreachable!("Bound Ref lifetime to non-lifetime type"),
        },

        TypeApplication(typ, args) => {
            let typ = replace_all_typevars_with_bindings(typ, new_bindings, cache);
            let args = fmap(args, |arg| replace_all_typevars_with_bindings(arg, new_bindings, cache));
            TypeApplication(Box::new(typ), args)
        },
        Struct(fields, id) => {
            if let Some(binding) = new_bindings.get(id) {
                binding.clone()
            } else {
                let fields = fields
                    .iter()
                    .map(|(name, typ)| {
                        let typ = replace_all_typevars_with_bindings(typ, new_bindings, cache);
                        (name.clone(), typ)
                    })
                    .collect();

                Struct(fields, *id)
            }
        },
        Effects(effects) => {
            Effects(effects.replace_all_typevars_with_bindings(new_bindings, cache))
        }
    }
}

/// If the given TypeVariableId is unbound then return the matching binding in new_bindings.
/// If there is no binding found, instantiate a new type variable and use that.
///
/// `default` should be either TypeVariable or Ref and controls which kind of type gets
/// created that wraps the newly-instantiated TypeVariableId if one is made.
fn replace_typevar_with_binding<'c>(
    id: TypeVariableId, new_bindings: &mut TypeBindings, default: fn(TypeVariableId) -> Type,
    cache: &mut ModuleCache<'c>,
) -> Type {
    if let Bound(typ) = &cache.type_bindings[id.0] {
        replace_all_typevars_with_bindings(&typ.clone(), new_bindings, cache)
    } else if let Some(var) = new_bindings.get(&id) {
        var.clone()
    } else {
        let new_typevar = next_type_variable_id(cache);
        new_bindings.insert(id, default(new_typevar));
        default(new_typevar)
    }
}

/// Replace any typevars found with the given type bindings
///
/// Compared to `replace_all_typevars_with_bindings`, this function does not instantiate
/// unbound type variables that were not in type_bindings. Thus if type_bindings is empty,
/// this function will just clone the original Type.
pub fn bind_typevars<'c>(typ: &Type, type_bindings: &TypeBindings, cache: &ModuleCache<'c>) -> Type {
    match typ {
        Primitive(p) => Primitive(*p),

        TypeVariable(id) => bind_typevar(*id, type_bindings, TypeVariable, cache),

        Function(function) => {
            let parameters = fmap(&function.parameters, |parameter| bind_typevars(parameter, type_bindings, cache));
            let return_type = Box::new(bind_typevars(&function.return_type, type_bindings, cache));
            let environment = Box::new(bind_typevars(&function.environment, type_bindings, cache));
            let is_varargs = function.is_varargs;
            let effects = function.effects.bind_typevars(type_bindings, cache);
            Function(FunctionType { parameters, return_type, environment, is_varargs, effects })
        },
        UserDefined(id) => UserDefined(*id),

        Ref(lifetime) => match bind_typevar(*lifetime, type_bindings, Ref, cache) {
            TypeVariable(new_lifetime) => Ref(new_lifetime),
            Ref(new_lifetime) => Ref(new_lifetime),
            _ => unreachable!("Bound Ref lifetime to non-lifetime type"),
        },

        TypeApplication(typ, args) => {
            let typ = bind_typevars(typ, type_bindings, cache);
            let args = fmap(args, |arg| bind_typevars(arg, type_bindings, cache));
            TypeApplication(Box::new(typ), args)
        },
        Struct(fields, id) => {
            match type_bindings.get(&id) {
                Some(TypeVariable(binding_id)) => {
                    let fields = fields
                        .iter()
                        .map(|(name, field)| (name.clone(), bind_typevars(field, type_bindings, cache)))
                        .collect();
                    Struct(fields, *binding_id)
                },
                // TODO: Should we follow all typevars here?
                Some(binding) => binding.clone(),
                None => {
                    if let Bound(typ) = &cache.type_bindings[id.0] {
                        bind_typevars(&typ.clone(), type_bindings, cache)
                    } else {
                        let fields = fields
                            .iter()
                            .map(|(name, typ)| {
                                let typ = bind_typevars(typ, type_bindings, cache);
                                (name.clone(), typ)
                            })
                            .collect();

                        Struct(fields, *id)
                    }
                },
            }
        },
        Effects(effects) => {
            Effects(effects.bind_typevars(type_bindings, cache))
        }
    }
}

/// Helper for bind_typevars which binds a single TypeVariableId if it is Unbound
/// and it is found in the type_bindings. If a type_binding wasn't found, a
/// default TypeVariable or Ref is constructed by passing the relevant constructor to `default`.
fn bind_typevar<'c>(
    id: TypeVariableId, type_bindings: &TypeBindings, default: fn(TypeVariableId) -> Type, cache: &ModuleCache<'c>,
) -> Type {
    // TODO: This ordering of checking type_bindings first is important.
    // There seems to be an issue currently where forall-bound variables
    // can be bound in the cache, so checking the cache for bindings first
    // can prevent us from instantiating these variables.
    match type_bindings.get(&id) {
        Some(binding) => binding.clone(),
        None => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                bind_typevars(&typ.clone(), type_bindings, cache)
            } else {
                default(id)
            }
        },
    }
}

/// Recurse on typ, returning true if it contains any of the TypeVariableIds
/// contained within list.
pub fn contains_any_typevars_from_list<'c>(typ: &Type, list: &[TypeVariableId], cache: &ModuleCache<'c>) -> bool {
    match typ {
        Primitive(_) => false,
        UserDefined(_) => false,

        TypeVariable(id) => type_variable_contains_any_typevars_from_list(*id, list, cache),

        Function(function) => {
            function.parameters.iter().any(|parameter| contains_any_typevars_from_list(parameter, list, cache))
                || contains_any_typevars_from_list(&function.return_type, list, cache)
                || contains_any_typevars_from_list(&function.environment, list, cache)
                || function.effects.contains_any_typevars_from_list(list, cache)
        },

        Ref(lifetime) => type_variable_contains_any_typevars_from_list(*lifetime, list, cache),

        TypeApplication(typ, args) => {
            contains_any_typevars_from_list(typ, list, cache)
                || args.iter().any(|arg| contains_any_typevars_from_list(arg, list, cache))
        },
        Struct(fields, id) => {
            type_variable_contains_any_typevars_from_list(*id, list, cache)
                || fields.iter().any(|(_, field)| contains_any_typevars_from_list(field, list, cache))
        },
        Effects(effects) => effects.contains_any_typevars_from_list(list, cache),
    }
}

fn type_variable_contains_any_typevars_from_list<'c>(
    id: TypeVariableId, list: &[TypeVariableId], cache: &ModuleCache<'c>,
) -> bool {
    if let Bound(typ) = &cache.type_bindings[id.0] {
        contains_any_typevars_from_list(typ, list, cache)
    } else {
        list.contains(&id)
    }
}

/// Helper function for getting the next type variable at the current level
pub fn next_type_variable_id(cache: &mut ModuleCache) -> TypeVariableId {
    let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
    cache.next_type_variable_id(level)
}

pub fn next_type_variable(cache: &mut ModuleCache) -> Type {
    let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
    cache.next_type_variable(level)
}

fn to_trait_constraints(
    id: DefinitionInfoId, scope: ImplScopeId, callsite: VariableId, cache: &mut ModuleCache,
) -> TraitConstraints {
    let info = &cache.definition_infos[id.0];
    let current_constraint_id = &mut cache.current_trait_constraint_id;

    let mut traits = fmap(&info.required_traits, |required_trait| {
        let id = current_constraint_id.next();
        required_trait.as_constraint(scope, callsite, id)
    });

    // If this definition is from a trait, we must add the initial constraint directly
    if let Some((trait_id, args)) = &info.trait_info {
        let id = current_constraint_id.next();

        traits.push(TraitConstraint {
            required: RequiredTrait {
                signature: ConstraintSignature { trait_id: *trait_id, args: args.clone(), id },
                callsite: Callsite::Direct(callsite),
            },
            scope,
        });
    }

    traits
}

/// specializes the polytype s by copying the term and replacing the
/// bound type variables consistently by new monotype variables.
/// Returns the type bindings used to instantiate the type.
///
/// E.g.   instantiate (forall a b. a -> b -> a) = c -> d -> c
///
/// This will also instantiate each given trait constraint, replacing
/// each free typevar of the constraint's argument types.
impl GeneralizedType {
    pub fn instantiate<'b>(
        &self, mut constraints: TraitConstraints, cache: &mut ModuleCache<'b>,
    ) -> (Type, TraitConstraints, TypeBindings) {
        // Note that the returned type is no longer a PolyType,
        // this means it is now monomorphic and not forall-quantified
        match self {
            GeneralizedType::MonoType(typ) => (typ.clone(), constraints, HashMap::new()),
            GeneralizedType::PolyType(typevars, typ) => {
                // Must replace all typevars in typ and the required_traits list with new ones
                let mut typevars_to_replace = HashMap::new();
                for var in typevars.iter().copied() {
                    typevars_to_replace.insert(var, next_type_variable_id(cache));
                }
                let typ = replace_typevars(typ, &typevars_to_replace, cache);

                for var in find_all_typevars_in_traits(&constraints, cache).iter().copied() {
                    typevars_to_replace.entry(var).or_insert_with(|| next_type_variable_id(cache));
                }

                for constraint in constraints.iter_mut() {
                    for typ in constraint.args_mut() {
                        *typ = replace_typevars(typ, &typevars_to_replace, cache);
                    }
                }

                let type_bindings = typevars_to_replace.into_iter().map(|(k, v)| (k, TypeVariable(v))).collect();
                (typ, constraints, type_bindings)
            },
        }
    }
}

/// Similar to instantiate but uses an explicitly passed map to map
/// the old type variables to. This version is used during trait impl
/// type inference to ensure all definitions in the trait impl are
/// mapped to the same typevars, rather than each definition instantiated
/// separately as is normal.
///
/// This version is also different in that it also replaces the type variables
/// of monotypes.
fn instantiate_impl_with_bindings<'b>(
    typ: &GeneralizedType, bindings: &mut TypeBindings, cache: &mut ModuleCache<'b>,
) -> GeneralizedType {
    use GeneralizedType::*;
    match typ {
        MonoType(typ) => MonoType(replace_all_typevars_with_bindings(typ, bindings, cache)),
        PolyType(_, typ) => {
            // unreachable!("Impl already inferred to have polymorphic typ, {}", typ.debug(cache)),
            MonoType(replace_all_typevars_with_bindings(typ, bindings, cache))
        },
    }
}

fn find_binding<'b>(id: TypeVariableId, map: &UnificationBindings, cache: &ModuleCache<'b>) -> TypeBinding {
    match &cache.type_bindings[id.0] {
        Bound(typ) => Bound(typ.clone()),
        Unbound(level, kind) => match map.bindings.get(&id) {
            Some(typ) => Bound(typ.clone()),
            None => Unbound(*level, kind.clone()),
        },
    }
}

pub(super) struct OccursResult {
    occurs: bool,
    level_bindings: LevelBindings,
}

impl OccursResult {
    pub(super) fn does_not_occur() -> OccursResult {
        OccursResult { occurs: false, level_bindings: vec![] }
    }

    fn new(occurs: bool, level_bindings: LevelBindings) -> OccursResult {
        OccursResult { occurs, level_bindings }
    }

    fn then(mut self, mut f: impl FnMut() -> OccursResult) -> OccursResult {
        if !self.occurs {
            let mut other = f();
            self.occurs = other.occurs;
            self.level_bindings.append(&mut other.level_bindings);
        }
        self
    }

    pub(super) fn then_all<'a>(
        mut self, types: impl IntoIterator<Item = &'a Type>, mut f: impl FnMut(&'a Type) -> OccursResult,
    ) -> OccursResult {
        if !self.occurs {
            for typ in types {
                let mut other = f(typ);
                self.occurs = other.occurs;
                self.level_bindings.append(&mut other.level_bindings);
                if self.occurs {
                    return self;
                }
            }
        }
        self
    }
}

/// Can a monomorphic TypeVariable(id) be found inside this type?
/// This will mutate any typevars found to increase their LetBindingLevel.
/// Doing so increases the lifetime of the typevariable and lets us keep
/// track of which type variables to generalize later on. It also means
/// that occurs should only be called during unification however.
pub(super) fn occurs<'b>(
    id: TypeVariableId, level: LetBindingLevel, typ: &Type, bindings: &mut UnificationBindings, fuel: u32,
    cache: &mut ModuleCache<'b>,
) -> OccursResult {
    if fuel == 0 {
        panic!("Recursion limit reached in occurs");
    }

    let fuel = fuel - 1;
    match typ {
        Primitive(_) => OccursResult::does_not_occur(),
        UserDefined(_) => OccursResult::does_not_occur(),

        TypeVariable(var_id) => typevars_match(id, level, *var_id, bindings, fuel, cache),
        Function(function) => occurs(id, level, &function.return_type, bindings, fuel, cache)
            .then(|| occurs(id, level, &function.environment, bindings, fuel, cache))
            .then(|| function.effects.occurs(id, level, bindings, fuel, cache))
            .then_all(&function.parameters, |param| occurs(id, level, param, bindings, fuel, cache)),
        TypeApplication(typ, args) => occurs(id, level, typ, bindings, fuel, cache)
            .then_all(args, |arg| occurs(id, level, arg, bindings, fuel, cache)),
        Ref(lifetime) => typevars_match(id, level, *lifetime, bindings, fuel, cache),
        Struct(fields, var_id) => typevars_match(id, level, *var_id, bindings, fuel, cache)
            .then_all(fields.iter().map(|(_, typ)| typ), |field| occurs(id, level, field, bindings, fuel, cache)),
        Effects(effects) => effects.occurs(id, level, bindings, fuel, cache),
    }
}

/// Helper function for the `occurs` check.
///
/// Recurse within `haystack` to try to find an Unbound typevar and check if it
/// has the same Id as the needle TypeVariableId.
pub(super) fn typevars_match<'c>(
    needle: TypeVariableId, level: LetBindingLevel, haystack: TypeVariableId, bindings: &mut UnificationBindings,
    fuel: u32, cache: &mut ModuleCache<'c>,
) -> OccursResult {
    match find_binding(haystack, bindings, cache) {
        Bound(binding) => occurs(needle, level, &binding, bindings, fuel, cache),
        Unbound(original_level, _) => {
            let binding = if level < original_level { vec![(needle, level)] } else { vec![] };
            OccursResult::new(needle == haystack, binding)
        },
    }
}

/// Returns what a given type is bound to, following all typevar links until it reaches an Unbound one.
pub fn follow_bindings_in_cache_and_map<'b>(
    typ: &Type, bindings: &UnificationBindings, cache: &ModuleCache<'b>,
) -> Type {
    match typ {
        TypeVariable(id) | Ref(id) => match find_binding(*id, bindings, cache) {
            Bound(typ) => follow_bindings_in_cache_and_map(&typ, bindings, cache),
            Unbound(..) => typ.clone(),
        },
        _ => typ.clone(),
    }
}

pub fn follow_bindings_in_cache<'b>(typ: &Type, cache: &ModuleCache<'b>) -> Type {
    match typ {
        TypeVariable(id) | Ref(id) => match &cache.type_bindings[id.0] {
            Bound(typ) => follow_bindings_in_cache(typ, cache),
            Unbound(..) => typ.clone(),
        },
        _ => typ.clone(),
    }
}

/// Try to unify the two given types, with the given addition set of type bindings.
/// This will not perform any binding of type variables in-place, instead it will insert
/// their mapping into the given set of bindings, letting the user of this function decide
/// whether to use the unification results or not.
///
/// If there is an error during unification, an appropriate error message is returned,
/// and the given bindings set may still be modified with prior type bindings.
///
/// This function performs the bulk of the work for the various unification functions.
#[allow(clippy::nonminimal_bool)]
pub fn try_unify_with_bindings_inner<'b>(
    t1: &Type, t2: &Type, bindings: &mut UnificationBindings, location: Location<'b>, cache: &mut ModuleCache<'b>,
) -> Result<(), ()> {
    match (t1, t2) {
        (Primitive(p1), Primitive(p2)) if p1 == p2 => Ok(()),

        (UserDefined(id1), UserDefined(id2)) if id1 == id2 => Ok(()),

        // Any type variable can be bound or unbound.
        // - If bound: unify the bound type with the other type.
        // - If unbound: 'unify' the LetBindingLevel of the type variable by setting
        //   it to the minimum scope of type variables in b. This happens within the occurs check.
        //   The unification of the LetBindingLevel here is a form of lifetime inference for the
        //   typevar and is used during generalization to determine which variables to generalize.
        (TypeVariable(id), _) => try_unify_type_variable_with_bindings(*id, t1, t2, bindings, location, cache),

        (_, TypeVariable(id)) => try_unify_type_variable_with_bindings(*id, t2, t1, bindings, location, cache),

        (Function(function1), Function(function2)) => {
            if function1.parameters.len() != function2.parameters.len() {
                // Whether a function is varargs or not is never unified,
                // so if one function is varargs, assume they both should be.
                if !(function1.is_varargs && function2.parameters.len() >= function1.parameters.len())
                    && !(function2.is_varargs && function1.parameters.len() >= function2.parameters.len())
                {
                    return Err(());
                }
            }

            for (a_arg, b_arg) in function1.parameters.iter().zip(function2.parameters.iter()) {
                try_unify_with_bindings_inner(a_arg, b_arg, bindings, location, cache)?
            }

            try_unify_with_bindings_inner(&function1.return_type, &function2.return_type, bindings, location, cache)?;
            try_unify_with_bindings_inner(&function1.environment, &function2.environment, bindings, location, cache)?;
            function1.effects.try_unify_with_bindings(&function2.effects, bindings, cache);
            Ok(())
        },

        (TypeApplication(a_constructor, a_args), TypeApplication(b_constructor, b_args)) => {
            // Unify the constructors before checking the arg lengths, it gives better error messages
            try_unify_with_bindings_inner(a_constructor, b_constructor, bindings, location, cache)?;

            if a_args.len() != b_args.len() {
                return Err(());
            }

            for (a_arg, b_arg) in a_args.iter().zip(b_args.iter()) {
                try_unify_with_bindings_inner(a_arg, b_arg, bindings, location, cache)?;
            }

            Ok(())
        },

        // Refs have a hidden lifetime variable we need to unify here
        (Ref(a_lifetime), Ref(_)) => {
            try_unify_type_variable_with_bindings(*a_lifetime, t1, t2, bindings, location, cache)
        },

        // Follow any bindings here for convenience so we don't have to check if a or b
        // are bound in all Struct cases below.
        (Struct(_, var), t2) | (t2, Struct(_, var)) if matches!(&cache.type_bindings[var.0], Bound(_)) => {
            match &cache.type_bindings[var.0] {
                Bound(bound) => try_unify_with_bindings_inner(&bound.clone(), t2, bindings, location, cache),
                _ => unreachable!(),
            }
        },

        (Struct(fields1, rest1), Struct(fields2, rest2)) => {
            bind_struct_fields(fields1, fields2, *rest1, *rest2, bindings, location, cache)
        },

        (Struct(fields1, rest), other) | (other, Struct(fields1, rest)) => {
            let fields2 = get_fields(other, &[], bindings, cache)?;
            bind_struct_fields_subset(fields1, &fields2, bindings, location, cache)?;
            bindings.bindings.insert(*rest, other.clone());
            Ok(())
        },

        (Effects(effects1), Effects(effects2)) => {
            effects1.try_unify_with_bindings(effects2, bindings, cache);
            Ok(())
        }

        _ => Err(()),
    }
}

fn bind_struct_fields<'c>(
    fields1: &BTreeMap<String, Type>, fields2: &BTreeMap<String, Type>, rest1: TypeVariableId, rest2: TypeVariableId,
    bindings: &mut UnificationBindings, location: Location<'c>, cache: &mut ModuleCache<'c>,
) -> Result<(), ()> {
    let mut new_fields = fields1.clone();
    for (name, typ2) in fields2 {
        if let Some(typ1) = new_fields.get(name) {
            try_unify_with_bindings_inner(typ1, typ2, bindings, location, cache)?;
        } else {
            new_fields.insert(name.clone(), typ2.clone());
        }
    }

    if new_fields.len() != fields1.len() && new_fields.len() != fields2.len() {
        try_unify_type_variable_with_bindings(
            rest1,
            &TypeVariable(rest1),
            &TypeVariable(rest2),
            bindings,
            location,
            cache,
        )?;
        let new_rest = new_row_variable(rest1, rest2, cache);
        let new_struct = Struct(new_fields, new_rest);
        // We set rest1 := rest2 above, so we should insert into rest2 to bind both structs
        bindings.bindings.insert(rest2, new_struct);
    } else if new_fields.len() != fields1.len() {
        // Set 1 := 2
        let struct2 = Struct(new_fields, rest2);
        try_unify_type_variable_with_bindings(rest1, &TypeVariable(rest1), &struct2, bindings, location, cache)?;
    } else if new_fields.len() != fields2.len() {
        // Set 2 := 1
        let struct1 = Struct(new_fields, rest1);
        try_unify_type_variable_with_bindings(rest2, &TypeVariable(rest2), &struct1, bindings, location, cache)?;
    }

    Ok(())
}

/// Create a new row variable with a LetBindingLevel of the min of the
/// levels of the two given row variables. Expects both given variables
/// to be unbound.
fn new_row_variable(row1: TypeVariableId, row2: TypeVariableId, cache: &mut ModuleCache) -> TypeVariableId {
    match (&cache.type_bindings[row1.0], &cache.type_bindings[row2.0]) {
        (Unbound(level1, _), Unbound(level2, _)) => {
            let new_level = std::cmp::min(*level1, *level2);
            cache.next_type_variable_id(new_level)
        },
        _ => unreachable!(),
    }
}

/// Like bind_struct_fields but enforces `fields` must be a subset of the fields in the template.
fn bind_struct_fields_subset<'c>(
    fields: &BTreeMap<String, Type>, template: &BTreeMap<String, Type>, bindings: &mut UnificationBindings,
    location: Location<'c>, cache: &mut ModuleCache<'c>,
) -> Result<(), ()> {
    // FIXME: Enforcing a struct type's fields are a subset of
    // a data type's fields works for cases like
    // ```
    // foo bar = bar.x
    //
    // type T = x: i32, y: i32
    // foo (T 2)
    // ```
    // But for the following case it'd be unsound if we ever allowed struct literals:
    // ```
    // baz (t: T) = t.x + t.y
    //
    // baz { x: 3 }
    // ```
    // Since the struct has a subset of T's fields this would currently pass.
    if fields.len() > template.len() {
        return Err(());
    }

    for (name, field) in fields {
        match template.get(name) {
            Some(template_field) => {
                try_unify_with_bindings_inner(template_field, field, bindings, location, cache)?;
            },
            None => return Err(()),
        }
    }

    Ok(())
}

fn get_fields<'c>(
    typ: &Type, args: &[Type], bindings: &mut UnificationBindings, cache: &mut ModuleCache<'c>,
) -> Result<BTreeMap<String, Type>, ()> {
    match typ {
        UserDefined(id) => {
            let info = &cache[*id];
            match &info.body {
                TypeInfoBody::Alias(typ) => get_fields(&typ.clone(), args, bindings, cache),
                TypeInfoBody::Union(_) => Err(()),
                TypeInfoBody::Unknown => unreachable!(),
                TypeInfoBody::Struct(fields) => {
                    let mut more_bindings = HashMap::new();
                    if !args.is_empty() {
                        more_bindings = type_application_bindings(info, args, cache);
                    }
                    Ok(fields
                        .iter()
                        .map(|field| {
                            let typ = if more_bindings.is_empty() {
                                field.field_type.clone()
                            } else {
                                bind_typevars(&field.field_type, &more_bindings, cache)
                            };

                            (field.name.clone(), typ)
                        })
                        .collect())
                },
            }
        },
        TypeApplication(constructor, args) => match follow_bindings_in_cache_and_map(constructor, bindings, cache) {
            Ref(_) => get_fields(&args[0], &[], bindings, cache),
            other => get_fields(&other, args, bindings, cache),
        },
        Struct(fields, rest) => match &cache.type_bindings[rest.0] {
            Bound(binding) => get_fields(&binding.clone(), args, bindings, cache),
            Unbound(_, _) => Ok(fields.clone()),
        },
        TypeVariable(id) => match &cache.type_bindings[id.0] {
            Bound(binding) => get_fields(&binding.clone(), args, bindings, cache),
            Unbound(_, _) => Err(()),
        },
        _ => Err(()),
    }
}

/// Unify a single type variable (id arising from the type a) with an expected type b.
/// Follows the given TypeBindings in bindings and the cache if a is Bound.
fn try_unify_type_variable_with_bindings<'c>(
    id: TypeVariableId, a: &Type, b: &Type, bindings: &mut UnificationBindings, location: Location<'c>,
    cache: &mut ModuleCache<'c>,
) -> Result<(), ()> {
    match find_binding(id, bindings, cache) {
        Bound(a) => try_unify_with_bindings_inner(&a, b, bindings, location, cache),
        Unbound(a_level, _a_kind) => {
            // Create binding for boundTy that is currently empty.
            // Ensure not to create recursive bindings to the same variable
            let b = follow_bindings_in_cache_and_map(b, bindings, cache);
            if *a != b {
                let result = occurs(id, a_level, &b, bindings, RECURSION_LIMIT, cache);
                if result.occurs {
                    // TODO: Need better error messages for recursive types
                    Err(())
                } else {
                    bindings.bindings.insert(id, b);
                    Ok(())
                }
            } else {
                Ok(())
            }
        },
    }
}

pub fn try_unify_with_bindings<'b>(
    t1: &Type, t2: &Type, bindings: &mut UnificationBindings, location: Location<'b>, cache: &mut ModuleCache<'b>,
    error_message: &str,
) -> Result<(), ErrorMessage<'b>> {
    match try_unify_with_bindings_inner(t1, t2, bindings, location, cache) {
        Ok(()) => Ok(()),
        Err(()) => Err(error::from_template(error_message, location, t1, t2, cache)),
    }
}

/// A convenience wrapper for try_unify_with_bindings, creating an empty
/// set of type bindings, and returning all the newly-created bindings on success,
/// or the unification error message on error.
pub fn try_unify<'c>(
    t1: &Type, t2: &Type, location: Location<'c>, cache: &mut ModuleCache<'c>, error_message: &str,
) -> UnificationResult<'c> {
    let mut bindings = UnificationBindings::empty();
    try_unify_with_bindings(t1, t2, &mut bindings, location, cache, error_message).map(|()| bindings)
}

/// Try to unify all the given type, with the given bindings in scope.
/// Will add new bindings to the given TypeBindings and return them all on success.
pub fn try_unify_all_with_bindings<'c>(
    vec1: &[Type], vec2: &[Type], mut bindings: UnificationBindings, location: Location<'c>,
    cache: &mut ModuleCache<'c>, error_message: &'static str,
) -> UnificationResult<'c> {
    if vec1.len() != vec2.len() {
        // This bad error message is the reason this function isn't used within
        // try_unify_with_bindings! We'd need access to the full type to give better
        // errors like the other function does.
        return Err(make_error!(
            location,
            "Type-length mismatch: {} versus {} when unifying [{}] and [{}]",
            vec1.len(),
            vec2.len(),
            concat_type_strings(vec1, cache),
            concat_type_strings(vec2, cache)
        ));
    }

    for (t1, t2) in vec1.iter().zip(vec2.iter()) {
        try_unify_with_bindings(t1, t2, &mut bindings, location, cache, error_message)?;
    }
    Ok(bindings)
}

/// Concatenate all the types into a comma-separated string for error messages.
fn concat_type_strings<'c>(types: &[Type], cache: &ModuleCache<'c>) -> String {
    let types = fmap(types, |typ| typ.display(cache).to_string());
    join_with(&types, ", ")
}

/// Unifies the two given types, remembering the unification results in the cache.
/// If this operation fails, a user-facing error message is emitted.
pub fn unify<'c>(t1: &Type, t2: &Type, location: Location<'c>, cache: &mut ModuleCache<'c>, error_message: &str) {
    perform_bindings_or_print_error(try_unify(t1, t2, location, cache, error_message), cache);
}

/// Helper for committing to the results of try_unify.
/// Places all the typevar bindings in the cache to be remembered,
/// or otherwise prints out the given error message.
pub fn perform_bindings_or_print_error<'c>(unification_result: UnificationResult<'c>, cache: &mut ModuleCache<'c>) {
    match unification_result {
        Ok(bindings) => bindings.perform(cache),
        Err(message) => eprintln!("{}", message),
    }
}

/// Remember all the given type bindings in the cache,
/// permanently binding the given type variables to the given bindings.
fn perform_type_bindings(bindings: TypeBindings, cache: &mut ModuleCache) {
    for (id, binding) in bindings.into_iter() {
        cache.type_bindings[id.0] = Bound(binding);
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
        UserDefined(_) => vec![],
        TypeVariable(id) => find_typevars_in_typevar_binding(*id, polymorphic_only, cache),
        Function(function) => {
            let mut type_variables = vec![];
            for parameter in &function.parameters {
                type_variables.append(&mut find_all_typevars(parameter, polymorphic_only, cache));
            }
            type_variables.append(&mut find_all_typevars(&function.environment, polymorphic_only, cache));
            type_variables.append(&mut find_all_typevars(&function.return_type, polymorphic_only, cache));
            type_variables.append(&mut function.effects.find_all_typevars(polymorphic_only, cache));
            type_variables
        },
        TypeApplication(constructor, args) => {
            let mut type_variables = find_all_typevars(constructor, polymorphic_only, cache);
            for arg in args {
                type_variables.append(&mut find_all_typevars(arg, polymorphic_only, cache));
            }
            type_variables
        },
        Ref(lifetime) => find_typevars_in_typevar_binding(*lifetime, polymorphic_only, cache),
        Struct(fields, id) => match &cache.type_bindings[id.0] {
            Bound(t) => find_all_typevars(t, polymorphic_only, cache),
            Unbound(..) => {
                let mut vars = find_typevars_in_typevar_binding(*id, polymorphic_only, cache);
                for (_, field) in fields {
                    vars.append(&mut find_all_typevars(field, polymorphic_only, cache));
                }
                vars
            },
        },
        Effects(effects) => effects.find_all_typevars(polymorphic_only, cache),
    }
}

/// Helper for find_all_typevars which gets the TypeBinding for a given
/// TypeVariableId and either recurses on it if it is bound or returns it.
fn find_typevars_in_typevar_binding(
    id: TypeVariableId, polymorphic_only: bool, cache: &ModuleCache,
) -> Vec<TypeVariableId> {
    match &cache.type_bindings[id.0] {
        Bound(t) => find_all_typevars(t, polymorphic_only, cache),
        Unbound(level, _) => {
            if level_is_polymorphic(*level) || !polymorphic_only {
                vec![id]
            } else {
                vec![]
            }
        },
    }
}

fn find_all_typevars_in_traits<'a>(traits: &TraitConstraints, cache: &ModuleCache<'a>) -> Vec<TypeVariableId> {
    let mut typevars = vec![];
    for constraint in traits.iter() {
        for typ in constraint.args() {
            typevars.append(&mut find_all_typevars(typ, true, cache));
        }
    }
    typevars
}

/// Find all typevars declared inside the current LetBindingLevel and wrap the type in a PolyType
/// e.g.  generalize (a -> b -> b) = forall a b. a -> b -> b
fn generalize<'a>(typ: &Type, cache: &ModuleCache<'a>) -> GeneralizedType {
    let mut typevars = find_all_typevars(typ, true, cache);
    if typevars.is_empty() {
        GeneralizedType::MonoType(typ.clone())
    } else {
        // TODO: This can be sped up, e.g. we wouldn't need to dedup at all if we didn't use a Vec
        typevars.sort();
        typevars.dedup();
        GeneralizedType::PolyType(typevars, typ.clone())
    }
}

/// Mark a given DefinitionInfoId as currently being type checked
fn mark_id_in_progress(id: DefinitionInfoId, cache: &mut ModuleCache) {
    cache.call_stack.push(id);

    let info = &mut cache.definition_infos[id.0];

    // Should this be under the typ.is_none check?
    // It seems to only differ for trait impl definitions
    info.undergoing_type_inference = true;

    if info.typ.is_none() {
        let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
        let typevar = cache.next_type_variable(level);

        // Mark the definition with a fresh typevar for recursive references
        let info = &mut cache.definition_infos[id.0];
        info.typ = Some(GeneralizedType::MonoType(typevar));
    }
}

fn mark_id_finished(id: DefinitionInfoId, cache: &mut ModuleCache) {
    cache.call_stack.pop();
    if !definition_is_mutually_recursive(id, cache) {
        cache[id].undergoing_type_inference = false;
    }
}

fn infer_nested_definition(
    definition_id: DefinitionInfoId, impl_scope: ImplScopeId, callsite: VariableId, cache: &mut ModuleCache,
) -> (GeneralizedType, TraitConstraints) {
    let definition = cache[definition_id].definition.as_mut().unwrap();

    // DefinitionKind::Definition marks its ids internally when we call infer(definition, _).
    // We need to avoid doing it twice.
    let need_to_mark_definition = !matches!(definition, DefinitionKind::Definition(_));

    if need_to_mark_definition {
        mark_id_in_progress(definition_id, cache);
    }

    let definition = cache[definition_id].definition.as_mut().unwrap();

    let mut constraints = match definition {
        DefinitionKind::Definition(definition) => {
            let definition = trustme::extend_lifetime(*definition);
            infer(definition, cache).traits
        },
        DefinitionKind::TraitDefinition(definition) => {
            let definition = trustme::extend_lifetime(*definition);
            infer(definition, cache).traits
        },
        DefinitionKind::EffectDefinition(definition) => {
            let definition = trustme::extend_lifetime(*definition);
            infer(definition, cache).traits
        },
        DefinitionKind::Extern(declaration) => {
            let definition = trustme::extend_lifetime(*declaration);
            infer(definition, cache).traits
        },
        DefinitionKind::Parameter => vec![],
        DefinitionKind::MatchPattern => vec![],
        DefinitionKind::TypeConstructor { .. } => vec![],
    };

    if need_to_mark_definition {
        mark_id_finished(definition_id, cache);
    }

    constraints.append(&mut to_trait_constraints(definition_id, impl_scope, callsite, cache));

    let info = &cache.definition_infos[definition_id.0];
    (info.typ.clone().unwrap(), constraints)
}

/// Infer the type of all the closed-over variables within a lambda so when we
/// type check the body their type will already be known.
fn bind_closure_environment<'c>(environment: &mut ClosureEnvironment, cache: &mut ModuleCache<'c>) {
    for (from, (_, to, to_bindings)) in environment {
        if let Some(from) = cache.definition_infos[from.0].typ.as_ref() {
            let (from, _, bindings) = from.clone().instantiate(vec![], cache);

            let to_type = &mut cache[*to].typ;
            assert!(to_type.is_none());

            // The 'to' ids are the variables used within the closure, so they should
            // be monomorphic like other function parameters are.
            *to_type = Some(GeneralizedType::MonoType(from));
            *to_bindings = Rc::new(bindings);
        }
    }
}

fn infer_closure_environment<'c>(environment: &ClosureEnvironment, cache: &mut ModuleCache<'c>) -> Type {
    let mut environment =
        fmap(environment, |(_from, (_, to, _))| cache[*to].typ.as_ref().unwrap().clone().into_monotype());

    if environment.is_empty() {
        // Non-closure functions have an environment of type unit
        Type::UNIT
    } else if environment.len() == 1 {
        environment.pop().unwrap()
    } else {
        make_tuple_type(environment)
    }
}

/// Makes a tuple out of nested pairs with elements from the
/// given Vec of types. Since this is made from nested pairs
/// and includes no type terminator, it requires at least 2
/// types to be passed in.
fn make_tuple_type(mut types: Vec<Type>) -> Type {
    assert!(types.len() > 1);
    let mut ret = types.pop().unwrap();

    while !types.is_empty() {
        let typ = types.pop().unwrap();
        let pair = Box::new(Type::UserDefined(PAIR_TYPE));
        ret = Type::TypeApplication(pair, vec![typ, ret]);
    }

    ret
}

/// Binds a given type to an irrefutable pattern, recursing on the pattern and verifying
/// that it is indeed irrefutable. If should_generalize is true, this generalizes the type given
/// to any variable encountered. Appends the given required_traits list in the DefinitionInfo's
/// required_traits field.
pub(super) fn bind_irrefutable_pattern<'c>(
    ast: &mut ast::Ast<'c>, typ: &Type, required_traits: &[RequiredTrait], should_generalize: bool,
    cache: &mut ModuleCache<'c>,
) {
    use ast::Ast::*;
    use ast::LiteralKind;

    match ast {
        Literal(literal) => match literal.kind {
            LiteralKind::Unit => {
                literal.set_type(Type::UNIT);
                let msg = "Expected a unit type from this pattern, but the corresponding value has the type $1";
                unify(typ, &Type::UNIT, ast.locate(), cache, msg);
            },
            _ => error!(ast.locate(), "Pattern is not irrefutable"),
        },
        Variable(variable) => {
            let definition_id = variable.definition.unwrap();
            let info = &cache.definition_infos[definition_id.0];

            // The type may already be set (e.g. from a trait impl this definition belongs to).
            // If it is, unify the existing type and new type before generalizing them.
            if let Some(existing_type) = &info.typ {
                match existing_type {
                    GeneralizedType::MonoType(existing_type) => {
                        let msg = "variable type $2 does not match its declared type of $1";
                        unify(&existing_type.clone(), typ, variable.location, cache, msg);
                    },
                    GeneralizedType::PolyType(_, _) => {
                        unreachable!("Cannot unify a polytype: {}", existing_type.debug(cache))
                    },
                }
            }

            let typ = if should_generalize { generalize(typ, cache) } else { GeneralizedType::MonoType(typ.clone()) };

            let info = &mut cache.definition_infos[definition_id.0];
            info.required_traits.extend_from_slice(required_traits);

            variable.typ = Some(typ.remove_forall().clone());
            info.typ = Some(typ);
        },
        TypeAnnotation(annotation) => {
            let msg = "Pattern type $1 does not match the annotated type $2";
            unify(typ, annotation.typ.as_ref().unwrap(), annotation.location, cache, msg);
            bind_irrefutable_pattern(annotation.lhs.as_mut(), typ, required_traits, should_generalize, cache);
        },
        // TODO: All struct patterns
        FunctionCall(call) if call.is_pair_constructor() => {
            let args = fmap(&call.args, |_| next_type_variable(cache));
            let pair_type = Box::new(Type::UserDefined(PAIR_TYPE));

            let pair_type = Type::TypeApplication(pair_type, args.clone());
            unify(typ, &pair_type, call.location, cache, "Expected a pair type from this pattern, but found $1");

            let function_type = Type::Function(FunctionType {
                parameters: args,
                return_type: Box::new(pair_type.clone()),
                environment: Box::new(Type::UNIT),
                effects: EffectSet::any(cache),
                is_varargs: false,
            });

            call.function.set_type(function_type);
            call.set_type(pair_type.clone());

            match pair_type {
                Type::TypeApplication(_, args) => {
                    for (element, element_type) in call.args.iter_mut().zip(args) {
                        bind_irrefutable_pattern(element, &element_type, required_traits, should_generalize, cache);
                    }
                },
                _ => unreachable!(),
            }
        },
        _ => {
            error!(ast.locate(), "Invalid syntax in irrefutable pattern");
        },
    }
}

fn lookup_definition_type_in_trait<'a>(
    name: &str, trait_id: TraitInfoId, cache: &mut ModuleCache<'a>,
) -> GeneralizedType {
    let trait_info = &cache.trait_infos[trait_id.0];
    for definition_id in trait_info.definitions.iter() {
        let definition_info = &cache.definition_infos[definition_id.0];
        if definition_info.name == name {
            match definition_info.typ.as_ref() {
                Some(typ) => return typ.clone(),
                None => return infer_trait_definition(name, trait_id, cache),
            }
        }
    }
    unreachable!()
}

fn lookup_definition_traits_in_trait(name: &str, trait_id: TraitInfoId, cache: &mut ModuleCache) -> Vec<RequiredTrait> {
    let trait_info = &cache.trait_infos[trait_id.0];
    for definition_id in trait_info.definitions.iter() {
        let definition_info = &cache.definition_infos[definition_id.0];
        if definition_info.name == name {
            // Check if this trait definition has already been type-checked
            if definition_info.typ.is_some() {
                // TODO: Shouldn't need to clone here. Seems to be a limitation of the current
                // borrow checker.
                return definition_info.required_traits.clone();
            } else {
                return infer_trait_definition_traits(name, trait_id, cache);
            }
        }
    }
    unreachable!()
}

/// Perform type inference on the ast::TraitDefinition that defines the given trait function name.
/// The type returned will be that of the named trait member rather than the trait as a whole.
fn infer_trait_definition<'c>(name: &str, trait_id: TraitInfoId, cache: &mut ModuleCache<'c>) -> GeneralizedType {
    let trait_info = &mut cache.trait_infos[trait_id.0];
    match &mut trait_info.trait_node {
        Some(node) => {
            infer(trustme::extend_lifetime(*node), cache);
            lookup_definition_type_in_trait(name, trait_id, cache)
        },
        None => unreachable!("Type for {} has not been filled in yet", name),
    }
}

fn infer_trait_definition_traits(name: &str, trait_id: TraitInfoId, cache: &mut ModuleCache) -> Vec<RequiredTrait> {
    let trait_info = &mut cache.trait_infos[trait_id.0];
    match &mut trait_info.trait_node {
        Some(node) => {
            infer(trustme::extend_lifetime(*node), cache);
            lookup_definition_traits_in_trait(name, trait_id, cache)
        },
        None => unreachable!("Type for {} has not been filled in yet", name),
    }
}

/// Perform some action for each variable within a pattern
pub(super) fn foreach_variable(pattern: &ast::Ast, f: &mut impl FnMut(&ast::Variable)) {
    use ast::Ast::*;
    match pattern {
        Variable(variable) => f(variable),
        TypeAnnotation(annotation) => foreach_variable(annotation.lhs.as_ref(), f),
        FunctionCall(call) => {
            for arg in &call.args {
                foreach_variable(arg, f);
            }
        },
        _ => {
            error!(pattern.locate(), "Invalid syntax in irrefutable pattern in trait impl, expected a pattern of some kind (a name, type annotation, or type constructor)");
        },
    }
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
fn bind_irrefutable_pattern_in_impl<'a>(
    ast: &ast::Ast<'a>, trait_id: TraitInfoId, bindings: &mut TypeBindings, cache: &mut ModuleCache<'a>,
) {
    foreach_variable(ast, &mut |variable| {
        let name = variable.to_string();
        let trait_type = lookup_definition_type_in_trait(&name, trait_id, cache);

        let trait_type = instantiate_impl_with_bindings(&trait_type, bindings, cache);
        cache[variable.definition.unwrap()].typ = Some(trait_type);
    });
}

/// Checks that the traits used in `pattern` are a subset of traits used in the `given` list of
/// an impl or in the `given` list of the corresponding function in the trait declaration.
fn check_impl_propagated_traits(
    pattern: &ast::Ast, trait_id: TraitInfoId, given: &[ConstraintSignature], cache: &mut ModuleCache,
) {
    foreach_variable(pattern, &mut |variable| {
        let name = variable.to_string();

        // Given a trait:
        // ```
        // trait Foo a with
        //     foo : a -> a
        //         given Bar a, Baz a
        // ```
        // This list will contain [Bar a, Baz a]
        let useable_traits = lookup_definition_traits_in_trait(&name, trait_id, cache);

        let definition_id = variable.definition.unwrap();
        let used_traits = cache[definition_id].required_traits.clone();

        cache[definition_id].required_traits = used_traits
            .into_iter()
            .filter_map(|mut used| {
                if let Some(id) = find_matching_trait(&used, &useable_traits, given, cache) {
                    used.signature.id = id;
                    Some(used)
                } else {
                    let constraint = TraitConstraint { required: used, scope: variable.impl_scope.unwrap() };
                    // Any traits used that are not in the 'given' clause must be resolved
                    // TODO: Should issue this error earlier to give a better callsite for the error
                    traitchecker::force_resolve_trait(constraint, cache);
                    None
                }
            })
            .collect();
    });
}

// TODO: `useable_traits` here is always going to be empty. We'll likely need a
// `Vec<ConstraintSignature>` field on each definition to account for trait definitions
// with no body.
fn find_matching_trait(
    used: &RequiredTrait, useable_traits: &[RequiredTrait], given: &[ConstraintSignature], cache: &mut ModuleCache,
) -> Option<TraitConstraintId> {
    for useable in useable_traits {
        if useable.signature.trait_id == used.signature.trait_id {
            if let Ok(bindings) = try_unify_all_with_bindings(
                &used.signature.args,
                &useable.signature.args,
                UnificationBindings::empty(),
                Location::builtin(),
                cache,
                "error never shown",
            ) {
                if bindings.bindings.is_empty() {
                    // bindings.perform(cache);
                    return Some(useable.signature.id);
                }
            }
        }
    }

    for useable in given {
        if useable.trait_id == used.signature.trait_id {
            if let Ok(bindings) = try_unify_all_with_bindings(
                &used.signature.args,
                &useable.args,
                UnificationBindings::empty(),
                Location::builtin(),
                cache,
                "error never shown",
            ) {
                if bindings.bindings.is_empty() {
                    // bindings.perform(cache);
                    return Some(useable.id);
                }
            }
        }
    }

    None
}

fn initialize_pattern_types(pattern: &ast::Ast, cache: &mut ModuleCache) {
    foreach_variable(pattern, &mut |variable| {
        mark_id_in_progress(variable.definition.unwrap(), cache);
    });
}

fn finish_pattern(pattern: &ast::Ast, cache: &mut ModuleCache) {
    foreach_variable(pattern, &mut |variable| {
        mark_id_finished(variable.definition.unwrap(), cache);
    });
}

pub trait Inferable<'a> {
    fn infer_impl(&mut self, checker: &mut ModuleCache<'a>) -> TypeResult;
}

/// Compile an entire program, starting from main then lazily compiling
/// each used function as it is called.
pub fn infer_ast<'a>(ast: &mut ast::Ast<'a>, cache: &mut ModuleCache<'a>) {
    CURRENT_LEVEL.store(INITIAL_LEVEL, Ordering::SeqCst);
    let result = infer(ast, cache);
    CURRENT_LEVEL.store(INITIAL_LEVEL - 1, Ordering::SeqCst);

    let exposed_traits = traitchecker::resolve_traits(result.traits, &[], cache);
    // No traits should be propogated above the top-level main function
    assert!(exposed_traits.is_empty());

    // TODO: Better error message, check for IO effect
    assert!(result.effects.effects.is_empty());
}

pub fn infer<'a, T>(ast: &mut T, cache: &mut ModuleCache<'a>) -> TypeResult
where
    T: Inferable<'a> + Typed + std::fmt::Display,
{
    let result = ast.infer_impl(cache);
    ast.set_type(result.typ.clone());
    result
}

/// Note: each Ast's inference rule is given above the impl if available.
impl<'a> Inferable<'a> for ast::Ast<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        dispatch_on_expr!(self, Inferable::infer_impl, cache)
    }
}

impl<'a> Inferable<'a> for ast::Literal<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        use ast::LiteralKind::*;
        match self.kind {
            Integer(x, kind) => {
                if kind == IntegerKind::Unknown {
                    // Mutate this unknown integer literal to an IntegerKind::Inferred(int_type).
                    let int_type = next_type_variable_id(cache);
                    self.kind = Integer(x, IntegerKind::Inferred(int_type));
                    let int_trait = TraitConstraint::int_constraint(int_type, self.location, cache);
                    let mut result = TypeResult::of(Type::TypeVariable(int_type), cache);
                    result.traits = vec![int_trait];
                    result
                } else {
                    TypeResult::of(Type::Primitive(PrimitiveType::IntegerType(kind)), cache)
                }
            },
            Float(_) => TypeResult::of(Type::Primitive(PrimitiveType::FloatType), cache),
            String(_) => TypeResult::of(Type::UserDefined(STRING_TYPE), cache),
            Char(_) => TypeResult::of(Type::Primitive(PrimitiveType::CharType), cache),
            Bool(_) => TypeResult::of(Type::Primitive(PrimitiveType::BooleanType), cache),
            Unit => TypeResult::of(Type::UNIT, cache),
        }
    }
}

/*
 *  x : s ∊ cache
 *  t = instantiate s
 *  --------------------- [Var]
 *  infer cache x = t | ε
 */
impl<'a> Inferable<'a> for ast::Variable<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let definition_id = self.definition.unwrap();
        let impl_scope = self.impl_scope.unwrap();
        let id = self.id.unwrap();

        let info = &cache[definition_id];

        // Lookup the type of the definition.
        // We'll need to recursively infer the type if it is not found
        let (s, traits) = match &info.typ {
            Some(typ) => {
                let typ = typ.clone();

                let constraints = to_trait_constraints(definition_id, impl_scope, id, cache);
                (typ, constraints)
            },
            None => {
                // If the variable has a definition we can infer from then use that
                // to determine the type, otherwise fill in a type variable for it.
                let (typ, traits) = if info.definition.is_some() {
                    infer_nested_definition(self.definition.unwrap(), impl_scope, id, cache)
                } else {
                    (GeneralizedType::MonoType(next_type_variable(cache)), vec![])
                };

                let info = &mut cache.definition_infos[self.definition.unwrap().0];
                info.typ = Some(typ.clone());
                (typ, traits)
            },
        };

        // Check if the definition is still undergoing inference to see if it is mutually recursive.
        // If so we need to avoid generalizing the current definition until all definitions in the
        // mutual recursion set can be generalized at once.
        cache.update_mutual_recursion_sets(definition_id, self.id.unwrap());

        let (t, traits, mapping) = s.instantiate(traits, cache);
        self.instantiation_mapping = Rc::new(mapping);
        TypeResult::new(t, traits, cache)
    }
}

/*
 * Γ, x:t1 ⊢ e:t2 | ε′
 * -------------------------- [Lam]
 * Γ ⊢ λx. e : t1 → t2 can ε′ | ε
 */
impl<'a> Inferable<'a> for ast::Lambda<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        // The newvars for the parameters are filled out during name resolution
        let parameter_types = fmap(&self.args, |_| next_type_variable(cache));

        for (parameter, parameter_type) in self.args.iter_mut().zip(parameter_types.iter()) {
            bind_irrefutable_pattern(parameter, parameter_type, &[], false, cache);
        }

        bind_closure_environment(&mut self.closure_environment, cache);

        // return_type, traits
        let body = if let Some(typ) = self.body.get_type() {
            // Check if user specified a return type
            let typ = typ.clone();
            let body = self.body.infer_impl(cache);
            let msg = "Function body type $1 does not match declared return type of $2";
            unify(&typ, &body.typ, self.location, cache, msg);
            body
        } else {
            infer(self.body.as_mut(), cache)
        };

        let typ = Function(FunctionType {
            parameters: parameter_types,
            return_type: Box::new(body.typ),
            environment: Box::new(infer_closure_environment(&self.closure_environment, cache)),
            effects: body.effects,
            is_varargs: false,
        });

        TypeResult::new(typ, body.traits, cache)
    }
}

/*
 * Γ ⊢ f: t2 → t can ε | ε    Γ ⊢ x: t2 | ε
 * ----------------------------------------- [App]
 *             Γ ⊢ f x : t | ε
 */
impl<'a> Inferable<'a> for ast::FunctionCall<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let mut f = infer(self.function.as_mut(), cache);

        let parameters = fmap(&mut self.args, |arg| {
            let mut arg_result = infer(arg, cache);
            f.combine(&mut arg_result, cache);
            arg_result.typ
        });

        let return_type = next_type_variable(cache);
        let new_effect = f.effects.clone();

        let new_function = Function(FunctionType {
            parameters,
            return_type: Box::new(return_type.clone()),
            environment: Box::new(next_type_variable(cache)),
            effects: new_effect,
            is_varargs: false,
        });

        // Don't need a match here, but if we already know f is a function type
        // it improves error messages to unify parameter by parameter.
        match try_unify(&f.typ, &new_function, self.location, cache, "this error is never shown") {
            Ok(bindings) => bindings.perform(cache),
            Err(_) => issue_argument_types_error(self, f.typ.clone(), new_function, cache),
        }

        f.with_type(return_type)
    }
}

fn issue_argument_types_error<'c>(call: &ast::FunctionCall<'c>, f: Type, args: Type, cache: &mut ModuleCache<'c>) {
    let (expected, actual) = unwrap_functions(f, args, cache);

    if expected.parameters.len() != actual.parameters.len() && !expected.is_varargs && !actual.is_varargs {
        error!(
            call.location,
            "Function {} declared to take {} parameter(s), but {} were supplied",
            Function(expected.clone()).display(cache),
            expected.parameters.len(),
            actual.parameters.len()
        )
    }

    for ((arg, param), arg_ast) in actual.parameters.into_iter().zip(expected.parameters).zip(&call.args) {
        unify(&arg, &param, arg_ast.locate(), cache, "Expected argument of type $2, but found $1");
    }
}

fn unwrap_functions(f: Type, new_function: Type, cache: &ModuleCache) -> (FunctionType, FunctionType) {
    let f = follow_bindings_in_cache(&f, cache);

    match (f, new_function) {
        (Type::Function(f1), Type::Function(f2)) => (f1, f2),
        _ => unreachable!(),
    }
}

/* Let
 *   infer cache expr = t
 *   infer (pattern:(generalize t) :: cache) rest = t'
 *   -----------------
 *   infer cache (let pattern = expr in rest) = t'
 */
impl<'a> Inferable<'a> for ast::Definition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let unit = Type::UNIT;

        if self.typ.is_some() {
            return TypeResult::of(unit, cache);
        }

        // Without this self.typ wouldn't be set yet while inferring the type of self.expr
        // if this definition is recursive. If this is removed we would recursively infer
        // this definition repeatedly until eventually reaching an error when the previous type
        // is generalized but the new one is not.
        self.typ = Some(unit.clone());
        initialize_pattern_types(&self.pattern, cache);

        let level = self.level.unwrap();
        let previous_level = CURRENT_LEVEL.swap(level.0, Ordering::SeqCst);

        // t, traits
        let mut result = infer(self.expr.as_mut(), cache);
        if self.mutable {
            let lifetime = next_type_variable_id(cache);
            result.typ = Type::TypeApplication(Box::new(Type::Ref(lifetime)), vec![result.typ]);
        }

        // The rhs of a Definition must be inferred at a greater LetBindingLevel than
        // the lhs below. Here we use level for the rhs and level - 1 for the lhs
        CURRENT_LEVEL.store(level.0 - 1, Ordering::SeqCst);

        // TODO: the inferred type t needs to be unified with the patterns type before
        // resolve_traits is called. For now it is sufficient to call bind_irrefutable_pattern
        // twice - the first time with no traits, however in the future bind_irrefutable_pattern
        // should be split up into two parts.
        bind_irrefutable_pattern(self.pattern.as_mut(), &result.typ, &[], false, cache);

        // TODO investigate this check, should be unneeded. It is breaking on the `input` function
        // in the stdlib.
        if self.pattern.get_type().is_none() {
            self.pattern.set_type(result.typ.clone());
        }

        // If this definition is of a lambda or variable we try to generalize it,
        // which entails wrapping type variables in a forall, and finding which traits
        // usages of this definition require.
        let traits = try_generalize_definition(self, result.typ, result.traits, cache);

        // TODO: Can these operations on the LetBindingLevel be simplified?
        CURRENT_LEVEL.store(previous_level, Ordering::SeqCst);

        // Done with this definition, remove it from callstack and mark each variable
        // definied within its pattern as no longer undergoing type inference
        finish_pattern(&self.pattern, cache);

        TypeResult::new(unit, traits, cache)
    }
}

impl<'a> Inferable<'a> for ast::If<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let mut result = infer(self.condition.as_mut(), cache);
        let bool_type = Type::Primitive(PrimitiveType::BooleanType);

        let msg = "$1 should be a bool to be used in an if condition";
        unify(&result.typ, &bool_type, self.condition.locate(), cache, msg);

        let mut then = infer(self.then.as_mut(), cache);
        result.combine(&mut then, cache);

        if let Some(otherwise) = &mut self.otherwise {
            let mut otherwise = infer(otherwise.as_mut(), cache);
            result.combine(&mut otherwise, cache);

            let msg = "Expected 'then' and 'else' branch types to match, but found $1 and $2 respectively";
            unify(&then.typ, &otherwise.typ, self.location, cache, msg);
            result.with_type(then.typ)
        } else {
            result.with_type(Type::UNIT)
        }
    }
}

impl<'a> Inferable<'a> for ast::Match<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let error_count = get_error_count();

        let mut result = infer(self.expression.as_mut(), cache);
        let mut return_type = Type::UNIT;

        if !self.branches.is_empty() {
            // Unroll the first iteration of inferring (pattern, branch) types so each
            // subsequent (pattern, branch) types can be unified against the first.
            let mut pattern = infer(&mut self.branches[0].0, cache);
            result.combine(&mut pattern, cache);

            let msg = "This pattern of type $2 does not match the type $1 that is being matched on";
            unify(&result.typ, &pattern.typ, self.branches[0].0.locate(), cache, msg);

            let mut branch = infer(&mut self.branches[0].1, cache);
            result.combine(&mut branch, cache);
            return_type = branch.typ;

            for (pattern, branch) in self.branches.iter_mut().skip(1) {
                let mut pattern_result = infer(pattern, cache);
                let mut branch_result = infer(branch, cache);

                let msg = "This pattern of type $2 does not match the type $1 that is being matched on";
                unify(&result.typ, &pattern_result.typ, pattern.locate(), cache, msg);

                let msg = "This branch's return type $2 does not match the previous branches which return $1";
                unify(&return_type, &branch_result.typ, branch.locate(), cache, msg);

                result.combine(&mut pattern_result, cache);
                result.combine(&mut branch_result, cache);
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

        result.with_type(return_type)
    }
}

impl<'a> Inferable<'a> for ast::TypeDefinition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        TypeResult::of(Type::UNIT, cache)
    }
}

impl<'a> Inferable<'a> for ast::TypeAnnotation<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let lhs = infer(self.lhs.as_mut(), cache);

        let msg = "Expression of type $1 does not match its annotated type $2";
        unify(&lhs.typ, self.typ.as_mut().unwrap(), self.location, cache, msg);
        lhs
    }
}

impl<'a> Inferable<'a> for ast::Import<'a> {
    /// Type checker doesn't need to follow imports.
    /// It typechecks definitions as-needed when it finds a variable whose type is still unknown.
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        TypeResult::of(Type::UNIT, cache)
    }
}

impl<'a> Inferable<'a> for ast::TraitDefinition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let previous_level = CURRENT_LEVEL.swap(self.level.unwrap().0, Ordering::SeqCst);

        for declaration in self.declarations.iter_mut() {
            let rhs = declaration.typ.as_ref().unwrap();
            bind_irrefutable_pattern(declaration.lhs.as_mut(), rhs, &[], true, cache);
        }

        CURRENT_LEVEL.store(previous_level, Ordering::SeqCst);
        TypeResult::of(Type::UNIT, cache)
    }
}

impl<'a> Inferable<'a> for ast::TraitImpl<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        if self.typ.is_some() {
            return TypeResult::of(Type::UNIT, cache);
        }

        let trait_info = &cache.trait_infos[self.trait_info.unwrap().0];

        let mut typevars_to_replace = trait_info.typeargs.clone();
        typevars_to_replace.append(&mut trait_info.fundeps.clone());

        // Need to replace all typevars here so we do not rebind over them.
        // E.g. an impl for `Cmp a given Int a` could be accidentally bound to `Cmp usz`
        // TODO: Is the above comment correct? replace_all_typevars causes `impl Print (HashMap a b)`
        //       in the stdlib to fail (the given list would need to use the same type bindings)
        //       and removing it still lets all tests pass, despite builtin_int.an
        //       testing several traits like `Add a given Int a` for several integer types.
        // let (trait_arg_types, _) = replace_all_typevars(&self.trait_arg_types, cache);

        let trait_arg_types = self.trait_arg_types.clone();

        // Instantiate the typevars in the parent trait to bind their definition
        // types against the types in this trait impl. This needs to be done once
        // at the trait level rather than at each definition so that each definition
        // refers to the same type variable instances/bindings.
        //
        // This is because only these bindings in trait_to_impl are unified against
        // the types declared in self.typeargs
        let mut impl_bindings: HashMap<_, _> = typevars_to_replace.into_iter().zip(trait_arg_types).collect();

        for definition in self.definitions.iter_mut() {
            bind_irrefutable_pattern_in_impl(
                definition.pattern.as_ref(),
                self.trait_info.unwrap(),
                &mut impl_bindings,
                cache,
            );

            // TODO: Check effects for trait impls
            let definition_result = infer(definition, cache);

            // Need to check we only use traits that are `given` by the definition
            // in question or by the overall impl.
            check_impl_propagated_traits(
                definition.pattern.as_ref(),
                self.trait_info.unwrap(),
                &cache[self.impl_id.unwrap()].given.clone(),
                cache,
            );

            // No traits should be propagated outside of the impl. The only way this can happen
            // is if the definition is not generalized and traits are used.
            for trait_ in definition_result.traits {
                error!(
                    definition.location,
                    "Definition requires {}, but it needs to be a function to add this trait",
                    trait_.display(cache)
                );
            }
        }

        TypeResult::of(Type::UNIT, cache)
    }
}

impl<'a> Inferable<'a> for ast::Return<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let result = infer(self.expression.as_mut(), cache);
        result.with_type(next_type_variable(cache))
    }
}

impl<'a> Inferable<'a> for ast::Sequence<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let ignore_len = self.statements.len() - 1;
        let mut result = TypeResult::of(Type::UNIT, cache);

        for statement in self.statements.iter_mut().take(ignore_len) {
            result.combine(&mut infer(statement, cache), cache);
        }

        let mut last = infer(self.statements.last_mut().unwrap(), cache);
        result.combine(&mut last, cache);
        result.with_type(last.typ)
    }
}

impl<'a> Inferable<'a> for ast::Extern<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let previous_level = CURRENT_LEVEL.swap(self.level.unwrap().0, Ordering::SeqCst);
        for declaration in self.declarations.iter_mut() {
            bind_irrefutable_pattern(declaration.lhs.as_mut(), declaration.typ.as_ref().unwrap(), &[], true, cache);
        }
        CURRENT_LEVEL.store(previous_level, Ordering::SeqCst);
        TypeResult::of(Type::UNIT, cache)
    }
}

impl<'a> Inferable<'a> for ast::MemberAccess<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let mut result = infer(self.lhs.as_mut(), cache);

        let level = LetBindingLevel(CURRENT_LEVEL.load(Ordering::SeqCst));
        let mut field_type = cache.next_type_variable(level);

        if self.is_offset {
            let collection_variable = next_type_variable(cache);
            let expected = ref_of(collection_variable.clone(), cache);
            let msg = "Expected a struct reference but found $1 instead";
            unify(&result.typ, &expected, self.lhs.locate(), cache, msg);
            result.typ = collection_variable;
        }

        let mut fields = BTreeMap::new();
        fields.insert(self.field.clone(), field_type.clone());

        // The '..' or 'rest of the struct' stand-in variable
        let rho = cache.next_type_variable_id(level);
        let struct_type = Type::Struct(fields, rho);

        let msg = &format!("$1 has no field {} of type $2", self.field);
        unify(&result.typ, &struct_type, self.location, cache, msg);

        if self.is_offset {
            field_type = ref_of(field_type, cache);
        }

        result.with_type(field_type)
    }
}

impl<'a> Inferable<'a> for ast::Assignment<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let mut result = infer(self.lhs.as_mut(), cache);
        let mut rhs = infer(self.rhs.as_mut(), cache);
        result.combine(&mut rhs, cache);

        let lifetime = next_type_variable_id(cache);
        let mutref = Type::TypeApplication(Box::new(Type::Ref(lifetime)), vec![rhs.typ.clone()]);

        match try_unify(&result.typ, &mutref, self.location, cache, "never shown") {
            Ok(bindings) => bindings.perform(cache),
            Err(_) => issue_assignment_error(&result.typ, self.lhs.locate(), &rhs.typ, self.location, cache),
        }

        result.with_type(Type::UNIT)
    }
}

fn issue_assignment_error<'c>(
    lhs: &Type, lhs_loc: Location<'c>, rhs: &Type, location: Location<'c>, cache: &mut ModuleCache<'c>,
) {
    // Try to offer a more specific error message
    let lifetime = next_type_variable_id(cache);
    let var = next_type_variable(cache);
    let mutref = Type::TypeApplication(Box::new(Type::Ref(lifetime)), vec![var]);

    let msg = "Expression of type $1 must be a `ref a` type to be assigned to";
    if let Err(msg) = try_unify(&lhs, &mutref, lhs_loc, cache, msg) {
        eprintln!("{}", msg);
    } else {
        let inner_type = match follow_bindings_in_cache(&lhs, cache) {
            TypeApplication(_, mut args) => args.remove(0),
            _ => unreachable!(),
        };

        let msg = "Cannot assign expression of type $2 to a ref of type $1";
        let msg = try_unify(&inner_type, &rhs, location, cache, msg).unwrap_err();
        eprintln!("{}", msg);
    }
}

impl<'a> Inferable<'a> for ast::EffectDefinition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        let previous_level = CURRENT_LEVEL.swap(self.level.unwrap().0, Ordering::SeqCst);

        let effect_id = self.effect_info.unwrap();
        let effect_args = fmap(&cache[effect_id].typeargs, |id| TypeVariable(*id));

        for declaration in self.declarations.iter_mut() {
            let rhs = declaration.typ.as_ref().unwrap();

            bind_irrefutable_pattern(declaration.lhs.as_mut(), rhs, &[], true, cache);

            foreach_variable(&declaration.lhs, &mut |var| {
                let info = &mut cache[var.definition.unwrap()];
                inject_effect(info, effect_id, effect_args.clone());
            });
        }

        CURRENT_LEVEL.store(previous_level, Ordering::SeqCst);
        TypeResult::of(Type::UNIT, cache)
    }
}

fn inject_effect(info: &mut crate::cache::DefinitionInfo, effect_id: EffectInfoId, effect_args: Vec<Type>) {
    match info.typ.as_mut().unwrap() {
        GeneralizedType::MonoType(Type::Function(f)) => {
            f.effects.effects.push((effect_id, effect_args));
        },
        GeneralizedType::PolyType(_, Type::Function(f)) => {
            f.effects.effects.push((effect_id, effect_args));
        },
        // Name resolution should verify all effect declarations must have a function type
        _ => (),
    }
}

impl<'a> Inferable<'a> for ast::Handle<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> TypeResult {
        TypeResult::of(Type::UNIT, cache)
    }
}
