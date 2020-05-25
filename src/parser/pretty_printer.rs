use super::ast::{ self, Expr };
use std::fmt::{ self, Display, Formatter };

impl<'a, T> Display for Expr<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        dispatch_on_expr!(self, Display::fmt, f)
    }
}

impl<'a, T> Display for ast::Literal<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ast::Literal::*;
        match self {
            Integer(x, _, _) => write!(f, "{}", x),
            Float(x, _, _) => write!(f, "{}", x),
            String(s, _, _) => write!(f, "\"{}\"", s),
            Char(c, _, _) => write!(f, "'{}'", c),
            Bool(b, _, _) => write!(f, "{}", if *b { "true" } else { "false" }),
            Unit(_, _) => write!(f, "()"),
        }
    }
}

impl<'a, T> Display for ast::Variable<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ast::Variable::*;
        match self {
            Identifier(name, _, _) => write!(f, "{}", name),
            Operator(token, _, _) => write!(f, "{}", token),
        }
    }
}

impl<'a, T> Display for ast::Lambda<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(\\")?;
        for arg in self.args.iter() {
            write!(f, " {}", arg)?;
        }
        write!(f, " = {})", self.body)
    }
}

impl<'a, T> Display for ast::FunctionCall<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ast::{Expr::Variable, Variable::Operator};
        use crate::lexer::token::Token::Semicolon;

        let args = self.args.iter()
            .map(|arg| format!("{}", arg))
            .collect::<Vec<_>>();

        // pretty-print calls to ';' on separate lines
        match self.function.as_ref() {
            Variable(Operator(Semicolon, _, _)) => {
                write!(f, "{}", args.join(";\n"))
            },
            _ => {
                write!(f, "({} {})", self.function, args.join(" "))
            },
        }
    }
}

impl<'a, T> Display for ast::Definition<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({} = {})", self.pattern, self.expr)
    }
}

impl<'a, T> Display for ast::If<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(ref otherwise) = self.otherwise {
            write!(f, "(if {} {} {})", self.condition, self.then, otherwise)
        } else {
            write!(f, "(if {} {})", self.condition, self.then)
        }
    }
}

impl<'a, T> Display for ast::Match<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "(match {}", self.expression)?;
        for (pattern, branch) in self.branches.iter() {
            write!(f, " ({} {})", pattern, branch)?;
        }
        write!(f, ")")
    }
}
