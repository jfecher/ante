//! This module defines the High-level Intermediate Representation's AST.
//!
//! The goal of this Ast is to function as a simpler Ast for the backends
//! to consume. In comparison to the main Ast, this one:
//! - Has no reliance on the ModuleCache
//! - Has all generic types removed either through monomorphisation or boxing
//! - All trait function calls are replaced with references to the exact
//!   function to call statically (monomorphisation) or are passed in as
//!   arguments to calling functions (boxing).
use std::{rc::Rc, collections::BTreeMap};

// These parts of mir are all identical to the hir
pub use crate::hir::{PrimitiveType, DefinitionId, Literal, FunctionType, Type, Effect, Extern, IntegerKind};

#[derive(Debug, Clone, Eq)]
pub struct Variable {
    pub definition_id: DefinitionId,

    pub typ: Rc<Type>,

    // This field isn't needed, it is used only to make the output
    // of --show-mir more human readable for debugging.
    pub name: Rc<String>,
}

impl PartialEq for Variable {
    fn eq(&self, other: &Self) -> bool {
        self.definition_id == other.definition_id
    }
}

impl std::hash::Hash for Variable {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.definition_id.hash(state);
    }
}

/// \a b. expr
/// Function definitions are also desugared to a ast::Definition with a ast::Lambda as its body
#[derive(Debug, Clone)]
pub struct Lambda {
    pub args: Vec<Variable>,
    pub body: Box<Ast>,
    pub typ: FunctionType,

    /// True if this lambda should be evaluated at compile-time.
    /// This is used to create static lambdas and static calls for specializing
    /// effect handlers at compile-time.
    pub compile_time: bool,
}

/// foo a b c
#[derive(Debug, Clone)]
pub struct FunctionCall {
    pub function: Atom,
    pub args: Vec<Atom>,
    pub function_type: FunctionType,

    /// True if this function call should be evaluated at compile-time.
    /// This is used to create static lambdas and static calls for specializing
    /// effect handlers at compile-time.
    pub compile_time: bool,
}

/// let <variable>: <typ> = <expr> in <body>
#[derive(Debug, Clone)]
pub struct Let<Body> {
    pub variable: DefinitionId,
    pub name: Rc<String>,
    pub expr: Box<Ast>,

    pub body: Box<Body>,

    /// The type of the defined variable.
    /// The result type of a Definition is always Unit.
    pub typ: Rc<Type>,
}

impl Let<Ast> {
    /// A `let` is considered trivial if it is in the form `let x = ... in x`
    pub fn is_trivial(&self) -> bool {
        match self.body.as_ref() {
            Ast::Atom(Atom::Variable(variable)) => variable.definition_id == self.variable,
            _ => false,
        }
    }
}

/// if condition then expression else expression
#[derive(Debug, Clone)]
pub struct If {
    pub condition: Atom,
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
    Let(Let<DecisionTree>),
    Switch { int_to_switch_on: Atom, cases: Vec<(u32, DecisionTree)>, else_case: Option<Box<DecisionTree>> },
}

/// return expression
#[derive(Debug, Clone)]
pub struct Return {
    pub expression: Atom,
    pub typ: Type,
}

/// lhs := rhs
#[derive(Debug, Clone)]
pub struct Assignment {
    pub lhs: Atom,
    pub rhs: Atom,
}

#[derive(Debug, Clone)]
pub struct MemberAccess {
    pub lhs: Atom,
    pub member_index: u32,
    pub typ: Type,
}

#[derive(Debug, Clone)]
pub struct Tuple {
    pub fields: Vec<Atom>,
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
    pub result_type: Type,

    pub branch_args: Vec<Variable>,
    pub branch_body: Box<Ast>,
}

#[derive(Debug, Clone)]
pub enum Builtin {
    AddInt(Atom, Atom),
    AddFloat(Atom, Atom),

    SubInt(Atom, Atom),
    SubFloat(Atom, Atom),

    MulInt(Atom, Atom),
    MulFloat(Atom, Atom),

    DivSigned(Atom, Atom),
    DivUnsigned(Atom, Atom),
    DivFloat(Atom, Atom),

    ModSigned(Atom, Atom),
    ModUnsigned(Atom, Atom),
    ModFloat(Atom, Atom),

