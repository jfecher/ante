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
    pub fn start(&mut self, ast: &impl FmtAst, f: &mut Formatter) -> fmt::Result {
        ast.fmt_ast(self, f)?;

        while let Some(ast) = self.queue.pop_front() {
            write!(f, "\n\n")?;
            ast.fmt_ast(self, f)?;
        }

        Ok(())
    }

    fn block(&mut self, ast: &impl FmtAst, f: &mut Formatter) -> fmt::Result {
        self.indent_level += 1;
        ast.fmt_ast(self, f)?;
        self.indent_level -= 1;
        Ok(())
    }

    fn newline(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f)?;
        for _ in 0..self.indent_level {
            write!(f, "    ")?;
        }
        Ok(())
    }

    fn fmt_call(&mut self, func: impl FmtAst, args: &[impl FmtAst], f: &mut Formatter) -> fmt::Result {
        write!(f, "(")?;
        func.fmt_ast(self, f)?;

        for arg in args {
            write!(f, " ")?;
            arg.fmt_ast(self, f)?;
        }

        write!(f, ")")
    }

    fn fmt_cast(&mut self, func: impl FmtAst, arg: impl FmtAst, typ: &Type, f: &mut Formatter) -> fmt::Result {
        write!(f, "(")?;
        func.fmt_ast(self, f)?;
        write!(f, " ")?;
        arg.fmt_ast(self, f)?;
        write!(f, " {})", typ)
    }

    fn fmt_offset(&mut self, ptr: impl FmtAst, offset: impl FmtAst, size: u32, f: &mut Formatter) -> fmt::Result {
        write!(f, "(#Offset")?;
        write!(f, " ")?;
        ptr.fmt_ast(self, f)?;
        write!(f, " ")?;
        offset.fmt_ast(self, f)?;
        write!(f, " {})", size)
    }
}

pub trait FmtAst {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result;
}

impl FmtAst for &'static str {
    fn fmt_ast(&self, _: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl FmtAst for Ast {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        dispatch_on_hir!(self, FmtAst::fmt_ast, printer, f)
    }
}

impl<'a> FmtAst for &'a Ast {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        dispatch_on_hir!(self, FmtAst::fmt_ast, printer, f)
    }
}

impl<'a> FmtAst for &'a Box<Ast> {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        dispatch_on_hir!(self.as_ref(), FmtAst::fmt_ast, printer, f)
    }
}

impl FmtAst for Literal {
    fn fmt_ast(&self, _printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        match self {
            Literal::Integer(x, kind) => {
                write!(f, "{}_{}", x, kind)
            },
            Literal::Float(x, kind) => write!(f, "{}_{}", f64::from_bits(*x), kind),
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

        if let Some(name) = &self.name {
            write!(f, "{}_v{}", name, self.definition_id.0)
        } else {
            write!(f, "v{}", self.definition_id.0)
        }
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
        printer.fmt_call(self.function.as_ref(), &self.args, f)
    }
}

impl FmtAst for Definition {
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        printer.already_printed.insert(self.variable);

        if let Some(name) = &self.name {
            write!(f, "{}_v{} = ", name, self.variable.0)?;
        } else {
            write!(f, "v{} = ", self.variable.0)?;
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

        writeln!(f)
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
        write!(f, "(")?;
        self.lhs.fmt_ast(printer, f)?;
        write!(f, " . {})", self.member_index)
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
    fn fmt_ast(&self, printer: &mut AstPrinter, f: &mut Formatter) -> fmt::Result {
        match self {
            Builtin::AddInt(a, b) => printer.fmt_call("#AddInt", &[a, b], f),
            Builtin::AddFloat(a, b) => printer.fmt_call("#AddFloat", &[a, b], f),
            Builtin::SubInt(a, b) => printer.fmt_call("#SubInt", &[a, b], f),
            Builtin::SubFloat(a, b) => printer.fmt_call("#SubFloat", &[a, b], f),
            Builtin::MulInt(a, b) => printer.fmt_call("#MulInt", &[a, b], f),
            Builtin::MulFloat(a, b) => printer.fmt_call("#MulFloat", &[a, b], f),
            Builtin::DivSigned(a, b) => printer.fmt_call("#DivSigned", &[a, b], f),
            Builtin::DivUnsigned(a, b) => printer.fmt_call("#DivUnsigned", &[a, b], f),
            Builtin::DivFloat(a, b) => printer.fmt_call("#DivFloat", &[a, b], f),
            Builtin::ModSigned(a, b) => printer.fmt_call("#ModSigned", &[a, b], f),
            Builtin::ModUnsigned(a, b) => printer.fmt_call("#ModSigned", &[a, b], f),
            Builtin::ModFloat(a, b) => printer.fmt_call("#ModFloat", &[a, b], f),
            Builtin::LessSigned(a, b) => printer.fmt_call("#LessSigned", &[a, b], f),
            Builtin::LessUnsigned(a, b) => printer.fmt_call("#LessUnsigned", &[a, b], f),
            Builtin::LessFloat(a, b) => printer.fmt_call("#LessFloat", &[a, b], f),
            Builtin::EqInt(a, b) => printer.fmt_call("#EqInt", &[a, b], f),
            Builtin::EqFloat(a, b) => printer.fmt_call("#EqFloat", &[a, b], f),
            Builtin::EqChar(a, b) => printer.fmt_call("#EqChar", &[a, b], f),
            Builtin::EqBool(a, b) => printer.fmt_call("#EqBool", &[a, b], f),
            Builtin::SignExtend(a, b) => printer.fmt_cast("#SignExtend", a, b, f),
            Builtin::ZeroExtend(a, b) => printer.fmt_cast("#ZeroExtend", a, b, f),
            Builtin::SignedToFloat(a, b) => printer.fmt_cast("#SignedToFloat", a, b, f),
            Builtin::UnsignedToFloat(a, b) => printer.fmt_cast("#UnsignedToFloat", a, b, f),
            Builtin::FloatToSigned(a, b) => printer.fmt_cast("#FloatToSigned", a, b, f),
            Builtin::FloatToUnsigned(a, b) => printer.fmt_cast("#FloatToUnsigned", a, b, f),
            Builtin::FloatPromote(a) => printer.fmt_call("#FloatPromote", &[a], f),
            Builtin::FloatDemote(a) => printer.fmt_call("#FloatDemote", &[a], f),
            Builtin::BitwiseAnd(a, b) => printer.fmt_call("#BitwiseAnd", &[a, b], f),
            Builtin::BitwiseOr(a, b) => printer.fmt_call("#BitwiseOr", &[a, b], f),
            Builtin::BitwiseXor(a, b) => printer.fmt_call("#BitwiseXor", &[a, b], f),
            Builtin::BitwiseNot(a) => printer.fmt_call("#BitwiseNot", &[a], f),
            Builtin::Truncate(a, b) => printer.fmt_cast("#Truncate", a, b, f),
            Builtin::Deref(a, b) => printer.fmt_cast("#Deref", a, b, f),
            Builtin::Offset(a, b, size) => printer.fmt_offset(a, b, *size, f),
            Builtin::Transmute(a, b) => printer.fmt_cast("#Transmute", a, b, f),
            Builtin::StackAlloc(value) => printer.fmt_call("#StackAlloc", &[value], f),
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
            DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
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
