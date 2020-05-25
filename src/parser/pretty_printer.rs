use super::ast::{ self, Expr };
use std::fmt::{ self, Display, Debug };

impl<'a, T: Debug> Display for Expr<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Literal(literal) => Display::fmt(literal, f),
            Expr::Variable(variable) => Display::fmt(variable, f),
            Expr::Lambda(lambda) => Display::fmt(lambda, f),
            Expr::FunctionCall(function_call) => Display::fmt(function_call, f),
            Expr::Definition(definition) => Display::fmt(definition, f),
            Expr::If(if_expr) => Display::fmt(if_expr, f),
        }
    }
}

impl<'a, T: Debug> Display for ast::Literal<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl<'a, T: Debug> Display for ast::Variable<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ast::Variable::*;
        match self {
            Identifier(name, _, _) => write!(f, "{}", name),
            Operator(token, _, _) => write!(f, "{}", token),
        }
    }
}

impl<'a, T: Debug> Display for ast::Lambda<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(\\")?;
        for arg in self.args.iter() {
            write!(f, " {}", arg)?;
        }
        write!(f, " = {})", self.body)
    }
}

impl<'a, T: Debug> Display for ast::FunctionCall<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ast::{Expr::Variable, Variable::Operator};
        use crate::lexer::token::Token::Semicolon;

        // pretty-print calls to ';' on separate lines
        match self.function.as_ref() {
            Variable(Operator(Semicolon, _, _)) => {
                for arg in self.args.iter() {
                    write!(f, "{};\n", arg)?;
                }
                write!(f, "")
            },
            _ => {
                write!(f, "({}", self.function)?;
                for arg in self.args.iter() {
                    write!(f, " {}", arg)?;
                }
                write!(f, ")")
            },
        }
    }
}

impl<'a, T: Debug> Display for ast::Definition<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} = {})", self.pattern, self.expr)
    }
}

impl<'a, T: Debug> Display for ast::If<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref otherwise) = self.otherwise {
            write!(f, "(if {} {} {})", self.condition, self.then, otherwise)
        } else {
            write!(f, "(if {} {})", self.condition, self.then)
        }
    }
}
