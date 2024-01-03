//! This module defines the High-level Intermediate Representation's AST.
//!
//! The goal of this Ast is to function as a simpler Ast for the backends
//! to consume. In comparison to the main Ast, this one:
//! - Has no reliance on the ModuleCache
//! - Has all generic types removed either through monomorphisation or boxing
//! - All trait function calls are replaced with references to the exact
//!   function to call statically (monomorphisation) or are passed in as
//!   arguments to calling functions (boxing).
mod closures;
mod decision_tree_monomorphisation;
mod definitions;
mod monomorphisation;
mod printer;
mod types;

pub use monomorphisation::monomorphise;
pub use types::{FunctionType, IntegerKind, PrimitiveType, Type};

use std::{rc::Rc, cell::RefCell};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefinitionId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Literal {
    Integer(u64, IntegerKind),
    Float(u64, FloatKind),
    CString(String),
    Char(char),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    /// The Ast for the Ast::Definition which defines this Variable.
    /// This may be None if this variable was defined from a function
    /// parameter or a match pattern.
    ///
    /// This Ast is expected to contain a hir::Definition in the form
    /// `id = expr` where id == self.definition_id. Most definitions will
    /// be exactly this, but others may be a sequence of several definitions
    /// in the case of e.g. tuple unpacking.
    pub definition: Option<Rc<RefCell<Option<Ast>>>>,

    pub definition_id: DefinitionId,

    pub typ: Rc<Type>,

    // This field isn't needed, it is used only to make the output
    // of --show-hir more human readable for debugging.
    pub name: Option<String>,
}

pub type Variable = DefinitionInfo;

impl From<Variable> for Ast {
    fn from(v: Variable) -> Ast {
        Ast::Variable(v)
    }
}

impl Variable {
    fn new(definition_id: DefinitionId, typ: Rc<Type>) -> Variable {
        Variable { definition_id, typ, definition: None, name: None }
    }

    fn with_definition(def: Definition, typ: Rc<Type>) -> Self {
        let name = def.name.clone();
        DefinitionInfo {
            definition_id: def.variable,
            typ,
            definition: Some(Rc::new(RefCell::new(Some(Ast::Definition(def))))),
            name,
        }
    }
}

/// \a b. expr
/// Function definitions are also desugared to a ast::Definition with a ast::Lambda as its body
#[derive(Debug, Clone)]
pub struct Lambda {
    pub args: Vec<Variable>,
    pub body: Box<Ast>,
    pub typ: FunctionType,
}

/// foo a b c
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub function: Box<Ast>,
    pub args: Vec<Ast>,
    pub function_type: FunctionType,
}

/// Unlike ast::Definition, hir::Definition
/// is desugared of any patterns, its lhs must
/// be a single variable to simplify backends.
#[derive(Debug, Clone)]
pub struct Definition {
    pub variable: DefinitionId,
    pub name: Option<String>,
    pub expr: Box<Ast>,

    /// The type of the defined variable.
    /// The result type of a Definition is always Unit.
    pub typ: Type,
}

/// if condition then expression else expression
#[derive(Debug, Clone)]
pub struct If {
    pub condition: Box<Ast>,
    pub then: Box<Ast>,
    pub otherwise: Box<Ast>,
    pub result_type: Type,
}

#[derive(Debug, Clone)]
pub struct Match {
    // Unlike ast::Match this only contains the parts of the
    // branch after the ->.
    pub branches: Vec<Ast>,
    pub decision_tree: DecisionTree,
    pub result_type: Type,
}

// This cannot be desugared into Ast::If due to the sharing
// of Leafs across separate branches. E.g. a match on:
// ```
// match foo
// | None, None -> ...
// | _ -> ...
// ```
// Compiles to the tree:
// ```
// Switch value1 {
//     Some -> Leaf(1)
//     None -> {
//         switch value2 {
//             Some -> Leaf(1)
//             None -> Leaf(0)
//         }
//     }
// }
// ```
// Where two different paths need to share the same leaf branch.
#[derive(Debug, Clone)]
pub enum DecisionTree {
    Leaf(usize),
    Definition(Definition, Box<DecisionTree>),
    Switch { int_to_switch_on: Box<Ast>, cases: Vec<(u32, DecisionTree)>, else_case: Option<Box<DecisionTree>> },
}

/// return expression
#[derive(Debug, Clone)]
pub struct Return {
    pub expression: Box<Ast>,
    pub typ: Type,
}

/// statement1
/// statement2
/// ...
/// statementN
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct Extern {
    pub name: String,
    pub typ: Type,
}

/// lhs := rhs
#[derive(Debug, Clone)]
pub struct Assignment {
    pub lhs: Box<Ast>,
    pub rhs: Box<Ast>,
}

