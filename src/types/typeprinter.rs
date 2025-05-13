//! typeprinter.rs - Utilities for printing out types and traits.
//! Since a type may contain TypeVariables with their TypeBindings in the cache,
//! printing out a bound type requires using the cache as well. Resultingly,
//! types/traits are displayed via `type.display(cache)` rather than directly having
//! a Display impl.
use crate::cache::{ModuleCache, TraitInfoId};
use crate::types::traits::{ConstraintSignature, ConstraintSignaturePrinter, RequiredTrait, TraitConstraintId};
use crate::types::typechecker::find_all_typevars;
use crate::types::{FunctionType, PrimitiveType, Type, TypeBinding, TypeInfoId, TypeVariableId};

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Display, Formatter};

use colored::*;

use super::effects::EffectSet;
use super::typechecker::follow_bindings_in_cache;
use super::GeneralizedType;
use super::TypePriority;

/// Wrapper containing the information needed to print out a type
pub struct TypePrinter<'a, 'b> {
    typ: GeneralizedType,

    /// Maps unique type variable IDs to human readable names like a, b, c, etc.
    typevar_names: HashMap<TypeVariableId, String>,

    /// Controls whether to show or hide some hidden data, like ref lifetimes
    debug: bool,

    cache: &'a ModuleCache<'b>,
}

impl<'a, 'b> Display for TypePrinter<'a, 'b> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_generalized_type(&self.typ, f)
    }
}

impl<'a, 'b> Debug for TypePrinter<'a, 'b> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_generalized_type(&self.typ, f)
    }
}

/// Fill a HashMap with human readable names for each typevar in the given Vec.
/// For example, given [TypeVariableId(53), TypeVariableId(92)] this may yield `a` and `b`
/// respectively.
fn fill_typevar_map(map: &mut HashMap<TypeVariableId, String>, typevars: Vec<TypeVariableId>, current: &mut char) {
    for typevar in typevars {
        if let Entry::Vacant(entry) = map.entry(typevar) {
            entry.insert(current.to_string());
            *current = (*current as u8 + 1) as char;
            assert!(*current != 'z'); // TODO: wrap to aa, ab, ac...
        }
    }
}

/// Returns a string of the given type and traits it requires.
/// The type and traits are all taken in together so that any repeated typevariables e.g.
/// `TypeVariableId(55)` that may be used in both the type and any traits are given the same
/// name in both. Printing out the type separately from the traits would cause type variable
/// naming to restart at `a` which may otherwise give them different names.
pub fn show_type_and_traits(
    name: &str, typ: &GeneralizedType, traits: &[RequiredTrait], trait_info: &Option<(TraitInfoId, Vec<Type>)>,
    cache: &ModuleCache<'_>, debug: bool,
) -> String {
    let mut map = HashMap::new();
    let mut current = 'a';

    let typevars = typ.find_all_typevars(false, cache);
    fill_typevar_map(&mut map, typevars, &mut current);

    let typ = typ.clone();
    let printer = TypePrinter { typ, cache, debug, typevar_names: map.clone() };
    let type_string = format!("{} : {}", name, printer);

    let mut traits = traits
        .iter()
        .map(|required_trait| {
            fill_typevar_map(&mut map, required_trait.find_all_typevars(cache), &mut current);
            ConstraintSignaturePrinter {
                signature: required_trait.signature.clone(),
                cache,
                debug,
                typevar_names: map.clone(),
            }
            .to_string()
        })
        .collect::<Vec<String>>();

    // If this is a trait function, we must add the trait it originates from manually
    if let Some((trait_id, args)) = trait_info {
        for arg in args {
            fill_typevar_map(&mut map, find_all_typevars(arg, false, cache), &mut current);
        }
        let signature = ConstraintSignature {
            trait_id: *trait_id,
            args: args.clone(),
            id: TraitConstraintId(0), // Dummy value
        };
        let p = ConstraintSignaturePrinter { signature, cache, debug, typevar_names: map.clone() };
        traits.push(p.to_string());
    }

    // Remove "duplicate" traits so users don't see `given Add a, Add a`.
    // These contain usage information that is different within them but this
    // isn't used in their Display impl so they look like duplicates.
    traits.sort();
    traits.dedup();
    if traits.is_empty() {
        type_string
    } else {
        format!("{}\n  given {}", type_string, traits.join(", "))
    }
}

