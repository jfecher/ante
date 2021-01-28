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
use crate::cache::{ ModuleCache, TraitInfoId, DefinitionInfoId, ImplScopeId, TraitBindingId, VariableId };
use crate::types::{ Type, TypeVariableId, typeprinter::TypePrinter };
use crate::types::typechecker::find_all_typevars;
use crate::error::location::Location;

use colored::Colorize;

use std::collections::HashMap;
use std::fmt::Display;

/// A trait required for a Definition to be compiled.
/// The specific impl to use is unknown to the definition since
/// different impls may be used at different callsites.
/// RequiredImpls are the callsite version of this.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequiredTrait {
    pub trait_id: TraitInfoId,
    pub args: Vec<Type>,

    /// The original _callsite_ that this constraint arose from.
    /// Not the variable id from the name of the definition of the trait!
    /// This is None if stored in the original definition of the trait
    /// since there is no callsite yet in that case.
    pub origin: Option<VariableId>,
}

/// An instantiated version of a RequiredTrait that is stored
/// in ast::Variable nodes. These point to specific impls to use.
#[derive(Debug, Clone)]
pub struct RequiredImpl {
    /// The ast::Variable the Impl constraint arises from.
    /// This is not the Variable this RequiredImpl is
    /// contained within, it refers to the original variable the constraint
    /// arose from within the definition of the current variable. For example, in:
    ///
    /// 1| foo x =
    /// 2|    x + x
    /// 3|
    /// 4| foo 1
    ///
    /// The RequiredImpl is stored within the callsite variable (the `foo` on line 4)
    /// and the constraint origin is the `+` on line 2. In this example, the trait was
    /// generalized to be in foo's signature. If it were not (e.g. `1 + 2`) then the
    /// origin and callsite variables will be equal.
    pub origin: VariableId,

    pub args: Vec<Type>,

    /// The DefinitionInfoId (within a selected impl) to
    /// map the above VariableId to.
    pub binding: DefinitionInfoId,
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
    pub trait_id: TraitInfoId,
    pub args: Vec<Type>,
    pub scope: ImplScopeId,

    /// The ast::Variable to store the RequiredImpl within.
    /// This is different from RequiredImpl::origin which refers to the original
    /// variable the constraint arose from. The later is usually within the definition
    /// of the callsite variable. For example, in
    ///
    /// 1| foo x =
    /// 2|    x + x
    /// 3|
    /// 4| foo 1
    ///
    /// The callsite variable is the `foo` on line 4 and the constraint arises from
    /// the `+` on line 2. Callsites are where the compiler stores the trait information
    /// needed to continue compiling that definition with the given types.
    pub callsite: TraitBindingId,

    /// The origin of this TraitConstraint, to be stored in RequiredImpl::origin
    pub origin: VariableId,
}

pub type TraitConstraints = Vec<TraitConstraint>;

impl RequiredTrait {
    pub fn as_constraint(&self, scope: ImplScopeId, callsite_id: VariableId, callsite: TraitBindingId) -> TraitConstraint {
        TraitConstraint {
            trait_id: self.trait_id,
            args: self.args.clone(),
            scope,
            callsite,
            origin: self.origin.unwrap_or_else(|| {
                callsite_id
            }),
        }
    }

    pub fn find_all_typevars<'b>(&self, cache: &ModuleCache<'b>) -> Vec<TypeVariableId> {
        let mut typevars = vec![];
        for typ in self.args.iter() {
            typevars.append(&mut find_all_typevars(typ, false, cache));
        }
        typevars
    }

    pub fn display<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> RequiredTraitPrinter<'a, 'b> {
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

        RequiredTraitPrinter { required_trait: self.clone(), typevar_names, debug: false, cache }
    }

    #[allow(dead_code)]
    pub fn debug<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> RequiredTraitPrinter<'a, 'b> {
        let mut typevar_names = HashMap::new();

        for typ in self.args.iter() {
            let typevars = find_all_typevars(typ, false, cache);

            for typevar in typevars {
                if typevar_names.get(&typevar).is_none() {
                    typevar_names.insert(typevar, typevar.0.to_string());
                }
            }
        }

        RequiredTraitPrinter { required_trait: self.clone(), typevar_names, debug: true, cache }
    }
}

