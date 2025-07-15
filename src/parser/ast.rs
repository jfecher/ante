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
use crate::cache::{DefinitionInfoId, VariableId};
use crate::error::location::Location;
use crate::lexer::token::{FloatKind, IntegerKind, Token};
use crate::types::typechecker::TypeBindings;
use crate::types::{self, TypeVariableId};
use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;
use std::rc::Rc;

/// ExprIds are used to associate additional information with an Ast node.
/// This information is either rarely used or filled out later. Most nodes
/// will have a Location and eventually a Type associated with their id.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct ExprId(u32);

#[derive(Clone, Debug, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    Integer(u64, Option<IntegerKind>),
    Float(u64, Option<FloatKind>),
    String(String),
    Char(char),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub struct Literal {
    pub id: ExprId,
    pub kind: LiteralKind,
}

#[derive(Debug, PartialEq, Clone)]
pub enum VariableKind {
    Identifier(String),
    Operator(Token),
    TypeConstructor(String),
}

impl VariableKind {
    pub fn name(&self) -> Cow<str> {
        match self {
            VariableKind::Identifier(name) => Cow::Borrowed(name),
            VariableKind::TypeConstructor(name) => Cow::Borrowed(name),
            VariableKind::Operator(token) => Cow::Owned(token.to_string()),
        }
    }
}

/// a, b, (+), Some, etc.
#[derive(Debug, Clone)]
pub struct Variable {
    // Variable's can have the following associated with their id:
    // - A Location
    // - A DefinitionInfoId referring to this variable's definition
    // - Instantiation bindings referring to the type variables this was instantiated with
    // - A Type
    pub id: ExprId,

    pub kind: VariableKind,

    /// module prefix path
    pub module_prefix: Vec<String>,
}

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
pub struct Lambda {
    /// Associated with: Type, Location, ClosureEnvironment, required_traits: Vec<RequiredTrait>
    pub id: ExprId,

    pub args: Vec<Ast>,
    pub body: Box<Ast>,
    pub return_type: Option<Type>,

    pub effects: Option<Vec<EffectAst>>,
}

pub type EffectAst = (EffectName, Location, Vec<Type>);

#[derive(Debug, Clone)]
pub enum EffectName {
    Name(String),
    ImplicitEffect(TypeVariableId),
}

/// foo a b c
#[derive(Debug, Clone)]
pub struct FunctionCall {
    /// Associated with: Location, Type
    pub id: ExprId,
    pub function: Box<Ast>,
    pub args: Vec<Ast>,
}

impl FunctionCall {
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
pub struct Definition {
    /// Associated with: Location, Type, LetBindingLevel
    pub id: ExprId,
    pub pattern: Box<Ast>,
    pub expr: Box<Ast>,
    pub mutable: bool,
}

/// if condition then expression else expression
#[derive(Debug, Clone)]
pub struct If {
    /// Associated with: Location, Type
    pub id: ExprId,
    pub condition: Box<Ast>,
    pub then: Box<Ast>,
    pub otherwise: Box<Ast>,
}

/// match expression
/// | pattern1 -> branch1
/// | pattern2 -> branch2
/// ...
/// | patternN -> branchN
#[derive(Debug, Clone)]
pub struct Match {
    /// Associated with: Location, Type, DecisionTree
    /// The decision tree is outputted from the completeness checking
    /// step and is used during codegen to efficiently compile each pattern branch.
    pub id: ExprId,

    pub expression: Box<Ast>,
    pub branches: Vec<(Ast, Ast)>,
}

/// Type nodes in the AST, different from the representation of types during type checking.
/// PointerType and potentially UserDefinedType are actually type constructors
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum Type {
    // Optional IntegerKind, None = polymorphic int
    Integer(Option<IntegerKind>, Location),
    // Optional FloatKind, None = polymorphic float
    Float(Option<FloatKind>, Location),
    Char(Location),
    String(Location),
    Pointer(Location),
    Boolean(Location),
    Unit(Location),
    Reference(Sharedness, Mutability, Location),
    Function(FunctionType),
    TypeVariable(String, Location),
    UserDefined(String, Location),
    TypeApplication(Box<Type>, Vec<Type>, Location),
    Pair(Box<Type>, Box<Type>, Location),
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub has_varargs: bool,
    pub is_closure: bool,
    pub effects: Option<Vec<EffectAst>>,
    pub location: Location,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sharedness {
    Polymorphic,
    Shared,
    Owned,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mutability {
    Polymorphic,
    Immutable,
    Mutable,
}

impl Mutability {
    pub(crate) fn as_tag(&self) -> types::TypeTag {
        match self {
            Mutability::Polymorphic => panic!("as_tag called on Mutability::Polymorphic"),
            Mutability::Immutable => types::TypeTag::Immutable,
            Mutability::Mutable => types::TypeTag::Mutable,
        }
    }
}

impl Display for Sharedness {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sharedness::Polymorphic => Ok(()),
            Sharedness::Shared => write!(f, "shared"),
            Sharedness::Owned => write!(f, "owned"),
        }
    }
}

impl Display for Mutability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mutability::Polymorphic => write!(f, "?"),
            Mutability::Immutable => write!(f, "&"),
            Mutability::Mutable => write!(f, "!"),
        }
    }
}