#[derive(Debug, Clone)]
pub struct MemberAccess {
    pub lhs: Box<Ast>,
    pub member_index: u32,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub struct Tuple {
    pub fields: Vec<Ast>,
}

/// Essentially the same as Builtin::Transmute.
/// Enum variants are padded with extra bytes
/// then lowered to this. lhs's type should be the same
/// size as the target type, though there may be
/// padding differences currently.
#[derive(Debug, Clone)]
pub struct ReinterpretCast {
    pub lhs: Box<Ast>,
    pub target_type: Type,
}

/// handle expression
/// | pattern -> branch_body   (resume_var in scope)
///
/// Handles handling multiple cases are translated into nested handles:
///
/// handle 
///   handle 
///     ..
///     handle expression
///     | patternN -> branch_bodyN   (resume_varN in scope)
///   | pattern2 -> branch_body2   (resume_var2 in scope)
/// | pattern1 -> branch_body1   (resume_var1 in scope)
#[derive(Debug, Clone)]
pub struct Handle {
    pub expression: Box<Ast>,
    pub effect: Effect,
    pub resume: Variable,
    pub branch_body: Lambda,
    pub result_type: Type,
}

/// A single effect declaration. Effect declarations declaring effects
/// with multiple functions will be split into multiple hir::Effects.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Effect {
    pub id: DefinitionId,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub enum Builtin {
    AddInt(Box<Ast>, Box<Ast>),
    AddFloat(Box<Ast>, Box<Ast>),

    SubInt(Box<Ast>, Box<Ast>),
    SubFloat(Box<Ast>, Box<Ast>),

    MulInt(Box<Ast>, Box<Ast>),
    MulFloat(Box<Ast>, Box<Ast>),

    DivSigned(Box<Ast>, Box<Ast>),
    DivUnsigned(Box<Ast>, Box<Ast>),
    DivFloat(Box<Ast>, Box<Ast>),

    ModSigned(Box<Ast>, Box<Ast>),
    ModUnsigned(Box<Ast>, Box<Ast>),
    ModFloat(Box<Ast>, Box<Ast>),

    LessSigned(Box<Ast>, Box<Ast>),
    LessUnsigned(Box<Ast>, Box<Ast>),
    LessFloat(Box<Ast>, Box<Ast>),

    EqInt(Box<Ast>, Box<Ast>),
    EqFloat(Box<Ast>, Box<Ast>),
    EqChar(Box<Ast>, Box<Ast>),
    EqBool(Box<Ast>, Box<Ast>),

    SignExtend(Box<Ast>, Type),
    ZeroExtend(Box<Ast>, Type),

    SignedToFloat(Box<Ast>, Type),
    UnsignedToFloat(Box<Ast>, Type),
    FloatToSigned(Box<Ast>, Type),
    FloatToUnsigned(Box<Ast>, Type),
    FloatPromote(Box<Ast>, Type),
    FloatDemote(Box<Ast>, Type),

    BitwiseAnd(Box<Ast>, Box<Ast>),
    BitwiseOr(Box<Ast>, Box<Ast>),
    BitwiseXor(Box<Ast>, Box<Ast>),
    BitwiseNot(Box<Ast>),

    Truncate(Box<Ast>, Type),
    Deref(Box<Ast>, Type),
    Offset(Box<Ast>, Box<Ast>, Type),
    Transmute(Box<Ast>, Type),

    /// Allocate space for the given value on the stack, and store it there. Return the stack address
    StackAlloc(Box<Ast>),
}

#[derive(Debug, Clone)]
pub enum Ast {
    Literal(Literal),
    Variable(Variable),
    Lambda(Lambda),
    FunctionCall(FunctionCall),
    Definition(Definition),
    If(If),
    Match(Match),
    Return(Return),
    Sequence(Sequence),
    Extern(Extern),
    Assignment(Assignment),
    MemberAccess(MemberAccess),
    Tuple(Tuple),
    ReinterpretCast(ReinterpretCast),
    Builtin(Builtin),
    Effect(Effect),
    Handle(Handle),
}

impl std::fmt::Display for DefinitionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

impl Ast {
    /// Construct the unit literal
    pub fn unit() -> Ast {
        Ast::Literal(Literal::Unit)
    }

    /// Construct a runtime call expression
    pub fn rt_call(function: Ast, args: Vec<Ast>, function_type: FunctionType) -> Ast {
        Ast::FunctionCall(FunctionCall {
            function: Box::new(function),
            args,
            function_type,
        })
    }

    /// Construct a runtime call expression with one argument.
    pub fn rt_call1(function: Ast, arg: Ast, function_type: FunctionType) -> Ast {
        Ast::FunctionCall(FunctionCall {
            function: Box::new(function),
            args: vec![arg],
            function_type,
        })
    }

    /// Construct a compile-time call expression with one argument.
    /// The function type here is unused since we expect this node to be removed anyway.
    pub fn ct_call1(function: Ast, arg: Ast) -> Ast {
        Ast::FunctionCall(FunctionCall {
            function: Box::new(function),
            args: vec![arg],
            function_type: FunctionType {
                parameters: Vec::new(),
                return_type: Box::new(Type::unit()),
                effects: Vec::new(),
                is_varargs: false,
            },
        })
    }
}

macro_rules! dispatch_on_hir {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            $crate::hir::Ast::Literal(inner) =>         $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Variable(inner) =>        $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Lambda(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::FunctionCall(inner) =>    $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Definition(inner) =>      $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::If(inner) =>              $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Match(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Return(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Sequence(inner) =>        $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Extern(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Assignment(inner) =>      $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::MemberAccess(inner) =>    $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Tuple(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::ReinterpretCast(inner) => $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Builtin(inner) =>         $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Effect(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::hir::Ast::Handle(inner) =>          $function(inner $(, $($args),* )? ),
        }
    });
}

pub(crate) use dispatch_on_hir;

use crate::lexer::token::FloatKind;

// Rust won't let us impl<T: FmtAst> Display for T
macro_rules! impl_display {
    ($typ:ty) => {
        impl std::fmt::Display for $typ {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                printer::AstPrinter::default().start(self, f)
            }
        }
    };
}

impl_display!(Ast);
impl_display!(Literal);
impl_display!(Variable);
impl_display!(Lambda);
impl_display!(FunctionCall);
impl_display!(Definition);
impl_display!(If);
impl_display!(Match);
impl_display!(Return);
impl_display!(Sequence);
impl_display!(Extern);
impl_display!(Assignment);
impl_display!(MemberAccess);
impl_display!(Tuple);
impl_display!(ReinterpretCast);
impl_display!(Builtin);
impl_display!(Effect);
impl_display!(Handle);