impl<'a, 'b> TypePrinter<'a, 'b> {
    pub fn new(
        typ: GeneralizedType, typevar_names: HashMap<TypeVariableId, String>, debug: bool, cache: &'a ModuleCache<'b>,
    ) -> Self {
        TypePrinter { typ, typevar_names, debug, cache }
    }

    pub fn debug_type(typ: GeneralizedType, cache: &'a ModuleCache<'b>) -> Self {
        let typevars = typ.find_all_typevars(false, cache);
        let mut typevar_names = HashMap::new();

        for typevar in typevars {
            if typevar_names.get(&typevar).is_none() {
                typevar_names.insert(typevar, typevar.0.to_string());
            }
        }

        Self::new(typ, typevar_names, true, cache)
    }

    pub fn display_type(typ: GeneralizedType, cache: &'a ModuleCache<'b>) -> Self {
        let typevars = typ.find_all_typevars(false, cache);
        let mut typevar_names = HashMap::new();
        let mut current = 'a';

        for typevar in typevars {
            if typevar_names.get(&typevar).is_none() {
                typevar_names.insert(typevar, current.to_string());
                current = (current as u8 + 1) as char;
                assert!(current != 'z'); // TODO: wrap to aa, ab, ac...
            }
        }

        Self::new(typ, typevar_names, false, cache)
    }

    fn fmt_generalized_type(&self, typ: &GeneralizedType, f: &mut Formatter) -> std::fmt::Result {
        match typ {
            GeneralizedType::MonoType(typ) => self.fmt_type(typ, f),
            GeneralizedType::PolyType(typevars, typ) => self.fmt_forall(typevars, typ, f),
        }
    }

    fn fmt_type(&self, typ: &Type, f: &mut Formatter) -> std::fmt::Result {
        match typ {
            Type::Primitive(primitive) => self.fmt_primitive(primitive, f),
            Type::Function(function) => self.fmt_function(function, f),
            Type::TypeVariable(id) => self.fmt_type_variable(*id, f),
            Type::UserDefined(id) => self.fmt_user_defined_type(*id, f),
            Type::TypeApplication(constructor, args) => self.fmt_type_application(constructor, args, f),
            Type::Ref { sharedness, mutability, lifetime } => self.fmt_ref(sharedness, mutability, lifetime, f),
            Type::Struct(fields, rest) => self.fmt_struct(fields, *rest, f),
            Type::Effects(effects) => self.fmt_effects(effects, f),
            Type::Tag(tag) => write!(f, "{}", tag.to_string().blue()),
        }
    }

    fn fmt_primitive(&self, primitive: &PrimitiveType, f: &mut Formatter) -> std::fmt::Result {
        match primitive {
            PrimitiveType::IntegerTag(kind) => write!(f, "{}", kind.to_string().blue()),
            PrimitiveType::FloatTag(kind) => write!(f, "{}", kind.to_string().blue()),
            PrimitiveType::IntegerType => write!(f, "{}", "Int".blue()),
            PrimitiveType::FloatType => write!(f, "{}", "Float".blue()),
            PrimitiveType::CharType => write!(f, "{}", "Char".blue()),
            PrimitiveType::BooleanType => write!(f, "{}", "Bool".blue()),
            PrimitiveType::UnitType => write!(f, "{}", "Unit".blue()),
            PrimitiveType::Ptr => write!(f, "{}", "Ptr".blue()),
        }
    }