/// The AST representation of a trait usage.
/// A trait's definition would be a TraitDefinition node.
/// This struct is used in e.g. `given` to list the required traits.
#[derive(Debug, Clone)]
pub struct Trait {
    pub name: String,
    pub args: Vec<Type>,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub enum TypeDefinitionBody {
    Union(Vec<(String, Vec<Type>, Location)>),
    Struct(Vec<(String, Type, Location)>),
    Alias(Type),
}

/// type Name arg1 arg2 ... argN = definition
#[derive(Debug, Clone)]
pub struct TypeDefinition {
    /// Associated with: Location, TypeInfoId
    pub id: ExprId,

    #[allow(unused)]
    pub shared: bool,
    pub name: String,
    pub args: Vec<String>,
    pub definition: TypeDefinitionBody,
}

/// lhs : rhs
#[derive(Debug, Clone)]
pub struct TypeAnnotation {
    /// Associated with: Location, Type
    pub id: ExprId,

    pub lhs: Box<Ast>,
    pub rhs: Type,
}

/// import Path1 . Path2 ... PathN
#[derive(Debug, Clone)]
pub struct Import {
    /// Associated with: Location, ModuleId
    pub id: ExprId,

    pub path: Vec<String>,
    pub symbols: HashSet<String>,
}

/// trait Name arg1 arg2 ... argN -> fundep1 fundep2 ... fundepN with
///     declaration1
///     declaration2
///     ...
///     declarationN
#[derive(Debug, Clone)]
pub struct TraitDefinition {
    /// Associated with: Location, TraitInfoId, LetBindingLevel
    pub id: ExprId,

    pub name: String,
    pub args: Vec<String>,
    pub fundeps: Vec<String>,

    // Storing function declarations as TypeAnnotations here
    // throws away any names given to parameters. In practice
    // this shouldn't matter until refinement types are implemented
    // that can depend upon these names.
    pub declarations: Vec<TypeAnnotation>,
}

/// impl TraitName TraitArg1 TraitArg2 ... TraitArgN
///     definition1
///     definition2
///     ...
///     definitionN
#[derive(Debug, Clone)]
pub struct TraitImpl {
    /// Associated with: Location, TraitInfoId, ImplInfoId, trait_arg_types: Vec<types::Type>
    pub id: ExprId,

    pub trait_name: String,
    pub trait_args: Vec<Type>,
    pub given: Vec<Trait>,
    pub definitions: Vec<Definition>,
}

/// return expression
#[derive(Debug, Clone)]
pub struct Return {
    /// Associated with: Location, Type
    pub id: ExprId,

    pub expression: Box<Ast>,
}

/// statement1
/// statement2
/// ...
/// statementN
#[derive(Debug, Clone)]
pub struct Sequence {
    /// Associated with: Location, Type
    pub id: ExprId,

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
    /// Associated with: Location, LetBindingLevel
    pub id: ExprId,

    pub declarations: Vec<TypeAnnotation>,
}

/// lhs.field
#[derive(Debug, Clone)]
pub struct MemberAccess {
    /// Associated with: Location, Type
    pub id: ExprId,

    pub lhs: Box<Ast>,
    pub field: String,

    /// If this member access is an offset rather
    /// than a move/copy, this will contain the mutability of the offset.
    pub offset: Option<Mutability>,
}

/// lhs := rhs
#[derive(Debug, Clone)]
pub struct Assignment {
    /// Associated with: Location
    pub id: ExprId,

    pub lhs: Box<Ast>,
    pub rhs: Box<Ast>,
}

/// effect Name arg1 arg2 ... argN with
///     declaration1
///     declaration2
///     ...
///     declarationN
#[derive(Debug, Clone)]
pub struct EffectDefinition {
    /// Associated with: Location, LetBindingLevel, EffectInfoId
    pub id: ExprId,

    pub name: String,
    pub args: Vec<String>,
    pub declarations: Vec<TypeAnnotation>,
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
pub struct Handle {
    /// Associated with: Location, Type, effects_handled: Vec<Effect>, resumes: Vec<DefinitionInfoId>
    ///
    /// Each id in `resumes` is a definition id for the `resume` in each branch
    pub id: ExprId,

