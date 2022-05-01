use core::fmt;
use std::collections::VecDeque;
use std::rc::Rc;
use std::{collections::HashSet, fmt::Formatter};

use super::*;
use crate::hir::Ast;

#[derive(Default)]
pub struct AstPrinter {
    indent_level: u32,
    already_printed: HashSet<DefinitionId>,
    pub queue: VecDeque<Rc<Ast>>,
}

impl AstPrinter {
    fn block(&mut self, ast: &impl FmtAst, f: &mut Formatter) -> fmt::Result {
        self.indent_level += 1;
        ast.fmt_ast(self, f)?;
        self.indent_level -= 1;
        Ok(())
    }

    fn newline(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "\n")?;
        for _ in 0..self.indent_level {
            write!(f, "    ")?;
        }
        Ok(())
    }
}

pub trait FmtAst {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result;
}

impl FmtAst for Ast {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        dispatch_on_hir!(self, FmtAst::fmt_ast, printer, f)
    }
}

impl FmtAst for Literal {
    fn fmt_ast(&self, _printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        match self {
            Literal::Integer(x, kind) => {
                write!(f, "{}_{}", x, kind)
            },
            Literal::Float(x) => write!(f, "{}", f64::from_bits(*x)),
            Literal::CString(cstr) => write!(f, "\"{}\"", cstr),
            Literal::Char(c) => write!(f, "'{}'", c),
            Literal::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Literal::Unit => write!(f, "()"),
        }
    }
}

impl FmtAst for Variable {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        if !printer.already_printed.contains(&self.definition_id) {
            if let Some(def) = self.definition.clone() {
                printer.already_printed.insert(self.definition_id);
                printer.queue.push_back(def);
            }
        }

        write!(f, "v{}", self.definition_id.0)
    }
}

impl FmtAst for Lambda {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "(fn")?;

        for arg in &self.args {
            write!(f, " ")?;
            arg.fmt_ast(printer, f)?;
        }

        write!(f, " : {} = ", self.typ)?;
        printer.block(self.body.as_ref(), f)?;
        write!(f, ")")
    }
}

impl FmtAst for FunctionCall {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "(")?;
        self.function.fmt_ast(printer, f)?;

        for arg in &self.args {
            write!(f, " ")?;
            arg.fmt_ast(printer, f)?;
        }

        write!(f, ")")
    }
}

impl FmtAst for Definition {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        printer.already_printed.insert(self.variable);

        write!(f, "v{} = ", self.variable.0)?;
        if self.mutable {
            write!(f, "mut ")?;
        }
        printer.block(self.expr.as_ref(), f)
    }
}

impl FmtAst for If {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "if ")?;
        printer.block(self.condition.as_ref(), f)?;
        write!(f, " then ")?;
        printer.block(self.then.as_ref(), f)?;

        if let Some(otherwise) = &self.otherwise {
            write!(f, " else ")?;
            printer.block(otherwise.as_ref(), f)?;
        }

        write!(f, " endif")
    }
}

impl FmtAst for Return {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "return ")?;
        self.expression.fmt_ast(printer, f)
    }
}

impl FmtAst for Sequence {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        for (i, statement) in self.statements.iter().enumerate() {
            printer.newline(f)?;
            statement.fmt_ast(printer, f)?;
            if i != self.statements.len() - 1 {
                write!(f, ";")?;
            }
        }

        write!(f, "\n")
    }
}

impl FmtAst for Extern {
    fn fmt_ast(&self, _printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "extern {} : {}", self.name, self.typ)
    }
}

impl FmtAst for Assignment {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        self.lhs.fmt_ast(printer, f)?;
        write!(f, " := ")?;
        self.rhs.fmt_ast(printer, f)
    }
}

impl FmtAst for MemberAccess {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "(extract_field {} from ", self.member_index)?;
        self.lhs.fmt_ast(printer, f)?;
        write!(f, ")")
    }
}

impl FmtAst for Tuple {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "(")?;

        for (i, field) in self.fields.iter().enumerate() {
            field.fmt_ast(printer, f)?;
            if i != self.fields.len() - 1 {
                write!(f, ", ")?;
            }
        }

        write!(f, ")")
    }
}

impl FmtAst for ReinterpretCast {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "(reinterpret ")?;
        self.lhs.fmt_ast(printer, f)?;
        write!(f, " as {})", self.target_type)
    }
}

impl FmtAst for Builtin {
    fn fmt_ast(&self, _printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "#")?;
        match self {
            Builtin::AddInt => write!(f, "AddInt"),
            Builtin::AddFloat => write!(f, "AddFloat"),
            Builtin::SubInt => write!(f, "SubInt"),
            Builtin::SubFloat => write!(f, "SubFloat"),
            Builtin::MulInt => write!(f, "MulInt"),
            Builtin::MulFloat => write!(f, "MulFloat"),
            Builtin::DivInt => write!(f, "DivInt"),
            Builtin::DivFloat => write!(f, "DivFloat"),
            Builtin::ModInt => write!(f, "ModInt"),
            Builtin::ModFloat => write!(f, "ModFloat"),
            Builtin::LessInt => write!(f, "LessInt"),
            Builtin::LessFloat => write!(f, "LessFloat"),
            Builtin::GreaterInt => write!(f, "GreaterInt"),
            Builtin::GreaterFloat => write!(f, "GreaterFloat"),
            Builtin::EqInt => write!(f, "EqInt"),
            Builtin::EqFloat => write!(f, "EqFloat"),
            Builtin::EqChar => write!(f, "EqChar"),
            Builtin::EqBool => write!(f, "EqBool"),
            Builtin::SignExtend => write!(f, "SignExtend"),
            Builtin::ZeroExtend => write!(f, "ZeroExtend"),
            Builtin::Truncate => write!(f, "Truncate"),
            Builtin::Deref => write!(f, "Deref"),
            Builtin::Offset => write!(f, "Offset"),
            Builtin::Transmute => write!(f, "Transmute"),
        }
    }
}

impl FmtAst for Match {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        self.decision_tree.fmt_ast(printer, f)?;
        for (i, branch) in self.branches.iter().enumerate() {
            printer.newline(f)?;
            write!(f, "branch {} -> ", i)?;
            printer.block(branch, f)?;
        }
        Ok(())
    }
}

impl FmtAst for DecisionTree {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        match self {
            DecisionTree::Leaf(i) => write!(f, "goto branch {}", i),
            DecisionTree::Definition(def, tree) => {
                def.fmt_ast(printer, f)?;
                printer.newline(f)?;
                tree.fmt_ast(printer, f)
            },
            DecisionTree::Switch {
                int_to_switch_on,
                cases,
                else_case,
            } => {
                write!(f, "switch ")?;
                int_to_switch_on.fmt_ast(printer, f)?;
                for (tag, case) in cases {
                    printer.newline(f)?;
                    write!(f, "case {}:", tag)?;
                    printer.indent_level += 1;
                    printer.newline(f)?;
                    case.fmt_ast(printer, f)?;
                    printer.indent_level -= 1;
                }
                if let Some(case) = else_case {
                    printer.newline(f)?;
                    write!(f, "_ -> ")?;
                    printer.block(case.as_ref(), f)?;
                }
                Ok(())
            },
        }
    }
}
