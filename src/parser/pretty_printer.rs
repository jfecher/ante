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
        }
    }
}

impl<T: Debug> Display for ast::Literal<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ast::Literal::*;
        match self {
            Integer(x, _data) => write!(f, "{}", x),
            Float(x, _data) => write!(f, "{}", x),
            String(s, _data) => write!(f, "\"{}\"", s),
            Char(c, _data) => write!(f, "'{}'", c),
            Bool(b, _data) => write!(f, "{}", if *b { "true" } else { "false" }),
            Unit(_data) => write!(f, "()"),
        }
    }
}

impl<'a, T: Debug> Display for ast::Variable<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ast::Variable::*;
        match self {
            Identifier(name, _data) => write!(f, "{}", name),
            Operator(token, _data) => write!(f, "{}", token),
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
            Variable(Operator(Semicolon, _)) => {
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
