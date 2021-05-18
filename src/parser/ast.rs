//! parser/ast.rs - Defines the abstract syntax tree (Ast)
//! used to hold the source program. This syntax tree is
//! produced as a result of parsing and is used in every
//! subsequent pass.
//!
//! Design-wise, instead of producing a new Ast with the
//! results of a given compiler pass (e.g. returning a TypedAst
//! as the result of type inference that is the same as Ast but
//! with an additional Type field for each node) ante instead
//! uses Option fields and mutably fills in this missing values.
//! For example:
//! - Name resolution fills out all these fields for various types:
//!   - For `ast::Variable`s:
//!       `definition: Option<DefinitionInfoId>`,
//!       `impl_scope: Option<ImplScopeId>,
//!       `id: Option<VariableId>`,
//!   - `level: Option<LetBindingLevel>` for
//!       `ast::Definition`s, `ast::TraitDefinition`s, and `ast::Extern`s,
//!   - `info: Option<DefinitionInfoId>` for `ast::Definition`s,
//!   - `type_info: Option<TypeInfoId>` for `ast::TypeDefinition`s,
//!   - `trait_info: Option<TraitInfoId>` for `ast::TraitDefinition`s and `ast::TraitImpl`s
//!   - `module_id: Option<ModuleId>` for `ast::Import`s,
//! 
//! - Type inference fills out:
//!   `typ: Option<Type>` for all nodes,
//!   `trait_binding: Option<TraitBindingId>` for `ast::Variable`s,
//!   `decision_tree: Option<DecisionTree>` for `ast::Match`s
use crate::lexer::token::{ Token, IntegerKind };
use crate::error::location::{ Location, Locatable };
use crate::cache::{ DefinitionInfoId, TraitInfoId, ModuleId, ImplScopeId, TraitBindingId, VariableId };
use crate::types::{ self, TypeInfoId, LetBindingLevel };
use crate::types::pattern::DecisionTree;
use crate::util::reinterpret_as_bits;

#[derive(Clone, Debug, Eq, Hash, PartialOrd, Ord)]
pub enum LiteralKind {
    Integer(u64, IntegerKind),
    Float(u64),
    String(String),
    Char(char),
    Bool(bool),
    Unit,
}

#[derive(Debug)]
pub struct Literal<'a> {
    pub kind: LiteralKind,
    pub location: Location<'a>,
    pub typ: Option<types::Type>
}

#[derive(Debug, PartialEq)]
pub enum VariableKind {
    Identifier(String),
    Operator(Token),
    TypeConstructor(String),
}

/// a, b, (+), Some, etc.
#[derive(Debug)]
pub struct Variable<'a> {
    pub kind: VariableKind,
    pub location: Location<'a>,

    /// A variable's definition is initially undefined.
    /// During name resolution, every definition is filled
    /// out - becoming Some(id)
    pub definition: Option<DefinitionInfoId>,

    /// If the variable's definition refers to some trait value,
    /// then this variable is generic over any matching
    /// value in any impl of its trait since it may need to
    /// be monomorphised to refer to any such impl.
    /// This field is used during codegen to map this variable to
    /// a given impl definition for static dispatch of traits.
    pub trait_binding: Option<TraitBindingId>,

    /// The trait impls in scope. Used during trait resolution.
    pub impl_scope: Option<ImplScopeId>,

    /// A unique ID that can be used to identify this variable node
    pub id: Option<VariableId>,

    pub typ: Option<types::Type>,
}

/// \a b. expr
/// Function definitions are also desugared to a ast::Definition with a ast::Lambda as its body
#[derive(Debug)]
pub struct Lambda<'a> {
    pub args: Vec<Ast<'a>>,
    pub body: Box<Ast<'a>>,
    pub return_type: Option<Type<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// foo a b c
#[derive(Debug)]
pub struct FunctionCall<'a> {
    pub function: Box<Ast<'a>>,
    pub args: Vec<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

impl<'a> FunctionCall<'a> {
    pub fn is_pair_constructor(&self) -> bool {
        if let Ast::Variable(variable) = self.function.as_ref() {
            variable.kind == VariableKind::Operator(Token::Comma)
        } else {
            false
        }
    }
}

