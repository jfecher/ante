//! traits.rs - Defines the core data structures for trait inference.
//!
//! Trait inference is a part of type inference which determines:
//! 1. Which traits are required for a given Definition to be compiled
//! 2. When a `ast::Variable` is encountered whose Definition has some required traits
//!    whether these traits should be propagated up to be required for the current definition
//!    or whether they should be solved in place instead.
//! 3. Solving trait constraints, yielding the impl that should be used for that specific
//!    constraint and attaching this impl to the relevant callsite variable.
//!
//! For more information on the trait inference part of the type inference pass,
//! see `types/traitchecker.rs` for the file defining the pass itself.
//!
//! This module defines the three types that are core to trait inference:
//! 1. RequiredTrait - A trait required for a definition to be compiled. Note that required
//!    traits are just that - they only require the trait is present for the definition to be
//!    compiled, they do not require a specific impl to be used. For example the function
//!    `my_print a = print a` would have the RequiredTrait `Print a` which could be given any
//!    matching impl when `my_print` is later used at its callsite.
//! 2. RequiredImpl - An instantiated version of a RequiredTrait stored at the callsite of a
//!    function/variable that should point to a certain impl for a trait. When `my_print` is
//!    later called with `my_print 3`, `Print i32` will be a RequiredImpl. Note that RequiredImpls
//!    may still have type variables for blanket impls.
//! 3. TraitConstraint - TraitConstraints are what are actually passed around during the majority
//!    of the type inference pass, undergoing unification until the outer `ast::Definition`
//!    finishes compiling and trait inference needs to decide if each TraitConstraint needs to be
//!    propagated up to that `ast::Definition` as a RequiredTrait or solved in place.
//!
//! These types are mostly useful for their data they hold - they only have a few simple
//! methods on them for displaying them or converting between them.
use colored::Colorize;

use crate::cache::{ImplInfoId, ImplScopeId, ModuleCache, TraitInfoId, VariableId};
use crate::error::location::Location;
use crate::types::typechecker::find_all_typevars;
use crate::types::{typeprinter::TypePrinter, Type, TypeVariableId};

use std::collections::HashMap;
use std::fmt::Display;

use super::GeneralizedType;

/// Trait constraints do not map to anything. Instead,
/// they provide a way to map an impl through multiple
/// functions as if it were passed as arguments.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraitConstraintId(pub u32);

/// A (trait) ConstraintSignature contains the signature
/// of a trait constraint - the trait it refers to and the type
/// arguments it requires - in addition to a unique ID identifying
/// this constraint.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstraintSignature {
    pub trait_id: TraitInfoId,
    pub args: Vec<Type>,
    pub id: TraitConstraintId,
}

/// A trait required for a Definition to be compiled.
/// The specific impl to use is unknown to the definition since
/// different impls may be used at different callsites.
/// RequiredImpls are the callsite version of this.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequiredTrait {
    pub signature: ConstraintSignature,

    pub callsite: Callsite,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Callsite {
    /// This required trait originates from the given variable inside the function.
    /// That variable's definition should be replaced with the selected impl's.
    Direct(VariableId),

    /// This required trait originates from a definition outside of the current
    /// function where it has the given TraitConstraintId. It is required transitively
    /// to the current function by the given variable callsite.
    Indirect(VariableId, Vec<TraitConstraintId>),
}

impl Callsite {
    pub fn id(&self) -> VariableId {
        match self {
            Callsite::Direct(callsite) => *callsite,
            Callsite::Indirect(callsite, _) => *callsite,
        }
    }
}

/// An instantiated version of a RequiredTrait that is stored
/// in ast::Variable nodes. These point to specific impls to use.
#[derive(Debug, Clone)]
pub struct RequiredImpl {
    /// The specific trait impl to map the callsite to
    pub binding: ImplInfoId,
    pub callsite: Callsite,
}

/// The trait/impl constrait passed around during type inference.
/// - If at the end of a function an impl constraint contains a type
///   variable that escapes the current function (ie. is used in a
///   parameter or return type of the function) then it is turned into a
///   RequiredTrait for the function's DefinitionInfo.
/// - If no type variables escape outside of the current function then
///   an impl is searched for.
///   - If one is found it is stored in a RequiredImpl for the ast::Variable
///     node to reference while compiling that variable's definition.
///   - If an impl is not found a compile error for no matching impl is issued.
#[derive(Debug, Clone)]
pub struct TraitConstraint {
    pub required: RequiredTrait,
    pub scope: ImplScopeId,
}

pub type TraitConstraints = Vec<TraitConstraint>;

impl RequiredTrait {
    pub fn as_constraint(&self, scope: ImplScopeId, callsite: VariableId, id: TraitConstraintId) -> TraitConstraint {
        let mut required = self.clone();
        required.callsite = Callsite::Indirect(callsite, vec![self.signature.id]);
        required.signature.id = id;
        TraitConstraint { required, scope }
    }

