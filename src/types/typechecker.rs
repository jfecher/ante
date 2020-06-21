use crate::nameresolution::modulecache::{ ModuleCache, TraitInfoId, DefinitionInfoId, DefinitionNode };
use crate::parser::ast;
use crate::types::{ Type, Type::*, TypeVariableId, PrimitiveType, LetBindingLevel, TypeBinding::* };
use crate::types::typed::Typed;
use crate::util::*;
use crate::error::location::{ Location, Locatable };

use std::collections::HashMap;
use std::sync::atomic::{ AtomicUsize, Ordering };

// Note: most of this file is directly translated from:
// https://github.com/jfecher/algorithm-j


pub static CURRENT_LEVEL: AtomicUsize = AtomicUsize::new(1);


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

/// specializes the polytype s by copying the term and replacing the
/// bound type variables consistently by new monotype variables
/// E.g.   instantiate (forall a b. a -> b -> a) = c -> d -> c
fn instantiate<'b>(s: &Type, cache: &mut ModuleCache<'b>) -> Type {
    // Note that the returned type is no longer a PolyType,
    // this means it is now monomorphic and not forall-quantified
    match s {
        TypeVariable(id) => {
            if let Bound(typ) = &cache.type_bindings[id.0] {
                instantiate(&typ.clone(), cache)
            } else {
                TypeVariable(*id)
            }
        },
        ForAll(typevars, typ) => {
            let mut typevars_to_replace = HashMap::new();
            for var in typevars.iter().copied() {
                typevars_to_replace.insert(var, next_type_variable_id(cache));
            }
            replace_typevars(&typ, &typevars_to_replace, cache)
        },
        other => other.clone(),
    }
}

/// Can a monomorphic TypeVariable(id) be found inside this type?
/// This will mutate any typevars found to increase their LetBindingLevel.
/// Doing so increases the lifetime of the typevariable and lets us keep
/// track of which type variables to generalize later on. It also means
/// that occurs should only be called during unification however.
fn occurs<'b>(id: TypeVariableId, level: LetBindingLevel, typ: &Type, cache: &mut ModuleCache<'b>) -> bool {
    match typ {
        Primitive(_) => false,
        UserDefinedType(_) => false,

        TypeVariable(var_id) => {
            match &cache.type_bindings[id.0] {
                Bound(binding) => occurs(id, level, &binding.clone(), cache),
                Unbound(original_level) => {
                    let min_level = std::cmp::min(level, *original_level);
                    cache.type_bindings[id.0] = Unbound(min_level);
                    id == *var_id
                }
            }
        },
        Function(parameters, return_type) => {
            occurs(id, level, return_type, cache)
            || parameters.iter().any(|parameter| occurs(id, level, parameter, cache))
        },
        TypeApplication(typ, args) => {
            occurs(id, level, typ, cache)
            || args.iter().any(|arg| occurs(id, level, arg, cache))
        }
        ForAll(typevars, typ) => {
            !typevars.iter().any(|typevar| *typevar == id)
            && occurs(id, level, typ, cache)
        },
    }
}

