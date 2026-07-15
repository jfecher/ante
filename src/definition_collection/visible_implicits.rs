use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    incremental::{DbHandle, GetItem, Resolve, VisibleDefinitions, VisibleImplicits},
    name_resolution::Origin,
    parser::{
        cst::{Name, TopLevelItemKind},
        ids::{TopLevelId, TopLevelName},
    },
    type_inference::{
        get_type::{get_partial_type, try_get_generalized_type},
        types::{PrimitiveType, Type},
    },
};

/// Maps each ability to its impls. The maps inside are split up for performance so that
/// impl search can search through fewer items.
///
/// E.g. an impl `foo` for `Print (Vec t)` will be stored in:
///   `known_ability_to_impls: [Print -> { type_to_impls: [Vec -> foo] }]`
///
/// Generic types makes this a bit more complex. An impl for a generic type like
/// `bar: Print t` will be stored as:
///   `known_ability_to_impls: [Print -> { generic_type_impls: [bar] }]`
///
/// Similarly, a fully-generic impl `baz: a` will not be in a known ability:
///   `unknown_ability_to_impls: [baz]`
///
/// When searching for a given impl for `Ability Arg`, we must search the `type_to_impls`
/// for that ability & arg pair, the `generic_type_impls` for the ability, and the
/// `unknown_ability_to_impls` which may be impls for any ability.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Implicits {
    /// Maps each implicit, e.g. `Add` to its impls.
    known_ability_to_impls: BTreeMap<TopLevelId, ImplicitImpls>,

    /// Maps any implicit that does not have a known type to its impls.
    /// We have to check these for every impl which makes these much more expensive.
    unknown_ability_to_impls: Vec<Implicit>,
}

type Implicit = (Name, TopLevelName);

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
struct ImplicitImpls {
    /// Maps the first argument of this impl to the set of impls that may
    /// apply to that argument.
    type_to_impls: BTreeMap<TypeKey, Vec<Implicit>>,

    /// Any impls for generic types. These will need to be checked against
    /// all argument types even if an impl in `type_to_impls` matches.
    generic_type_impls: Vec<Implicit>,
}

/// Key each type by its variant for faster mappings. This lets us retrieve
/// a much smaller set of implicits to search through for each type.
///
/// Type applications are stored as their constructor's type key instead.
///
/// Generic types return `None`, we need to sort them separately since they should
/// be checked against every argument type.
///
/// TODO: [Origin] makes this enum too large
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum TypeKey {
    Primitive(PrimitiveType),
    UserDefined(Origin),
    Function,
    Tuple,
    Effects,
}

impl TypeKey {
    /// Converts a type (with no bound type variables) to a [TypeKey].
    /// This returns [None] for generics & type variables so they can be
    /// sorted into another map to be checked against any other type.
    fn from_type(typ: &Type) -> Option<TypeKey> {
        Some(match typ {
            Type::Primitive(primitive) => TypeKey::Primitive(*primitive),
            Type::Generic(_) | Type::Variable(_) => return None,
            Type::Function(_) => TypeKey::Function,
            Type::Application(constructor, _) => return TypeKey::from_type(constructor),
            Type::UserDefined(origin) => TypeKey::UserDefined(*origin),
            // TODO: Is this correct?
            Type::Forall(_, typ) => return TypeKey::from_type(typ),
            Type::Tuple(_) => TypeKey::Tuple,
            Type::U32(_) => return None,
            Type::Effects(_, _) => TypeKey::Effects,
        })
    }
}

/// Returns any global implicits visible to the given item in the context.
/// This will always be a subset of all VisibleDefinitions to the same item.
pub fn visible_implicits_impl(context: &VisibleImplicits, db: &DbHandle) -> Arc<Implicits> {
    let definitions = VisibleDefinitions(context.0).get(db);
    let mut implicits = Implicits::default();

    for (name, top_level_name) in definitions.definitions.iter() {
        let (item, item_context) = GetItem(top_level_name.top_level_item).get(db);
        let TopLevelItemKind::Definition(definition) = &item.kind else { continue };
        if !definition.implicit {
            continue;
        }

        let resolution = Resolve(top_level_name.top_level_item).get(db);

        // A top-level implicit whose type cannot be derived from its annotation or RHS shape is
        // reported as a missing annotation error and must not contribute to global implicit resolution.
        // Otherwise, we get cascading errors for every implicit search
        if try_get_generalized_type(definition, &item_context, &resolution, db).is_none() {
            continue;
        }

        let typ = get_partial_type(definition, &item_context, &resolution, db, &mut 0);
        let mut inserted = false;

        if let Some((ability_id, arg_key)) = get_ability_id_and_first_argument(&typ, true) {
            // Fast path: ability and argument types are known
            let impls = implicits.known_ability_to_impls.entry(ability_id).or_default();

            match arg_key {
                // If this implicit is a function, register it as a candidate for both functions
                // and for its return type
                KeyKind::Function(return_key) => {
                    impls.type_to_impls.entry(TypeKey::Function).or_default().push((name.clone(), *top_level_name));
                    impls.type_to_impls.entry(return_key).or_default().push((name.clone(), *top_level_name));
                },
                KeyKind::Key(key) => {
                    impls.type_to_impls.entry(key).or_default().push((name.clone(), *top_level_name));
                },
                KeyKind::GenericOrUnknown => {
                    impls.generic_type_impls.push((name.clone(), *top_level_name));
                },
            }

            inserted = true;
        }

        if !inserted {
            implicits.unknown_ability_to_impls.push((name.clone(), *top_level_name));
        }
    }

    Arc::new(implicits)
}

