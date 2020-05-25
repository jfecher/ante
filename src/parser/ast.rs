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
pub enum Expr<'a, T> {
    Literal(Literal<'a, T>),
    Variable(Variable<'a, T>),
    Lambda(Lambda<'a, T>),
    FunctionCall(FunctionCall<'a, T>),
    Definition(Definition<'a, T>),
    If(If<'a, T>),
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

    pub fn definition(pattern: Expr<'a, T>, expr: Expr<'a, T>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::Definition(Definition { pattern: Box::new(pattern), expr: Box::new(expr), location, data })
    }

    pub fn if_expr(condition: Expr<'a, T>, then: Expr<'a, T>, otherwise: Option<Expr<'a, T>>, location: Location<'a>, data: T) -> Expr<'a, T> {
        Expr::If(If { condition: Box::new(condition), then: Box::new(then), otherwise: otherwise.map(Box::new), location, data })
    }
}

impl<'a, T> Locatable<'a> for Expr<'a, T> {
    fn locate(&self) -> Location<'a> {
        use Expr::*;
        match self {
            Literal(literal) => literal.locate(),
            Variable(variable) => variable.locate(),
            Lambda(lambda) => lambda.locate(),
            FunctionCall(function_call) => function_call.locate(),
            Definition(definition) => definition.locate(),
            If(if_expr) => if_expr.locate(),
        }
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

impl<'a, T> Locatable<'a> for Lambda<'a, T> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

impl<'a, T> Locatable<'a> for FunctionCall<'a, T> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

impl<'a, T> Locatable<'a> for Definition<'a, T> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

impl<'a, T> Locatable<'a> for If<'a, T> {
    fn locate(&self) -> Location<'a> {
        self.location
    }
}

// TODO:
// Module = RootNode | ExtNode
// Collection = ArrayNode | TupleNode
// FunctionCall = UnOpNode | BinOpNode | SeqNode? | TypeCastNode | NamedValNode
// TraitImpl = ExtNode
// TypeNode
// RetNode
// ImportNode
// JumpNode
// Loop = WhileNode | ForNode
// MatchNode (pattern -> expr*)
// IfNode
// DataDeclNode
// TraitNode
