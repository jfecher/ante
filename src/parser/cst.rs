use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    diagnostics::{ErrorDefault, Location},
    lexer::token::{F64, FloatKind, Integer, IntegerKind, Token},
    shared_arc::Shared,
};

use super::ids::{ExprId, IdStore, NameId, PathId, PatternId, TopLevelId, TopLevelName};

/// The Concrete Syntax Tree (CST) is the output of parsing a source file.
/// This is expected to mirror the source file without removing too much information.
/// This isn't a perfect mirroring - we keep only enough information for pretty-printing
/// the CST back into a file. So while things like comments are kept, certain syntax
/// constructs like `foo = fn a -> expr` may be sugared into `foo a = expr`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Cst {
    pub imports: Vec<Import>,
    /// `None` when the file has no `export` statement (all items are exported by default).
    /// `Some(list)` when an explicit `export` statement restricts visibility to those items.
    pub exports: Option<Vec<ExportEntry>>,
    pub top_level_items: Vec<Shared<TopLevelItem>>,

    /// Comments after the last top level item
    pub ending_comments: Vec<String>,
}

/// One entry in an export list
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ExportEntry {
    /// The `Vec` in `Vec.push`. `None` for unqualified identifiers.
    pub qualifier: Option<(Name, Location)>,
    pub name: Name,
    pub location: Location,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TopLevelItem {
    pub attributes: Vec<Attribute>,
    pub comments: Vec<String>,
    pub kind: TopLevelItemKind,
    pub id: TopLevelId,
}

/// A `#name arg0 arg1 .. argN` annotation on a top-level item
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Attribute {
    pub name: Name,
    pub args: Vec<ExprId>,
    pub location: Location,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TopLevelItemKind {
    Definition(Definition),
    TypeDefinition(TypeDefinition),
    TraitDefinition(TraitOrEffectDefinition),
    EffectDefinition(TraitOrEffectDefinition),
    TraitImpl(TraitImpl),
    Comptime(Comptime),
}

impl TopLevelItemKind {
    /// True if this item is an `impl` or an `implicit`
    pub fn is_implicit(&self) -> bool {
        match self {
            TopLevelItemKind::TraitImpl(_) => true,
            TopLevelItemKind::Definition(definition) => definition.implicit,
            _ => false,
        }
    }

    pub fn name(&self) -> ItemName {
        match self {
            TopLevelItemKind::Definition(definition) => ItemName::Pattern(definition.pattern),
            TopLevelItemKind::TypeDefinition(type_definition) => ItemName::Single(type_definition.name),
            TopLevelItemKind::TraitDefinition(def) | TopLevelItemKind::EffectDefinition(def) => {
                ItemName::Single(def.name)
            },
            TopLevelItemKind::TraitImpl(trait_impl) => ItemName::Single(trait_impl.name),
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
    pub fn to_string(&self, context: &impl IdStore) -> String {
        match self {
            ItemName::Single(name) => context.get_name(*name).to_string(),
            ItemName::Pattern(pattern) => pattern.name(context),
            ItemName::None => "no-name".to_string(),
        }
    }
}

impl PatternId {
    pub fn name(self, context: &impl IdStore) -> String {
        match context.get_pattern(self) {
            Pattern::Error => "#error".to_string(),
            Pattern::Variable(name) => context.get_name(*name).to_string(),
            Pattern::Literal(_) => "#literal".to_string(),
            Pattern::Constructor(..) => "#constructor".to_string(),
            Pattern::ConstructorRest(_, _, Some(name)) => context.get_name(*name).to_string(),
            Pattern::ConstructorRest(_, _, None) => "#constructor".to_string(),
            Pattern::TypeAnnotation(pattern, _) => pattern.name(context),
            Pattern::Or(alts) => match alts.first() {
                Some(alt) => alt.name(context),
                None => "#or".to_string(),
            },
            Pattern::Alias(name, _) => context.get_name(*name).to_string(),
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

    /// Does this type contain at least one `ImplicitPlace` outside of function types?
    ///
    /// TODO: Kinds for type variables aren't tracked yet so only places on reference types are
    /// checked at the moment.
    pub fn has_elided_place(&self) -> bool {
        match &self.kind {
            TypeKind::Application(f, args) => {
                let is_elided_ref_place = matches!(f.kind, TypeKind::Reference(_))
                    && args.first().is_some_and(|arg| matches!(arg.kind, TypeKind::ImplicitPlace { .. }));
                is_elided_ref_place || args.iter().any(Type::has_elided_place)
            },
            TypeKind::Tuple(elements) => elements.iter().any(Type::has_elided_place),
            _ => false,
        }
    }

    /// True if this is a function type without a `can` clause
    pub fn is_function_without_effects(&self) -> bool {
        matches!(&self.kind, TypeKind::Function(function) if function.effects.is_none())
    }

    /// True if this type has a hole to infer, such as the environment of a `=>` function type
    pub fn contains_hole(&self) -> bool {
        self.contains_hole_with(&mut |_, _| false)
    }

    /// Like [Type::contains_hole], but also calls `on_named(path, arg_count)` on each named type
    pub fn contains_hole_with(&self, on_named: &mut impl FnMut(PathId, usize) -> bool) -> bool {
        self.contains_hole_where(true, on_named)
    }

    /// True if this type has a `_` hole, ignoring any [TypeKind::RigidHole]
    pub fn contains_inferred_hole(&self) -> bool {
        self.contains_hole_where(false, &mut |_, _| false)
    }

    fn contains_hole_where(&self, count_rigid: bool, on_named: &mut impl FnMut(PathId, usize) -> bool) -> bool {
        let mut recur = |typ: &Type| typ.contains_hole_where(count_rigid, on_named);
        match &self.kind {
            TypeKind::Hole => true,
            TypeKind::RigidHole => count_rigid,
            TypeKind::Named(path) => on_named(*path, 0),
            TypeKind::Function(function) => function.types().any(recur),
            TypeKind::Application(function, args) => {
                args.iter().any(&mut recur)
                    || match &function.kind {
                        TypeKind::Named(path) => on_named(*path, args.len()),
                        _ => recur(function),
                    }
            },
            TypeKind::Tuple(types) | TypeKind::EffectUnion(types) => types.iter().any(recur),
            TypeKind::Forall(_, typ) => recur(typ),
            _ => false,
        }
    }

    /// The path naming the type constructor at the head of this type, skipping any type arguments
    /// so that `Vec a` returns the path of `Vec`
    pub fn head_path(&self) -> Option<PathId> {
        match &self.kind {
            TypeKind::Named(path) => Some(*path),
            TypeKind::Application(constructor, _) => constructor.head_path(),
            _ => None,
        }
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

    /// A type-level integer literal, used in type positions to supply arguments to
    /// kind-`U32` parameters. Example: the `4` in `Array 4 I32`.
    IntegerConstant(u32),

    /// This is an internal type only created when desugaring closure environments in ability impls.
    /// Most tuple types in source code refer to the `,` type defined in the prelude. While they
    /// could use this type instead, using a UserDefinedType for them lets us reuse the existing
    /// mechanisms to automatically define their constructor and retrieve their fields.
    Tuple(Vec<Type>),

    /// The `&` combinator to combine two effects, e.g. `Fs & Net`
    EffectUnion(Vec<Type>),

    /// This type can't be parsed, it is only used by `GetItem` to desugar
    /// ability types into in some cases.
    NoClosureEnv,

    /// A filler type which corresponds to an unbound type variable to be inferred later
    Hole,

    /// Like a [TypeKind::Hole] but is replaced by a fresh, rigid generic rather than an inferred
    /// type variable. If a top-level item's type is not fully specified, this will still be inferred.
    RigidHole,

    /// When `ref t` or similar are used the parser fills in `ImplicitPlace`s for the missing
    /// places. If `unstable` is true, this is `ref! t` or similar.
    ImplicitPlace {
        unstable: bool,
    },

    /// A place written with `'`, e.g. `'a`, `'p.f!` or `'(a, b!)`
    Place(Vec<PlacePath>),

    /// A place with a mutability modifier, e.g. the `mut 'a` in `Slice (mut 'a) t`
    PlaceMode(ReferenceKind, Box<Type>),

    /// An explicit `forall (n: U32) t. T` polytype
    Forall(Generics, Box<Type>),

    /// An empty effect set
    Pure,
}

/// A single place in a type, e.g. `'a.b!`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct PlacePath {
    pub root: NameId,
    pub segments: Vec<PlaceSegment>,
}

/// One step along a place's path after its root
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, PartialOrd, Ord)]
pub enum PlaceSegment {
    /// `.foo`
    Field(Name),
    /// `!`, this place may be invalidated by a mutation to any ancestor place.
    Unstable,
}

impl std::fmt::Display for PlaceSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PlaceSegment::Field(name) => write!(f, ".{name}"),
            PlaceSegment::Unstable => write!(f, "!"),
        }
    }
}

/// Displays a place's segments in source form, like `.a!.b`
pub struct Segments<'a>(pub &'a [PlaceSegment]);

