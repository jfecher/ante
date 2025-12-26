use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{iterator_extensions::vecmap, name_resolution::{Origin, ResolutionResult, builtin::Builtin}, parser::cst, type_inference::{generics::Generic, types::{FunctionType, GenericSubstitutions, PrimitiveType, Type}}};


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
    fn as_type(&self) -> Type {
        match self {
            TopLevelType::Primitive(primitive_type) => return Type::primitive(*primitive_type),
            TopLevelType::Generic(name) => Type::Generic(*name),
            TopLevelType::UserDefined(origin) => Type::UserDefined(*origin),
            TopLevelType::Function { parameters, return_type } => {
                Type::Function(Arc::new(FunctionType {
                    parameters: vecmap(parameters, Self::as_type),
                    return_type: return_type.as_type(),
                    effects: Type::UNIT, // TODO: Effects
                }))
            },
            TopLevelType::TypeApplication(constructor, args) => {
                let constructor = Arc::new(constructor.as_type());
                let args = Arc::new(vecmap(args, Self::as_type));
                Type::Application(constructor, args)
            },
        }
    }

    pub fn substitute(&self, substitutions: &GenericSubstitutions) -> Type {
        match self {
            TopLevelType::Primitive(primitive) => Type::primitive(*primitive),
            TopLevelType::UserDefined(origin) => Type::UserDefined(*origin),
            TopLevelType::Generic(generic) => {
                substitutions.get(generic).cloned().unwrap_or_else(|| Type::Generic(*generic))
            },
            TopLevelType::Function { parameters, return_type } => {
                Type::Function(Arc::new(FunctionType {
                    parameters: vecmap(parameters, |typ| typ.substitute(substitutions)),
                    return_type: return_type.substitute(substitutions),
                    effects: Type::UNIT, // TODO: Effects
                }))
            },
            TopLevelType::TypeApplication(constructor, args) => {
                let constructor = constructor.substitute(substitutions);
                let args = vecmap(args, |arg| arg.substitute(substitutions));
                Type::Application(Arc::new(constructor), Arc::new(args))
            },
        }
    }

    /// Convert this into a GeneralizedType
    pub fn generalize(self) -> GeneralizedType {
        GeneralizedType::from_top_level_type(self)
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
    pub fn as_type(&self) -> Type {
        self.typ.as_type()
    }
}

impl std::fmt::Display for GeneralizedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.generics.is_empty() {
            write!(f, "forall")?;
            for generic in &self.generics {
                write!(f, " {generic}")?;
            }
            write!(f, ". ")?;
        }
        self.typ.fmt(f)
    }
}

impl std::fmt::Display for TopLevelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = |t: &TopLevelType, f: &mut std::fmt::Formatter| {
            if matches!(t, TopLevelType::Primitive(_) | TopLevelType::Generic(_) | TopLevelType::UserDefined(_)) {
                write!(f, "{t}")
            } else {
                write!(f, "({t})")
            }
        };

        match self {
            TopLevelType::Primitive(primitive_type) => write!(f, "{primitive_type}"),
            TopLevelType::Generic(generic) => write!(f, "{generic}"),
            TopLevelType::Function { parameters, return_type } => {
                write!(f, "fn")?;
                for parameter in parameters {
                    write!(f, " ")?;
                    display(parameter, f)?;
                }
                write!(f, " -> ")?;
                display(return_type, f)
            },
            TopLevelType::TypeApplication(constructor, args) => {
                display(constructor, f)?;
                for arg in args {
                    write!(f, " ")?;
                    display(arg, f)?;
                }
                Ok(())
            },
            TopLevelType::UserDefined(origin) => write!(f, "{origin}"),
        }
    }
}
