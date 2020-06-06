use crate::lexer::token::Token;
use crate::error::location::{ Location, Locatable };
use crate::nameresolution::modulecache::{ DefinitionInfoId, TraitInfoId, ModuleId };
use crate::types::{ self, TypeInfoId };

#[derive(Debug)]
pub enum Literal<'a> {
    Integer(u64, Location<'a>),
    Float(f64, Location<'a>),
    String(String, Location<'a>),
    Char(char, Location<'a>),
    Bool(bool, Location<'a>),
    Unit(Location<'a>),
}

#[derive(Debug)]
pub enum Variable<'a> {
    Identifier(String, Location<'a>, Option<DefinitionInfoId>, Option<types::Type>),
    Operator(Token, Location<'a>, Option<DefinitionInfoId>, Option<types::Type>),
    TypeConstructor(String, Location<'a>, Option<TypeInfoId>, Option<types::Type>),
}

#[derive(Debug)]
pub struct Lambda<'a> {
    pub args: Vec<Ast<'a>>,
    pub body: Box<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct FunctionCall<'a> {
    pub function: Box<Ast<'a>>,
    pub args: Vec<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct Definition<'a> {
    pub pattern: Box<Ast<'a>>,
    pub expr: Box<Ast<'a>>,
    pub location: Location<'a>,
    pub info: Option<DefinitionInfoId>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct If<'a> {
    pub condition: Box<Ast<'a>>,
    pub then: Box<Ast<'a>>,
    pub otherwise: Option<Box<Ast<'a>>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct Match<'a> {
    pub expression: Box<Ast<'a>>,
    pub branches: Vec<(Ast<'a>, Ast<'a>)>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

// Type nodes in the AST, different from the representation of types during type checking.
// PointerType and potentially UserDefinedType are actually type constructors
#[derive(Debug)]
pub enum Type<'a> {
    IntegerType(Location<'a>),
    FloatType(Location<'a>),
    CharType(Location<'a>),
    StringType(Location<'a>),
    BooleanType(Location<'a>),
    UnitType(Location<'a>),
    ReferenceType(Location<'a>),
    FunctionType(Vec<Type<'a>>, Box<Type<'a>>, Location<'a>),
    TypeVariable(String, Location<'a>),
    UserDefinedType(String, Location<'a>),
    TypeApplication(Box<Type<'a>>, Vec<Type<'a>>, Location<'a>),
}

#[derive(Debug)]
pub enum TypeDefinitionBody<'a> {
    UnionOf(Vec<(String, Vec<Type<'a>>, Location<'a>)>),
    StructOf(Vec<(String, Type<'a>, Location<'a>)>),
    AliasOf(Type<'a>),
}

#[derive(Debug)]
pub struct TypeDefinition<'a> {
    pub name: String,
    pub args: Vec<String>,
    pub definition: TypeDefinitionBody<'a>,
    pub location: Location<'a>,
    pub type_info: Option<TypeInfoId>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct TypeAnnotation<'a> {
    pub lhs: Box<Ast<'a>>,
    pub rhs: Type<'a>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct Import<'a> {
    pub path: Vec<String>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
    pub module_id: Option<ModuleId>,
}

#[derive(Debug)]
pub struct TraitDefinition<'a> {
    pub name: String,
    pub args: Vec<String>,
    pub fundeps: Vec<String>,

    // Storing function declarations as TypeAnnotations here
    // throws away any names given to parameters. In practice
    // this shouldn't matter until refinement types are implemented
    // that can depend upon these names.
    pub declarations: Vec<TypeAnnotation<'a>>,
    pub location: Location<'a>,
    pub trait_info: Option<TraitInfoId>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct TraitImpl<'a> {
    pub trait_name: String,
    pub trait_args: Vec<Type<'a>>,
    pub definitions: Vec<Definition<'a>>,
    pub location: Location<'a>,
    pub trait_info: Option<TraitInfoId>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
pub struct Return<'a> {
    pub expression: Box<Ast<'a>>,
    pub location: Location<'a>,
    pub typ: Option<types::Type>,
}

#[derive(Debug)]
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
}

impl<'a> Ast<'a> {
    pub fn integer(x: u64, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal::Integer(x, location))
    }

    pub fn float(x: f64, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal::Float(x, location))
    }

    pub fn string(x: String, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal::String(x, location))
    }

    pub fn char_literal(x: char, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal::Char(x, location))
    }

    pub fn bool_literal(x: bool, location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal::Bool(x, location))
    }

    pub fn unit_literal(location: Location<'a>) -> Ast<'a> {
        Ast::Literal(Literal::Unit(location))
    }

    pub fn variable(name: String, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable::Identifier(name, location, None, None))
    }

    pub fn operator(operator: Token, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable::Operator(operator, location, None, None))
    }

    pub fn type_constructor(name: String, location: Location<'a>) -> Ast<'a> {
        Ast::Variable(Variable::TypeConstructor(name, location, None, None))
    }

    pub fn lambda(args: Vec<Ast<'a>>, body: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::Lambda(Lambda { args, body: Box::new(body), location, typ: None })
    }

    pub fn function_call(function: Ast<'a>, args: Vec<Ast<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::FunctionCall(FunctionCall { function: Box::new(function), args, location, typ: None })
    }

    pub fn if_expr(condition: Ast<'a>, then: Ast<'a>, otherwise: Option<Ast<'a>>, location: Location<'a>) -> Ast<'a> {
        Ast::If(If { condition: Box::new(condition), then: Box::new(then), otherwise: otherwise.map(Box::new), location, typ: None })
    }

    pub fn match_expr(expression: Ast<'a>, branches: Vec<(Ast<'a>, Ast<'a>)>, location: Location<'a>) -> Ast<'a> {
        Ast::Match(Match { expression: Box::new(expression), branches, location, typ: None })
    }

    pub fn type_definition(name: String, args: Vec<String>, definition: TypeDefinitionBody<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::TypeDefinition(TypeDefinition { name, args, definition, location, type_info: None, typ: None })
    }

    pub fn type_annotation(lhs: Ast<'a>, rhs: Type<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::TypeAnnotation(TypeAnnotation { lhs: Box::new(lhs), rhs, location, typ: None })
    }

    pub fn import(path: Vec<String>, location: Location<'a>) -> Ast<'a> {
        assert!(!path.is_empty());
        Ast::Import(Import { path, location, typ: None, module_id: None, })
    }

    pub fn trait_definition(name: String, args: Vec<String>, fundeps: Vec<String>, declarations: Vec<TypeAnnotation<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!args.is_empty());
        Ast::TraitDefinition(TraitDefinition { name, args, fundeps, declarations, location, trait_info: None, typ: None })
    }

    pub fn trait_impl(trait_name: String, trait_args: Vec<Type<'a>>, definitions: Vec<Definition<'a>>, location: Location<'a>) -> Ast<'a> {
        assert!(!trait_args.is_empty());
        Ast::TraitImpl(TraitImpl { trait_name, trait_args, definitions, location, trait_info: None, typ: None })
    }

    pub fn return_expr(expression: Ast<'a>, location: Location<'a>) -> Ast<'a> {
        Ast::Return(Return { expression: Box::new(expression), location, typ: None })
    }
}

macro_rules! dispatch_on_expr {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            crate::parser::ast::Ast::Literal(inner) =>         $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::Variable(inner) =>        $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::Lambda(inner) =>          $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::FunctionCall(inner) =>    $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::Definition(inner) =>      $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::If(inner) =>              $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::Match(inner) =>           $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::TypeDefinition(inner) =>  $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::TypeAnnotation(inner) =>  $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::Import(inner) =>          $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::TraitDefinition(inner) => $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::TraitImpl(inner) =>       $function(inner $(, $($args),* )? ),
            crate::parser::ast::Ast::Return(inner) =>          $function(inner $(, $($args),* )? ),
        }
    });
}

impl<'a> Locatable<'a> for Ast<'a> {
    fn locate(&self) -> Location<'a> {
        dispatch_on_expr!(self, Locatable::locate)
    }
}

impl<'a> Locatable<'a> for Literal<'a> {
    fn locate(&self) -> Location<'a> {
        use Literal::*;
        match self {
            Integer(_, loc) => *loc,
            Float(_, loc) => *loc,
            String(_, loc) => *loc,
            Char(_, loc) => *loc,
            Bool(_, loc) => *loc,
            Unit(loc) => *loc,
        }
    }
}

impl<'a> Locatable<'a> for Variable<'a> {
    fn locate(&self) -> Location<'a> {
        use Variable::*;
        match self {
            Identifier(_, loc, _, _) => *loc,
            Operator(_, loc, _, _) => *loc,
            TypeConstructor(_, loc, _, _) => *loc,
        }
    }
}

macro_rules! impl_locatable_for {( $name:tt ) => {
    impl<'a> Locatable<'a> for $name<'a> {
        fn locate(&self) -> Location<'a> {
            self.location
        }
    }
};}

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

// TODO:
// Module = RootNode | ExtNode
// Collection = ArrayNode | TupleNode
// JumpNode
// Loop = WhileNode | ForNode
