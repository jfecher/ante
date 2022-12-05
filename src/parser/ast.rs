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
//!   `decision_tree: Option<DecisionTree>` for `ast::Match`s
use crate::cache::{DefinitionInfoId, EffectInfoId, ImplInfoId, ImplScopeId, ModuleId, TraitInfoId, VariableId};
use crate::error::location::{Locatable, Location};
use crate::lexer::token::{FloatKind, IntegerKind, Token};
use crate::types::pattern::DecisionTree;
use crate::types::traits::RequiredTrait;
use crate::types::typechecker::TypeBindings;
use crate::types::{self, LetBindingLevel, TypeInfoId};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    Integer(u64, IntegerKind),
    Float(u64, FloatKind),
    String(String),
    Char(char),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub struct Literal<'a> {
    pub kind: LiteralKind,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum VariableKind {
    Identifier(String),
    Operator(Token),
    TypeConstructor(String),
}

/// a, b, (+), Some, etc.
#[derive(Debug, Clone)]
pub struct Variable<'a> {
    pub kind: VariableKind,
    pub location: Location<'a>,

    /// module prefix path
    pub module_prefix: Vec<String>,

    /// A variable's definition is initially undefined.
    /// During name resolution, every definition is filled
    /// out - becoming Some(id)
    pub definition: Option<DefinitionInfoId>,

    /// The module this Variable is contained in. Determines which
    /// impls are visible to it during type inference.
    pub impl_scope: Option<ImplScopeId>,

    /// The mapping used to instantiate the definition type of this
    /// variable into a monotype, if any.
    pub instantiation_mapping: Rc<TypeBindings>,

    /// A unique ID that can be used to identify this variable node
    pub id: Option<VariableId>,

    pub typ: Option<types::Type>,
}

// TODO: Remove. This is only used for experimenting with ante-lsp
// which does not refer to the instantiation_mapping field at all.
unsafe impl<'c> Send for Variable<'c> {}
unsafe impl<'c> Sync for Variable<'c> {}

/// Maps DefinitionInfoIds closed over in the environment to their new
/// IDs within the closure which shadow their previous definition.
/// These new IDs may be instantiations of a type that was generalized
/// (but is now bound to a concrete type as a function parameter as the new id),
/// so we need to remember these instatiation bindings as well.
///
/// Needed because closure environment variables are converted to
/// parameters of the function which need separate IDs.
pub type ClosureEnvironment = BTreeMap<
    DefinitionInfoId,
    (
        /*Confusing: This is a variable id for the DefinitionInfoId key, used for trait dispatch.*/
        VariableId,
        DefinitionInfoId,
        Rc<TypeBindings>,
    ),
>;

/// \a b. expr
/// Function definitions are also desugared to a ast::Definition with a ast::Lambda as its body
#[derive(Debug, Clone)]
pub struct Lambda<'a> {
    pub args: Vec<Ast<'a>>,
    pub body: Box<Ast<'a>>,
    pub return_type: Option<Type<'a>>,

    pub closure_environment: ClosureEnvironment,

    pub required_traits: Vec<RequiredTrait>,

    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

// TODO: Remove. This is only used for experimenting with ante-lsp
// which does not refer to the instantiation_mapping field at all.
unsafe impl<'c> Send for Lambda<'c> {}
unsafe impl<'c> Sync for Lambda<'c> {}

/// foo a b c
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct If<'a> {
    pub condition: Box<Ast<'a>>,
    pub then: Box<Ast<'a>>,
    pub otherwise: Option<Box<Ast<'a>>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// match expression
