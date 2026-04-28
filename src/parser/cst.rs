use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{ErrorDefault, Location},
    lexer::token::{F64, FloatKind, IntegerKind, Token},
};

use super::ids::{ExprId, IdStore, NameId, NameStore, PathId, PatternId, TopLevelId};

/// The Concrete Syntax Tree (CST) is the output of parsing a source file.
/// This is expected to mirror the source file without removing too much information.
/// This isn't a perfect mirroring - we keep only enough information for pretty-printing
/// the CST back into a file. So while things like comments are kept, certain syntax
/// constructs like `foo = fn a -> expr` may be sugared into `foo x = expr`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Cst {
    pub imports: Vec<Import>,
    /// `None` when the file has no `export` statement (all items are exported by default).
    /// `Some(list)` when an explicit `export` statement restricts visibility to those items.
    pub exports: Option<Vec<(String, Location)>>,
    pub top_level_items: Vec<Arc<TopLevelItem>>,

    /// Comments after the last top level item
    pub ending_comments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TopLevelItem {
    pub comments: Vec<String>,
    pub kind: TopLevelItemKind,
    pub id: TopLevelId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TopLevelItemKind {
    Definition(Definition),
    TypeDefinition(TypeDefinition),
    TraitDefinition(TraitDefinition),
    TraitImpl(TraitImpl),
    EffectDefinition(EffectDefinition),
    Comptime(Comptime),
}

impl TopLevelItemKind {
    pub fn name(&self) -> ItemName {
        match self {
            TopLevelItemKind::Definition(definition) => ItemName::Pattern(definition.pattern),
            TopLevelItemKind::TypeDefinition(type_definition) => ItemName::Single(type_definition.name),
            TopLevelItemKind::TraitDefinition(trait_definition) => ItemName::Single(trait_definition.name),
            TopLevelItemKind::TraitImpl(trait_impl) => ItemName::Single(trait_impl.name),
            TopLevelItemKind::EffectDefinition(effect_definition) => ItemName::Single(effect_definition.name),
            TopLevelItemKind::Comptime(_) => ItemName::None,
        }
    }
}

#[derive(Debug)]
pub enum ItemName {
    Single(NameId),
    Pattern(PatternId),
    None,
}

impl ItemName {
    /// Give an approximate name for this item for debugging.
    pub fn to_string(&self, context: &(impl IdStore + NameStore)) -> String {
        match self {
            ItemName::Single(name) => context.get_name(*name).to_string(),
            ItemName::Pattern(pattern) => pattern.name(context),
            ItemName::None => "no-name".to_string(),
        }
    }
}

impl PatternId {
    pub fn name(self, context: &(impl IdStore + NameStore)) -> String {
        match context.get_pattern(self) {
            Pattern::Error => "#error".to_string(),
            Pattern::Variable(name) => context.get_name(*name).to_string(),
            Pattern::Literal(_) => "#literal".to_string(),
            Pattern::Constructor(..) => "#constructor".to_string(),
            Pattern::TypeAnnotation(pattern, _) => pattern.name(context),
            Pattern::MethodName { type_name, item_name } => {
                format!("{}.{}", context.get_name(*type_name), context.get_name(*item_name))
            },
        }
    }
}

/// TODO: Types should probably be interned like expressions & patterns are
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Type {
    pub kind: TypeKind,
    pub location: Location,
}

impl Type {
    pub fn new(kind: TypeKind, location: Location) -> Type {
        Type { kind, location }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum TypeKind {
    Error,
    Unit,
    Char,
    Named(PathId),
    Variable(NameId),
    Integer(IntegerKind),
    Float(FloatKind),
    Function(FunctionType),
    Application(Box<Type>, Vec<Type>),
    Reference(ReferenceKind),

    /// This is an internal type only created when desugaring closure environments in trait impls.
    /// Most tuple types in source code refer to the `,` type defined in the prelude. While they
    /// could use this type instead, using a UserDefinedType for them lets us reuse the existing
    /// mechanisms to automatically define their constructor and retrieve their fields.
    Tuple(Vec<Type>),

    /// This type can't be parsed, it is only used by `GetItem` to desugar
    /// trait types into in some cases.
    NoClosureEnv,

    /// A filler type which corresponds to an unbound type variable to be inferred later
    Hole,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, PartialOrd, Ord)]
pub enum ReferenceKind {
    Ref,
    Mut,
    Imm,
    Uniq,
}

impl ReferenceKind {
    /// Convert the given token into a reference kind, panics if
    /// the token is not a reference keyword.
    pub(crate) fn from_token(operator: &Token) -> ReferenceKind {
        match operator {
            Token::Ref => Self::Ref,
            Token::Mut => Self::Mut,
            Token::Imm => Self::Imm,
            Token::Uniq => Self::Uniq,
            other => panic!("Non-reference token given: {other}"),
        }
    }
}

impl ErrorDefault for Type {
    fn error_default(location: Location) -> Self {
        Type::new(TypeKind::Error, location)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct FunctionType {
    pub parameters: Vec<ParameterType>,
    pub environment: Option<Box<Type>>,
    pub return_type: Box<Type>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct ParameterType {
    pub is_implicit: bool,
    pub typ: Type,
}

impl ParameterType {
    pub fn new(typ: Type, is_implicit: bool) -> ParameterType {
        ParameterType { typ, is_implicit }
    }

    pub fn explicit(typ: Type) -> ParameterType {
        Self::new(typ, false)
    }

    pub fn implicit(typ: Type) -> ParameterType {
        Self::new(typ, true)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TypeDefinition {
    pub shared: bool,
    /// TraitDefinitions are desugared into type definitions, but for
    /// some checks, we still need to know the origin. Namely, only
    /// traits allow rank-1 fields, and only traits expose their fields
    /// under the type's namespace.
    pub is_trait: bool,
    pub name: NameId,
    pub generics: Generics,
    pub body: TypeDefinitionBody,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TypeDefinitionBody {
    Error,
    Struct(Vec<(NameId, Type)>),
    Enum(Vec<(NameId, Vec<Type>)>),
    Alias(Type),
}

impl ErrorDefault for TypeDefinitionBody {
    fn error_default(_: Location) -> Self {
        Self::Error
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Expr {
    Error,
    Literal(Literal),
    Variable(PathId),
    Sequence(Vec<SequenceItem>),
    Definition(Definition),
    MemberAccess(MemberAccess),
    Call(Call),
    Lambda(Lambda),
    If(If),
    Match(Match),
    Is(Is),
    Bind(Bind),
    Handle(Handle),
    Reference(Reference),
    TypeAnnotation(TypeAnnotation),
    Constructor(Constructor),
    Loop(Loop),
    While(While),
    For(For),
    Break,
    Continue,
    Quoted(Quoted),
    Return(Return),
    Assignment(Assignment),
    Extern(Extern),
    InterpolatedString(InterpolatedString),
}

impl ErrorDefault for Expr {
    fn error_default(_: Location) -> Self {
        Self::Error
    }
}

impl Expr {
    /// Are parenthesis not required when printing this Expr within another?
    pub fn is_atom(&self) -> bool {
        matches!(self, Expr::Error | Expr::Literal(_) | Expr::Variable(_) | Expr::MemberAccess(_))
    }
}

/// Path Can't contain any ExprIds since it is used for hashing top-level definition names
///
/// A path is always guaranteed to have at least 1 component
#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Path {
    pub components: Vec<(String, Location)>,
}

impl Path {
    pub fn ident(name: String, location: Location) -> Path {
        Path { components: vec![(name, location)] }
    }

    pub fn into_file_path(self) -> Arc<PathBuf> {
        let mut path = PathBuf::new();
        for (component, _) in self.components {
            path.push(component);
        }
        Arc::new(path)
    }

    /// Retrieve the last identifier of this path.
    ///
    /// Paths are guaranteed to have at least 1 component, so this will never panic.
    pub fn last_ident(&self) -> &str {
        &self.components.last().unwrap().0
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Import {
    pub comments: Vec<String>,

    /// For a given import `import Foo.Bar.Baz.a, b, c`, `crate_name` will contain `Foo`
    pub crate_name: String,

    /// For a given import `import Foo.Bar.Baz.a, b, c`, `module_path` will contain `Bar/Baz.an`
    /// TODO: Investigate whether this breaks serialization stability across Windows <-> Unix
    pub module_path: Arc<PathBuf>,

    /// For a given import `import Foo.Bar.Baz.a, b, c`, `items` will contain `a, b, c`
    pub items: Vec<(String, Location)>,
    pub location: Location,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct SequenceItem {
    pub comments: Vec<String>,
    pub expr: ExprId,
}

/// An interpolated string literal like `"foo ${bar} baz"`.
/// `fragments` and `exprs` are interspersed such that:
/// - The string always starts and ends with a (possibly empty) fragment
/// - Each expression is between two fragments
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct InterpolatedString {
    pub fragments: Vec<String>,
    pub exprs: Vec<ExprId>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Literal {
    Unit,
    Bool(bool),
    Integer(u64, Option<IntegerKind>),
    Float(F64, Option<FloatKind>),
    String(String),
    Char(char),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Definition {
    pub implicit: bool,
    pub mutable: bool,
    pub pattern: PatternId,
    pub rhs: ExprId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Call {
    pub function: ExprId,
    pub arguments: Vec<Argument>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone)]
pub struct Argument {
    pub is_implicit: bool,
    pub expr: ExprId,
}

impl Argument {
    pub fn explicit(expr: ExprId) -> Self {
        Self { expr, is_implicit: false }
    }

    pub fn implicit(expr: ExprId) -> Self {
        Self { expr, is_implicit: true }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct MemberAccess {
    pub object: ExprId,
    pub member: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Lambda {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: ExprId,
    pub is_move: bool,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Parameter {
    pub is_implicit: bool,
    pub is_mutable: bool,
    pub pattern: PatternId,
}

impl Parameter {
    /// Creates a new, non-implicit, immutable parameter
    pub fn new(pattern: PatternId) -> Parameter {
        Parameter { pattern, is_implicit: false, is_mutable: false }
    }

    /// Creates a new, implicit, immutable parameter
    pub fn implicit(pattern: PatternId) -> Parameter {
        Parameter { pattern, is_implicit: true, is_mutable: false }
    }

    /// Creates a new, immutable parameter with the given `is_implicit` value
    pub fn with_implicit(pattern: PatternId, is_implicit: bool) -> Parameter {
        Parameter { pattern, is_implicit, is_mutable: false }
    }

    /// Creates a new, non-implicit, mutable parameter
    pub fn mutable(pattern: PatternId) -> Parameter {
        Parameter { pattern, is_implicit: false, is_mutable: true }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct If {
    pub condition: ExprId,
    pub then: ExprId,
    pub else_: Option<ExprId>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Match {
    /// The expression being matched
    pub expression: ExprId,
    pub cases: Vec<(PatternId, ExprId)>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Handle {
    /// The effectful expression being handled
    pub expression: ExprId,
    pub cases: Vec<(HandlePattern, ExprId)>,
}

/// `lhs is pattern` - always desugared during `GetItem`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Is {
    pub lhs: ExprId,
    pub pattern: PatternId,
}

/// `<pattern> <- <rhs>` followed by newline-separated statements.
/// Always desugared during `GetItem`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Bind {
    pub pattern: PatternId,
    pub rhs: ExprId,
    pub body: ExprId,
}

/// `&rhs`, `!rhs`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Reference {
    pub kind: ReferenceKind,
    pub rhs: ExprId,
}

/// A constructor with named fields such as `Foo with bar = 1, baz = 2`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Constructor {
    pub typ: Type,
    pub fields: Vec<(NameId, ExprId)>,
}

/// `return expr`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Return {
    pub expression: ExprId,
}

/// The binary operator in a compound assignment (e.g. `+` in `+=`).
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone)]
pub enum CompoundAssignOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

/// `lhs := rhs` or `lhs += rhs` etc.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Assignment {
    pub lhs: ExprId,
    pub rhs: ExprId,
    /// For compound assignments (+=, -=, etc.): the operator kind and a synthetic
    /// Variable expression for the operator function, resolved via normal trait dispatch.
    pub op: Option<(CompoundAssignOp, ExprId)>,
}

/// `while cond do body` — imperative loop, body is Unit, whole expression is Unit.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct While {
    pub condition: ExprId,
    pub body: ExprId,
}

/// `for variable in start .. end do body` — end-exclusive numeric range loop.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct For {
    pub variable: NameId,
    pub start: ExprId,
    pub end: ExprId,
    pub body: ExprId,
}

/// Sugar for an immediately invoked helper function: `loop x (i = 0) -> ...`
/// The `recur` identifier is defined within bound to the name of the new helper.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Loop {
    pub parameters: Vec<LoopParameter>,
    pub body: ExprId,
}

/// A `loop` parameter is either an existing variable in scope (e.g. `x`)
/// or a pattern, expression pair where the pattern is the loop helper function
/// parameter and the expression is its initial value - e.g `(y = 3)`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum LoopParameter {
    Variable(NameId),
    PatternAndExpr(PatternId, ExprId),
    UnitLiteral(Location),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Pattern {
    Error,
    Variable(NameId),
    Literal(Literal),
    Constructor(PathId, Vec<PatternId>),
    TypeAnnotation(PatternId, Type),
    MethodName { type_name: NameId, item_name: NameId },
}

impl ErrorDefault for Pattern {
    fn error_default(_: Location) -> Self {
        Self::Error
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct HandlePattern {
    pub function: PathId,
    pub args: Vec<PatternId>,

    /// Synthetic `resume` binding for this branch
    pub resume_name: NameId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct TypeAnnotation {
    pub lhs: ExprId,
    pub rhs: Type,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Quoted {
    pub tokens: Vec<Token>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Declaration {
    pub name: NameId,
    pub typ: Type,
}

pub type Generics = Vec<NameId>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraitDefinition {
    pub name: NameId,
    pub generics: Generics,
    pub functional_dependencies: Generics,
    pub body: Vec<Declaration>,
}

pub type Name = Arc<String>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraitImpl {
    pub name: NameId,
    pub parameters: Vec<Parameter>,
    pub trait_path: PathId,
    pub trait_arguments: Vec<Type>,
    pub body: Vec<(NameId, ExprId)>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EffectDefinition {
    pub name: NameId,
    pub generics: Generics,
    pub body: Vec<Declaration>,
}

/// An extern has a name and a type determined by the expected type
/// when it is used in an expression. Most often this is bounded
/// by a type annotation on the extern expression itself.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Extern {
    pub name: String,
}

/// A top-level item evaluated at compile-time, e.g:
/// ```ante
/// #if foo then
///     function () = 3
///
/// // or
/// #modify
/// foo bar = ()
///
/// // or
/// derive Foo Bar
/// type MyType = x: I32
/// ```
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Comptime {
    Expr(ExprId),
    Derive(Vec<PathId>),
    Definition(Definition),
}
