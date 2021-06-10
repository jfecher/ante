//! types/mod.rs - Unlike other modules for compiler passes,
//! the type inference compiler pass is defined in types/typechecker.rs
//! rather than the mod.rs file here. Instead, this file defines
//! the representation of `Type`s - which represent any Type in ante's
//! type system - and `TypeInfo`s - which hold more information about the
//! definition of a user-defined type.
use crate::cache::{ ModuleCache, DefinitionInfoId };
use crate::error::location::{ Locatable, Location };
use crate::lexer::token::IntegerKind;
use crate::lifetimes;

use std::collections::HashMap;

pub mod pattern;
pub mod typed;
pub mod typechecker;
pub mod traitchecker;
pub mod typeprinter;
pub mod traits;

/// The type to default any Inferred integer types to that were
/// not bound to any other concrete integer type (e.g. via `1 + 2u8`).
pub const DEFAULT_INTEGER_TYPE: Type =
    Type::Primitive(PrimitiveType::IntegerType(IntegerKind::I32));


#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct TypeVariableId(pub usize);

/// Primitive types are the easy cases when unifying types.
/// They're equal simply if the other type is also the same PrimitiveType variant,
/// there is no recursion needed like with other Types. If the `Type`
/// enum forms a tree, then these are the leaf nodes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum PrimitiveType {
    IntegerType(IntegerKind), // : *
    FloatType,                // : *
    CharType,                 // : *
    BooleanType,              // : *
    UnitType,                 // : *
    Ptr,                      // : * -> *
}

/// Any type in ante. Note that a trait is not a type. Traits are
/// relations between 1 or more types rather than being types themselves.
///
/// NOTE: PartialEq and Hash impls here are somewhat unsafe since any
/// type variables will not have access to the cache to follow their bindings.
/// Thus, PartialEq/Hash may think two types aren't equal when they otherwise
/// would be. For this reason, these impls are currently only used after
/// following all type bindings via `follow_bindings` or a similar function.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    /// int, char, bool, etc
    Primitive(PrimitiveType),

    /// Any function type
    /// Note that all functions in ante take at least 1 argument
    /// The boolean indicates whether this is a varargs function or not.
    Function(Vec<Type>, Box<Type>, bool),

    /// Any stand-in type e.g. `a` in `Vec a`. The original names are
    /// translated into unique TypeVariableIds during name resolution.
    /// Each TypeVariableId is either Bound or Unbound in the ModuleCache.
    /// Bound type variables should be treated as equal to what they're bound
    /// to. Unbound type variables may stand in for any type. During type
    /// inference, the `unify` function may bind unbound type variables
    /// into bound type variables when asserting two types are equal.
    TypeVariable(TypeVariableId),

    /// Any user defined type defined via the `type` keyword
    /// These have a unique UserDefinedTypeId which points to
    /// additional information about the contents of the type
    /// not needed for most type checking.
    UserDefinedType(TypeInfoId),

    /// Any type in the form `constructor arg1 arg2 ... argN`
    TypeApplication(Box<Type>, Vec<Type>),

    /// A region-allocated reference to some data.
    /// Contains a region variable that is unified with other refs during type
    /// inference. All these refs will be allocated in the same region.
    Ref(lifetimes::LifetimeVariableId),

    /// These are currently used internally to indicate polymorphic
    /// type variables for let-polymorphism. There is no syntax to
    /// specify these explicitly in ante code. Each type variable in
    /// the Vec is polymorphic in the Box<Type>. This differentiates
    /// generic functions from normal functions whose arguments are
    /// just type variables of unknown types yet to be inferenced.
    ForAll(Vec<TypeVariableId>, Box<Type>),
}

impl Type {
    pub fn is_pair_type(&self) -> bool {
        self == &Type::UserDefinedType(PAIR_TYPE)
    }

    /// Pretty-print each type with each typevar substituted for a, b, c, etc.
    pub fn display<'a, 'b>(&'a self, cache: &'a ModuleCache<'b>) -> typeprinter::TypePrinter<'a, 'b> {
        let typevars = typechecker::find_all_typevars(self, false, cache);
        let mut typevar_names = HashMap::new();
        let mut current = 'a';

        for typevar in typevars {
            if typevar_names.get(&typevar).is_none() {
                typevar_names.insert(typevar, current.to_string());
                current = (current as u8 + 1) as char;
                assert!(current != 'z'); // TODO: wrap to aa, ab, ac...
            }
        }

        typeprinter::TypePrinter::new(self, typevar_names, false, cache)
    }

