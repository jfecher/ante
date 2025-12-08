use std::{collections::BTreeMap, sync::Arc};

use inc_complete::DbGet;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    incremental::GetItem,
    iterator_extensions::vecmap,
    lexer::token::{FloatKind, IntegerKind},
    name_resolution::{Origin, ResolutionResult, builtin::Builtin},
    parser::{
        cst::{self, Mutability, Sharedness},
        ids::NameId,
    },
    type_inference::{generics::Generic, type_context::TypeContext, type_id::TypeId},
    vecmap::VecMap,
};

/// A top-level type is a type which may be in a top-level signature.
/// This notably excludes unbound type variables. Unlike `Type`, top-level
/// types must also be thread-safe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TopLevelType {
    /// Any primitive type which can be compared for unification via primitive equality
    Primitive(PrimitiveType),
    /// A user-supplied generic type. We don't want to bind over these like we do with type variables.
    Generic(Generic),
    Function {
        parameters: Vec<TopLevelType>,
        return_type: Box<TopLevelType>,
    },
    TypeApplication(Box<TopLevelType>, Vec<TopLevelType>),
    UserDefined(Origin),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Type {
    /// Any primitive type which can be compared for unification via primitive equality
    Primitive(PrimitiveType),

    /// A user-supplied generic type. We don't want to bind over these like we do with type variables.
    Generic(Generic),

    /// We represent type variables with unique ids and an external bindings map instead of a
    /// `Arc<RwLock<..>>` or similar because these need to be compared for equality, serialized, and
    /// be performant. We want the faster insertion of a local BTreeMap compared to a thread-safe
    /// version so we use a BTreeMap internally then freeze it in an Arc when finished to be
    /// able to access it from other threads.
    Variable(TypeVariableId),
    Function(FunctionType),
    Application(TypeId, Vec<TypeId>),
    Reference(Mutability, Sharedness),
    UserDefined(Origin),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FunctionType {
    pub parameters: Vec<TypeId>,
    pub return_type: TypeId,
    pub effects: TypeId,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PrimitiveType {
    Error,
    Unit,
    Bool,
    // * -> *
    Pointer,
    Char,
    /// TODO: This should be a struct type
    String,
    // * -> * -> *
    Pair,
    Int(IntegerKind),
    Float(FloatKind),
    Reference(Mutability, Sharedness),
}

/// Maps type variables to their bindings
pub type TypeBindings = BTreeMap<TypeVariableId, TypeId>;

#[allow(unused)]
impl TopLevelType {
    pub fn error() -> Self {
        Self::Primitive(PrimitiveType::Error)
    }

    pub fn unit() -> Self {
        Self::Primitive(PrimitiveType::Unit)
    }

    pub fn from_ast_type(typ: &cst::Type, resolve: &ResolutionResult) -> TopLevelType {
        match typ {
            cst::Type::Error => TopLevelType::error(),
            cst::Type::Unit => TopLevelType::unit(),
            cst::Type::Char => TopLevelType::Primitive(PrimitiveType::Char),
            cst::Type::String => TopLevelType::Primitive(PrimitiveType::String),
            cst::Type::Pair => TopLevelType::Primitive(PrimitiveType::Pair),
            cst::Type::Named(path) => {
                let origin = resolve.path_origins.get(path).copied();
                Self::convert_origin_to_type(origin, TopLevelType::UserDefined)
            },
            cst::Type::Variable(name) => {
                let origin = resolve.name_origins.get(name).copied();
                Self::convert_origin_to_type(origin, |origin| TopLevelType::Generic(Generic::Named(origin)))
            },
            cst::Type::Integer(kind) => TopLevelType::Primitive(PrimitiveType::Int(*kind)),
            cst::Type::Float(kind) => TopLevelType::Primitive(PrimitiveType::Float(*kind)),
            cst::Type::Function(function_type) => {
                // TODO: Effects
                let parameters = vecmap(&function_type.parameters, |typ| Self::from_ast_type(typ, resolve));
                let return_type = Box::new(Self::from_ast_type(&function_type.return_type, resolve));
                Self::Function { parameters, return_type }
            },
            cst::Type::Application(constructor, args) => {
                let constructor = Box::new(Self::from_ast_type(constructor, resolve));
                let args = vecmap(args, |arg| Self::from_ast_type(arg, resolve));
                Self::TypeApplication(constructor, args)
            },
            cst::Type::Reference(mutability, sharedness) => {
                Self::Primitive(PrimitiveType::Reference(*mutability, *sharedness))
            },
        }
    }

    fn convert_origin_to_type(origin: Option<Origin>, make_type: impl FnOnce(Origin) -> Self) -> Self {
        match origin {
            Some(Origin::Builtin(builtin)) => match builtin {
                Builtin::Unit => Self::Primitive(PrimitiveType::Unit),
                Builtin::Char => Self::Primitive(PrimitiveType::Char),
                Builtin::Int => Self::error(),   // TODO: Polymorphic integers
                Builtin::Float => Self::error(), // TODO: Polymorphic floats
                Builtin::String => Self::Primitive(PrimitiveType::String),
                Builtin::Ptr => Self::Primitive(PrimitiveType::Pointer),
                Builtin::PairType => Self::Primitive(PrimitiveType::Pair),
                Builtin::PairConstructor => {
                    // TODO: Error
                    Self::error()
                },
            },
            Some(origin) => {
                if !origin.may_be_a_type() {
                    // TODO: Error
                }
                make_type(origin)
            },
            None => TopLevelType::error(),
        }
    }

    fn find_generics(&self) -> Vec<Generic> {
        fn find_generics_helper(typ: &TopLevelType, generics: &mut Vec<Generic>) {
            match typ {
                TopLevelType::Primitive(_) | TopLevelType::UserDefined(_) => (),
                TopLevelType::Generic(generic) => {
                    if !generics.contains(generic) {
                        generics.push(*generic);
                    }
                },
                TopLevelType::Function { parameters, return_type } => {
                    parameters.iter().for_each(|typ| find_generics_helper(typ, generics));
                    find_generics_helper(return_type, generics);
                },
                TopLevelType::TypeApplication(constructor, args) => {
                    find_generics_helper(constructor, generics);
                    args.iter().for_each(|typ| find_generics_helper(typ, generics));
                },
            }
        }

        let mut generics = Vec::new();
        find_generics_helper(self, &mut generics);
        generics
    }

    /// Convert this `TopLevelType` into a `Type` without instantiating it
    fn as_type(&self, context: &mut TypeContext) -> TypeId {
        let typ = match self {
            TopLevelType::Primitive(primitive_type) => return TypeId::primitive(*primitive_type),
            TopLevelType::Generic(name) => Type::Generic(*name),
            TopLevelType::UserDefined(origin) => Type::UserDefined(*origin),
            TopLevelType::Function { parameters, return_type } => {
                Type::Function(FunctionType {
                    parameters: vecmap(parameters, |typ| typ.as_type(context)),
                    return_type: return_type.as_type(context),
                    effects: TypeId::UNIT, // TODO: Effects
                })
            },
            TopLevelType::TypeApplication(constructor, args) => {
                let constructor = constructor.as_type(context);
                let args = vecmap(args, |arg| arg.as_type(context));
                Type::Application(constructor, args)
            },
        };
        context.get_or_insert_type(typ)
    }

    pub fn substitute(&self, types: &mut TypeContext, substitutions: &GenericSubstitutions) -> TypeId {
        match self {
            TopLevelType::Primitive(primitive) => TypeId::primitive(*primitive),
            TopLevelType::UserDefined(origin) => types.get_or_insert_type(Type::UserDefined(*origin)),
            TopLevelType::Generic(generic) => {
                substitutions.get(generic).copied().unwrap_or_else(|| types.get_or_insert_type(Type::Generic(*generic)))
            },
            TopLevelType::Function { parameters, return_type } => {
                let typ = Type::Function(FunctionType {
                    parameters: vecmap(parameters, |typ| typ.substitute(types, substitutions)),
                    return_type: return_type.substitute(types, substitutions),
                    effects: TypeId::UNIT, // TODO: Effects
                });
                types.get_or_insert_type(typ)
            },
            TopLevelType::TypeApplication(constructor, args) => {
                let constructor = constructor.substitute(types, substitutions);
                let args = vecmap(args, |arg| arg.substitute(types, substitutions));
                types.get_or_insert_type(Type::Application(constructor, args))
            },
        }
    }

    /// Convert this into a GeneralizedType
    pub fn generalize(self) -> GeneralizedType {
        GeneralizedType::from_top_level_type(self)
    }
}

pub type GenericSubstitutions = FxHashMap<Generic, TypeId>;

#[allow(unused)]
impl Type {
    pub fn display<'local, Db>(
        &'local self, bindings: &'local TypeBindings, context: &'local TypeContext,
        names: &'local VecMap<NameId, Arc<String>>, db: &'local Db,
    ) -> TypePrinter<'local, Db>
    where
        Db: DbGet<GetItem>,
    {
        TypePrinter { typ: self, bindings, context, names, db }
    }

    pub fn unit() -> Self {
        Self::Primitive(PrimitiveType::Unit)
    }

    pub fn error() -> Self {
        Self::Primitive(PrimitiveType::Error)
    }
}

pub struct TypePrinter<'a, Db> {
    typ: &'a Type,
    bindings: &'a TypeBindings,
    context: &'a TypeContext,
    names: &'a VecMap<NameId, Arc<String>>,
    db: &'a Db,
}