impl std::fmt::Display for Segments<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.iter().try_for_each(|segment| write!(f, "{segment}"))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Copy, Clone, PartialOrd, Ord)]
pub enum ReferenceKind {
    Ref,
    Mut,
    Read,
}

impl ReferenceKind {
    /// Convert the given token into a reference kind, panics if
    /// the token is not a reference keyword.
    pub(crate) fn from_token(operator: &Token) -> ReferenceKind {
        match operator {
            Token::Ref => Self::Ref,
            Token::Mut => Self::Mut,
            Token::Read => Self::Read,
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
    /// True for a `once fn`, which may be called at most once
    pub once: bool,
    pub parameters: Vec<ParameterType>,
    pub environment: Option<Box<Type>>,
    pub return_type: Box<Type>,

    /// `None` for no clause, `Some(vec![])` for `is pure`, `Some(effects)` for `can e1, e2, ...`.
    pub effects: Option<Vec<Type>>,
}

impl FunctionType {
    /// Each type directly within this function type
    pub fn types(&self) -> impl Iterator<Item = &Type> {
        let parameters = self.parameters.iter().map(|parameter| &parameter.typ);
        parameters
            .chain(self.environment.as_deref())
            .chain(std::iter::once(&*self.return_type))
            .chain(self.effects.iter().flatten())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct ParameterType {
    pub is_implicit: bool,
    pub typ: Type,

    /// The parameter's name, if this is the signature of a definition
    pub name: Option<NameId>,
}

impl ParameterType {
    pub fn new(typ: Type, is_implicit: bool) -> ParameterType {
        ParameterType { typ, is_implicit, name: None }
    }

    pub fn explicit(typ: Type) -> ParameterType {
        Self::new(typ, false)
    }

    pub fn implicit(typ: Type) -> ParameterType {
        Self::new(typ, true)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TypeDefinitionKind {
    Type,
    Trait,
    /// Effect operations get `Kind::Effect` and are type-checked as top-level `can`-annotated functions.
    Effect,
}

impl TypeDefinitionKind {
    /// True for traits and effects
    pub fn is_ability(self) -> bool {
        matches!(self, TypeDefinitionKind::Trait | TypeDefinitionKind::Effect)
    }

    pub fn is_effect(self) -> bool {
        matches!(self, TypeDefinitionKind::Effect)
    }

    pub fn is_trait(self) -> bool {
        matches!(self, TypeDefinitionKind::Trait)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TypeDefinition {
    pub shared: bool,
    pub mutable: bool,
    pub kind: TypeDefinitionKind,
    pub name: NameId,
    pub generics: Generics,
    pub body: TypeDefinitionBody,
}

impl TypeDefinition {
    /// Number of generic parameters without a default
    pub fn required_generic_count(&self) -> usize {
        self.generics.iter().rposition(|param| param.default.is_none()).map_or(0, |index| index + 1)
    }

    /// If this is a trait, return the index of its impl generic.
    pub fn impl_generic_index(&self) -> Option<usize> {
        self.kind.is_trait().then(|| self.generics.len() - 1)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TypeDefinitionBody {
    Error,
    Struct(Vec<(NameId, Type)>),
    Enum(/*variants*/ Vec<(NameId, Vec<(Option<NameId>, Type)>)>, /*with-clause*/ Vec<(NameId, Type)>),
    Alias(Type),
    EffectAlias(Vec<Type>),
}

impl ErrorDefault for TypeDefinitionBody {
    fn error_default(_: Location) -> Self {
        Self::Error
    }
}

impl TypeDefinitionBody {
    /// If this is an enum body, returns the variant named `name` along with its index
    pub fn find_variant(&self, name: NameId) -> Option<(usize, &(NameId, Vec<(Option<NameId>, Type)>))> {
        match self {
            TypeDefinitionBody::Enum(variants, _) => variants.iter().enumerate().find(|(_, (n, _))| *n == name),
            _ => None,
        }
    }

    /// The `with`-clause common fields of an enum body, empty if this is not an enum
    pub fn common_fields(&self) -> &[(NameId, Type)] {
        match self {
            TypeDefinitionBody::Enum(_, common_fields) => common_fields,
            _ => &[],
        }
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
    Do(Do),
    Handle(Handle),
    Reference(Reference),
    TypeAnnotation(TypeAnnotation),
    Constructor(Constructor),
    Loop(Loop),
    While(While),
    For(For),
    /// Always desugared during `GetItem` to `Std.Stream.(break) ()`
    Break,
    /// Always desugared during `GetItem` to `Std.Stream.(continue) ()`
    Continue,
    Chain(Chain),
    Fallthrough,
    Quoted(Quoted),
    Return(Return),
    Assignment(Assignment),
    Extern(Extern),
    InterpolatedString(InterpolatedString),

    /// `[e0, e1, ..., eN-1]`. Constructs a value of type `Array N t` where `t` is the
    /// unified element type and `N` is the literal's length.
    ArrayLiteral(Vec<ExprId>),
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
    pub components: Vec<(Name, Location)>,
}

impl Path {
    pub fn ident(name: Name, location: Location) -> Path {
        Path { components: vec![(name, location)] }
    }

    pub fn into_file_path(self) -> Arc<PathBuf> {
        let mut path = PathBuf::new();
        for (component, _) in self.components {
            path.push(component.as_str());
        }
        Arc::new(path)
    }

    /// Retrieve the last identifier of this path.
    ///
    /// Paths are guaranteed to have at least 1 component, so this will never panic.
    pub fn last_ident(&self) -> &str {
        self.last_name()
    }

    pub fn last_name(&self) -> &Name {
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

    /// For a given import `import Foo.Bar.Baz.a, b, c`, `items` will contain `a, b, c`.
    pub items: Vec<(ImportItem, Location)>,
    pub location: Location,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ImportItem {
    /// An item imported by name
    Name(Name),
    // The rest are the keywords of the same name:
    Type,
    Module,
    Impl,
}

impl std::fmt::Display for ImportItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportItem::Name(name) => write!(f, "{name}"),
            ImportItem::Type => write!(f, "type"),
            ImportItem::Module => write!(f, "module"),
            ImportItem::Impl => write!(f, "impl"),
        }
    }
}

impl Import {
    /// Each item of this import which was imported by name, excluding `type`, `module`, and `impl`.
    pub fn names(&self) -> impl Iterator<Item = (&Name, &Location)> {
        self.items.iter().filter_map(|(item, location)| match item {
            ImportItem::Name(name) => Some((name, location)),
            ImportItem::Type | ImportItem::Module | ImportItem::Impl => None,
        })
    }

    /// True if `name` is one of the items imported by name
    pub fn imports_name(&self, name: &str) -> bool {
        self.names().any(|(item, _)| item.as_str() == name)
    }

    /// The location of the `type` item in this import, if there is one
    pub fn type_item(&self) -> Option<&Location> {
        self.keyword_item(&ImportItem::Type)
    }

    /// The location of the `module` item in this import, if there is one
    pub fn module_item(&self) -> Option<&Location> {
        self.keyword_item(&ImportItem::Module)
    }

    /// The location of the `impl` item in this import, if there is one
    pub fn impl_item(&self) -> Option<&Location> {
        self.keyword_item(&ImportItem::Impl)
    }

    fn keyword_item(&self, keyword: &ImportItem) -> Option<&Location> {
        self.items.iter().find(|(item, _)| item == keyword).map(|(_, location)| location)
    }

    /// The name of the type imported by this import's `type` item along with the location of
    /// that item, if there is one.
    pub fn imported_type(&self) -> Option<(&str, &Location)> {
        Some((self.module_name()?, self.type_item()?))
    }

    /// The name of the module this import imports from, e.g. `Vec` in
    /// `import Std.Collections.Vec.module`.
    pub fn module_name(&self) -> Option<&str> {
        self.module_path.file_name()?.to_str()
    }
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
    Integer(Integer, Option<IntegerKind>),
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

    /// Set on the top-level definition an `impl` desugars to
    pub impl_info: Option<Box<ImplInfo>>,
}

impl Definition {
    pub fn new(implicit: bool, mutable: bool, pattern: PatternId, rhs: ExprId) -> Self {
        Self { implicit, mutable, pattern, rhs, impl_info: None }
    }

    /// Each of the parameters of the `impl` this definition was desugared from. If there are any,
    /// the definition's body is a lambda taking them, otherwise the body is [ImplInfo::constructor] itself.
    pub fn impl_parameters<'a>(&self, context: &'a impl IdStore) -> &'a [Parameter] {
        match context.get_expr(self.rhs) {
            Expr::Lambda(lambda) if self.impl_info.is_some() => &lambda.parameters,
            _ => &[],
        }
    }
}

/// The parts of an `impl` which its desugared definition doesn't otherwise make explicit.
/// See [crate::parser::get_item] for how an impl is desugared.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct ImplInfo {
    /// The impl's name, which is also the name of its definition
    pub name: TopLevelName,

    /// The constructor creating the impl's value, the body of the definition or its lambda
    pub constructor: ExprId,
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
    pub member: NameId,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Lambda {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: ExprId,
    pub is_move: bool,

    pub kind: LambdaKind,

    /// `None` for no clause, `Some(vec![])` for `is pure`, `Some(effects)` for `can e1, e2, ...`.
    pub effects: Option<Vec<Type>>,
}

impl Expr {
    /// Returns every expression directly nested within this one. Expects loops, `is`, `do`, and
    /// interpolated strings to already be desugared.
    pub fn children(&self) -> Vec<ExprId> {
        match self {
            Expr::Sequence(items) => items.iter().map(|item| item.expr).collect(),
            Expr::Definition(definition) => vec![definition.rhs],
            Expr::MemberAccess(access) => vec![access.object],
            Expr::Call(call) => {
                std::iter::once(call.function).chain(call.arguments.iter().map(|argument| argument.expr)).collect()
            },
            Expr::Lambda(lambda) => vec![lambda.body],
            Expr::If(if_) => [if_.condition, if_.then].into_iter().chain(if_.else_).collect(),
            Expr::Match(match_) => {
                std::iter::once(match_.expression).chain(match_.cases.iter().map(|(_, branch)| *branch)).collect()
            },
            Expr::Handle(handle) => {
                std::iter::once(handle.expression).chain(handle.cases.iter().map(|(_, branch)| *branch)).collect()
            },
            Expr::Chain(chain) => vec![chain.body, chain.else_],
            Expr::Reference(reference) => vec![reference.rhs],
            Expr::TypeAnnotation(annotation) => vec![annotation.lhs],
            Expr::Constructor(constructor) => constructor.fields.iter().map(|(_, field)| *field).collect(),
            Expr::Return(ret) => vec![ret.expression],
            Expr::Assignment(assignment) => {
                [assignment.lhs, assignment.rhs].into_iter().chain(assignment.op.map(|(_, op_expr)| op_expr)).collect()
            },
            Expr::ArrayLiteral(elements) => elements.clone(),
            Expr::Literal(_)
            | Expr::Variable(_)
            | Expr::Fallthrough
            | Expr::Break
            | Expr::Continue
            | Expr::Error
            | Expr::Extern(_)
            | Expr::Quoted(_) => Vec::new(),
            Expr::Is(_) | Expr::Do(_) | Expr::Loop(_) | Expr::While(_) | Expr::For(_) | Expr::InterpolatedString(_) => {
                unreachable!("Expected this expression to be desugared already")
            },
        }
    }
}

impl Lambda {
    /// Creates a plain `fn parameters -> body` lambda
    pub fn new(parameters: Vec<Parameter>, body: ExprId) -> Lambda {
        Lambda { parameters, return_type: None, body, is_move: false, kind: LambdaKind::Function, effects: None }
    }

    /// True if this is the condition or body of a desugared loop
    pub fn is_loop_part(&self) -> bool {
        matches!(self.kind, LambdaKind::ForBody | LambdaKind::WhileCondition | LambdaKind::WhileBody)
    }

    /// True if a `return` written in this lambda should return from an enclosing function instead
    /// of the lambda itself. This is done by performing an effect, see `desugar_loop_call`.
    pub fn returns_from_enclosing_function(&self) -> bool {
        self.kind != LambdaKind::Function
    }
}

/// Describes where a lambda came from
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum LambdaKind {
    /// A normal lambda, i.e. anything not listed below
    Function,
    /// The body of a desugared `for` loop. Like the condition and body of a `while` loop below,
    /// these run in place once per iteration. So they never own their captures and are borrow
    /// checked as loop bodies.
    ForBody,
    /// The condition of a desugared `while` loop
    WhileCondition,
    /// The body of a desugared `while` loop
    WhileBody,
    /// The body of the `handle` which wraps a desugared loop containing a `return`
    LoopReturnHandler,
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

    /// Creates a new, implicit, mutable parameter
    pub fn implicit_mutable(pattern: PatternId) -> Parameter {
        Parameter { pattern, is_implicit: true, is_mutable: true }
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

/// `handle <expression> | <cases>`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Handle {
    /// Holdover from when `handler` expressions created named handlers
    /// There's no name in the source now but some type checking/mir code uses the
    /// effect type which is bound to this name.
    pub handler_name: NameId,
    pub expression: ExprId,
    pub cases: Vec<(HandlePattern, ExprId)>,
}

/// An `and` chain containing an `is`, desugared so that `body` tests each condition in turn and
/// evaluates to the chain's `then` if they all succeed. A failing condition hits an
/// [Expr::Fallthrough], which jumps out of `body` to evaluate `else_` instead.
///
/// The conditions are nested so that names bound by an `is` are in scope for the rest of the
/// chain. We can't just copy `else_` into each failure branch since then one expression would
/// appear in several places in the control flow, each owning different values.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Chain {
    pub body: ExprId,
    pub else_: ExprId,
}

/// `lhs is pattern` - always desugared during `GetItem`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Is {
    pub lhs: ExprId,
    pub pattern: PatternId,
}

/// `do <block>` or `do <non-indented-block>
/// Always desugared during `GetItem`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Do {
    pub body: ExprId,
}

/// `ref rhs`, `mut rhs`, `read rhs`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Reference {
    pub kind: ReferenceKind,
    pub rhs: ExprId,
    /// Inserted by desugaring `a.[i] := b` to borrow `a`
    pub implicit: bool,
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
    /// Variable expression for the operator function, resolved via normal ability dispatch.
    pub op: Option<(CompoundAssignOp, ExprId)>,
}

/// `while cond do body`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct While {
    pub condition: ExprId,
    pub body: ExprId,
}

/// `for pattern in iterable do body`
/// Always desugared during `GetItem` to `(for) iterable (fn pattern -> body)`
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct For {
    pub pattern: PatternId,
    pub iterable: ExprId,
    pub body: ExprId,
}

/// Sugar for an immediately invoked helper function: `loop x (i = 0) -> ...`
/// The `recur` identifier is defined within bound to the name of the new helper.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Loop {
    pub parameters: Vec<LoopParameter>,
    pub body: ExprId,
}

/// A `loop` parameter is either a pattern over existing bindings in scope,
/// or a `pattern = expression` pair where the pattern is the loop helper function
/// parameter and the expression is its initial value - e.g `(y = 3)`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum LoopParameter {
    Pattern(PatternId),
    PatternAndExpr(PatternId, ExprId),
    UnitLiteral(Location),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum Pattern {
    Error,
    Variable(NameId),
    Literal(Literal),
    Constructor(PathId, Vec<PatternId>),
    // E.g. `Circle ..`, `Circle ..c`, `Circle a b ..`, `Circle a b ..c`
    ConstructorRest(PathId, Vec<PatternId>, Option<NameId>),
    TypeAnnotation(PatternId, Type),
    Or(Vec<PatternId>),
    Alias(NameId, PatternId),
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

pub type Generics = Vec<GenericParam>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct GenericParam {
    pub name: NameId,
    /// When `None`, this parameter's kind defaults to `Type`.
    pub kind: Option<KindAnnotation>,
    /// The type used when this parameter's argument is omitted, e.g. `I32` for `(b = I32)`
    pub default: Option<Type>,
}

impl GenericParam {
    pub fn new(name: NameId) -> Self {
        Self { name, kind: None, default: None }
    }

    pub fn is_effect(&self) -> bool {
        self.kind == Some(KindAnnotation::Effect)
    }

    /// True if this parameter's argument may be omitted to have it inferred, as in `(t = _)`
    pub fn defaults_to_hole(&self) -> bool {
        self.default.as_ref().is_some_and(|default| matches!(default.kind, TypeKind::Hole))
    }
}

/// Surface syntax for kind annotations on generic parameters.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum KindAnnotation {
    Type,
    U32,
    Place,
    Effect,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraitOrEffectDefinition {
    pub name: NameId,
    pub generics: Generics,
    pub body: Vec<Declaration>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraitImpl {
    pub name: NameId,
    pub parameters: Vec<Parameter>,
    pub trait_path: PathId,
    pub trait_arguments: Vec<Type>,
    pub body: Vec<(NameId, ExprId)>,
}

pub type Name = Arc<String>;

/// An extern has a name and a type determined by the expected type
/// when it is used in an expression. Most often this is bounded
/// by a type annotation on the extern expression itself.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Extern {
    pub name: String,
}

/// A top-level item evaluated at compile-time
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Comptime {
    Expr(ExprId),
    Derive(Vec<PathId>),
    Definition(Definition),
}
