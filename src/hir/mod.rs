//! This module defines the High-level Intermediate Representation's AST.
//!
//! The goal of this Ast is to function as a simpler Ast for the backends
//! to consume. In comparison to the main Ast, this one:
//! - Has no reliance on the ModuleCache
//! - Has all generic types removed either through monomorphisation or boxing
//! - All trait function calls are replaced with references to the exact
//!   function to call statically (monomorphisation) or are passed in as
//!   arguments to calling functions (boxing).
mod types;
mod monomorphisation;

pub use monomorphisation::monomorphise;

use types::{ Type, IntegerKind, FunctionType };

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct AstId(usize);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Literal {
    Integer(u64, IntegerKind),
    Float(u64),
    CString(String),
    Char(char),
    Bool(bool),
    Unit,
}

/// a, b, (+), Some, etc.
#[derive(Debug)]
pub struct Variable {
    pub name: String,

    /// A variable's definition is initially undefined.
    /// During name resolution, every definition is filled
    /// out - becoming Some(id)
    pub definition: AstId,
}

/// \a b. expr
/// Function definitions are also desugared to a ast::Definition with a ast::Lambda as its body
#[derive(Debug)]
pub struct Lambda {
    pub args: Vec<Ast>,
    pub body: Box<Ast>,
    pub typ: FunctionType,
}

/// foo a b c
#[derive(Debug)]
pub struct FunctionCall {
    pub function: Box<Ast>,
    pub args: Vec<Ast>,
}

/// foo = 23
/// pattern a b = expr
#[derive(Debug)]
pub struct Definition {
    pub pattern: Box<Ast>,
    pub expr: Box<Ast>,
    pub mutable: bool,
}

/// if condition then expression else expression
#[derive(Debug)]
pub struct If {
    pub condition: Box<Ast>,
    pub then: Box<Ast>,
    pub otherwise: Option<Box<Ast>>,
}

/// return expression
#[derive(Debug)]
pub struct Return {
    pub expression: Box<Ast>,
}

/// statement1
/// statement2
/// ...
/// statementN
#[derive(Debug)]
pub struct Sequence {
    pub statements: Vec<Ast>,
}

/// extern declaration
/// // or
/// extern
///     declaration1
///     declaration2
///     ...
///     declarationN
#[derive(Debug)]
pub struct Extern {
    pub declarations: Vec<(Ast, Type)>,
}

/// lhs := rhs
#[derive(Debug)]
pub struct Assignment {
    pub lhs: Box<Ast>,
    pub rhs: Box<Ast>,
}

#[derive(Debug)]
pub struct MemberAccess{
    pub lhs: Box<Ast>,
    pub member_index: u32,
}

#[derive(Debug)]
pub struct Tuple {
    pub fields: Vec<Ast>,
}

#[derive(Debug)]
pub enum Builtin {
    AddInt,
    AddFloat,

    SubInt,
    SubFloat,

    MulInt,
    MulFloat,

    DivInt,
    DivFloat,

    ModInt,
    ModFloat,

    LessInt,
    LessFloat,

    GreaterInt,
    GreaterFloat,

    EqInt,
    EqFloat,
    EqChar,
    EqBool,

    SignExtend,
    ZeroExtend,
    Truncate,
    Deref,
    Offset,
    Transmute,
}

#[derive(Debug)]
pub enum Ast {
    Literal(Literal),
    Variable(Variable),
    Lambda(Lambda),
    FunctionCall(FunctionCall),
    Definition(Definition),
    If(If),
    Return(Return),
    Sequence(Sequence),
    Extern(Extern),
    Assignment(Assignment),
    MemberAccess(MemberAccess),
    Tuple(Tuple),
    Builtin(Builtin),
}

macro_rules! dispatch_on_hir {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            $crate::parser::ast::Ast::Literal(inner) =>      $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Variable(inner) =>     $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Lambda(inner) =>       $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::FunctionCall(inner) => $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Definition(inner) =>   $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::If(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Return(inner) =>       $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Sequence(inner) =>     $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Extern(inner) =>       $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Assignment(inner) =>   $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::MemberAccess(inner) => $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Tuple(Inner) =>        $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Builtin(inner) =>      $function(inner $(, $($args),* )? ),
        }
    });
}
