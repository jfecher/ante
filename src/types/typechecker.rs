use crate::nameresolution::modulecache::{ ModuleCache, TraitInfoId };
use crate::parser::ast;
use crate::types::{ Type, Type::*, TypeVariableId, PrimitiveType };
use crate::types::typed::Typed;
use crate::util::*;
use crate::error::location::{ Location, Locatable };
use std::collections::HashMap;

// Note: most of this file is directly translated from:
// https://github.com/jfecher/algorithm-j

/// Replace any typevars found in typevars_to_replace with the
/// associated value in the same table, leave them otherwise
fn replace_typevars<'b>(typ: &Type, typevars_to_replace: &HashMap<TypeVariableId, TypeVariableId>, cache: &mut ModuleCache<'b>) -> Type {
    match typ {
        Primitive(p) => Primitive(*p),
        TypeVariable(id) => {
            if let Some(typ) = cache.type_bindings[id.0].as_ref() {
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
    }
}

/// specializes the polytype s by copying the term and replacing the
/// bound type variables consistently by new monotype variables
/// E.g.   instantiate (forall a b. a -> b -> a) = c -> d -> c
fn instantiate<'b>(s: &Type, cache: &mut ModuleCache<'b>) -> Type {
    // Note that the returned type is no longer a PolyType,
    // this means it is now monomorphic and not forall-quantified
    match s {
        TypeVariable(id) => {
            if let Some(typ) = cache.type_bindings[id.0].as_ref() {
                instantiate(&typ.clone(), cache)
            } else {
                TypeVariable(*id)
            }
        },
        ForAll(typevars, typ) => {
            let mut typevars_to_replace = HashMap::new();
            for var in typevars.iter().copied() {
                typevars_to_replace.insert(var, cache.next_type_variable_id());
            }
            replace_typevars(&typ, &typevars_to_replace, cache)
        },
        other => other.clone(),
    }
}

/// Can a monomorphic TypeVariable(id) be found inside this type?
fn occurs<'b>(id: TypeVariableId, typ: &Type, cache: &ModuleCache<'b>) -> bool {
    match typ {
        Primitive(_) => false,
        UserDefinedType(_) => false,

        TypeVariable(var_id) => {
            if let Some(binding) = cache.type_bindings[id.0].as_ref() {
                occurs(id, binding, cache)
            } else {
                id == *var_id
            }
        },
        Function(parameters, return_type) => {
            occurs(id, return_type, cache)
            || parameters.iter().any(|parameter| occurs(id, parameter, cache))
        },
        TypeApplication(typ, args) => {
            occurs(id, typ, cache)
            || args.iter().any(|arg| occurs(id, arg, cache))
        }
        ForAll(typevars, typ) => {
            !typevars.iter().any(|typevar| *typevar == id)
            && occurs(id, typ, cache)
        },
    }
}

fn unify<'b>(t1: &Type, t2: &Type, location: Location<'b>, cache: &mut ModuleCache<'b>) {
    match (t1, t2) {
        (Primitive(p1), Primitive(p2)) if p1 == p2 => (),

        // These two recursive calls to the bound typevar replace
        // the 'find' in the union-find algorithm 
        (TypeVariable(id), b) if cache.type_bindings[id.0].is_some() => {
            let a = cache.type_bindings[id.0].as_ref().unwrap().clone();
            unify(&a, b, location, cache);
        },
        (a, TypeVariable(id)) if cache.type_bindings[id.0].is_some() => {
            let b = cache.type_bindings[id.0].as_ref().unwrap().clone();
            unify(a, &b, location, cache);
        }

        (TypeVariable(a), b) => {
            // Create binding for boundTy that is currently empty.
            // Ensure not to create recursive bindings to the same variable
            if t1 != t2 { 
                if occurs(*a, b, cache) {
                    error!(location, "Cannot construct recursive type: {:?} = {:?}", t1, t2);
                } else {
                    cache.type_bindings[a.0] = Some(b.clone());
                }
            }
        },
        (a, TypeVariable(b)) => {
            // Create binding for boundTy that is currently empty.
            // Ensure not to create recursive bindings to the same variable
            if t1 != t2 { 
                if occurs(*b, a, cache) {
                    error!(location, "Cannot construct recursive type: {:?} = {:?}", t1, t2);
                } else {
                    cache.type_bindings[b.0] = Some(a.clone());
                }
            }
        },

        (Function(a_args, a_ret), Function(b_args, b_ret)) => {
            if a_args.len() != b_args.len() {
                error!(location, "Type mismatch between {} and {}", t1.display(cache), t2.display(cache));
            }

            fmap2(a_args, b_args, |a_arg, b_arg| unify(a_arg, b_arg, location, cache));
            unify(a_ret, b_ret, location, cache);
        },

        (TypeApplication(a_constructor, a_args), TypeApplication(b_constructor, b_args)) => {
            if a_args.len() != b_args.len() {
                error!(location, "Type mismatch between {} and {}", t1.display(cache), t2.display(cache));
            }

            unify(a_constructor, b_constructor, location, cache);
            fmap2(a_args, b_args, |a_arg, b_arg| unify(a_arg, b_arg, location, cache));
        },

        (ForAll(a_vars, a), ForAll(b_vars, b)) => {
            if a_vars.len() != b_vars.len() {
                error!(location, "Type mismatch between {} and {}", a.display(cache), b.display(cache))
            }
            unify(a, b, location, cache);
        },

        (a, b) => error!(location, "Type mismatch between {} and {}", a.display(cache), b.display(cache)),
    }
}