    LessSigned(Atom, Atom),
    LessUnsigned(Atom, Atom),
    LessFloat(Atom, Atom),

    EqInt(Atom, Atom),
    EqFloat(Atom, Atom),
    EqChar(Atom, Atom),
    EqBool(Atom, Atom),

    SignExtend(Atom, Type),
    ZeroExtend(Atom, Type),

    SignedToFloat(Atom, Type),
    UnsignedToFloat(Atom, Type),
    FloatToSigned(Atom, Type),
    FloatToUnsigned(Atom, Type),
    FloatPromote(Atom, Type),
    FloatDemote(Atom, Type),

    BitwiseAnd(Atom, Atom),
    BitwiseOr(Atom, Atom),
    BitwiseXor(Atom, Atom),
    BitwiseNot(Atom),

    Truncate(Atom, Type),
    Deref(Atom, Type),
    Offset(Atom, Atom, Type),
    Transmute(Atom, Type),

    /// Allocate space for the given value on the stack, and store it there. Return the stack address
    StackAlloc(Atom),
}

#[derive(Debug, Clone)]
pub enum Ast {
    Atom(Atom),
    FunctionCall(FunctionCall),
    Let(Let<Ast>),
    If(If),
    Match(Match),
    Return(Return),
    Assignment(Assignment),
    MemberAccess(MemberAccess),
    Tuple(Tuple),
    Builtin(Builtin),
    Handle(Handle),
}

#[derive(Debug, Clone)]
pub enum Atom {
    Literal(Literal),
    Variable(Variable),
    Lambda(Lambda),
    Extern(Extern),
    Effect(Effect),
}

impl Ast {
    /// Construct the unit literal
    pub fn unit() -> Ast {
        Ast::Atom(Atom::Literal(Literal::Unit))
    }

    /// Construct a runtime call expression
    pub fn rt_call(function: Atom, args: Vec<Atom>, function_type: FunctionType) -> Ast {
        Ast::FunctionCall(FunctionCall { function, args, function_type, compile_time: false })
    }

    /// Construct a runtime call expression with one argument.
    pub fn rt_call1(function: Atom, arg: Atom, function_type: FunctionType) -> Ast {
        Ast::FunctionCall(FunctionCall {
            function,
            args: vec![arg],
            function_type,
            compile_time: false,
        })
    }

    /// Construct a compile-time call expression with one argument.
    /// The function type here is unused since we expect this node to be removed anyway.
    pub fn ct_call1(function: Atom, arg: Atom) -> Ast {
        Ast::FunctionCall(FunctionCall {
            function,
            args: vec![arg],
            compile_time: true,
            function_type: FunctionType {
                parameters: Vec::new(),
                return_type: Box::new(Type::Primitive(PrimitiveType::Unit)),
                is_varargs: false,
                effects: Vec::new(),
            },
        })
    }
}

pub struct Mir {
    pub main: DefinitionId,
    pub functions: BTreeMap<DefinitionId, (Rc<String>, Ast)>,
    pub next_id: usize,
}

macro_rules! dispatch_on_mir {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            $crate::mir::Ast::Atom(inner) =>         $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::FunctionCall(inner) => $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::Let(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::If(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::Match(inner) =>        $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::Return(inner) =>       $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::Assignment(inner) =>   $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::MemberAccess(inner) => $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::Tuple(inner) =>        $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::Builtin(inner) =>      $function(inner $(, $($args),* )? ),
            $crate::mir::Ast::Handle(inner) =>       $function(inner $(, $($args),* )? ),
        }
    });
}

macro_rules! dispatch_on_atom {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            $crate::mir::Atom::Literal(inner) =>  $function(inner $(, $($args),* )? ),
            $crate::mir::Atom::Variable(inner) => $function(inner $(, $($args),* )? ),
            $crate::mir::Atom::Lambda(inner) =>   $function(inner $(, $($args),* )? ),
            $crate::mir::Atom::Extern(inner) =>   $function(inner $(, $($args),* )? ),
            $crate::mir::Atom::Effect(inner) =>   $function(inner $(, $($args),* )? ),
        }
    });
}

pub(crate) use dispatch_on_mir;
