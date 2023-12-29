mod id;

use std::{collections::{HashMap, BTreeMap}, rc::Rc};

use crate::hir::{Literal, PrimitiveType};
pub use id::*;

#[derive(Default)]
pub struct Mir {
    pub functions: BTreeMap<FunctionId, Function>,

    pub extern_symbols: HashMap<ExternId, (String, Type)>,

    pub next_function_id: u32,
}

impl Mir {
    pub fn main_id() -> FunctionId {
        FunctionId { id: 0, name: Rc::new("main".into()) }
    }

    /// Returns the next available function id but does not set the current id
    pub fn next_function_id(&mut self, name: Rc<String>) -> FunctionId {
        let id = self.next_function_id;
        self.next_function_id += 1;
        FunctionId { id, name }
    }
}

// Functions can be cloned during mangling
#[derive(Clone)]
pub struct Function {
    pub id: FunctionId,
    pub argument_type: Type,
    pub body: Expr,

    /// True if this function should be evaluated at compile-time.
    /// This is used to specialize functions to effect continuations automatically.
    pub compile_time: bool,
}

impl Function {
    /// Return an empty function with the given id that is expected to have its body filled in later
    pub fn empty(id: FunctionId, argument_type: Type) -> Self {
        Self { id, body: Expr::Literal(Literal::Unit), argument_type, compile_time: false }
    }

    pub fn parameters(&self) -> impl ExactSizeIterator<Item = ParameterId> {
        let function = self.id.clone();
        std::iter::once(ParameterId {
            function: function.clone(),
            parameter_index: 0,
        })
    }

    pub(super) fn for_each_id<T, F, P>(&self, data: &mut T, mut on_function: F, mut on_parameter: P) where
        F: FnMut(&mut T, &FunctionId),
        P: FnMut(&mut T, &ParameterId),
    {
        self.body.for_each_id(data, &mut on_function, &mut on_parameter);
    }

    /// Mutate any FunctionIds in this function's body to a new FunctionId.
    ///
    /// Unlike `for_each_id`, this method also applies to FunctionIds within ParameterIds.
    pub(super) fn map_functions(&mut self, substitutions: &ExprMap) {
        self.body.map_functions(substitutions);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    /// An if-else branching. Expects 3 arguments: (cond, k_then, k_else)
    If(Box<Expr>, Box<Expr>, Box<Expr>),

    /// A switch case (derived from a match expression).
    /// Expects the value to match along with the list of cases with the value
    /// to match for each along with the continuation to jump to that case.
    /// Also expects an additional optional default case continuation.
    Switch(Box<Expr>, Vec<(u32, FunctionId)>, Option<FunctionId>),

    Call(Box<Expr>, /*arg:*/Box<Expr>, /*compile_time:*/bool),

    Literal(Literal),
    Parameter(ParameterId),

    Function(FunctionId),

    Tuple(Vec<Expr>),
    MemberAccess(Box<Expr>, u32, Type),

    /// Assignment expects 3 arguments: [lvalue, rvalue, k]
    Assign,

    /// Extern expects 1 argument: [k]
    ///
    /// The ExternId can be used to retrieve the name and type
    /// of the symbol being referenced. An ID is used to ensure
    /// the same symbol is not imported multiple times.
    Extern(ExternId),

    AddInt(Box<Expr>, Box<Expr>),
    AddFloat(Box<Expr>, Box<Expr>),

    SubInt(Box<Expr>, Box<Expr>),
    SubFloat(Box<Expr>, Box<Expr>),

    MulInt(Box<Expr>, Box<Expr>),
    MulFloat(Box<Expr>, Box<Expr>),

    DivSigned(Box<Expr>, Box<Expr>),
    DivUnsigned(Box<Expr>, Box<Expr>),
    DivFloat(Box<Expr>, Box<Expr>),

    ModSigned(Box<Expr>, Box<Expr>),
    ModUnsigned(Box<Expr>, Box<Expr>),
    ModFloat(Box<Expr>, Box<Expr>),

    LessSigned(Box<Expr>, Box<Expr>),
    LessUnsigned(Box<Expr>, Box<Expr>),
    LessFloat(Box<Expr>, Box<Expr>),

    EqInt(Box<Expr>, Box<Expr>),
    EqFloat(Box<Expr>, Box<Expr>),
    EqChar(Box<Expr>, Box<Expr>),
    EqBool(Box<Expr>, Box<Expr>),

    SignExtend(Box<Expr>, Type),
    ZeroExtend(Box<Expr>, Type),

    SignedToFloat(Box<Expr>, Type),
    UnsignedToFloat(Box<Expr>, Type),
    FloatToSigned(Box<Expr>, Type),
    FloatToUnsigned(Box<Expr>, Type),
    FloatPromote(Box<Expr>, Type),
    FloatDemote(Box<Expr>, Type),

    BitwiseAnd(Box<Expr>, Box<Expr>),
    BitwiseOr(Box<Expr>, Box<Expr>),
    BitwiseXor(Box<Expr>, Box<Expr>),
    BitwiseNot(Box<Expr>),

    Truncate(Box<Expr>, Type),
    Deref(Box<Expr>, Type),
    Offset(Box<Expr>, Box<Expr>, Type),
    Transmute(Box<Expr>, Type),

    /// Allocate space for the given value on the stack, and store it there. Return the stack address
    StackAlloc(Box<Expr>),
}

impl Expr {
    /// Returns a unit literal
    pub(super) fn unit() -> Self {
        Self::Literal(Literal::Unit)
    }