/// Collects all the type variables contained within typ into a Vec.
/// If monomorphic_only is true, any polymorphic type variables will be filtered out.
pub fn find_all_typevars<'a>(typ: &Type, monomorphic_only: bool, cache: &ModuleCache<'a>) -> Vec<TypeVariableId> {
    match typ {
        Primitive(_) => vec![],
        UserDefinedType(_) => vec![],
        TypeVariable(id) => {
            if let Some(t) = &cache.type_bindings[id.0] {
                find_all_typevars(t, monomorphic_only, cache)
            } else {
                vec![*id]
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

/// Find all typevars and wrap the type in a PolyType
/// e.g.  create_polytype (a -> b -> b) = forall a b. a -> b -> b
fn create_polytype<'a>(typ: &Type, cache: &ModuleCache<'a>) -> Type {
    let mut typevars = find_all_typevars(typ, true, cache);
    // TODO: This can be sped up, e.g. we wouldn't need to dedup at all if we didn't use a Vec
    typevars.sort();
    typevars.dedup();
    ForAll(typevars, Box::new(typ.clone()))
}

/// Returns true if we should generalize the given definition by wrapping
/// it in a ForAll type. Currently only true for lambda definitions.
fn should_generalize<'a>(definition: &ast::Definition<'a>) -> bool {
    match definition.expr.as_ref() {
        ast::Ast::Lambda(_) => {
            match definition.pattern.as_ref() {
                ast::Ast::Variable(_) => true,
                _ => false,
            }
        },
        _ => false,
    }
}


type TraitList = Vec<(TraitInfoId, Vec<Type>)>;

pub trait Inferable<'a> {
    fn infer_impl(&mut self, checker: &mut ModuleCache<'a>) -> (Type, TraitList);
}

pub fn infer<'a, T>(ast: &mut T, cache: &mut ModuleCache<'a>) -> (Type, TraitList)
    where T: Inferable<'a> + Typed
{
    let (typ, traits) = ast.infer_impl(cache);
    ast.set_type(typ.clone());
    (typ, traits)
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
            String(_) => (Type::Primitive(PrimitiveType::StringType), vec![]),
            Char(_) => (Type::Primitive(PrimitiveType::CharType), vec![]),
            Bool(_) => (Type::Primitive(PrimitiveType::BooleanType), vec![]),
            Unit => (Type::Primitive(PrimitiveType::UnitType), vec![]),
        }
    }
}

/* Var
 *   x : s ∊ env
 *   t = instantiate s
 *   -----------
 *   infer env x = t
 */
impl<'a> Inferable<'a> for ast::Variable<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let s = cache.definition_infos[self.definition.unwrap().0].typ.clone();
        let t = instantiate(&s, cache);
        (t, vec![])
    }
}