    pub expression: Box<Ast>,
    pub branches: Vec<(Ast, Ast)>,
}

/// MyStruct with
///     field1 = expr1
///     field2 = expr2
#[derive(Debug, Clone)]
pub struct NamedConstructor {
    /// Associated with: Location, Type
    pub id: ExprId,

    pub constructor: Box<Ast>,
    pub sequence: Box<Ast>,
}

/// &expr or !expr
#[derive(Debug, Clone)]
pub struct Reference {
    /// Associated with: Location, Type
    pub id: ExprId,

    pub mutability: Mutability,
    pub expression: Box<Ast>,
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
    TypeDefinition(TypeDefinition),
    TypeAnnotation(TypeAnnotation),
    Import(Import),
    TraitDefinition(TraitDefinition),
    TraitImpl(TraitImpl),
    Return(Return),
    Sequence(Sequence),
    Extern(Extern),
    MemberAccess(MemberAccess),
    Assignment(Assignment),
    EffectDefinition(EffectDefinition),
    Handle(Handle),
    NamedConstructor(NamedConstructor),
    Reference(Reference),
}

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
impl Ast {
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

    pub fn integer(x: u64, kind: Option<IntegerKind>, id: ExprId) -> Ast {
        Ast::Literal(Literal { kind: LiteralKind::Integer(x, kind), id })
    }

    pub fn float(x: f64, kind: Option<FloatKind>, id: ExprId) -> Ast {
        Ast::Literal(Literal { kind: LiteralKind::Float(x.to_bits(), kind), id })
    }

    pub fn string(x: String, id: ExprId) -> Ast {
        Ast::Literal(Literal { kind: LiteralKind::String(x), id })
    }

    pub fn char_literal(x: char, id: ExprId) -> Ast {
        Ast::Literal(Literal { kind: LiteralKind::Char(x), id })
    }

    pub fn bool_literal(x: bool, id: ExprId) -> Ast {
        Ast::Literal(Literal { kind: LiteralKind::Bool(x), id })
    }

    pub fn unit_literal(id: ExprId) -> Ast {
        Ast::Literal(Literal { kind: LiteralKind::Unit, id })
    }

    pub fn variable(module_prefix: Vec<String>, name: String, id: ExprId) -> Ast {
        Ast::Variable(Variable {
            kind: VariableKind::Identifier(name),
            module_prefix,
            id,
        })
    }

    pub fn operator(operator: Token, id: ExprId) -> Ast {
        Ast::Variable(Variable {
            kind: VariableKind::Operator(operator),
            module_prefix: vec![],
            id,
        })
    }

    pub fn type_constructor(module_prefix: Vec<String>, name: String, id: ExprId) -> Ast {
        Ast::Variable(Variable {
            kind: VariableKind::TypeConstructor(name),
            module_prefix,
            id,
        })
    }

    pub fn lambda(
        args: Vec<Ast>, return_type: Option<Type>, effects: Option<Vec<EffectAst>>, body: Ast,
        id: ExprId,
    ) -> Ast {
        assert!(!args.is_empty());
        Ast::Lambda(Lambda {
            args,
            effects,
            body: Box::new(body),
            return_type,
            id,
        })
    }

    pub fn function_call(function: Ast, args: Vec<Ast>, id: ExprId) -> Ast {
        assert!(!args.is_empty());
        Ast::FunctionCall(FunctionCall { function: Box::new(function), args, id })
    }

    pub fn if_expr(condition: Ast, then: Ast, otherwise: Option<Ast>, id: ExprId) -> Ast {
        if let Some(otherwise) = otherwise {
            Ast::If(If {
                condition: Box::new(condition),
                then: Box::new(then),
                otherwise: Box::new(otherwise),
                id,
            })
        } else {
            super::desugar::desugar_if_with_no_else(condition, then, id)
        }
    }

    pub fn definition(pattern: Ast, expr: Ast, id: ExprId) -> Ast {
        Ast::Definition(Definition {
            pattern: Box::new(pattern),
            expr: Box::new(expr),
            mutable: false,
            id,
        })
    }

    pub fn match_expr(expression: Ast, mut branches: Vec<(Ast, Ast)>, id: ExprId) -> Ast {
        // (Issue #80) When compiling a match statement with a single variable branch e.g:
        // `match ... | x -> ... ` a single Leaf node will be emitted as the decision tree
        // after type checking which causes us to fail since `x` will not be bound to anything
        // without a `Case` node being present. This is a hack to avoid this situation by compiling
        // this class of expressions into let bindings instead.
        if branches.len() == 1 && branches[0].0.is_matchable_variable() {
            let (pattern, rest) = branches.pop().unwrap();
            let definition = Ast::definition(pattern, expression, id);
            // TODO: turning this into a sequence can leak names in the match branch to surrounding
            // code. Soundness-wise this isn't an issue since in this case we know it will always
            // match, but it is an inconsistency that should be fixed.
            Ast::sequence(vec![definition, rest], id)
        } else {
            Ast::Match(Match { expression: Box::new(expression), branches, id })
        }
    }

