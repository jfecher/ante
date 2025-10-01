use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{ErrorDefault, Location},
    lexer::token::{FloatKind, IntegerKind, Token, F64},
};

use super::ids::{ExprId, NameId, PathId, PatternId, TopLevelId};

/// The Concrete Syntax Tree (CST) is the output of parsing a source file.
/// This is expected to mirror the source file without removing too much information.
/// This isn't a perfect mirroring - we keep only enough information for pretty-printing
/// the CST back into a file. So while things like comments are kept, certain syntax
/// constructs like `foo = fn a -> expr` may be sugared into `foo x = expr`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Cst {
    pub imports: Vec<Import>,
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
    Extern(Extern),
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
            TopLevelItemKind::Extern(extern_) => ItemName::Single(extern_.declaration.name),
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Type {
    Error,
    Unit,
    Char,
    String,
    Named(PathId),
    Variable(NameId),
    Integer(IntegerKind),
    Float(FloatKind),
    Function(FunctionType),
    Application(Box<Type>, Vec<Type>),
    Reference(Mutability, Sharedness),
}

impl ErrorDefault for Type {
    fn error_default() -> Self {
        Self::Error
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,

    /// Any effects that were specified on this function.
    /// - `None` means none were specified
    /// - `Some(Vec::new())` means it was specified to be `pure`
    pub effects: Option<Vec<EffectType>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EffectType {
    Known(PathId, Vec<Type>),
    Variable(NameId),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TypeDefinition {
    pub shared: bool,
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
    fn error_default() -> Self {
        Self::Error
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Expr {
    Error,
    Literal(Literal),
    Variable(PathId),
    Sequence(Vec<SequenceItem>),
    Definition(Definition),
    MemberAccess(MemberAccess),
    Index(Index),
    Call(Call),
    Lambda(Lambda),
    If(If),
    Match(Match),
    Handle(Handle),
    Reference(Reference),
    TypeAnnotation(TypeAnnotation),
    Quoted(Quoted),
}

impl ErrorDefault for Expr {
    fn error_default() -> Self {
        Self::Error
    }
}

impl Expr {
    /// Are parenthesis not required when printing this Expr within another?
    pub fn is_atom(&self) -> bool {
        matches!(
            self,
            Expr::Error
                | Expr::Literal(_)
                | Expr::Variable(_)
                | Expr::MemberAccess(_)
                | Expr::Index(_)
                | Expr::Reference(_)
        )
    }
}

/// Path Can't contain any ExprIds since it is used for hashing top-level definition names
///
/// A path is always guaranteed to have at least 1 component
#[derive(Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path {
    pub components: Vec<(String, Location)>,
}

impl Path {
    pub fn into_file_path(self) -> Arc<PathBuf> {
        let mut path = PathBuf::new();
        for (component, _) in self.components {
            path.push(component);
        }
        Arc::new(path)
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SequenceItem {
    pub comments: Vec<String>,
    pub expr: ExprId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Literal {
    Unit,
    Bool(bool),
    Integer(u64, Option<IntegerKind>),
    Float(F64, Option<FloatKind>),
    String(String),
    Char(char),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Definition {
    pub mutable: bool,
    pub pattern: PatternId,
    pub rhs: ExprId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Call {
    pub function: ExprId,
    pub arguments: Vec<ExprId>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MemberAccess {
    pub object: ExprId,
    pub member: String,
    pub ownership: OwnershipMode,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Index {
    pub object: ExprId,
    pub index: ExprId,
    pub ownership: OwnershipMode,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OwnershipMode {
    Owned,
    Borrow,
    BorrowMut,
}

impl OwnershipMode {
    pub fn from_token(token: &Token) -> Option<Self> {
        match token {
            Token::MemberAccess | Token::Index => Some(Self::Owned),
            Token::MemberRef | Token::IndexRef => Some(Self::Borrow),
            Token::MemberMut | Token::IndexMut => Some(Self::BorrowMut),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Lambda {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub effects: Option<Vec<EffectType>>,
    pub body: ExprId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct If {
    pub condition: ExprId,
    pub then: ExprId,
    pub else_: Option<ExprId>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Match {
    /// The expression being matched
    pub expression: ExprId,
    pub cases: Vec<(PatternId, ExprId)>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Handle {
    /// The effectful expression being handled
    pub expression: ExprId,
    pub cases: Vec<(HandlePattern, ExprId)>,
}

/// `&rhs`, `!rhs`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Reference {
    pub mutability: Mutability,
    pub sharedness: Sharedness,
    pub rhs: ExprId,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mutability {
    Immutable,
    Mutable,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sharedness {
    Shared,
    Owned,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Parameter {
    pub implicit: bool,
    pub pattern: PatternId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Pattern {
    Error,
    Variable(NameId),
    Literal(Literal),
    Constructor(PathId, Vec<PatternId>),
    TypeAnnotation(PatternId, Type),
    MethodName { type_name: NameId, item_name: NameId },
}

impl ErrorDefault for Pattern {
    fn error_default() -> Self {
        Self::Error
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct HandlePattern {
    pub function: NameId,
    pub args: Vec<PatternId>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TypeAnnotation {
    pub lhs: ExprId,
    pub rhs: Type,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Quoted {
    pub tokens: Vec<Token>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    pub body: Vec<Definition>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EffectDefinition {
    pub name: NameId,
    pub generics: Generics,
    pub body: Vec<Declaration>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Extern {
    pub declaration: Declaration,
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
