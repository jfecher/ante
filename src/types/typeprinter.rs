use crate::types::{ Type, TypeVariableId, TypeInfoId, PrimitiveType, TypeBinding };
use crate::types::traits::{ RequiredTrait, RequiredTraitPrinter };
use crate::types::typechecker::find_all_typevars;
use crate::cache::ModuleCache;
use crate::util::join_with;

use std::collections::HashMap;
use std::fmt::{ Display, Debug, Formatter };

use colored::*;

pub struct TypePrinter<'a, 'b> {
    typ: &'a Type,

    /// Maps unique type variable IDs to human readable names like a, b, c, etc.
    typevar_names: HashMap<TypeVariableId, String>,

    cache: &'a ModuleCache<'b>
}

impl<'a, 'b> Display for TypePrinter<'a, 'b> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_type(&self.typ, f)
    }
}

impl<'a, 'b> Debug for TypePrinter<'a, 'b> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.fmt_type(&self.typ, f)
    }
}

fn fill_typevar_map(map: &mut HashMap<TypeVariableId, String>, typevars: Vec<TypeVariableId>, current: &mut char) {
    for typevar in typevars {
        if !map.contains_key(&typevar) {
            map.insert(typevar, current.to_string());
            *current = (*current as u8 + 1) as char;
            assert!(*current != 'z'); // TODO: wrap to aa, ab, ac...
        }
    }
}

pub fn show_type_and_traits<'b>(typ: &Type, traits: &[RequiredTrait], cache: &ModuleCache<'b>) {
    let mut map = HashMap::new();
    let mut current = 'a';

    let typevars = find_all_typevars(typ, false, cache);
    fill_typevar_map(&mut map, typevars, &mut current);

    print!("{}", TypePrinter { typ, cache, typevar_names: map.clone() });

    let mut traits = traits.iter().map(|required_trait| {
        fill_typevar_map(&mut map, required_trait.find_all_typevars(cache), &mut current);
        RequiredTraitPrinter { required_trait: required_trait.clone(), cache, typevar_names: map.clone() }
            .to_string()
    }).collect::<Vec<String>>();

    // Remove "duplicate" traits so users don't see `given Add a, Add a`.
    // These contain usage information that is different within them but this
    // isn't used in their Display impl so they look like duplicates.
    traits.sort();
    traits.dedup();

    if !traits.is_empty() {
        print!("\n  given {}", join_with(&traits, ", "));
    }

    println!("");
}

impl<'a, 'b> TypePrinter<'a, 'b> {
    pub fn new(typ: &'a Type, typevar_names: HashMap<TypeVariableId, String>, cache: &'a ModuleCache<'b>) -> TypePrinter<'a, 'b> {
        TypePrinter { typ, typevar_names, cache }
    }

    fn fmt_type(&self, typ: &Type, f: &mut Formatter) -> std::fmt::Result {
        match typ {
            Type::Primitive(primitive) => self.fmt_primitive(primitive, f),
            Type::Function(params, ret) => self.fmt_function(params, ret, f),
            Type::TypeVariable(id) => self.fmt_type_variable(*id, f),
            Type::UserDefinedType(id) => self.fmt_user_defined_type(*id, f),
            Type::TypeApplication(constructor, args) => self.fmt_type_application(constructor, args, f),
            Type::Tuple(elements) => self.fmt_tuple(elements, f),
            Type::ForAll(typevars, typ) => self.fmt_forall(typevars, typ, f),
        }
    }

    fn fmt_primitive(&self, primitive: &PrimitiveType, f: &mut Formatter) -> std::fmt::Result {
        match primitive {
            PrimitiveType::IntegerType(kind) => write!(f, "{}", kind.to_string().blue()),
            PrimitiveType::FloatType => write!(f, "{}", "float".blue()),
            PrimitiveType::CharType => write!(f, "{}", "char".blue()),
            PrimitiveType::BooleanType => write!(f, "{}", "bool".blue()),
            PrimitiveType::UnitType => write!(f, "{}", "unit".blue()),
            PrimitiveType::ReferenceType => write!(f, "{}", "ref".blue()),
        }
    }

    fn fmt_function(&self, params: &Vec<Type>, ret: &Box<Type>, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", "(".blue())?;
        for param in params.iter() {
            self.fmt_type(param, f)?;
            write!(f, " ")?;
        }
        write!(f, "{}", "-> ".blue())?;
        self.fmt_type(ret.as_ref(), f)?;
        write!(f, "{}", ")".blue())
    }

    fn fmt_type_variable(&self, id: TypeVariableId, f: &mut Formatter) -> std::fmt::Result {
        match &self.cache.type_bindings[id.0] {
            TypeBinding::Bound(typ) => self.fmt_type(typ, f),
            TypeBinding::Unbound(..) => {
                let default = "?".to_string();
                let name = self.typevar_names.get(&id).unwrap_or(&default).blue();
                write!(f, "{}", name)
            }
        }
    }

    fn fmt_user_defined_type(&self, id: TypeInfoId, f: &mut Formatter) -> std::fmt::Result {
        let name = self.cache.type_infos[id.0].name.blue();
        write!(f, "{}", name)
    }

    fn fmt_type_application(&self, constructor: &Box<Type>, args: &Vec<Type>, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", "(".blue())?;
        self.fmt_type(constructor.as_ref(), f)?;
        for arg in args.iter() {
            write!(f, " ")?;
            self.fmt_type(arg, f)?;
        }
        write!(f, "{}", ")".blue())
    }

    fn fmt_tuple(&self, elements: &[Type], f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", "(".blue())?;
        for (i, arg) in elements.iter().enumerate() {
            self.fmt_type(arg, f)?;

            if i + 1 < elements.len() {
                write!(f, ", ")?;
            }
        }
        write!(f, "{}", ")".blue())
    }

    fn fmt_forall(&self, typevars: &Vec<TypeVariableId>, typ: &Box<Type>, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", "(forall".blue())?;
        for typevar in typevars.iter() {
            write!(f, " ")?;
            self.fmt_type_variable(*typevar, f)?;
        }
        write!(f, "{}", ". ".blue())?;
        self.fmt_type(typ.as_ref(), f)?;
        write!(f, "{}", ")".blue())
    }
}