/// | pattern1 -> branch1
/// | pattern2 -> branch2
/// ...
/// | patternN -> branchN
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Type<'a> {
    Integer(IntegerKind, Location<'a>),
    Float(FloatKind, Location<'a>),
    Char(Location<'a>),
    String(Location<'a>),
    Pointer(Location<'a>),
    Boolean(Location<'a>),
    Unit(Location<'a>),
    Reference(Location<'a>),
    Function(Vec<Type<'a>>, Box<Type<'a>>, /*varargs:*/ bool, Location<'a>),
    TypeVariable(String, Location<'a>),
    UserDefined(String, Location<'a>),
    TypeApplication(Box<Type<'a>>, Vec<Type<'a>>, Location<'a>),
    Pair(Box<Type<'a>>, Box<Type<'a>>, Location<'a>),
}

/// The AST representation of a trait usage.
/// A trait's definition would be a TraitDefinition node.
/// This struct is used in e.g. `given` to list the required traits.
#[derive(Debug, Clone)]
pub struct Trait<'a> {
    pub name: String,
    pub args: Vec<Type<'a>>,
    pub location: Location<'a>,
}

#[derive(Debug, Clone)]
pub enum TypeDefinitionBody<'a> {
    Union(Vec<(String, Vec<Type<'a>>, Location<'a>)>),
    Struct(Vec<(String, Type<'a>, Location<'a>)>),
    Alias(Type<'a>),
}

/// type Name arg1 arg2 ... argN = definition
#[derive(Debug, Clone)]
pub struct TypeDefinition<'a> {
    pub name: String,
    pub args: Vec<String>,
    pub definition: TypeDefinitionBody<'a>,
    pub location: Location<'a>,
    pub type_info: Option<TypeInfoId>,
    pub typ: Option<types::Type>,
}

/// lhs : rhs
#[derive(Debug, Clone)]
pub struct TypeAnnotation<'a> {
    pub lhs: Box<Ast<'a>>,
    pub rhs: Type<'a>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// import Path1 . Path2 ... PathN
#[derive(Debug, Clone)]
pub struct Import<'a> {
    pub path: Vec<String>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
    pub module_id: Option<ModuleId>,
    pub symbols: HashSet<String>,
}

/// trait Name arg1 arg2 ... argN -> fundep1 fundep2 ... fundepN with
///     declaration1
///     declaration2
///     ...
///     declarationN
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct TraitImpl<'a> {
    pub trait_name: String,
    pub trait_args: Vec<Type<'a>>,
    pub given: Vec<Trait<'a>>,

    pub definitions: Vec<Definition<'a>>,
    pub location: Location<'a>,
    pub trait_info: Option<TraitInfoId>,
    pub impl_id: Option<ImplInfoId>,
    pub typ: Option<types::Type>,
    pub trait_arg_types: Vec<types::Type>, // = fmap(trait_args, convert_type)
}

/// return expression
#[derive(Debug, Clone)]
pub struct Return<'a> {
    pub expression: Box<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// statement1
/// statement2
/// ...
/// statementN
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct Extern<'a> {
    pub declarations: Vec<TypeAnnotation<'a>>,
    pub level: Option<LetBindingLevel>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// lhs.field
#[derive(Debug, Clone)]
pub struct MemberAccess<'a> {
    pub lhs: Box<Ast<'a>>,
    pub field: String,
    pub location: Location<'a>,
    /// True if this is an offset .& operation
    pub is_offset: bool,
    pub typ: Option<types::Type>,
}

/// lhs := rhs
#[derive(Debug, Clone)]
pub struct Assignment<'a> {
    pub lhs: Box<Ast<'a>>,
    pub rhs: Box<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

/// effect Name arg1 arg2 ... argN with
///     declaration1
///     declaration2
///     ...
///     declarationN
#[derive(Debug, Clone)]
pub struct EffectDefinition<'a> {
    pub name: String,
    pub args: Vec<String>,

    pub declarations: Vec<TypeAnnotation<'a>>,
    pub level: Option<LetBindingLevel>,
    pub location: Location<'a>,
    pub effect_info: Option<EffectInfoId>,
    pub typ: Option<types::Type>,
}

/// handle expression
/// | pattern1 -> branch1
/// | pattern2 -> branch2
/// ...
/// | patternN -> branchN
///
/// Handle expressions desugar to 1 case per
/// effect or `return`, with any nested patterns
/// deferring to match expressions.
#[derive(Debug, Clone)]
pub struct Handle<'a> {
    pub expression: Box<Ast<'a>>,
    pub branches: Vec<(Ast<'a>, Ast<'a>)>,

    /// IDs for each 'resume' variable (1 per branch) of this handle expression.
    /// This is filled out during name resolution.
    pub resumes: Vec<DefinitionInfoId>,

    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug, Clone)]
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
    EffectDefinition(EffectDefinition<'a>),
    Handle(Handle<'a>),
}

unsafe impl<'c> Send for Ast<'c> {}

impl PartialEq for LiteralKind {
    /// Ignoring any type tags, are these literals equal?
    fn eq(&self, other: &Self) -> bool {
        use LiteralKind::*;
        match (self, other) {
            (Integer(x, _), Integer(y, _)) => x == y,
            (Float(x, _), Float(y, _)) => x == y,
            (String(x), String(y)) => x == y,
            (Char(x), Char(y)) => x == y,
            (Bool(x), Bool(y)) => x == y,
            (Unit, Unit) => true,
            _ => false,
        }
    }
}

impl std::hash::Hash for LiteralKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            LiteralKind::Integer(x, _) => x.hash(state),
            LiteralKind::Float(x, _) => x.hash(state),
            LiteralKind::String(x) => x.hash(state),
            LiteralKind::Char(x) => x.hash(state),
            LiteralKind::Bool(x) => x.hash(state),
            LiteralKind::Unit => (),
        }
    }
}

/// These are all convenience functions for creating various Ast nodes from the parser
impl<'a> Ast<'a> {
    pub fn get_operator(self) -> Option<Token> {
        match self {
            Ast::Variable(variable) => match variable.kind {
                VariableKind::Operator(token) => Some(token),
                _ => None,
            },
            _ => None,
        }
    }

    /// True if this variable can be matched on, ie. it
    /// is both a Variable node and is not a VariableKind::TypeConstructor
    fn is_matchable_variable(&self) -> bool {
        match self {
            Ast::Variable(variable) => !matches!(variable.kind, VariableKind::TypeConstructor(..)),
            _ => false,
        }
    }

    pub fn integer(x: u64, kind: IntegerKind, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::Integer(x, kind), location, typ: None })
    }

    pub fn float(x: f64, kind: FloatKind, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal { kind: LiteralKind::Float(x.to_bits(), kind), location, typ: None })
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

    pub fn variable(module_prefix: Vec<String>, name: String, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable {
            kind: VariableKind::Identifier(name),
            module_prefix,
            location,
            definition: None,
            id: None,
            impl_scope: None,
            instantiation_mapping: Rc::new(HashMap::new()),
            typ: None,
        })
    }

    pub fn operator(operator: Token, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable {
            kind: VariableKind::Operator(operator),
            module_prefix: vec![],
            location,
            definition: None,
            id: None,
            impl_scope: None,
            instantiation_mapping: Rc::new(HashMap::new()),
            typ: None,
        })
    }

    pub fn type_constructor(module_prefix: Vec<String>, name: String, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable {
            kind: VariableKind::TypeConstructor(name),
            location,
            module_prefix,
            definition: None,
            id: None,
            impl_scope: None,
            instantiation_mapping: Rc::new(HashMap::new()),
            typ: None,
        })
    }

    pub fn lambda(args: Vec<Ast<'a>>, return_type: Option<Type<'a>>, body: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::Lambda(Lambda {
            args,
            body: Box::new(body),
            closure_environment: BTreeMap::new(),
            return_type,
            location,
            required_traits: vec![],
            typ: None,
        })
    }

    pub fn function_call(function: Ast<'a>, args: Vec<Ast<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::FunctionCall(FunctionCall { function: Box::new(function), args, location, typ: None })
    }

    pub fn if_expr(condition: Ast<'a>, then: Ast<'a>, otherwise: Option<Ast<'a>>, location: Location<'a>) -> Ast<'a> {
        
 
        match &otherwise{
            Some(_ast) => Ast::If(If {
            condition: Box::new(condition),
            then: Box::new(then),
            otherwise: otherwise.map(Box::new),
            location,
            typ: None,
        }),
            None => Ast::if_expr(condition,then, Some(Ast::unit_literal(location)), location)
        }
        //super::desugar::desugar_if(if_ast)
    }

    pub fn definition(pattern: Ast<'a>, expr: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::Definition(Definition {
            pattern: Box::new(pattern),
            expr: Box::new(expr),
            location,
            mutable: false,
            level: None,
            info: None,
            typ: None,
        })
    }

    pub fn match_expr(expression: Ast<'a>, mut branches: Vec<(Ast<'a>, Ast<'a>)>, location: Location<'a>) -> Ast<'a> {
        // (Issue #80) When compiling a match statement with a single variable branch e.g:
        // `match ... | x -> ... ` a single Leaf node will be emitted as the decision tree
        // after type checking which causes us to fail since `x` will not be bound to anything
        // without a `Case` node being present. This is a hack to avoid this situation by compiling
        // this class of expressions into let bindings instead.
        if branches.len() == 1 && branches[0].0.is_matchable_variable() {
            let (pattern, rest) = branches.pop().unwrap();
            let definition = Ast::definition(pattern, expression, location);
            // TODO: turning this into a sequence can leak names in the match branch to surrounding
            // code. Soundness-wise this isn't an issue since in this case we know it will always
            // match, but it is an inconsistency that should be fixed.
            Ast::sequence(vec![definition, rest], location)
        } else {
            Ast::Match(Match { expression: Box::new(expression), branches, decision_tree: None, location, typ: None })
        }
    }

    pub fn type_definition(
        name: String, args: Vec<String>, definition: TypeDefinitionBody<'a>, location: Location<'a>,
    ) -> Ast<'a> {
        Ast::TypeDefinition(TypeDefinition { name, args, definition, location, type_info: None, typ: None })
    }

    pub fn type_annotation(lhs: Ast<'a>, rhs: Type<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::TypeAnnotation(TypeAnnotation { lhs: Box::new(lhs), rhs, location, typ: None })
    }

    pub fn import(path: Vec<String>, location: Location<'a>, symbols: HashSet<String>) -> Ast<'a> {
        assert!(!path.is_empty());
        Ast::Import(Import { path, location, typ: None, module_id: None, symbols })
    }

    pub fn trait_definition(
        name: String, args: Vec<String>, fundeps: Vec<String>, declarations: Vec<TypeAnnotation<'a>>,
        location: Location<'a>,
    ) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::TraitDefinition(TraitDefinition {
            name,
            args,
            fundeps,
            declarations,
            location,
            level: None,
            trait_info: None,
            typ: None,
        })
    }

    pub fn trait_impl(
        trait_name: String, trait_args: Vec<Type<'a>>, given: Vec<Trait<'a>>, definitions: Vec<Definition<'a>>,
        location: Location<'a>,
    ) -> Ast<'a> {
        assert!(!trait_args.is_empty());
        Ast::TraitImpl(TraitImpl {
            trait_name,
            trait_args,
            given,
            definitions,
            location,
            trait_arg_types: vec![],
            impl_id: None,
            trait_info: None,
            typ: None,
        })
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

    pub fn member_access(lhs: Ast<'a>, field: String, is_offset: bool, location: Location<'a>) -> Ast<'a> {
        Ast::MemberAccess(MemberAccess { lhs: Box::new(lhs), field, is_offset, location, typ: None })
    }

    pub fn assignment(lhs: Ast<'a>, rhs: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::Assignment(Assignment { lhs: Box::new(lhs), rhs: Box::new(rhs), location, typ: None })
    }

    pub fn effect_definition(
        name: String, args: Vec<String>, declarations: Vec<TypeAnnotation<'a>>, location: Location<'a>,
    ) -> Ast<'a> {
        Ast::EffectDefinition(EffectDefinition {
            name,
            args,
            declarations,
            location,
            level: None,
            typ: None,
            effect_info: None,
        })
    }

    pub fn handle(expression: Ast<'a>, branches: Vec<(Ast<'a>, Ast<'a>)>, location: Location<'a>) -> Ast<'a> {
        let branches = super::desugar::desugar_handle_branches_into_matches(branches);
        Ast::Handle(Handle { expression: Box::new(expression), branches, location, resumes: vec![], typ: None })
    }

    /// This is a bit of a hack.
    /// Create a new 'scope' by wrapping body in `match () | () -> body`
    pub fn new_scope(body: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::match_expr(Ast::unit_literal(location), vec![(Ast::unit_literal(location), body)], location)
    }
}