    /// Returns a runtime call `f(arg)`
    pub(super) fn rt_call(f: Expr, arg: Expr) -> Self {
        Expr::Call(Box::new(f), Box::new(arg), false)
    }

    /// Returns a compile-time call `f(arg)`
    pub(super) fn ct_call(f: Expr, arg: Expr) -> Self {
        Expr::Call(Box::new(f), Box::new(arg), true)
    }

    /// Traverse this atom, apply the given functions to each FunctionId and ParameterId
    pub(super) fn for_each_id<T, F, P>(&self, data: &mut T, mut on_function: F, mut on_parameter: P) where
        F: FnMut(&mut T, &FunctionId),
        P: FnMut(&mut T, &ParameterId),
    {
        self.for_each_id_helper(data, &mut on_function, &mut on_parameter);
    }

    fn for_each_id_helper<T, F, P>(&self, data: &mut T, on_function: &mut F, on_parameter: &mut P) where
        F: FnMut(&mut T, &FunctionId),
        P: FnMut(&mut T, &ParameterId),
    {
        let mut both = |data: &mut T, lhs: &Expr, rhs: &Expr| {
            lhs.for_each_id_helper(data, on_function, on_parameter);
            rhs.for_each_id_helper(data, on_function, on_parameter);
        };

        match self {
            Expr::If(condition, then, otherwise) => {
                condition.for_each_id_helper(data, on_function, on_parameter);
                then.for_each_id_helper(data, on_function, on_parameter);
                otherwise.for_each_id_helper(data, on_function, on_parameter);
            },
            Expr::Switch(expr, cases, else_case) => {
                expr.for_each_id_helper(data, on_function, on_parameter);
                for (_, case_continuation) in cases {
                    on_function(data, case_continuation);
                }
                if let Some(else_continuation) = else_case {
                    on_function(data, else_continuation);
                }
            },
            Expr::Call(f, arg, _) => {
                f.for_each_id_helper(data, on_function, on_parameter);
                arg.for_each_id_helper(data, on_function, on_parameter);
            }
            Expr::Literal(_) => (),
            Expr::Parameter(parameter_id) => on_parameter(data, parameter_id),
            Expr::Function(function_id) => on_function(data, function_id),
            Expr::Tuple(fields) => {
                for field in fields {
                    field.for_each_id_helper(data, on_function, on_parameter);
                }
            },
            Expr::Assign => (),
            Expr::Extern(_) => (),
            Expr::MemberAccess(lhs, _, _) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::AddInt(lhs, rhs) => both(data, lhs, rhs),
            Expr::AddFloat(lhs, rhs) => both(data, lhs, rhs),
            Expr::SubInt(lhs, rhs) => both(data, lhs, rhs),
            Expr::SubFloat(lhs, rhs) => both(data, lhs, rhs),
            Expr::MulInt(lhs, rhs) => both(data, lhs, rhs),
            Expr::MulFloat(lhs, rhs) => both(data, lhs, rhs),
            Expr::DivSigned(lhs, rhs) => both(data, lhs, rhs),
            Expr::DivUnsigned(lhs, rhs) => both(data, lhs, rhs),
            Expr::DivFloat(lhs, rhs) => both(data, lhs, rhs),
            Expr::ModSigned(lhs, rhs) => both(data, lhs, rhs),
            Expr::ModUnsigned(lhs, rhs) => both(data, lhs, rhs),
            Expr::ModFloat(lhs, rhs) => both(data, lhs, rhs),
            Expr::LessSigned(lhs, rhs) => both(data, lhs, rhs),
            Expr::LessUnsigned(lhs, rhs) => both(data, lhs, rhs),
            Expr::LessFloat(lhs, rhs) => both(data, lhs, rhs),
            Expr::EqInt(lhs, rhs) => both(data, lhs, rhs),
            Expr::EqFloat(lhs, rhs) => both(data, lhs, rhs),
            Expr::EqChar(lhs, rhs) => both(data, lhs, rhs),
            Expr::EqBool(lhs, rhs) => both(data, lhs, rhs),
            Expr::SignExtend(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::ZeroExtend(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::SignedToFloat(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::UnsignedToFloat(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::FloatToSigned(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::FloatToUnsigned(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::FloatPromote(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::FloatDemote(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::BitwiseAnd(lhs, rhs) => both(data, lhs, rhs),
            Expr::BitwiseOr(lhs, rhs) => both(data, lhs, rhs),
            Expr::BitwiseXor(lhs, rhs) => both(data, lhs, rhs),
            Expr::BitwiseNot(lhs) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::Truncate(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::Deref(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::Offset(lhs, rhs, _typ) => both(data, lhs, rhs),
            Expr::Transmute(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Expr::StackAlloc(lhs) => lhs.for_each_id_helper(data, on_function, on_parameter),
        }
    }

    fn map_functions(&mut self, substitutions: &ExprMap) {
        let both = |lhs: &mut Expr, rhs: &mut Expr| {
            lhs.map_functions(substitutions);
            rhs.map_functions(substitutions);
        };

        match self {
            Expr::If(condition, then, otherwise) => {
                condition.map_functions(substitutions);
                then.map_functions(substitutions);
                otherwise.map_functions(substitutions);
            },
            Expr::Switch(expr, cases, else_case) => {
                expr.map_functions(substitutions);
                for (_, case_continuation) in cases {
                    if let Some(substitution) = substitutions.functions.get(case_continuation) {
                        *case_continuation = substitution.clone();
                    }
                }
                if let Some(else_continuation) = else_case {
                    if let Some(substitution) = substitutions.functions.get(else_continuation) {
                        *else_continuation = substitution.clone();
                    }
                }
            },
            Expr::Call(f, arg, _) => {
                f.map_functions(substitutions);
                arg.map_functions(substitutions);
            }
            Expr::Literal(_) => (),
            Expr::Parameter(parameter_id) => {
                if let Some(substitution) = substitutions.parameters.get(parameter_id) {
                    *self = substitution.clone();
                } else if let Some(substitution) = substitutions.functions.get(&parameter_id.function) {
                    parameter_id.function = substitution.clone();
                }
            },
            Expr::Function(function_id) => {
                if let Some(substitution) = substitutions.functions.get(function_id) {
                    *function_id = substitution.clone();
                }
            },
            Expr::Tuple(fields) => {
                for field in fields {
                    field.map_functions(substitutions);
                }
            },
            Expr::Assign => (),
            Expr::Extern(_) => (),
            Expr::MemberAccess(lhs, _, _) => lhs.map_functions(substitutions),
            Expr::AddInt(lhs, rhs) => both(lhs, rhs),
            Expr::AddFloat(lhs, rhs) => both(lhs, rhs),
            Expr::SubInt(lhs, rhs) => both(lhs, rhs),
            Expr::SubFloat(lhs, rhs) => both(lhs, rhs),
            Expr::MulInt(lhs, rhs) => both(lhs, rhs),
            Expr::MulFloat(lhs, rhs) => both(lhs, rhs),
            Expr::DivSigned(lhs, rhs) => both(lhs, rhs),
            Expr::DivUnsigned(lhs, rhs) => both(lhs, rhs),
            Expr::DivFloat(lhs, rhs) => both(lhs, rhs),
            Expr::ModSigned(lhs, rhs) => both(lhs, rhs),
            Expr::ModUnsigned(lhs, rhs) => both(lhs, rhs),
            Expr::ModFloat(lhs, rhs) => both(lhs, rhs),
            Expr::LessSigned(lhs, rhs) => both(lhs, rhs),
            Expr::LessUnsigned(lhs, rhs) => both(lhs, rhs),
            Expr::LessFloat(lhs, rhs) => both(lhs, rhs),
            Expr::EqInt(lhs, rhs) => both(lhs, rhs),
            Expr::EqFloat(lhs, rhs) => both(lhs, rhs),
            Expr::EqChar(lhs, rhs) => both(lhs, rhs),
            Expr::EqBool(lhs, rhs) => both(lhs, rhs),
            Expr::SignExtend(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::ZeroExtend(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::SignedToFloat(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::UnsignedToFloat(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::FloatToSigned(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::FloatToUnsigned(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::FloatPromote(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::FloatDemote(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::BitwiseAnd(lhs, rhs) => both(lhs, rhs),
            Expr::BitwiseOr(lhs, rhs) => both(lhs, rhs),
            Expr::BitwiseXor(lhs, rhs) => both(lhs, rhs),
            Expr::BitwiseNot(lhs) => lhs.map_functions(substitutions),
            Expr::Truncate(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::Deref(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::Offset(lhs, rhs, _typ) => both(lhs, rhs),
            Expr::Transmute(lhs, _typ) => lhs.map_functions(substitutions),
            Expr::StackAlloc(lhs) => lhs.map_functions(substitutions),
        }
    }
}

/// This type representation is largely the same as a HIR type
/// except functions have a continuation parameter instead of a return type.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Function(/*arg:*/Box<Type>, /*ret:*/ Option<Box<Type>>, /*compile_time:*/bool),

    /// Tuples have a TypeId to allow for struct recursion
    Tuple(Vec<Type>),
}

impl Type {
    /// Create a function type with the given arguments and return type
    pub(super) fn function(mut args: Vec<Type>, return_type: Type, compile_time: bool) -> Type {
        if args.len() == 1 {
            let arg = args.pop().unwrap();
            Type::Function(Box::new(arg), Some(Box::new(return_type)), compile_time)
        } else {
            let first = args.remove(0);
            let rest = Type::function(args, return_type, compile_time);
            Type::Function(Box::new(first), Some(Box::new(rest)), compile_time)
        }
    }

    pub(super) fn continuation(arg: Type, compile_time: bool) -> Type {
        Type::Function(Box::new(arg), None, compile_time)
    }

    pub(super) fn unit() -> Type {
        Type::Primitive(PrimitiveType::Unit)
    }

    /// True if this type is a function or indirectly contains one
    pub(super) fn contains_function(&self) -> bool {
        match self {
            Type::Primitive(_) => false,
            Type::Function(..) => true,
            Type::Tuple(args) => args.iter().any(|arg| arg.contains_function()),
        }
    }
}

/// Each effect used modifies a function's type and inserts 3 extra parameters specified in the
/// comment for each field below. This struct stores the indices of these parameters in a FunctionType
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct EffectIndices {
    pub effect_id: EffectId,

    /// effect: A parameter for the effectful operation itself.
    ///         An effectful function of type `A -> B` will have the
    ///         type `fn(A, fn(B, fn(H)), fn(H))` in this IR where `H`
    ///         is the return type of the effect handler.
    pub effect_index: u16,

    /// k: All functions in CPS form have a continuation argument normally
    ///    of type `fn(Ret)`. In effectful functions, the type of this continuation
    ///    is modified to `fn(Ret, fn(H))` where `H` is the handler type.
    pub k_index: u16,

    /// effect_k: The effect handler's continuation of type `fn(H)`.
    pub effect_k_index: u16,
}

#[derive(Default)]
pub struct ExprMap {
    pub parameters: HashMap<ParameterId, Expr>,
    pub functions: HashMap<FunctionId, FunctionId>,
}
