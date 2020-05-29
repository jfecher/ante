use crate::lexer::token::Token;
use crate::error::location::{ Location, Locatable };

#[derive(Debug)]
pub enum Literal<'a, T> {
    Integer(u64, Location<'a>, T),
    Float(f64, Location<'a>, T),
    String(String, Location<'a>, T),
    Char(char, Location<'a>, T),
    Bool(bool, Location<'a>, T),
    Unit(Location<'a>, T),
}

#[derive(Debug)]
pub enum Variable<'a, T> {
    Identifier(&'a str, Location<'a>, T),
    Operator(Token<'a>, Location<'a>, T),
}

#[derive(Debug)]
pub struct Lambda<'a, T> {
    pub args: Vec<Expr<'a, T>>,
    pub body: Box<Expr<'a, T>>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct FunctionCall<'a, T> {
    pub function: Box<Expr<'a, T>>,
    pub args: Vec<Expr<'a, T>>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct Definition<'a, T> {
    pub pattern: Box<Expr<'a, T>>,
    pub expr: Box<Expr<'a, T>>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct If<'a, T> {
    pub condition: Box<Expr<'a, T>>,
    pub then: Box<Expr<'a, T>>,
    pub otherwise: Option<Box<Expr<'a, T>>>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct Match<'a, T> {
    pub expression: Box<Expr<'a, T>>,
    pub branches: Vec<(Expr<'a, T>, Expr<'a, T>)>,
    pub location: Location<'a>,
    pub data: T,
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
    TypeVariable(&'a str, Location<'a>),
    UserDefinedType(&'a str, Location<'a>),
    TypeApplication(Box<Type<'a>>, Vec<Type<'a>>, Location<'a>),
}

#[derive(Debug)]
pub enum TypeDefinitionBody<'a> {
    UnionOf(Vec<Type<'a>>),
    StructOf(Vec<(&'a str, Type<'a>)>),
    AliasOf(Type<'a>),
}

#[derive(Debug)]
pub struct TypeDefinition<'a, T> {
    pub name: &'a str,
    pub args: Vec<&'a str>,
    pub definition: TypeDefinitionBody<'a>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct TypeAnnotation<'a, T> {
    pub lhs: Box<Expr<'a, T>>,
    pub rhs: Type<'a>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct Import<'a, T> {
    pub path: Vec<&'a str>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct TraitDefinition<'a, T> {
    pub name: &'a str,
    pub args: Vec<&'a str>,
    pub fundeps: Vec<&'a str>,

    // Storing function declarations as TypeAnnotations here
    // throws away any names given to parameters. In practice
    // this shouldn't matter until refinement types are implemented
    // that can depend upon these names.
    pub declarations: Vec<TypeAnnotation<'a, T>>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub struct TraitImpl<'a, T> {
    pub trait_name: &'a str,
    pub trait_args: Vec<Type<'a>>,
    pub definitions: Vec<Definition<'a, T>>,
    pub location: Location<'a>,
    pub data: T,
}

#[derive(Debug)]
pub enum Expr<'a, T> {
    Literal(Literal<'a, T>),
    Variable(Variable<'a, T>),
    Lambda(Lambda<'a, T>),
    FunctionCall(FunctionCall<'a, T>),
    Definition(Definition<'a, T>),
    If(If<'a, T>),
    Match(Match<'a, T>),
    TypeDefinition(TypeDefinition<'a, T>),
    TypeAnnotation(TypeAnnotation<'a, T>),
    Import(Import<'a, T>),
    TraitDefinition(TraitDefinition<'a, T>),
    TraitImpl(TraitImpl<'a, T>),
}

impl<'a, T> Expr<'a, T> {
    pub fn integer(x: u64, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Integer(x, location, data))
    }

    pub fn float(x: f64, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Float(x, location, data))
    }

    pub fn string(x: String, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::String(x, location, data))
    }

    pub fn char_literal(x: char, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Char(x, location, data))
    }

    pub fn bool_literal(x: bool, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Bool(x, location, data))
    }

    pub fn unit_literal(location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Unit(location, data))
    }

    pub fn variable(name: &'a str, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Variable(Variable::Identifier(name, location, data))
    }

    pub fn operator(operator: Token<'a>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Variable(Variable::Operator(operator, location, data))
    }

    pub fn lambda(args: Vec<Expr<'a, T>>, body: Expr<'a, T>, location: Location<'a>, data: T) -> Expr<'a, T> {
        assert!(!args.is_empty());
        Expr::Lambda(Lambda { args, body: Box::new(body), location, data })
    }

    pub fn function_call(function: Expr<'a, T>, args: Vec<Expr<'a, T>>, location: Location<'a>, data: T) -> Expr<'a, T> {
        assert!(!args.is_empty());
        Expr::FunctionCall(FunctionCall { function: Box::new(function), args, location, data })
    }

    pub fn if_expr(condition: Expr<'a, T>, then: Expr<'a, T>, otherwise: Option<Expr<'a, T>>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::If(If { condition: Box::new(condition), then: Box::new(then), otherwise: otherwise.map(Box::new), location, data })
    }

    pub fn match_expr(expression: Expr<'a, T>, branches: Vec<(Expr<'a, T>, Expr<'a, T>)>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Match(Match { expression: Box::new(expression), branches, location, data })
    }

    pub fn type_definition(name: &'a str, args: Vec<&'a str>, definition: TypeDefinitionBody<'a>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::TypeDefinition(TypeDefinition { name, args, definition, location, data })
    }

    pub fn type_annotation(lhs: Expr<'a, T>, rhs: Type<'a>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::TypeAnnotation(TypeAnnotation { lhs: Box::new(lhs), rhs, location, data })
    }

    pub fn import(path: Vec<&'a str>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Import(Import { path, location, data })
    }

    pub fn trait_definition(name: &'a str, args: Vec<&'a str>, fundeps: Vec<&'a str>, declarations: Vec<TypeAnnotation<'a, T>>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::TraitDefinition(TraitDefinition { name, args, fundeps, declarations, location, data })
    }

    pub fn trait_impl(trait_name: &'a str, trait_args: Vec<Type<'a>>, definitions: Vec<Definition<'a, T>>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::TraitImpl(TraitImpl { trait_name, trait_args, definitions, location, data })
    }
}

macro_rules! dispatch_on_expr {
    ( $expr_name:expr, $function:expr $(, $($args:expr),* )? ) => ({
        match $expr_name {
            Expr::Literal(inner) =>         $function(inner $(, $($args),* )? ),
            Expr::Variable(inner) =>        $function(inner $(, $($args),* )? ),
            Expr::Lambda(inner) =>          $function(inner $(, $($args),* )? ),
            Expr::FunctionCall(inner) =>    $function(inner $(, $($args),* )? ),
            Expr::Definition(inner) =>      $function(inner $(, $($args),* )? ),
            Expr::If(inner) =>              $function(inner $(, $($args),* )? ),
            Expr::Match(inner) =>           $function(inner $(, $($args),* )? ),
            Expr::TypeDefinition(inner) =>  $function(inner $(, $($args),* )? ),
            Expr::TypeAnnotation(inner) =>  $function(inner $(, $($args),* )? ),
            Expr::Import(inner) =>          $function(inner $(, $($args),* )? ),
            Expr::TraitDefinition(inner) => $function(inner $(, $($args),* )? ),
            Expr::TraitImpl(inner) =>       $function(inner $(, $($args),* )? ),
        }
    });
}

impl<'a, T> Locatable<'a> for Expr<'a, T> {
    fn locate(&self) -> Location<'a> {
        dispatch_on_expr!(self, Locatable::locate)
    }
}

impl<'a, T> Locatable<'a> for Literal<'a, T> {
    fn locate(&self) -> Location<'a> {
        use Literal::*;
        match self {
            Integer(_, loc, _) => *loc,
            Float(_, loc, _) => *loc,
            String(_, loc, _) => *loc,
            Char(_, loc, _) => *loc,
            Bool(_, loc, _) => *loc,
            Unit(loc, _) => *loc,
        }
    }
}

impl<'a, T> Locatable<'a> for Variable<'a, T> {
    fn locate(&self) -> Location<'a> {
        use Variable::*;
        match self {
            Identifier(_, loc, _) => *loc,
            Operator(_, loc, _) => *loc,
        }
    }
}

macro_rules! impl_locatable_for {( $name:tt ) => {
    impl<'a, T> Locatable<'a> for $name<'a, T> {
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

// TODO:
// Module = RootNode | ExtNode
// Collection = ArrayNode | TupleNode
// RetNode
// JumpNode
// Loop = WhileNode | ForNode