/// foo = 23
/// pattern a b = expr
#[derive(Debug)]
pub struct Definition<'a> {
    pub pattern: Box<Ast<'a>>,
    pub expr: Box<Ast<'a>>,
    pub mutable: bool,
    pub location: Location<'a>,
    pub level: Option<LetBindingLevel>,
    pub info: Option<DefinitionInfoId>,
    pub typ: Option<types::Type>,
}

/// if condition then expression else expression
#[derive(Debug)]
pub struct If<'a> {
    pub condition: Box<Ast<'a>>,
    pub then: Box<Ast<'a>>,
    pub otherwise: Option<Box<Ast<'a>>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// match expression with
/// | pattern1 -> branch1
/// | pattern2 -> branch2
/// ...
/// | patternN -> branchN
#[derive(Debug)]
pub struct Match<'a> {
    pub expression: Box<Ast<'a>>,
    pub branches: Vec<(Ast<'a>, Ast<'a>)>,

    /// The decision tree is outputted from the completeness checking
    /// step and is used during codegen to efficiently compile each pattern branch.
    pub decision_tree: Option<DecisionTree>,

    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// Type nodes in the AST, different from the representation of types during type checking.
/// PointerType and potentially UserDefinedType are actually type constructors
#[derive(Debug)]
pub enum Type<'a> {
    IntegerType(IntegerKind, Location<'a>),
    FloatType(Location<'a>),
    CharType(Location<'a>),
    StringType(Location<'a>),
    BooleanType(Location<'a>),
    UnitType(Location<'a>),
    ReferenceType(Location<'a>),
    FunctionType(Vec<Type<'a>>, Box<Type<'a>>, /*varargs:*/bool, Location<'a>),
    TypeVariable(String, Location<'a>),
    UserDefinedType(String, Location<'a>),
    TypeApplication(Box<Type<'a>>, Vec<Type<'a>>, Location<'a>),
    PairType(Box<Type<'a>>, Box<Type<'a>>, Location<'a>),
}

/// The AST representation of a trait usage.
/// A trait's definition would be a TraitDefinition node.
/// This struct is used in e.g. `given` to list the required traits.
#[derive(Debug)]
pub struct Trait<'a> {
    pub name: String,
    pub args: Vec<Type<'a>>,
    pub location: Location<'a>,
}

#[derive(Debug)]
pub enum TypeDefinitionBody<'a> {
    UnionOf(Vec<(String, Vec<Type<'a>>, Location<'a>)>),
    StructOf(Vec<(String, Type<'a>, Location<'a>)>),
    AliasOf(Type<'a>),
}

/// type Name arg1 arg2 ... argN = definition
#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: String,
    pub args: Vec<String>,
    pub definition: TypeDefinitionBody<'a>,
    pub location: Location<'a>,
    pub type_info: Option<TypeInfoId>,
    pub typ: Option<types::Type>,
}

/// lhs : rhs
#[derive(Debug)]
pub struct TypeAnnotation<'a> {
    pub lhs: Box<Ast<'a>>,
    pub rhs: Type<'a>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// import Path1 . Path2 ... PathN
#[derive(Debug)]
pub struct Import<'a> {
    pub path: Vec<String>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
    pub module_id: Option<ModuleId>,
}

/// trait Name arg1 arg2 ... argN -> fundep1 fundep2 ... fundepN
///     declaration1
///     declaration2
///     ...
///     declarationN
#[derive(Debug)]
pub struct TraitDefinition<'a> {
    pub name: String,
    pub args: Vec<String>,
    pub fundeps: Vec<String>,

    // Storing function declarations as TypeAnnotations here
    // throws away any names given to parameters. In practice
    // this shouldn't matter until refinement types are implemented
    // that can depend upon these names.
    pub declarations: Vec<TypeAnnotation<'a>>,
    pub level: Option<LetBindingLevel>,
    pub location: Location<'a>,
    pub trait_info: Option<TraitInfoId>,
    pub typ: Option<types::Type>,
}

/// impl TraitName TraitArg1 TraitArg2 ... TraitArgN
///     definition1
///     definition2
///     ...
///     definitionN
#[derive(Debug)]
pub struct TraitImpl<'a> {
    pub trait_name: String,
    pub trait_args: Vec<Type<'a>>,
    pub given: Vec<Trait<'a>>,

    pub definitions: Vec<Definition<'a>>,
    pub location: Location<'a>,
    pub trait_info: Option<TraitInfoId>,
    pub typ: Option<types::Type>,
    pub trait_arg_types: Vec<types::Type>, // = fmap(trait_args, convert_type)
}

/// return expression
#[derive(Debug)]
pub struct Return<'a> {
    pub expression: Box<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// statement1
/// statement2
/// ...
/// statementN
#[derive(Debug)]
pub struct Sequence<'a> {
    pub statements: Vec<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// extern declaration
/// // or
/// extern
///     declaration1
///     declaration2
///     ...
///     declarationN
#[derive(Debug)]
pub struct Extern<'a> {
    pub declarations: Vec<TypeAnnotation<'a>>,
    pub level: Option<LetBindingLevel>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// lhs.field
#[derive(Debug)]
pub struct MemberAccess<'a> {
    pub lhs: Box<Ast<'a>>,
    pub field: String,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// lhs := rhs
#[derive(Debug)]
pub struct Assignment<'a> {
    pub lhs: Box<Ast<'a>>,
    pub rhs: Box<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub enum Ast<'a> {
    Literal(Literal<'a>),
    Variable(Variable<'a>),
    Lambda(Lambda<'a>),
    FunctionCall(FunctionCall<'a>),
    Definition(Definition<'a>),
    If(If<'a>),
    Match(Match<'a>),
    TypeDefinition(TypeDefinition<'a>),
    TypeAnnotation(TypeAnnotation<'a>),
    Import(Import<'a>),
    TraitDefinition(TraitDefinition<'a>),
    TraitImpl(TraitImpl<'a>),
    Return(Return<'a>),
    Sequence(Sequence<'a>),
    Extern(Extern<'a>),
    MemberAccess(MemberAccess<'a>),
    Assignment(Assignment<'a>),
}

impl PartialEq for LiteralKind {
    /// Ignoring any type tags, are these literals equal?
    fn eq(&self, other: &Self) -> bool {
        use LiteralKind::*;
        match (self, other) {
            (Integer(x, _), Integer(y, _)) => x == y,
            (Float(x), Float(y)) => x == y,
            (String(x), String(y)) => x == y,
            (Char(x), Char(y)) => x == y,
            (Bool(x), Bool(y)) => x == y,
            (Unit, Unit) => true,
            _ => false,
        }
    }
}

/// These are all convenience functions for creating various Ast nodes from the parser
impl<'a> Ast<'a> {
    pub fn integer(x: u64, kind: IntegerKind, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::Integer(x, kind), location, typ: None })
    }

    pub fn float(x: f64, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::Float(reinterpret_as_bits(x)), location, typ: None })
    }

    pub fn string(x: String, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::String(x), location, typ: None })
    }

    pub fn char_literal(x: char, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::Char(x), location, typ: None })
    }

    pub fn bool_literal(x: bool, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::Bool(x), location, typ: None })
    }

    pub fn unit_literal(location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::Unit, location, typ: None })
    }

    pub fn variable(name: String, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable { kind: VariableKind::Identifier(name), location, definition: None, id: None, impl_scope: None, trait_binding: None, typ: None })
    }

    pub fn operator(operator: Token, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable { kind: VariableKind::Operator(operator), location, definition: None, trait_binding: None, id: None, impl_scope: None, typ: None })
    }

    pub fn type_constructor(name: String, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable { kind: VariableKind::TypeConstructor(name), location, definition: None, trait_binding: None, id: None, impl_scope: None, typ: None })
    }

    pub fn lambda(args: Vec<Ast<'a>>, return_type: Option<Type<'a>>, body: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::Lambda(Lambda { args, body: Box::new(body), return_type, location, typ: None })
    }

    pub fn function_call(function: Ast<'a>, args: Vec<Ast<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::FunctionCall(FunctionCall { function: Box::new(function), args, location, typ: None })
    }

    pub fn if_expr(condition: Ast<'a>, then: Ast<'a>, otherwise: Option<Ast<'a>>, location: Location<'a>) -> Ast<'a> {
        Ast::If(If { condition: Box::new(condition), then: Box::new(then), otherwise: otherwise.map(Box::new), location, typ: None })
    }

    pub fn match_expr(expression: Ast<'a>, branches: Vec<(Ast<'a>, Ast<'a>)>, location: Location<'a>) -> Ast<'a> {
        Ast::Match(Match { expression: Box::new(expression), branches, decision_tree: None, location, typ: None })
    }

    pub fn type_definition(name: String, args: Vec<String>, definition: TypeDefinitionBody<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::TypeDefinition(TypeDefinition { name, args, definition, location, type_info: None, typ: None })
    }

    pub fn type_annotation(lhs: Ast<'a>, rhs: Type<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::TypeAnnotation(TypeAnnotation { lhs: Box::new(lhs), rhs, location, typ: None })
    }

    pub fn import(path: Vec<String>, location: Location<'a>) -> Ast<'a> {
        assert!(!path.is_empty());
        Ast::Import(Import { path, location, typ: None, module_id: None, })
    }

    pub fn trait_definition(name: String, args: Vec<String>, fundeps: Vec<String>, declarations: Vec<TypeAnnotation<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::TraitDefinition(TraitDefinition { name, args, fundeps, declarations, location, level: None, trait_info: None, typ: None })
    }

    pub fn trait_impl(trait_name: String, trait_args: Vec<Type<'a>>, given: Vec<Trait<'a>>, definitions: Vec<Definition<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!trait_args.is_empty());
        Ast::TraitImpl(TraitImpl { trait_name, trait_args, given, definitions, location, trait_arg_types: vec![], trait_info: None, typ: None })
    }

    pub fn return_expr(expression: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::Return(Return { expression: Box::new(expression), location, typ: None })
    }

    pub fn sequence(statements: Vec<Ast<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!statements.is_empty());
        Ast::Sequence(Sequence { statements, location, typ: None })
    }

    pub fn extern_expr(declarations: Vec<TypeAnnotation<'a>>, location: Location<'a>) -> Ast<'a> {
        Ast::Extern(Extern { declarations, location, level: None, typ: None })
    }

    pub fn member_access(lhs: Ast<'a>, field: String, location: Location<'a>) -> Ast<'a> {
        Ast::MemberAccess(MemberAccess { lhs: Box::new(lhs), field, location, typ: None })
    }

    pub fn assignment(lhs: Ast<'a>, rhs: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::Assignment(Assignment { lhs: Box::new(lhs), rhs: Box::new(rhs), location, typ: None })
    }
}

/// A macro for calling a method on every variant of an Ast node.
/// Useful for implementing a trait for the Ast and every node inside.
/// This is used for all compiler passes, as well as the Locatable trait below.
macro_rules! dispatch_on_expr {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            $crate::parser::ast::Ast::Literal(inner) =>         $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Variable(inner) =>        $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Lambda(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::FunctionCall(inner) =>    $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Definition(inner) =>      $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::If(inner) =>              $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Match(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TypeDefinition(inner) =>  $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TypeAnnotation(inner) =>  $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Import(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TraitDefinition(inner) => $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TraitImpl(inner) =>       $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Return(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Sequence(inner) =>        $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Extern(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::MemberAccess(inner) =>    $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Assignment(inner) =>      $function(inner $(, $($args),* )? ),
        }
    });
}

impl<'a> Locatable<'a> for Ast<'a> {
    fn locate(&self) -> Location<'a> {
        dispatch_on_expr!(self, Locatable::locate)
    }
}

macro_rules! impl_locatable_for {( $name:tt ) => {
    impl<'a> Locatable<'a> for $name<'a> {
        fn locate(&self) -> Location<'a> {
            self.location
        }
    }
};}

impl_locatable_for!(Literal);
impl_locatable_for!(Variable);
impl_locatable_for!(Lambda);
impl_locatable_for!(FunctionCall);
impl_locatable_for!(Definition);
impl_locatable_for!(If);
impl_locatable_for!(Match);
impl_locatable_for!(TypeDefinition);
impl_locatable_for!(TypeAnnotation);
impl_locatable_for!(Import);
impl_locatable_for!(TraitDefinition);
impl_locatable_for!(TraitImpl);
impl_locatable_for!(Return);
impl_locatable_for!(Sequence);
impl_locatable_for!(Extern);
impl_locatable_for!(MemberAccess);
impl_locatable_for!(Assignment);