/* Abs
 *   arg1 = newvar ()
 *   arg2 = newvar ()
 *   ...
 *   argN = newvar ()
 *   infer body (env with x1:arg1 x2:arg2 ... xN:argN) = return_type
 *   -------------
 *   infer (fun x1 x2 ... xN -> e) env = arg1 arg2 ... argN -> return_type
 */
impl<'a> Inferable<'a> for ast::Lambda<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        // The newvars for the parameters are filled out during name resolution
        let params = fmap_mut(&mut self.args, |arg| infer(arg, cache).0);
        let (return_type, _) = infer(self.body.as_mut(), cache);
        (Function(params, Box::new(return_type)), vec![])
    }
}

/* App
 *   infer env f = t0
 *   infer env x = t1
 *   t' = newvar ()
 *   unify t0 (t1 -> t')
 *   ---------------
 *   infer env (f x) = t'
 */
impl<'a> Inferable<'a> for ast::FunctionCall<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (f, _) = infer(self.function.as_mut(), cache);
        let args = fmap_mut(&mut self.args, |arg| infer(arg, cache).0);
        let return_type = cache.next_type_variable();
        unify(&f, &Function(args, Box::new(return_type.clone())), self.location, cache);
        (return_type, vec![])
    }
}

/* Let
 *   infer env e0 = t
 *   infer (SMap.add x (create_polytype t) env) e1 = t'
 *   -----------------
 *   infer env (let x = e0 in e1) = t'
 */
impl<'a> Inferable<'a> for ast::Definition<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (rhs, _) = infer(self.expr.as_mut(), cache);
        let (lhs, _) = infer(self.pattern.as_mut(), cache);

        if should_generalize(self) {
            let generalized = create_polytype(&rhs, cache);
            unify(&lhs, &generalized, self.location, cache);
        } else {
            unify(&lhs, &rhs, self.location, cache);
        }

        let unit = Type::Primitive(PrimitiveType::UnitType);
        (unit, vec![])
    }
}

impl<'a> Inferable<'a> for ast::If<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (condition, _) = infer(self.condition.as_mut(), cache);
        let bool_type = Type::Primitive(PrimitiveType::BooleanType);
        unify(&condition, &bool_type, self.condition.locate(), cache);

        let (then, _) = infer(self.then.as_mut(), cache);
        if let Some(otherwise) = &mut self.otherwise {
            let (otherwise, _) = infer(otherwise.as_mut(), cache);
            unify(&then, &otherwise, self.location, cache);

            (then, vec![])
        } else {
            (Type::Primitive(PrimitiveType::UnitType), vec![])
        }
    }
}

impl<'a> Inferable<'a> for ast::Match<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (expression, _) = infer(self.expression.as_mut(), cache);
        let mut return_type = Type::Primitive(PrimitiveType::UnitType);

        if self.branches.len() >= 1 {
            let (pattern_type, _) = infer(&mut self.branches[0].0, cache);
            unify(&expression, &pattern_type, self.branches[0].0.locate(), cache);

            return_type = infer(&mut self.branches[0].1, cache).0;
            for (pattern, branch) in self.branches.iter_mut().skip(1) {
                let (pattern_type, _) = infer(pattern, cache);
                let (branch_type, _) = infer(branch, cache);
                unify(&expression, &pattern_type, pattern.locate(), cache);
                unify(&return_type, &branch_type, branch.locate(), cache);
            }
        }
        (return_type, vec![])
    }
}

impl<'a> Inferable<'a> for ast::TypeDefinition<'a> {
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TypeAnnotation<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        let (typ, traits)= infer(self.lhs.as_mut(), cache);
        unify(&typ, self.typ.as_mut().unwrap(), self.location, cache);
        (typ, traits)
    }
}

impl<'a> Inferable<'a> for ast::Import<'a> {
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        unimplemented!();
    }
}

impl<'a> Inferable<'a> for ast::TraitDefinition<'a> {
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        unimplemented!();
    }
}

impl<'a> Inferable<'a> for ast::TraitImpl<'a> {
    fn infer_impl(&mut self, _cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        unimplemented!();
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
        for statement in self.statements.iter_mut().take(ignore_len) {
            infer(statement, cache);
        }
        let (last_statement_type, _) = infer(self.statements.last_mut().unwrap(), cache);
        (last_statement_type, vec![])
    }
}
