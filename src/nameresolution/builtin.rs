use crate::cache::{ ModuleCache, DefinitionInfoId, TraitInfoId, ImplScopeId, ImplInfoId };
use crate::error::location::Location;
use crate::nameresolution::{ NameResolver, MAX_BINDING_LEVEL };
use crate::lexer::token::Token;
use crate::types::{ Type, PrimitiveType, TypeInfoBody, TypeVariableId, Field, STRING_TYPE };

/// A struct containing all the definitions, traits, and impls that are builtin to
/// the compiler itself rather than being included in the stdlib or prelude.
/// These are things like the primitive numeric operators which must be implemented
/// as lower-level LLVM instructions directly. In addition to extern C definitions,
/// these make up the core building blocks upon which everything else in the stdlib is built upon.
#[derive(Debug, Default)]
pub struct Builtins {
    pub definitions: Vec<(String, DefinitionInfoId)>,
    pub traits: Vec<(String, TraitInfoId)>,
    pub impls: Vec<(TraitInfoId, ImplInfoId)>,
}

pub fn define_builtins<'a>(cache: &mut ModuleCache<'a>) {
    use Type::{ Function, ForAll, TypeVariable };

    Builtins::define_string(cache);
    let mut builtins = Builtins::default();

    // define builtin numeric traits
    let a = cache.next_type_variable_id(MAX_BINDING_LEVEL);
    let args = vec![a];

    let add_trait_id = builtins.add_builtin_trait("Add", args.clone(), cache);
    let sub_trait_id = builtins.add_builtin_trait("Sub", args.clone(), cache);
    let mul_trait_id = builtins.add_builtin_trait("Mul", args.clone(), cache);
    let div_trait_id = builtins.add_builtin_trait("Div", args.clone(), cache);
    let mod_trait_id = builtins.add_builtin_trait("Mod", args.clone(), cache);

    let impl_scope_id = cache.push_impl_scope();
    let typ = ForAll(vec![a], Box::new(Function(vec![TypeVariable(a), TypeVariable(a)], Box::new(TypeVariable(a)))));

    builtins.add_builtin_definition(&Token::Add.to_string(),      add_trait_id, typ.clone(), impl_scope_id, cache);
    builtins.add_builtin_definition(&Token::Subtract.to_string(), sub_trait_id, typ.clone(), impl_scope_id, cache);
    builtins.add_builtin_definition(&Token::Multiply.to_string(), mul_trait_id, typ.clone(), impl_scope_id, cache);
    builtins.add_builtin_definition(&Token::Divide.to_string(),   div_trait_id, typ.clone(), impl_scope_id, cache);
    builtins.add_builtin_definition(&Token::Modulus.to_string(),  mod_trait_id, typ.clone(), impl_scope_id, cache);

    builtins.add_builtin_impls(impl_scope_id, cache);
    cache.builtins = builtins;
}

impl Builtins {
    pub fn import_builtins(&self, resolver: &mut NameResolver) {
        let scope = resolver.current_scope();

        // If Builtins stores a Scope internally we'd get this import functionality for free
        // but we'd trade a bit of runtime performance in using hashmaps and checking for
        // duplicates when we know there won't be any.
        for (name, id) in self.definitions.iter() {
            scope.definitions.insert(name.clone(), *id);
        }

        for (name, id) in self.traits.iter() {
            scope.traits.insert(name.clone(), *id);
        }

        for (trait_id, impl_id) in self.impls.iter() {
            match scope.impls.get_mut(trait_id) {
                Some(impls) => impls.push(*impl_id),
                None => {
                    scope.impls.insert(*trait_id, vec![*impl_id]);
                }
            }
        }
    }

    fn add_builtin_definition<'a>(&mut self, name: &str, trait_id: TraitInfoId, typ: Type, impl_scope_id: ImplScopeId, cache: &mut ModuleCache<'a>) {
        let id = cache.push_definition(name.into(), Location::builtin());
        let info = &mut cache.definition_infos[id.0];
        info.uses += 1; // Suppress warnings of builtin operators being unused
        info.typ = Some(typ);
        NameResolver::attach_to_trait(id, trait_id, impl_scope_id, cache);
        self.definitions.push((name.into(), id));
    }

    fn add_builtin_trait<'a>(&mut self, name: &str, args: Vec<TypeVariableId>, cache: &mut ModuleCache<'a>) -> TraitInfoId {
        let id = cache.push_trait_definition(name.into(), args, vec![], Location::builtin());
        self.traits.push((name.into(), id));
        id
    }

    /// Defining the 'string' type is a bit different than most other builtins. Since 'string' has
    /// its own dedicated keyword it need not be imported into scope like each impl of + or - does.
    fn define_string<'a>(cache: &mut ModuleCache<'a>) {
        let location = Location::builtin();

        let ref_type = Type::Primitive(PrimitiveType::ReferenceType);
        let char_type = Type::Primitive(PrimitiveType::CharType);
        let c_string_type = Type::TypeApplication(Box::new(ref_type), vec![char_type]);

        let length_type = Type::Primitive(PrimitiveType::IntegerType);

        let string = cache.push_type_info("string".into(), vec![], location);
        assert!(string == STRING_TYPE);

        cache.type_infos[string.0].body = TypeInfoBody::Struct(vec![
            Field { name: "c_string".into(), field_type: c_string_type, location },
            Field { name: "length".into(),   field_type: length_type,   location },
        ]);
    }

    fn add_builtin_impls<'a>(&mut self, impl_scope_id: ImplScopeId, cache: &mut ModuleCache<'a>) {
        let int = vec![Type::Primitive(PrimitiveType::IntegerType)];
        let float = vec![Type::Primitive(PrimitiveType::FloatType)];

        for (_, trait_id) in self.traits.iter() {
            // TODO: Leaving the DefinitionInfoIds Vec empty here will cause asserts
            // later in the compiler if builtins are not properly handled
            let int_impl = cache.push_trait_impl(*trait_id, int.clone(), vec![], Location::builtin());
            let float_impl = cache.push_trait_impl(*trait_id, float.clone(), vec![], Location::builtin());

            self.impls.push((*trait_id, int_impl));
            self.impls.push((*trait_id, float_impl));

            cache.impl_scopes[impl_scope_id.0].push(int_impl);
            cache.impl_scopes[impl_scope_id.0].push(float_impl);
        }
    }
}
