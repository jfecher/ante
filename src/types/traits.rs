use crate::cache::{ ModuleCache, TraitInfoId, DefinitionInfoId, ImplScopeId, TraitBindingId, VariableId };
use crate::types::{ Type, TypeVariableId, typeprinter::TypePrinter };
use crate::types::typechecker::find_all_typevars;

use colored::Colorize;

use std::collections::HashMap;
use std::fmt::Display;

/// A trait required for a Definition to be compiled.
/// The specific impl to use is unknown to the definition since
/// different impls may be used at different callsites.
/// RequiredImpls are the callsite version of this.
#[derive(Debug, Clone)]
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
///   parameter or return type of the function) then it turned into a
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
    /// the `+` on line 2.
    pub callsite: TraitBindingId,

    /// The origin of this TraitConstraint, to be stored in RequiredImpl::origin
    pub origin: VariableId,
}

pub type TraitConstraints = Vec<TraitConstraint>;

/// Provide some pretty-printing functionality for golden tests
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

        RequiredTraitPrinter { required_trait: self.clone(), typevar_names, cache }
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

        RequiredTraitPrinter { required_trait: self.clone(), typevar_names, cache }
    }
}

pub struct RequiredTraitPrinter<'a, 'b> {
    pub required_trait: RequiredTrait,

    /// Maps unique type variable IDs to human readable names like a, b, c, etc.
    pub typevar_names: HashMap<TypeVariableId, String>,

    pub cache: &'a ModuleCache<'b>
}

impl<'a, 'b> Display for RequiredTraitPrinter<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let trait_info = &self.cache.trait_infos[self.required_trait.trait_id.0];

        write!(f, "{}", trait_info.name.blue())?;
        for arg in self.required_trait.args.iter() {
            let arg_printer =  TypePrinter::new(arg, self.typevar_names.clone(), self.cache);
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
    pub fn member_access_constraint(trait_id: TraitInfoId, args: Vec<Type>) -> TraitConstraint {
        TraitConstraint {
            trait_id,
            args,
            scope: ImplScopeId(0),
            callsite: TraitBindingId(0),
            origin: VariableId(0),
        }
    }

    pub fn as_required_trait(self) -> RequiredTrait {
        RequiredTrait {
            trait_id: self.trait_id,
            args: self.args,
            origin: Some(self.origin),
        }
    }

    pub fn as_required_impl(self, binding: DefinitionInfoId) -> RequiredImpl {
        RequiredImpl {
            args: self.args,
            origin: self.origin,
            binding,
        }
    }

    pub fn display<'a, 'c>(&self, cache: &'a ModuleCache<'c>) -> RequiredTraitPrinter<'a, 'c> {
        self.clone().as_required_trait().display(cache)
    }
}
