use crate::cache::{ ModuleCache, TraitInfoId, ImplBindingId, ImplScopeId };
use crate::types::{ Type, TypeVariableId, typeprinter::TypePrinter };
use crate::types::typechecker::find_all_typevars;

use colored::Colorize;

use std::collections::HashMap;
use std::fmt::Display;

pub type TraitList = Vec<Impl>;

#[derive(Debug, Clone)]
pub struct Impl {
    pub trait_id: TraitInfoId,
    pub scope: ImplScopeId,
    pub args: Vec<Type>,

    // Using the above fields the type checker needs to unify args
    // to eventually match this UnknownTraitImpl to a concrete impl.
    pub binding: ImplBindingId,
}

impl Impl {
    pub fn new(trait_id: TraitInfoId, scope: ImplScopeId, binding: ImplBindingId, args: Vec<Type>) -> Impl {
        Impl { trait_id, scope, args, binding }
    }

    pub fn display<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> ImplPrinter<'a, 'b> {
        let mut typevar_names = HashMap::new();
        let mut current = 'a';

        for typ in self.args.iter() {
            let typevars = find_all_typevars(typ, false, cache);

            for typevar in typevars {
                if typevar_names.get(&typevar).is_none() {
                    typevar_names.insert(typevar, current.to_string());
                    current = (current as u8 + 1) as char;
                    assert!(current != 'z'); // TODO: wrap to aa, ab, ac...
                }
            }
        }

        ImplPrinter { trait_impl: self.clone(), debug: false, typevar_names, cache }
    }

    #[allow(dead_code)]
    pub fn debug<'a, 'b>(&self, cache: &'a ModuleCache<'b>) -> ImplPrinter<'a, 'b> {
        let mut typevar_names = HashMap::new();

        for typ in self.args.iter() {
            let typevars = find_all_typevars(typ, false, cache);

            for typevar in typevars {
                if typevar_names.get(&typevar).is_none() {
                    typevar_names.insert(typevar, typevar.0.to_string());
                }
            }
        }

        ImplPrinter { trait_impl: self.clone(), debug: true, typevar_names, cache }
    }
}

pub struct ImplPrinter<'a, 'b> {
    trait_impl: Impl,

    /// True if this should also print what this impl is bound to/if it has a binding.
    debug: bool,

    /// Maps unique type variable IDs to human readable names like a, b, c, etc.
    typevar_names: HashMap<TypeVariableId, String>,

    cache: &'a ModuleCache<'b>
}

impl<'a, 'b> Display for ImplPrinter<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let trait_info = &self.cache.trait_infos[self.trait_impl.trait_id.0];

        write!(f, "{}", trait_info.name.blue())?;
        for arg in self.trait_impl.args.iter() {
            let arg_printer =  TypePrinter::new(arg, self.typevar_names.clone(), self.cache);

            let s = format!("{}", arg_printer);
            if s.contains(" ") {
                write!(f, " ({})", s)?;
            } else {
                write!(f, " {}", s)?;
            }
        }

        // Print the impl this impl is bound to
        if self.debug {
            write!(f, " => ")?;
            if !self.cache.impl_bindings.is_empty() {
                if let Some(id) = &self.cache.impl_bindings[self.trait_impl.binding.0] {
                    // A ImplInfo can't be printed directly so its wrapped in
                    // an Impl here as a workaround
                    let impl_info = &self.cache.impl_infos[id.0];
                    let trait_impl = Impl::new(impl_info.trait_id, self.trait_impl.scope, ImplBindingId(0), impl_info.typeargs.clone());
                    write!(f, "{}", trait_impl.display(self.cache))?;
                } else {
                    write!(f, "?")?;
                }
            } else {
                // TODO: Remove ImplBindingId(0) usage
                write!(f, "??")?;
            }
        }
        Ok(())
    }
}