/// A macro for calling a method on every variant of an Ast node.
/// Useful for implementing a trait for the Ast and every node inside.
/// This is used for all compiler passes, as well as the Locatable trait below.
macro_rules! dispatch_on_expr {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            $crate::parser::ast::Ast::Literal(inner) =>          $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Variable(inner) =>         $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Lambda(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::FunctionCall(inner) =>     $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Definition(inner) =>       $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::If(inner) =>               $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Match(inner) =>            $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TypeDefinition(inner) =>   $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TypeAnnotation(inner) =>   $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Import(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TraitDefinition(inner) =>  $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::TraitImpl(inner) =>        $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Return(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Sequence(inner) =>         $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Extern(inner) =>           $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::MemberAccess(inner) =>     $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Assignment(inner) =>       $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::EffectDefinition(inner) => $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Handle(inner) =>           $function(inner $(, $($args),* )? ),
        }
    });
}

impl<'a> Locatable<'a> for Ast<'a> {
    fn locate(&self) -> Location<'a> {
        dispatch_on_expr!(self, Locatable::locate)
    }
}

macro_rules! impl_locatable_for {
    ( $name:tt ) => {
        impl<'a> Locatable<'a> for $name<'a> {
            fn locate(&self) -> Location<'a> {
                self.location
            }
        }
    };
}

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
impl_locatable_for!(EffectDefinition);
impl_locatable_for!(Handle);

impl<'a> Locatable<'a> for Type<'a> {
    fn locate(&self) -> Location<'a> {
        match self {
            Type::Integer(_, location) => *location,
            Type::Float(_, location) => *location,
            Type::Char(location) => *location,
            Type::String(location) => *location,
            Type::Pointer(location) => *location,
            Type::Boolean(location) => *location,
            Type::Unit(location) => *location,
            Type::Reference(location) => *location,
            Type::Function(_, _, _, location) => *location,
            Type::TypeVariable(_, location) => *location,
            Type::UserDefined(_, location) => *location,
            Type::TypeApplication(_, _, location) => *location,
            Type::Pair(_, _, location) => *location,
        }
    }
}