    pub fn type_definition(
        boxed: bool, name: String, args: Vec<String>, definition: TypeDefinitionBody, id: ExprId,
    ) -> Ast {
        Ast::TypeDefinition(TypeDefinition { shared: boxed, name, args, definition, id })
    }

    pub fn type_annotation(lhs: Ast, rhs: Type, id: ExprId) -> Ast {
        Ast::TypeAnnotation(TypeAnnotation { lhs: Box::new(lhs), rhs, id })
    }

    pub fn import(path: Vec<String>, id: ExprId, symbols: HashSet<String>) -> Ast {
        assert!(!path.is_empty());
        Ast::Import(Import { path, id, symbols })
    }

    pub fn trait_definition(
        name: String, args: Vec<String>, fundeps: Vec<String>, declarations: Vec<TypeAnnotation>,
        id: ExprId,
    ) -> Ast {
        assert!(!args.is_empty());
        Ast::TraitDefinition(TraitDefinition {
            name,
            args,
            fundeps,
            declarations,
        })
    }

    pub fn trait_impl(
        trait_name: String, trait_args: Vec<Type>, given: Vec<Trait>, definitions: Vec<Definition>,
        id: ExprId,
    ) -> Ast {
        assert!(!trait_args.is_empty());
        Ast::TraitImpl(TraitImpl {
            trait_name,
            trait_args,
            given,
            definitions,
        })
    }

    pub fn return_expr(expression: Ast, id: ExprId) -> Ast {
        Ast::Return(Return { expression: Box::new(expression), id })
    }

    pub fn sequence(statements: Vec<Ast>, id: ExprId) -> Ast {
        assert!(!statements.is_empty());
        Ast::Sequence(Sequence { statements, id })
    }

    pub fn extern_expr(declarations: Vec<TypeAnnotation>, id: ExprId) -> Ast {
        Ast::Extern(Extern { declarations, id })
    }

    pub fn member_access(lhs: Ast, field: String, offset: Option<Mutability>, id: ExprId) -> Ast {
        Ast::MemberAccess(MemberAccess { lhs: Box::new(lhs), field, offset, id })
    }

    pub fn index(lhs: Ast, index: Ast, offset: Option<Mutability>, id: ExprId) -> Ast {
        let operator = match offset {
            Some(Mutability::Mutable) => Token::IndexMut,
            Some(Mutability::Immutable) => Token::IndexRef,
            _ => Token::Index,
        };
        let operator = Self::operator(operator, id);
        Ast::function_call(operator, vec![lhs, index], id)
    }

    pub fn assignment(lhs: Ast, rhs: Ast, id: ExprId) -> Ast {
        Ast::Assignment(Assignment { lhs: Box::new(lhs), rhs: Box::new(rhs), id })
    }

    pub fn effect_definition(
        name: String, args: Vec<String>, declarations: Vec<TypeAnnotation>, id: ExprId,
    ) -> Ast {
        Ast::EffectDefinition(EffectDefinition {
            name,
            args,
            declarations,
            id,
        })
    }

    pub fn handle(expression: Ast, branches: Vec<(Ast, Ast)>, id: ExprId) -> Ast {
        let branches = super::desugar::desugar_handle_branches_into_matches(branches);
        Ast::Handle(Handle {
            expression: Box::new(expression),
            branches,
            id,
        })
    }

    pub fn named_constructor(constructor: Ast, sequence: Ast, id: ExprId) -> Ast {
        Ast::NamedConstructor(NamedConstructor {
            constructor: Box::new(constructor),
            sequence: Box::new(sequence),
            id,
        })
    }

    /// This is a bit of a hack.
    /// Create a new 'scope' by wrapping body in `match () | () -> body`
    pub fn new_scope(body: Ast, id: ExprId) -> Ast {
        Ast::match_expr(Ast::unit_literal(id), vec![(Ast::unit_literal(id), body)], location)
    }

    pub fn reference(mutability: Token, expression: Ast, id: ExprId) -> Ast {
        let mutability = match mutability {
            Token::Ampersand => Mutability::Immutable,
            Token::ExclamationMark => Mutability::Mutable,
            other => panic!("Invalid token '{}' passed to Ast::reference", other),
        };
        Ast::Reference(Reference { mutability, expression: Box::new(expression), id })
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
            $crate::parser::ast::Ast::NamedConstructor(inner) => $function(inner $(, $($args),* )? ),
            $crate::parser::ast::Ast::Reference(inner) =>        $function(inner $(, $($args),* )? ),
        }
    });
}