enum KeyKind {
    Key(TypeKey),
    /// We can call implicit functions to get their return types so function-typed implicits
    /// get keyed both as a Function and as their return type
    Function(TypeKey),
    /// Generic implicits get put in a separate map for generic types
    GenericOrUnknown,
}

/// If `do_function_check` is true, we allow calling the function for its return type
/// to see if that return type is an ability with arguments. This should be true when
/// registering an implicit to register that it can be called, but not for looking for
/// an implicit in scope.
fn get_ability_id_and_first_argument(typ: &Type, do_function_check: bool) -> Option<(TopLevelId, KeyKind)> {
    match typ {
        Type::Application(constructor, args) => {
            let Some(Origin::TopLevelDefinition(ability_name)) = constructor.as_user_defined() else {
                return None;
            };

            let key = match TypeKey::from_type(&args[0]) {
                Some(key) => KeyKind::Key(key),
                None => KeyKind::GenericOrUnknown,
            };
            Some((ability_name.top_level_item, key))
        },
        Type::Function(function) if do_function_check => {
            let (id, key) = get_ability_id_and_first_argument(&function.return_type, do_function_check)?;
            let key = match key {
                KeyKind::Key(key) => KeyKind::Function(key),
                other => other,
            };
            Some((id, key))
        },
        _ => None,
    }
}

impl Implicits {
    /// Apply `f` to only the implicits that may possibly match the given type
    ///
    /// If there is only one type that matches `target_type` exactly, this is not guaranteed
    /// to iterate over only that type - we may iterate over possibly more items which could
    /// match, but we should never miss a matching definition.
    ///
    /// This is intended as an optimization compared to matching every implicit implicit
    /// in scope without performing any kind of filtering first.
    ///
    /// Return `true` to end the loop early
    pub fn iter_possibly_matching_impls(&self, target_type: &Type, mut f: impl FnMut(&Name, &TopLevelName) -> bool) {
        // Not being able to early-return from a closure means this is simpler as a macro
        macro_rules! apply_f_to_candidates {
            ($f: expr, $candidates: expr) => {{
                for candidate in $candidates {
                    if f(&candidate.0, &candidate.1) {
                        return;
                    }
                }
            }};
        }

        match get_ability_id_and_first_argument(target_type, false) {
            Some((ability_id, KeyKind::Key(argument))) => {
                // Fast case: need to iterate over:
                // 1. (ability match, argument match)
                // 2. (ability match, generic argument)
                if let Some(implicits) = self.known_ability_to_impls.get(&ability_id) {
                    if let Some(candidates) = implicits.type_to_impls.get(&argument) {
                        apply_f_to_candidates!(f, candidates);
                    }
                    apply_f_to_candidates!(f, &implicits.generic_type_impls);
                }
            },
            Some((ability_id, KeyKind::GenericOrUnknown)) => {
                // Unknown argument, need to iterate over every impl of the matching ability
                if let Some(implicits) = self.known_ability_to_impls.get(&ability_id) {
                    for candidates in implicits.type_to_impls.values() {
                        apply_f_to_candidates!(f, candidates);
                    }
                    apply_f_to_candidates!(f, &implicits.generic_type_impls);
                }
            },
            Some((_, KeyKind::Function(_))) => unreachable!("This variant is only used when registering implicits"),
            None => {
                // Unknown ability, need to iterate over everything
                for implicits in self.known_ability_to_impls.values() {
                    for candidates in implicits.type_to_impls.values() {
                        apply_f_to_candidates!(f, candidates);
                    }
                    apply_f_to_candidates!(f, &implicits.generic_type_impls);
                }
            },
        }
        // Finally, for any target type we need to consider all of the impls for unknown abilities
        apply_f_to_candidates!(f, &self.unknown_ability_to_impls);
    }

    /// Return true if there are <= 1 implicits for this type
    pub fn at_most_1_candidate(&self, target_type: &Type) -> bool {
        let mut count = self.unknown_ability_to_impls.len();

        match get_ability_id_and_first_argument(target_type, false) {
            Some((ability_id, KeyKind::Key(argument))) => {
                // Fast case: need to iterate over:
                // 1. (ability match, argument match)
                // 2. (ability match, generic argument)
                if let Some(implicits) = self.known_ability_to_impls.get(&ability_id) {
                    if let Some(candidates) = implicits.type_to_impls.get(&argument) {
                        count += candidates.len();
                    }
                    count += implicits.generic_type_impls.len();
                }
            },
            Some((ability_id, KeyKind::GenericOrUnknown)) => {
                // Unknown argument, need to iterate over every impl of the matching ability
                if let Some(implicits) = self.known_ability_to_impls.get(&ability_id) {
                    for candidates in implicits.type_to_impls.values() {
                        count += candidates.len();
                        if count > 1 {
                            return false;
                        }
                    }
                    count += implicits.generic_type_impls.len();
                }
            },
            Some((_, KeyKind::Function(_))) => unreachable!("This variant is only used when registering implicits"),
            None => {
                // Unknown ability, need to iterate over everything
                for implicits in self.known_ability_to_impls.values() {
                    for candidates in implicits.type_to_impls.values() {
                        count += candidates.len();
                        if count > 1 {
                            return false;
                        }
                    }
                    count += implicits.generic_type_impls.len();
                    if count > 1 {
                        return false;
                    }
                }
            },
        }
        count <= 1
    }
}