    /// Like display but show the real unique TypeVariableId for each typevar instead
    #[allow(dead_code)]
    pub fn debug<'a, 'b>(&'a self, cache: &'a ModuleCache<'b>) -> typeprinter::TypePrinter<'a, 'b> {
        let typevars = typechecker::find_all_typevars(self, false, cache);
        let mut typevar_names = HashMap::new();

        for typevar in typevars {
            if typevar_names.get(&typevar).is_none() {
                typevar_names.insert(typevar, typevar.0.to_string());
            }
        }

        typeprinter::TypePrinter::new(self, typevar_names, true, cache)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct LetBindingLevel(pub usize);

/// The initial LetBindingLevel used in nameresolution and typechecking.
/// This must be at least 1 since typechecker::infer_ast will set the CURRENT_LEVEL
/// to INITIAL_LEVEL - 1 when finishing type checking main to differentiate between
/// traits used within main and traits propagated up into main's signature.
/// Since the later case is an error (all traits must be resolved by the point
/// we finish typechecking) typechecker::infer_ast and typechecker::should_propagate
/// use this INITIAL_LEVEL - 1 to distinguish between the two cases. Note that since
/// at each ast::Definition the current LetBindingLevel is incremented when recursing
/// inside and decremented after finishing, this distinction is equivalent to if we
/// manually forced users to wrap their program in the following:
/// ```
/// main () = ...
/// ```
/// See okmij.org/ftp/ML/generalization.html for more information on the levels
/// algorithm used in the typechecker.
pub const INITIAL_LEVEL: usize = 1;

/// A given TypeVariableId is either bound to some type
/// or is unbound and has a given LetBindingLevel as its lifetime.
/// This LetBindingLevel is used to determine which type variables
/// can be generalized.
#[derive(Debug)]
pub enum TypeBinding {
    Bound(Type),
    Unbound(LetBindingLevel, Kind),
}

#[derive(Debug)]
pub struct TypeConstructor<'a> {
    pub name: String,
    pub args: Vec<Type>,
    pub id: DefinitionInfoId,
    pub location: Location<'a>,
}

#[derive(Debug)]
pub struct Field<'a> {
    pub name: String,
    pub field_type: Type,
    pub location: Location<'a>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct TypeInfoId(pub usize);

/// The string type is semi builtin in that it isn't a primitive type
/// but all string literals will nevertheless have the type `string`
/// even if the prelude isn't imported into scope.
pub const STRING_TYPE: TypeInfoId = TypeInfoId(0);

/// The pair type is another semi builtin type. Its constructor (,)
/// is also visible whether or not the prelude is imported.
/// It is somewhat special in that it is the only type defined with
/// an operator for its name, but it is otherwise a normal struct type.
pub const PAIR_TYPE: TypeInfoId = TypeInfoId(1);

#[derive(Debug)]
pub enum TypeInfoBody<'a> {
    Union(Vec<TypeConstructor<'a>>),
    Struct(Vec<Field<'a>>),
    Alias(Type),
    Unknown,
}

/// Holds additional information for a given `type T = ...` definition.
#[derive(Debug)]
pub struct TypeInfo<'a> {
    pub args: Vec<TypeVariableId>,
    pub name: String,
    pub body: TypeInfoBody<'a>,
    pub uses: u32,
    pub location: Location<'a>,
}

impl<'a> Locatable<'a> for TypeInfo<'a> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

impl<'a> TypeInfo<'a> {
    pub fn is_union(&self) -> bool {
        match &self.body {
            TypeInfoBody::Union(..) => true,
            _ => false,
        }
    }

    pub fn find_field<'b>(&'b self, field_name: &str) -> Option<(u32, &'b Field)> {
        match &self.body {
            TypeInfoBody::Struct(fields) => {
                fields.iter().enumerate()
                    .find(|(_, field)| field.name == field_name)
                    .map(|(i, field)| (i as u32, field))
            },
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Kind {
    /// usize is the number of type arguments it takes before it returns a type of kind *.
    /// For example, the kind Normal(2) : * -> * -> *
    Normal(usize),

    /// A higher order kind where each element in the Vec is an argument. For example, the kind:
    /// HigherOrder(vec![ Normal(0), HigherOrder(vec![ Normal(0), Normal(1) ]), Normal(1) ])
    /// has kind: * -> (* -> (* -> *)) -> (* -> *)
    #[allow(dead_code)]
    HigherOrder(Vec<Kind>),
}
