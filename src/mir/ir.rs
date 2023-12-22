mod id;

use std::{collections::HashMap, rc::Rc};

use crate::hir::{Literal, PrimitiveType};
pub use id::*;

#[derive(Default)]
pub struct Mir {
    pub functions: HashMap<FunctionId, Function>,

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

    pub(super) fn for_each_id<T, F, P>(&self, data: &mut T, mut on_function: F, mut on_parameter: P) where
        F: FnMut(&mut T, &FunctionId),
        P: FnMut(&mut T, &ParameterId),
    {
        self.body_continuation.for_each_id(data, &mut on_function, &mut on_parameter);
        
        for arg in &self.body_args {
            arg.for_each_id(data, &mut on_function, &mut on_parameter);
        }
    }

    /// Mutate any FunctionIds in this function's body to a new FunctionId.
    ///
    /// Unlike `for_each_id`, this method also applies to FunctionIds within ParameterIds.
    pub(super) fn map_functions(&mut self, substitutions: &AtomMap) {
        self.body_continuation.map_functions(substitutions);
        
        for arg in &mut self.body_args {
            arg.map_functions(substitutions);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    fn map_functions(&mut self, substitutions: &AtomMap) {
        let both = |lhs: &mut Atom, rhs: &mut Atom| {
            lhs.map_functions(substitutions);
            rhs.map_functions(substitutions);
        };

        match self {
            Atom::Branch => (),
            Atom::Switch(cases, else_case) => {
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
            Atom::Literal(_) => (),
            Atom::Parameter(parameter_id) => {
                if let Some(substitution) = substitutions.parameters.get(parameter_id) {
                    *self = substitution.clone();
                } else if let Some(substitution) = substitutions.functions.get(&parameter_id.function) {
                    parameter_id.function = substitution.clone();
                }
            },
            Atom::Function(function_id) => {
                if let Some(substitution) = substitutions.functions.get(function_id) {
                    *function_id = substitution.clone();
                }
            },
            Atom::Tuple(fields) => {
                for field in fields {
                    field.map_functions(substitutions);
                }
            },
            Atom::Assign => (),
            Atom::Extern(_) => (),
            Atom::MemberAccess(lhs, _, _) => lhs.map_functions(substitutions),
            Atom::AddInt(lhs, rhs) => both(lhs, rhs),
            Atom::AddFloat(lhs, rhs) => both(lhs, rhs),
            Atom::SubInt(lhs, rhs) => both(lhs, rhs),
            Atom::SubFloat(lhs, rhs) => both(lhs, rhs),
            Atom::MulInt(lhs, rhs) => both(lhs, rhs),
            Atom::MulFloat(lhs, rhs) => both(lhs, rhs),
            Atom::DivSigned(lhs, rhs) => both(lhs, rhs),
            Atom::DivUnsigned(lhs, rhs) => both(lhs, rhs),
            Atom::DivFloat(lhs, rhs) => both(lhs, rhs),
            Atom::ModSigned(lhs, rhs) => both(lhs, rhs),
            Atom::ModUnsigned(lhs, rhs) => both(lhs, rhs),
            Atom::ModFloat(lhs, rhs) => both(lhs, rhs),
            Atom::LessSigned(lhs, rhs) => both(lhs, rhs),
            Atom::LessUnsigned(lhs, rhs) => both(lhs, rhs),
            Atom::LessFloat(lhs, rhs) => both(lhs, rhs),
            Atom::EqInt(lhs, rhs) => both(lhs, rhs),
            Atom::EqFloat(lhs, rhs) => both(lhs, rhs),
            Atom::EqChar(lhs, rhs) => both(lhs, rhs),
            Atom::EqBool(lhs, rhs) => both(lhs, rhs),
            Atom::SignExtend(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::ZeroExtend(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::SignedToFloat(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::UnsignedToFloat(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::FloatToSigned(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::FloatToUnsigned(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::FloatPromote(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::FloatDemote(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::BitwiseAnd(lhs, rhs) => both(lhs, rhs),
            Atom::BitwiseOr(lhs, rhs) => both(lhs, rhs),
            Atom::BitwiseXor(lhs, rhs) => both(lhs, rhs),
            Atom::BitwiseNot(lhs) => lhs.map_functions(substitutions),
            Atom::Truncate(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::Deref(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::Offset(lhs, rhs, _typ) => both(lhs, rhs),
            Atom::Transmute(lhs, _typ) => lhs.map_functions(substitutions),
            Atom::StackAlloc(lhs) => lhs.map_functions(substitutions),
        }
    }
}

/// This type representation is largely the same as a HIR type
/// except functions have a continuation parameter instead of a return type.
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Type {
    Primitive(PrimitiveType),
    Function(Vec<Type>, Vec<EffectIndices>),

    /// Tuples have a TypeId to allow for struct recursion
    Tuple(Vec<Type>),
}

impl Type {
    /// Create a function type with the given arguments and return type
    pub(super) fn function(mut args: Vec<Type>, return_type: Type) -> Type {
        args.push(Type::Function(vec![return_type], vec![]));
        Type::Function(args, vec![])
    }

    /// True if this type is a function or indirectly contains one
    pub(super) fn contains_function(&self) -> bool {
        match self {
            Type::Primitive(_) => false,
            Type::Function(_, _) => true,
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
pub struct AtomMap {
    pub parameters: HashMap<ParameterId, Atom>,
    pub functions: HashMap<FunctionId, FunctionId>,
}