pub struct RequiredTraitPrinter<'a, 'b> {
    pub required_trait: RequiredTrait,

    /// Maps unique type variable IDs to human readable names like a, b, c, etc.
    pub typevar_names: HashMap<TypeVariableId, String>,

    /// Controls whether to show some hidden data, like lifetimes of each ref
    pub debug: bool,

    pub cache: &'a ModuleCache<'b>
}

impl<'a, 'b> Display for RequiredTraitPrinter<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let trait_info = &self.cache.trait_infos[self.required_trait.trait_id.0];

        write!(f, "{}", trait_info.name.blue())?;
        for arg in self.required_trait.args.iter() {
            let arg_printer =  TypePrinter::new(arg, self.typevar_names.clone(), self.debug, self.cache);
            write!(f, " {}", arg_printer)?;
        }
        Ok(())
    }
}

impl TraitConstraint {
    /// Member access traits are handled a bit differently, they are all implemented
    /// automatically so they don't need anything other than standard type inference
    /// to compile. Since they essentially don't have scopes, callsites, or origins
    /// care must be taken inside find_impl and Variable::codegen to avoid referring
    /// to these invalid values.
    pub fn member_access_constraint(trait_id: TraitInfoId, args: Vec<Type>, callsite: TraitBindingId) -> TraitConstraint {
        TraitConstraint {
            trait_id,
            args,
            scope: ImplScopeId(0),
            callsite,
            origin: VariableId(0),
        }
    }

    pub fn is_member_access<'c>(&self, cache: &ModuleCache<'c>) -> bool {
        cache.trait_infos[self.trait_id.0].is_member_access()
    }

    /// Each integer literal without a type suffix is given the generic type
    /// "a given Int a". This function returns a TraitConstraint for this
    /// builtin Int trait to be resolved later in typechecking to a specific
    /// integer type or propagataed to the function signature to take any Int.
    pub fn int_constraint<'c>(arg: TypeVariableId, cache: &ModuleCache<'c>) -> TraitConstraint {
        TraitConstraint {
            trait_id: cache.int_trait,
            args: vec![Type::TypeVariable(arg)],
            scope: ImplScopeId(0),
            callsite: TraitBindingId(0),
            origin: VariableId(0),
        }
    }

    pub fn is_int_constraint<'c>(&self, cache: &ModuleCache<'c>) -> bool {
        self.trait_id == cache.int_trait
    }

    pub fn as_required_trait(self) -> RequiredTrait {
        RequiredTrait {
            trait_id: self.trait_id,
            args: self.args,
            origin: Some(self.origin),
        }
    }

    pub fn as_required_impl(&self, binding: DefinitionInfoId) -> RequiredImpl {
        RequiredImpl {
            args: self.args.clone(),
            origin: self.origin,
            binding,
        }
    }

    /// Get the location of the callsite where this TraitConstraint arose from
    pub fn locate<'c>(&self, cache: &ModuleCache<'c>) -> Location<'c> {
        cache.trait_bindings[self.callsite.0].location
    }

    pub fn display<'a, 'c>(&self, cache: &'a ModuleCache<'c>) -> RequiredTraitPrinter<'a, 'c> {
        self.clone().as_required_trait().display(cache)
    }

    #[allow(dead_code)]
    pub fn debug<'a, 'c>(&self, cache: &'a ModuleCache<'c>) -> RequiredTraitPrinter<'a, 'c> {
        self.clone().as_required_trait().debug(cache)
    }
}

impl RequiredImpl {
    #[allow(dead_code)]
    pub fn debug<'c>(&self, cache: &ModuleCache<'c>) -> String {
        let name = &cache.definition_infos[self.binding.0].name;
        let args = Type::Tuple(self.args.clone());
        format!("{} with args {}", name, args.display(cache))
    }
}
