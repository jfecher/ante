use crate::lexer::token::Token;

#[derive(Debug)]
pub enum Literal<T> {
    Integer(u64, T),
    Float(f64, T),
    String(String, T),
    Char(char, T),
    Bool(bool, T),
}

#[derive(Debug)]
pub enum Variable<'a, T> {
    Identifier(&'a str, T),
    Operator(Token<'a>, T),
}

#[derive(Debug)]
pub struct Lambda<'a, T> {
    args: Vec<Expr<'a, T>>,
    body: Box<Expr<'a, T>>,
    data: T,
}

#[derive(Debug)]
pub struct FunctionCall<'a, T> {
    function: Box<Expr<'a, T>>,
    args: Vec<Expr<'a, T>>,
    data: T,
}

#[derive(Debug)]
pub struct Definition<'a, T> {
    pattern: Box<Expr<'a, T>>,
    expr: Box<Expr<'a, T>>,
    data: T,
}

#[derive(Debug)]
pub enum Expr<'a, T> {
    Literal(Literal<T>),
    Variable(Variable<'a, T>),
    Lambda(Lambda<'a, T>),
    FunctionCall(FunctionCall<'a, T>),
    Definition(Definition<'a, T>),
}

impl<'a, T> Expr<'a, T> {
    pub fn integer(x: u64, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Integer(x, data))
    }

    pub fn float(x: f64, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Float(x, data))
    }

    pub fn string(x: String, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::String(x, data))
    }

    pub fn char_literal(x: char, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Char(x, data))
    }

    pub fn bool_literal(x: bool, data: T) -> Expr<'a, T> {
        Expr::Literal(Literal::Bool(x, data))
    }

    pub fn variable(name: &'a str, data: T) -> Expr<'a, T> {
        Expr::Variable(Variable::Identifier(name, data))
    }

    pub fn operator(operator: Token<'a>, data: T) -> Expr<'a, T> {
        Expr::Variable(Variable::Operator(operator, data))
    }

    pub fn lambda(args: Vec<Expr<'a, T>>, body: Expr<'a, T>, data: T) -> Expr<'a, T> {
        Expr::Lambda(Lambda { args, body: Box::new(body), data })
    }

    pub fn function_call(function: Expr<'a, T>, args: Vec<Expr<'a, T>>, data: T) -> Expr<'a, T> {
        Expr::FunctionCall(FunctionCall { function: Box::new(function), args, data })
    }

    pub fn definition(pattern: Expr<'a, T>, expr: Expr<'a, T>, data: T) -> Expr<'a, T> {
        Expr::Definition(Definition { pattern: Box::new(pattern), expr: Box::new(expr), data })
    }
}

// Module = RootNode | ExtNode
// Literal = IntLitNode | FltLitNode | BoolLitNode | CharLitNode | StrLitNode
// Collection = ArrayNode | TupleNode
// FunctionCall = UnOpNode | BinOpNode | SeqNode?
// Trait = ExtNode
// BlockNode
// TypeNode
// TypeCastNode
// RetNode
// NamedValNode
// VarNode
// Decl = VarAssignNode | FuncDeclNode
// ImportNode
// JumpNode
// Loop = WhileNode | ForNode
// MatchNode (pattern -> expr*)
// IfNode
// DataDeclNode
// TraitNode
