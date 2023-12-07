mod id;

use std::{collections::HashMap, fmt::Display, rc::Rc};

use crate::hir::{Literal, PrimitiveType};
pub use id::*;

#[derive(Default)]
pub struct Mir {
    pub functions: HashMap<FunctionId, Function>,
    pub extern_symbols: HashMap<String, (Type, ExternId)>,
}

impl Mir {
    pub fn main_id() -> FunctionId {
        FunctionId { id: 0, name: Rc::new("main".into()) }
    }
}

pub struct Function {
    pub id: FunctionId,
    pub argument_types: Vec<Type>,

    // A function's body is always a function call
    pub body_continuation: Atom,
    pub body_args: Vec<Atom>,
}

impl Function {
    /// Return an empty function with the given id that is expected to have its body filled in later
    pub fn empty(id: FunctionId) -> Self {
        Self { id, body_continuation: Atom::Literal(Literal::Unit), body_args: Vec::new(), argument_types: Vec::new() }
    }

    pub fn parameters(&self) -> impl ExactSizeIterator<Item = ParameterId> {
        let parameter_count = self.argument_types.len();
        let function = self.id.clone();
        (0 .. parameter_count).map(move |i| ParameterId {
            function: function.clone(),
            parameter_index: i as u16,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atom {
    /// An if-else branching. Expects 3 arguments: [cond, k_then, k_else]
    Branch,

    /// A switch case (derived from a match expression).
    /// Expects a list of cases with the value to match for each along
    /// with the continuation to jump to that case. Also expects an additional
    /// optional default case continuation.
    /// 
    /// This also expects one argument when called: the value to match.
    Switch(Vec<(u32, FunctionId)>, Option<FunctionId>),

    Literal(Literal),
    Parameter(ParameterId),
    Function(FunctionId),

    Tuple(Vec<Atom>),
    MemberAccess(Box<Atom>, u32, Type),

    /// Assignment expects 3 arguments: [lvalue, rvalue, k]
    Assign,

    /// Extern expects 1 argument: [k]
    ///
    /// The ExternId can be used to retrieve the name and type
    /// of the symbol being referenced. An ID is used to ensure
    /// the same symbol is not imported multiple times.
    Extern(ExternId),

    /// Expects one argument: [expression]
    ///
    /// Where k is the start of the expression being handled.
    ///
    /// The HandlerId uniquely identifies this handler node, and the
    /// Box<Atom> corresponds to the branch to take when the effect
    /// is performed in the `expression` argument.
    ///
    /// Handlers initially correspond to the control-flow:
    /// fn with Atom::Handle -> `expression` fn -> end function
    /// When the handler is eventually removed from the Mir, the
    /// handler branch itself is eventually spliced within `expression`.
    Handle(HandlerId, Box<Atom>),

    /// Expects a varying number of arguments, depending on the effect
    /// defined by the user.
    ///
    /// Similar to Handle, this node will also be removed when effects
    /// are specialized away.
    Effect(EffectId, Type),

    AddInt(Box<Atom>, Box<Atom>),
    AddFloat(Box<Atom>, Box<Atom>),

    SubInt(Box<Atom>, Box<Atom>),
    SubFloat(Box<Atom>, Box<Atom>),

    MulInt(Box<Atom>, Box<Atom>),
    MulFloat(Box<Atom>, Box<Atom>),

    DivSigned(Box<Atom>, Box<Atom>),
    DivUnsigned(Box<Atom>, Box<Atom>),
    DivFloat(Box<Atom>, Box<Atom>),

    ModSigned(Box<Atom>, Box<Atom>),
    ModUnsigned(Box<Atom>, Box<Atom>),
    ModFloat(Box<Atom>, Box<Atom>),

    LessSigned(Box<Atom>, Box<Atom>),
    LessUnsigned(Box<Atom>, Box<Atom>),
    LessFloat(Box<Atom>, Box<Atom>),

    EqInt(Box<Atom>, Box<Atom>),
    EqFloat(Box<Atom>, Box<Atom>),
    EqChar(Box<Atom>, Box<Atom>),
    EqBool(Box<Atom>, Box<Atom>),

    SignExtend(Box<Atom>, Type),
    ZeroExtend(Box<Atom>, Type),

    SignedToFloat(Box<Atom>, Type),
    UnsignedToFloat(Box<Atom>, Type),
    FloatToSigned(Box<Atom>, Type),
    FloatToUnsigned(Box<Atom>, Type),
    FloatPromote(Box<Atom>, Type),
    FloatDemote(Box<Atom>, Type),

    BitwiseAnd(Box<Atom>, Box<Atom>),
    BitwiseOr(Box<Atom>, Box<Atom>),
    BitwiseXor(Box<Atom>, Box<Atom>),
    BitwiseNot(Box<Atom>),

    Truncate(Box<Atom>, Type),
    Deref(Box<Atom>, Type),
    Offset(Box<Atom>, Box<Atom>, Type),
    Transmute(Box<Atom>, Type),

    /// Allocate space for the given value on the stack, and store it there. Return the stack address
    StackAlloc(Box<Atom>),
}

impl Atom {
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
        let mut both = |data: &mut T, lhs: &Atom, rhs: &Atom| {
            lhs.for_each_id_helper(data, on_function, on_parameter);
            rhs.for_each_id_helper(data, on_function, on_parameter);
        };

        match self {
            Atom::Branch => (),
            Atom::Switch(cases, else_case) => {
                for (_, case_continuation) in cases {
                    on_function(data, case_continuation);
                }
                if let Some(else_continuation) = else_case {
                    on_function(data, else_continuation);
                }
            },
            Atom::Literal(_) => (),
            Atom::Parameter(parameter_id) => on_parameter(data, parameter_id),
            Atom::Function(function_id) => on_function(data, function_id),
            Atom::Tuple(fields) => {
                for field in fields {
                    field.for_each_id_helper(data, on_function, on_parameter);
                }
            },
            Atom::Assign => (),
            Atom::Extern(_) => (),
            Atom::MemberAccess(lhs, _, _) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::Handle(_, branch) => branch.for_each_id_helper(data, on_function, on_parameter),
            Atom::Effect(_, _) => (),
            Atom::AddInt(lhs, rhs) => both(data, lhs, rhs),
            Atom::AddFloat(lhs, rhs) => both(data, lhs, rhs),
            Atom::SubInt(lhs, rhs) => both(data, lhs, rhs),
            Atom::SubFloat(lhs, rhs) => both(data, lhs, rhs),
            Atom::MulInt(lhs, rhs) => both(data, lhs, rhs),
            Atom::MulFloat(lhs, rhs) => both(data, lhs, rhs),
            Atom::DivSigned(lhs, rhs) => both(data, lhs, rhs),
            Atom::DivUnsigned(lhs, rhs) => both(data, lhs, rhs),
            Atom::DivFloat(lhs, rhs) => both(data, lhs, rhs),
            Atom::ModSigned(lhs, rhs) => both(data, lhs, rhs),
            Atom::ModUnsigned(lhs, rhs) => both(data, lhs, rhs),
            Atom::ModFloat(lhs, rhs) => both(data, lhs, rhs),
            Atom::LessSigned(lhs, rhs) => both(data, lhs, rhs),
            Atom::LessUnsigned(lhs, rhs) => both(data, lhs, rhs),
            Atom::LessFloat(lhs, rhs) => both(data, lhs, rhs),
            Atom::EqInt(lhs, rhs) => both(data, lhs, rhs),
            Atom::EqFloat(lhs, rhs) => both(data, lhs, rhs),
            Atom::EqChar(lhs, rhs) => both(data, lhs, rhs),
            Atom::EqBool(lhs, rhs) => both(data, lhs, rhs),
            Atom::SignExtend(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::ZeroExtend(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::SignedToFloat(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::UnsignedToFloat(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::FloatToSigned(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::FloatToUnsigned(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::FloatPromote(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::FloatDemote(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::BitwiseAnd(lhs, rhs) => both(data, lhs, rhs),
            Atom::BitwiseOr(lhs, rhs) => both(data, lhs, rhs),
            Atom::BitwiseXor(lhs, rhs) => both(data, lhs, rhs),
            Atom::BitwiseNot(lhs) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::Truncate(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::Deref(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::Offset(lhs, rhs, _typ) => both(data, lhs, rhs),
            Atom::Transmute(lhs, _typ) => lhs.for_each_id_helper(data, on_function, on_parameter),
            Atom::StackAlloc(lhs) => lhs.for_each_id_helper(data, on_function, on_parameter),
        }
    }
}

/// This type representation is largely the same as a HIR type
/// except functions have a continuation parameter instead of a return type.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Function(/*parameters:*/Vec<Type>, /*effect parameters:*/ Vec<(EffectId, Type)>),

    /// Tuples have a TypeId to allow for struct recursion
    Tuple(Vec<Type>),
}

impl Type {
    /// Returns the arguments of the continuation of this function type. E.g:
    ///
    /// `fn(A, B, fn(C, D))`.get_continuation_types(..) = `vec![C, D]`
    ///
    /// Panics if this is not a function type, and prints the given debug_label in the error.
    pub(super) fn get_continuation_types(&self, debug_label: impl Display) -> Vec<Type> {
        match self {
            Type::Function(arguments, _effect_args) => {
                let continuation_type = arguments.last().unwrap_or_else(|| panic!("Expected at least 1 argument from {}", debug_label));
                match continuation_type {
                    Type::Function(arguments, _effect_args) => arguments.clone(),
                    other => unreachable!("Expected function type, found {} in {}", other, debug_label),
                }
            }
            other => unreachable!("Expected function type, found {} in {}", other, debug_label),
        }
    }
}