    pub fn find_all_typevars<'b>(&self, cache: &ModuleCache<'b>) -> Vec<TypeVariableId> {
        self.signature.find_all_typevars(cache)
    }

    pub fn display<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> ConstraintSignaturePrinter<'a, 'b> {
        self.signature.display(cache)
    }

    #[allow(dead_code)]
    pub fn debug<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> ConstraintSignaturePrinter<'a, 'b> {
        self.signature.debug(cache)
    }
}

impl ConstraintSignature {
    pub fn find_all_typevars<'b>(&self, cache: &ModuleCache<'b>) -> Vec<TypeVariableId> {
        let mut typevars = vec![];
        for typ in &self.args {
            typevars.append(&mut find_all_typevars(typ, false, cache));
        }
        typevars
    }

    pub fn display<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> ConstraintSignaturePrinter<'a, 'b> {
        let mut typevar_names = HashMap::new();
        let mut current = 'a';
        let typevars = self.find_all_typevars(cache);

        for typevar in typevars {
            if typevar_names.get(&typevar).is_none() {
                typevar_names.insert(typevar, current.to_string());
                current = (current as u8 + 1) as char;
                assert!(current != 'z'); // TODO: wrap to aa, ab, ac...
            }
        }

        ConstraintSignaturePrinter { signature: self.clone(), typevar_names, debug: false, cache }
    }

    #[allow(dead_code)]
    pub fn debug<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> ConstraintSignaturePrinter<'a, 'b> {
        let mut typevar_names = HashMap::new();

        for typ in &self.args {
            let typevars = find_all_typevars(typ, false, cache);

            for typevar in typevars {
                if typevar_names.get(&typevar).is_none() {
                    typevar_names.insert(typevar, typevar.0.to_string());
                }
            }
        }

        ConstraintSignaturePrinter { signature: self.clone(), typevar_names, debug: true, cache }
    }
}

pub struct ConstraintSignaturePrinter<'a, 'b> {
    pub signature: ConstraintSignature,

    /// Maps unique type variable IDs to human readable names like a, b, c, etc.
    pub typevar_names: HashMap<TypeVariableId, String>,

    /// Controls whether to show some hidden data, like lifetimes of each ref
    pub debug: bool,

    pub cache: &'a ModuleCache<'b>,
}

impl<'a, 'b> Display for ConstraintSignaturePrinter<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let trait_info = &self.cache[self.signature.trait_id];

        write!(f, "{}", trait_info.name.blue())?;
        for arg in &self.signature.args {
            let typ = GeneralizedType::MonoType(arg.clone());
            let arg_printer = TypePrinter::new(typ, self.typevar_names.clone(), self.debug, self.cache);
            write!(f, " {}", arg_printer)?;
        }
        Ok(())
    }
}

impl TraitConstraint {
    /// Creates a TraitConstraint from the ConstraintSignature of the 'given' clause
    /// of a trait impl and the constraint from the impl itself. These constraints are always Callsite::Indirect.
    pub fn impl_given_constraint(
        inner_id: TraitConstraintId, trait_id: TraitInfoId, args: Vec<Type>, impl_constraint: &TraitConstraint,
        cache: &mut ModuleCache,
    ) -> TraitConstraint {
        let id = cache.next_trait_constraint_id();
        let signature = ConstraintSignature { trait_id, args, id };

        let callsite = match &impl_constraint.required.callsite {
            Callsite::Direct(var) => Callsite::Indirect(*var, vec![inner_id]),
            Callsite::Indirect(var, ids) => {
                let mut ids = ids.clone();
                ids.push(inner_id);
                Callsite::Indirect(*var, ids)
            },
        };

        let required = RequiredTrait { signature, callsite };
        TraitConstraint { required, scope: impl_constraint.scope }
    }

    pub fn trait_id(&self) -> TraitInfoId {
        self.required.signature.trait_id
    }

    pub fn args(&self) -> &[Type] {
        &self.required.signature.args
    }

    pub fn args_mut(&mut self) -> &mut [Type] {
        &mut self.required.signature.args
    }

    pub fn into_required_trait(self) -> RequiredTrait {
        self.required
    }

    pub fn into_required_impl(self, binding: ImplInfoId) -> RequiredImpl {
        RequiredImpl { binding, callsite: self.required.callsite }
    }

    /// Get the location of the callsite where this TraitConstraint arose from
    pub fn locate<'c>(&self, cache: &ModuleCache<'c>) -> Location<'c> {
        cache[self.required.callsite.id()].location
    }

    pub fn display<'a, 'c>(&self, cache: &'a ModuleCache<'c>) -> ConstraintSignaturePrinter<'a, 'c> {
        self.clone().into_required_trait().display(cache)
    }

    #[allow(dead_code)]
    pub fn debug<'a, 'c>(&self, cache: &'a ModuleCache<'c>) -> ConstraintSignaturePrinter<'a, 'c> {
        self.clone().into_required_trait().debug(cache)
    }
}