    fn fmt_function(&self, function: &FunctionType, f: &mut Formatter) -> std::fmt::Result {
        for (i, param) in function.parameters.iter().enumerate() {
            if TypePriority::FUN >= param.priority(&self.cache) {
                write!(f, "{}", "(".blue())?;
            }
            self.fmt_type(param, f)?;
            if TypePriority::FUN >= param.priority(&self.cache) {
                write!(f, "{}", ")".blue())?;
            }
            write!(f, " ")?;

            if i != function.parameters.len() - 1 {
                write!(f, "{}", "- ".blue())?;
            }
        }

        if function.has_varargs {
            write!(f, "{}", "... ".blue())?;
        }

        if function.environment.is_unit(self.cache) {
            write!(f, "{}", "-> ".blue())?;
        } else {
            write!(f, "{}", "=> ".blue())?;
        }

        // No parentheses are necessary if the precedence is the same, because `->` is right associative.
        // i.e. `a -> b -> c` means `a -> (b -> c)`
        if TypePriority::FUN > function.return_type.priority(&self.cache) {
            write!(f, "{}", "(".blue())?;
        }
        self.fmt_type(function.return_type.as_ref(), f)?;
        if TypePriority::FUN > function.return_type.priority(&self.cache) {
            write!(f, "{}", ")".blue())?;
        }

        write!(f, " ")?;

        if let Type::TypeVariable(id) = self.cache.follow_bindings_shallow(&function.effects) {
            write!(f, "{}", "can ".blue())?;
            self.fmt_type_variable(*id, f)?;
        } else {
            self.fmt_type(&function.effects, f)?;
        }

        Ok(())
    }

    fn fmt_type_variable(&self, id: TypeVariableId, f: &mut Formatter) -> std::fmt::Result {
        match &self.cache.type_bindings[id.0] {
            TypeBinding::Bound(typ) => self.fmt_type(typ, f),
            TypeBinding::Unbound(..) => {
                let default = "?".to_string();
                let name = self.typevar_names.get(&id).unwrap_or(&default).blue();
                write!(f, "{}", name)
            },
        }
    }

    fn fmt_user_defined_type(&self, id: TypeInfoId, f: &mut Formatter) -> std::fmt::Result {
        let name = self.cache.type_infos[id.0].name.blue();
        write!(f, "{}", name)
    }

    fn fmt_type_application(&self, constructor: &Type, args: &[Type], f: &mut Formatter) -> std::fmt::Result {
        let constructor = self.cache.follow_bindings_shallow(constructor);

        if constructor.is_polymorphic_int_type() {
            self.fmt_polymorphic_numeral(&args[0], f, "Int")
        } else if constructor.is_polymorphic_float_type() {
            self.fmt_polymorphic_numeral(&args[0], f, "Float")
        } else if constructor.is_reference_type() {
            let separate = self.reference_type_has_shared_specifier(constructor);
            self.fmt_type(constructor, f)?;
            if separate {
                write!(f, " ")?;
            }
            self.fmt_type(&args[0], f)
        } else {
            if constructor.is_pair_type() {
                self.fmt_pair(&args[0], &args[1], f)?;
            } else {
                self.fmt_type(constructor, f)?;
                for arg in args {
                    write!(f, " ")?;
                    // `(app f (app a b))` should be represented as `f (a b)`
                    if TypePriority::APP >= arg.priority(&self.cache) {
                        write!(f, "{}", "(".blue())?;
                    }
                    self.fmt_type(arg, f)?;
                    if TypePriority::APP >= arg.priority(&self.cache) {
                        write!(f, "{}", ")".blue())?;
                    }
                }
            }
            Ok(())
        }
    }

    fn reference_type_has_shared_specifier(&self, typ: &Type) -> bool {
        let Type::Ref { mutability: _, sharedness, lifetime: _ } = typ else {
            return false;
        };

        let sharedness = self.cache.follow_bindings_shallow(sharedness);
        matches!(sharedness, Type::Tag(_))
    }

    fn fmt_pair(&self, arg1: &Type, arg2: &Type, f: &mut Formatter) -> std::fmt::Result {
        if TypePriority::PAIR >= arg1.priority(&self.cache) {
            write!(f, "{}", "(".blue())?;
        }
        self.fmt_type(arg1, f)?;
        if TypePriority::PAIR >= arg1.priority(&self.cache) {
            write!(f, "{}", ")".blue())?;
        }
        write!(f, "{}", ", ".blue())?;
        // Because `(,)` is right-associative, omit parentheses if it has equal priority.
        // e.g. `a, b, .., n, m` means `(a, (b, (.. (n, m)..)))`.
        if TypePriority::PAIR > arg2.priority(&self.cache) {
            write!(f, "{}", "(".blue())?;
        }
        self.fmt_type(arg2, f)?;
        if TypePriority::PAIR > arg2.priority(&self.cache) {
            write!(f, "{}", ")".blue())?;
        }
        Ok(())
    }

