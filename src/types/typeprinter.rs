use crate::types::{ Type, TypeVariableId, TypeInfoId, PrimitiveType };
use crate::nameresolution::modulecache::ModuleCache;

use std::collections::HashMap;
use std::fmt::{ Display, Formatter };

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
            Type::ForAll(typevars, typ) => self.fmt_forall(typevars, typ, f),
        }
    }

    fn fmt_primitive(&self, primitive: &PrimitiveType, f: &mut Formatter) -> std::fmt::Result {
        match primitive {
            PrimitiveType::IntegerType => write!(f, "{}", "int".blue()),
            PrimitiveType::FloatType => write!(f, "{}", "float".blue()),
            PrimitiveType::CharType => write!(f, "{}", "char".blue()),
            PrimitiveType::StringType => write!(f, "{}", "string".blue()),
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
        if let Some(typ) = &self.cache.type_bindings[id.0] {
            // Bound
            self.fmt_type(typ, f)
        } else { // Unbound
            let name = self.typevar_names[&id].blue();
            write!(f, "{}", name)
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