fn unify<'b>(t1: &Type, t2: &Type, location: Location<'b>, cache: &mut ModuleCache<'b>) {
    match (t1, t2) {
        (Primitive(p1), Primitive(p2)) if p1 == p2 => (),

        // Any type variable can be bound or unbound.
        // - If bound: unify the bound type with the other type.
        // - If unbound: 'unify' the LetBindingLevel of the type variable by setting
        //   it to the minimum scope of type variables in b. This happens within the occurs check.
        //   The unification of the LetBindingLevel here is a form of lifetime inference for the
        //   typevar and is used during generalization to determine which variables to generalize.
        (TypeVariable(id), b) => {
            match &cache.type_bindings[id.0] {
                Bound(a) => {
                    unify(&a.clone(), b, location, cache);
                },
                Unbound(a_level) => {
                    // Create binding for boundTy that is currently empty.
                    // Ensure not to create recursive bindings to the same variable
                    if t1 != t2 { 
                        if occurs(*id, *a_level, b, cache) {
                            error!(location, "Cannot construct recursive type: {:?} = {:?}", t1, t2);
                        } else {
                            cache.type_bindings[id.0] = Bound(b.clone());
                        }
                    }
                }
            }
        },

        (a, TypeVariable(id)) => {
            match &cache.type_bindings[id.0] {
                Bound(b) => {
                    unify(a, &b.clone(), location, cache);
                },
                Unbound(b_level) => {
                    // Create binding for boundTy that is currently empty.
                    // Ensure not to create recursive bindings to the same variable
                    if t1 != t2 { 
                        if occurs(*id, *b_level, a, cache) {
                            error!(location, "Cannot construct recursive type: {:?} = {:?}", t1, t2);
                        } else {
                            cache.type_bindings[id.0] = Bound(a.clone());
                        }
                    }
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
            match &cache.type_bindings[id.0] {
                Bound(t) => find_all_typevars(t, monomorphic_only, cache),
                Unbound(level) => {
                    if level.0 > CURRENT_LEVEL.fetch_or(0, Ordering::SeqCst) || !monomorphic_only {
                        vec![*id]
                    } else {
                        println!("Not generalizing {}, because {} <= cur level of {}", id.0, level.0, CURRENT_LEVEL.fetch_or(0, Ordering::SeqCst));
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

fn infer_nested_definition<'a>(definition_id: DefinitionInfoId, cache: &mut ModuleCache<'a>) -> Type {
    let level = LetBindingLevel(CURRENT_LEVEL.fetch_or(0, Ordering::SeqCst));
    let typevar = cache.next_type_variable(level);
    let info = &mut cache.definition_infos[definition_id.0];
    let definition = info.definition.as_mut().unwrap();
    // Mark the definition with a fresh typevar for recursive references
    info.typ = Some(typevar.clone());

    match definition {
        DefinitionNode::Definition(definition) => {
            let definition = trustme::extend_lifetime_mut(*definition);
            infer(definition, cache)
        }
        DefinitionNode::TraitDefinition(definition) => {
            let definition = trustme::extend_lifetime_mut(*definition);
            infer(definition, cache)
        }
    };

    let info = &mut cache.definition_infos[definition_id.0];
    info.typ.clone().unwrap()
}

/// Binds a given type to an irrefutable pattern, recursing on the pattern and verifying
/// that it is indeed irrefutable. If should_generalize is true, this generalizes the type given
/// to any variable encountered.
fn bind_irrefutable_pattern<'a>(ast: &ast::Ast<'a>, typ: &Type, should_generalize: bool, cache: &mut ModuleCache<'a>) {
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
            let typ = if should_generalize { generalize(typ, cache) } else { typ.clone() };
            let info = &mut cache.definition_infos[variable.definition.unwrap().0];

            // assert that the pattern doesn't yet have a type or that it is
            // otherwise an unbound type variable.
            match info.typ {
                Some(TypeVariable(id)) => {
                    match &cache.type_bindings[id.0] {
                        Bound(_) => unreachable!(),
                        _ => (),
                    }
                }
                _ => (),
            }

            info.typ = Some(typ);
        },
        TypeAnnotation(annotation) => {
            unify(typ, annotation.typ.as_ref().unwrap(), ast.locate(), cache);
            bind_irrefutable_pattern(annotation.lhs.as_ref(), typ, should_generalize, cache);
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
fn bind_irrefutable_pattern_in_impl<'a>(ast: &ast::Ast<'a>, trait_id: TraitInfoId, cache: &mut ModuleCache<'a>) {
    use ast::Ast::*;
    match ast {
        Variable(variable) => {
            let name = variable.to_string();
            let trait_type = lookup_definition_type_in_trait(&name, trait_id, cache);

            let info = &mut cache.definition_infos[variable.definition.unwrap().0];
            let impl_type = info.typ.clone().unwrap();
            println!("Unifying {} = {}", trait_type.debug(cache), impl_type.debug(cache));
            unify(&trait_type, &impl_type, ast.locate(), cache);
        },
        TypeAnnotation(annotation) => {
            bind_irrefutable_pattern_in_impl(annotation.lhs.as_ref(), trait_id, cache);
        },
        _ => {
            error!(ast.locate(), "Invalid syntax in irrefutable pattern in trait impl, expected a name or a tuple of names");
        }
    }
}

type TraitList = Vec<(TraitInfoId, Vec<Type>)>;

pub trait Inferable<'a> {
    fn infer_impl(&mut self, checker: &mut ModuleCache<'a>) -> (Type, TraitList);
}

pub fn infer<'a, T>(ast: &mut T, cache: &mut ModuleCache<'a>) -> (Type, TraitList)
    where T: Inferable<'a> + Typed
{
    match ast.get_type() {
        Some(typ) => (typ.clone(), vec![]),
        None => {
            let (typ, traits) = ast.infer_impl(cache);
            ast.set_type(typ.clone());
            (typ, traits)
        },
    }
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
 *   x : s ∊ cache
 *   t = instantiate s
 *   -----------
 *   infer cache x = t
 */
impl<'a> Inferable<'a> for ast::Variable<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        // TODO: we redeclare info 4 times in this function to get around single-mutability
        // issues. There should be another way.
        let info = &cache.definition_infos[self.definition.unwrap().0];

        // Lookup the type of the definition.
        // We'll need to recursively infer the type if it is not found
        let s = match &info.typ {
            Some(typ) => typ.clone(),
            None => {
                // If the variable has a definition we can infer from then use that
                // to determine the type, otherwise fill in a type variable for it.
                let typ = if info.definition.is_some() {
                    infer_nested_definition(self.definition.unwrap(), cache)
                } else {
                    next_type_variable(cache)
                };
                let info = &mut cache.definition_infos[self.definition.unwrap().0];
                info.typ = Some(typ.clone());
                typ
            },
        };

        let t = instantiate(&s, cache);
        (t, vec![])
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
        let arg_types = fmap_mut(&mut self.args, |arg| infer(arg, cache).0);
        let (return_type, _) = infer(self.body.as_mut(), cache);
        (Function(arg_types, Box::new(return_type)), vec![])
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
        let (f, _) = infer(self.function.as_mut(), cache);
        let args = fmap_mut(&mut self.args, |arg| infer(arg, cache).0);
        let return_type = next_type_variable(cache);
        unify(&f, &Function(args, Box::new(return_type.clone())), self.location, cache);
        (return_type, vec![])
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
        CURRENT_LEVEL.fetch_add(1, Ordering::SeqCst);
        let (t, _) = infer(self.expr.as_mut(), cache);
        CURRENT_LEVEL.fetch_sub(1, Ordering::SeqCst);

        bind_irrefutable_pattern(self.pattern.as_ref(), &t, true, cache);

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

            bind_irrefutable_pattern(declaration.lhs.as_ref(), rhs, true, cache);
        }
        (Type::Primitive(PrimitiveType::UnitType), vec![])
    }
}

impl<'a> Inferable<'a> for ast::TraitImpl<'a> {
    fn infer_impl(&mut self, cache: &mut ModuleCache<'a>) -> (Type, TraitList) {
        for definition in self.definitions.iter_mut() {
            infer(definition, cache);
            bind_irrefutable_pattern_in_impl(definition.pattern.as_ref(), self.trait_info.unwrap(), cache);
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
        for statement in self.statements.iter_mut().take(ignore_len) {
            infer(statement, cache);
        }
        let (last_statement_type, _) = infer(self.statements.last_mut().unwrap(), cache);
        (last_statement_type, vec![])
    }
}