    fn fmt_polymorphic_numeral(&self, arg: &Type, f: &mut Formatter, kind: &str) -> std::fmt::Result {
        match self.cache.follow_bindings_shallow(arg) {
            Type::TypeVariable(_) => {
                write!(f, "{} ", kind.blue())?;
                self.fmt_type(arg, f)
            },
            other => self.fmt_type(other, f),
        }
    }

    fn fmt_ref(&self, shared: &Type, mutable: &Type, lifetime: &Type, f: &mut Formatter) -> std::fmt::Result {
        let mutable = follow_bindings_in_cache(mutable, self.cache);
        let shared = follow_bindings_in_cache(shared, self.cache);

        match mutable {
            Type::Tag(tag) => write!(f, "{}", tag.to_string().blue())?,
            _ => write!(f, "{}", "?".blue())?,
        }

        if let Type::Tag(tag) = shared {
            write!(f, "{}", tag.to_string().blue())?;
        }

        if self.debug {
            write!(f, " ")?;
            self.fmt_type(lifetime, f)?;
        }

        Ok(())
    }

    fn fmt_forall(&self, typevars: &[TypeVariableId], typ: &Type, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", "forall".blue())?;
        for typevar in typevars {
            write!(f, " ")?;
            self.fmt_type_variable(*typevar, f)?;
        }
        write!(f, "{}", ". ".blue())?;
        if TypePriority::FORALL > typ.priority(&self.cache) {
            write!(f, "{}", "(".blue())?;
            self.fmt_type(typ, f)?;
            write!(f, "{}", ")".blue())?;
        } else {
            self.fmt_type(typ, f)?;
        }
        Ok(())
    }

    fn fmt_struct(
        &self, fields: &BTreeMap<String, Type>, rest: TypeVariableId, f: &mut Formatter,
    ) -> Result<(), std::fmt::Error> {
        match &self.cache.type_bindings[rest.0] {
            TypeBinding::Bound(typ) => self.fmt_type(typ, f),
            TypeBinding::Unbound(..) => {
                write!(f, "{}", "{ ".blue())?;

                for (name, field_type) in fields.iter() {
                    write!(f, "{}{}", name.blue(), ": ".blue())?;
                    self.fmt_type(field_type, f)?;
                    write!(f, "{}", ", ".blue())?;
                }

                if self.debug {
                    let default = "?".to_string();
                    let name = self.typevar_names.get(&rest).unwrap_or(&default).blue();
                    write!(f, "{}{}{}", "..".blue(), name, " }".blue())
                } else {
                    write!(f, "{}", ".. }".blue())
                }
            },
        }
    }

    fn fmt_effects(&self, effects: &EffectSet, f: &mut Formatter) -> std::fmt::Result {
        let effects = effects.flatten(&self.cache);

        if effects.effects.is_empty() && effects.extension.is_none() {
            return write!(f, "{}", "pure".blue());
        }

        if !effects.effects.is_empty() || effects.extension.is_some() {
            write!(f, "{}", "can ".blue())?;
        }

        for (i, (effect_id, effect_args)) in effects.effects.iter().enumerate() {
            let name = &self.cache.effect_infos[effect_id.0].name;
            write!(f, "{}", name.blue())?;

            for arg in effect_args {
                write!(f, " ")?;
                self.fmt_type(arg, f)?;
            }

            if i != effects.effects.len() - 1 {
                write!(f, "{}", ", ".blue())?;
            }
        }

        if let Some(extension) = effects.extension {
            if !effects.effects.is_empty() {
                write!(f, "{}", ", ".blue())?;
            }
            self.fmt_type_variable(extension, f)?;
        }

        Ok(())
    }
}
