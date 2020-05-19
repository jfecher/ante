
#[derive(Debug)]
pub enum Literal<T> {
    Integer(i64, T),
    Float(f64, T),
    String(String, T),
}

#[derive(Debug)]
pub struct Variable<T> {
    name: String,
    data: T,
}

#[derive(Debug)]
pub struct Lambda<T> {
    args: Vec<Expr<T>>,
    data: T,
}

#[derive(Debug)]
pub struct FunctionCall<T> {
    function: Box<Expr<T>>,
    args: Vec<Expr<T>>,
    data: T,
}

#[derive(Debug)]
pub struct Definition<T> {
    pattern: Box<Expr<T>>,
    expr: Box<Expr<T>>,
    data: T,
}

#[derive(Debug)]
pub struct Module<T> {
    pub definitions: Vec<Definition<T>>,
    pub contents: Box<Expr<T>>,
    pub data: T,
}

#[derive(Debug)]
pub enum Expr<T> {
    Literal(Literal<T>),
    Variable(Variable<T>),
    Lambda(Lambda<T>),
    FunctionCall(FunctionCall<T>),
    Definition(Definition<T>),
    Module(Module<T>),
}

impl Default for Expr<()> {
    fn default() -> Self {
        Expr::Literal(Literal::Integer(0, ()))
    }
}

impl<T> Expr<T> {
    pub fn integer(x: i64, data: T) -> Expr<T> {
        Expr::Literal(Literal::Integer(x, data))
    }

    pub fn float(x: f64, data: T) -> Expr<T> {
        Expr::Literal(Literal::Float(x, data))
    }

    pub fn string(x: String, data: T) -> Expr<T> {
        Expr::Literal(Literal::String(x, data))
    }

    pub fn variable(name: String, data: T) -> Expr<T> {
        Expr::Variable(Variable { name, data })
    }

    pub fn lambda(args: Vec<Expr<T>>, data: T) -> Expr<T> {
        Expr::Lambda(Lambda { args, data })
    }

    pub fn function_call(function: Expr<T>, args: Vec<Expr<T>>, data: T) -> Expr<T> {
        Expr::FunctionCall(FunctionCall { function: Box::new(function), args, data })
    }

    pub fn definition(pattern: Expr<T>, expr: Expr<T>, data: T) -> Expr<T> {
        Expr::Definition(Definition { pattern: Box::new(pattern), expr: Box::new(expr), data })
    }

    pub fn module(declarations: Vec<Definition<T>>, contents: Expr<T>, data: T) -> Expr<T> {
        Expr::Module(Module { definitions: declarations, contents: Box::new(contents), data })
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