impl<Db> std::fmt::Display for TypePrinter<'_, Db>
where
    Db: DbGet<GetItem>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.fmt_type(self.typ, false, f)
    }
}

impl<Db> TypePrinter<'_, Db>
where
    Db: DbGet<GetItem>,
{
    fn fmt_type_id(&self, id: TypeId, parenthesize: bool, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.fmt_type(self.context.get_type(id), parenthesize, f)
    }

    fn fmt_type(&self, typ: &Type, parenthesize: bool, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match typ {
            Type::Primitive(primitive_type) => write!(f, "{primitive_type}"),
            Type::UserDefined(origin) => self.fmt_type_origin(*origin, f),
            Type::Generic(Generic::Named(origin)) => self.fmt_type_origin(*origin, f),
            Type::Generic(Generic::Inferred(id)) => write!(f, "g{id}"),
            Type::Variable(id) => {
                if let Some(binding) = self.bindings.get(id) {
                    self.fmt_type_id(*binding, parenthesize, f)
                } else {
                    write!(f, "_{id}")
                }
            },
            Type::Function(function) => {
                if parenthesize {
                    write!(f, "(")?;
                }

                write!(f, "fn")?;
                for parameter in &function.parameters {
                    write!(f, " ")?;
                    self.fmt_type_id(*parameter, true, f)?;
                }
                write!(f, " -> ")?;
                self.fmt_type_id(function.return_type, false, f)?;

                if parenthesize {
                    write!(f, ")")?;
                }
                Ok(())
            },
            Type::Application(constructor, args) => {
                if parenthesize {
                    write!(f, "(")?;
                }

                if *constructor == TypeId::PAIR && args.len() == 2 {
                    self.fmt_type_id(args[0], true, f)?;
                    write!(f, ", ")?;
                    self.fmt_type_id(args[1], true, f)?;
                } else {
                    self.fmt_type_id(*constructor, true, f)?;
                    for arg in args {
                        write!(f, " ")?;
                        self.fmt_type_id(*arg, true, f)?;
                    }
                }

                if parenthesize {
                    write!(f, ")")?;
                }
                Ok(())
            },
            Type::Reference(mutability, sharedness) => write!(f, "{mutability}{sharedness}"),
        }
    }

    fn fmt_type_origin(&self, origin: Origin, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match origin {
            Origin::TopLevelDefinition(id) => {
                let (item, context) = GetItem(id.top_level_item).get(self.db);
                if let cst::ItemName::Single(name) = item.kind.name() {
                    write!(f, "{}", context.names[name])
                } else {
                    unreachable!()
                }
            },
            Origin::Local(name) => write!(f, "{}", self.names[name]),
            Origin::TypeResolution => write!(f, "TypeResolution"),
            Origin::Builtin(builtin) => write!(f, "{builtin}"),
        }
    }
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::Error => write!(f, "(error)"),
            PrimitiveType::Unit => write!(f, "Unit"),
            PrimitiveType::Bool => write!(f, "Bool"),
            PrimitiveType::Pointer => write!(f, "Ptr"),
            PrimitiveType::Int(kind) => write!(f, "{kind}"),
            PrimitiveType::Float(kind) => write!(f, "{kind}"),
            PrimitiveType::String => write!(f, "String"),
            PrimitiveType::Char => write!(f, "Char"),
            PrimitiveType::Pair => write!(f, ","),
            PrimitiveType::Reference(mutability, Sharedness::Shared) => write!(f, "{mutability}"),
            PrimitiveType::Reference(mutability, Sharedness::Owned) => write!(f, "{mutability}own"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TypeVariableId(pub u32);

impl std::fmt::Display for TypeVariableId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A top level definition's type may be generalized (made generic).
/// Other definitions like parameters are never generic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneralizedType {
    pub generics: Vec<Generic>,
    pub typ: TopLevelType,
}

impl GeneralizedType {
    fn new(generics: Vec<Generic>, typ: TopLevelType) -> Self {
        Self { typ, generics }
    }

    pub fn unit() -> GeneralizedType {
        Self::new(Vec::new(), TopLevelType::Primitive(PrimitiveType::Unit))
    }

    pub fn from_ast_type(typ: &cst::Type, resolve: &ResolutionResult) -> Self {
        let typ = TopLevelType::from_ast_type(typ, resolve);
        Self::from_top_level_type(typ)
    }

    /// Convert a TopLevelType into a GeneralizedType. TopLevelTypes never contain
    /// unbound type variables so this operation cannot fail.
    pub fn from_top_level_type(typ: TopLevelType) -> GeneralizedType {
        let generics = typ.find_generics();
        GeneralizedType { generics, typ }
    }

    /// Convert this `GeneralizedType` into a `Type` without instantiating it
    pub fn as_type(&self, context: &mut TypeContext) -> TypeId {
        self.typ.as_type(context)
    }
}
