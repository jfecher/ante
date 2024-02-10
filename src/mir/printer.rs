use std::fmt::{Display, Formatter, Result};

use crate::hir::Type;

use super::ir::{Mir, Ast, Atom, Variable, self};

#[derive(Default)]
struct Printer {
    indent_level: u32,
    display_types: bool,
}

impl Display for Mir {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut printer = Printer::default();
        for (id, (name, function)) in &self.functions {
            write!(f, "letrec {}{} = ", name, id)?;
            printer.display_ast_try_one_line(function, f)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Display for Ast {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Printer::default().display_ast(self, f)
    }
}

impl Display for Atom {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Printer::default().display_atom(self, f)
    }
}

impl Display for ir::Lambda {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Printer::default().display_lambda(self, f)
    }
}

impl<'ast> Printer {
    fn display_in_block(&mut self, ast: &'ast Ast, f: &mut Formatter) -> Result {
        self.indent_level += 1;
        self.display_ast(ast, f)?;
        self.indent_level -= 1;
        Ok(())
    }

    fn next_line(&self, f: &mut Formatter) -> Result {
        writeln!(f)?;
        for _ in 0 .. self.indent_level {
            write!(f, "  ")?;
        }
        Ok(())
    }

    fn display_ast(&mut self, ast: &'ast Ast, f: &mut Formatter) -> Result {
        self.next_line(f)?;
        self.display_ast_no_newline(ast, f)
    }

    fn display_ast_no_newline(&mut self, ast: &'ast Ast, f: &mut Formatter) -> Result {
        match ast {
            Ast::Atom(atom) => self.display_atom(atom, f),
            Ast::FunctionCall(call) => self.display_call(call, f),
            Ast::Let(let_) => self.display_let(let_, false, f),
            Ast::LetRec(let_) => self.display_let(let_, true, f),
            Ast::If(if_) => self.display_if(if_, f),
            Ast::Match(match_) => self.display_match(match_, f),
            Ast::Return(return_) => self.display_return(return_, f),
            Ast::Assignment(assign) => self.display_assign(assign, f),
            Ast::MemberAccess(access) => self.display_access(access, f),
            Ast::Tuple(tuple) => self.display_tuple(tuple, f),
            Ast::Builtin(builtin) => self.display_builtin(builtin, f),
            Ast::Handle(handle) => self.display_handle(handle, f),
        }
    }

    fn display_ast_try_one_line(&mut self, ast: &'ast Ast, f: &mut Formatter) -> Result {
        match ast {
            Ast::Let(_) => self.display_in_block(ast, f),
            other => self.display_ast_no_newline(other, f),
        }
    }

    fn display_decision_tree_in_block(&mut self, tree: &'ast ir::DecisionTree, f: &mut Formatter) -> Result {
        self.indent_level += 1;
        self.display_decision_tree(tree, f)?;
        self.indent_level -= 1;
        Ok(())
    }

    fn finish_block_with(&mut self, end: &str, ast: &Ast, f: &mut Formatter) -> Result {
        match ast {
            Ast::Let(_) => {
                self.next_line(f)?;
                write!(f, "{end}")
            },
            _ => write!(f, " {end}"),
        }
    }

    fn display_atom(&mut self, atom: &'ast Atom, f: &mut Formatter) -> Result {
        match atom {
            Atom::Literal(literal) => literal.fmt(f),
            Atom::Variable(variable) => variable.fmt(f),
            Atom::Lambda(lambda) => self.display_lambda(lambda, f),
            Atom::Extern(extern_) => extern_.fmt(f),
            Atom::Effect(effect) => effect.fmt(f),
        }
    }

    fn display_lambda(&mut self, lambda: &'ast ir::Lambda, f: &mut Formatter) -> Result {
        let start = if lambda.compile_time { "(\\" } else { "(fn" };
        write!(f, "{start}")?;

        for (i, arg) in lambda.args.iter().enumerate() {
            if self.display_types {
                let arg_type = lambda.typ.parameters.get(i).map(ToString::to_string).unwrap_or_else(|| "?".into());
                write!(f, " ({arg}: {arg_type})")?;
            } else {
                write!(f, " {arg}")?;
            }
        }

        if lambda.typ.parameters.len() > lambda.args.len() {
            let difference = lambda.typ.parameters.len() - lambda.args.len();
            write!(f, " !!! and {} more", difference)?;
        }

        if self.display_types {
            write!(f, " : {}", lambda.typ.return_type)?;
        }

        let arrow = if lambda.compile_time { "=>" } else { "->" };
        write!(f, " {arrow} ")?;

        self.display_in_block(&lambda.body, f)?;

        match lambda.body.as_ref() {
            Ast::Atom(_) => write!(f, ")"),
            _ => {
                self.next_line(f)?;
                write!(f, ")")
            },
        }
    }

    fn display_call(&mut self, call: &'ast ir::FunctionCall, f: &mut Formatter) -> Result {
        self.display_atom(&call.function, f)?;

        if call.compile_time {
            write!(f, " @")?;
        }

        for arg in &call.args {
            write!(f, " ")?;
            self.display_atom(arg, f)?;
        }

        Ok(())
    }

    fn display_let(&mut self, let_: &'ast ir::Let<Ast>, recursive: bool, f: &mut Formatter) -> Result {
        let rec = if recursive { "rec " } else { "" };
        write!(f, "{rec}{}{}", let_.name, let_.variable)?;

        if self.display_types {
            write!(f, ": {}", let_.typ)?;
        }

        write!(f, " = ")?;
        self.display_ast_try_one_line(&let_.expr, f)?;
        self.display_ast(&let_.body, f)
    }

    fn display_if(&mut self, if_: &'ast ir::If, f: &mut Formatter) -> Result {
        write!(f, "if ")?;
        self.display_atom(&if_.condition, f)?;
        write!(f, " : (if_type {}) then ", if_.result_type)?;
        self.display_ast_try_one_line(&if_.then, f)?;
        self.finish_block_with("else ", &if_.then, f)?;
        self.display_ast_try_one_line(&if_.otherwise, f)?;
        self.finish_block_with("endif", &if_.otherwise, f)
    }

    fn display_match(&mut self, match_: &'ast ir::Match, f: &mut Formatter) -> Result {
        self.display_decision_tree_no_newline(&match_.decision_tree, f)?;

        for (i, branch) in match_.branches.iter().enumerate() {
            self.next_line(f)?;
            write!(f, "branch {i} -> ")?;
            self.indent_level += 1;
            self.display_ast_try_one_line(branch, f)?;
            self.indent_level -= 1;
        }
        Ok(())
    }

    fn display_decision_tree(&mut self, tree: &'ast ir::DecisionTree, f: &mut Formatter) -> Result {
        self.next_line(f)?;
        self.display_decision_tree_no_newline(tree, f)
    }

    fn display_decision_tree_no_newline(&mut self, tree: &'ast ir::DecisionTree, f: &mut Formatter) -> Result {
        match tree {
            ir::DecisionTree::Leaf(index) => write!(f, "branch {index}"),
            ir::DecisionTree::Let(let_) => {
                write!(f, "let {}{}: {} = ", let_.name, let_.variable, let_.typ)?;
                self.display_ast_try_one_line(&let_.expr, f)?;
                self.display_decision_tree(&let_.body, f)
            },
            ir::DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
                write!(f, "switch ")?;
                self.display_atom(int_to_switch_on, f)?;

                for (i, case) in cases {
                    self.next_line(f)?;
                    write!(f, "case {i} -> ")?;
                    self.display_decision_tree_in_block(case, f)?;
                }

                if let Some(branch) = else_case {
                    self.next_line(f)?;
                    write!(f, "case _ -> ")?;
                    self.display_decision_tree_in_block(branch, f)?;
                }

                Ok(())
            },
        }
    }

    fn display_return(&mut self, return_: &'ast ir::Return, f: &mut Formatter) -> Result {
        write!(f, "return ")?;
        self.display_atom(&return_.expression, f)
    }

    fn display_assign(&mut self, assign: &'ast ir::Assignment, f: &mut Formatter) -> Result {
        self.display_atom(&assign.lhs, f)?;
        write!(f, " := ")?;
        self.display_atom(&assign.rhs, f)
    }

    fn display_access(&mut self, access: &'ast ir::MemberAccess, f: &mut Formatter) -> Result {
        self.display_atom(&access.lhs, f)?;
        write!(f, ".{}", access.member_index)
    }

    fn display_tuple(&mut self, tuple: &'ast ir::Tuple, f: &mut Formatter) -> Result {
        write!(f, "(")?;

        for (i, field) in tuple.fields.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            self.display_atom(field, f)?;
        }

        write!(f, ")")
    }

    fn display_builtin(&mut self, builtin: &'ast ir::Builtin, f: &mut Formatter) -> Result {
        let display = |this: &mut Self, f: &mut Formatter, name: &str, items: &[&'ast Atom]| {
            if items.len() == 1 {
                write!(f, "{name} ")?;
                this.display_atom(items[0], f)
            } else {
                assert_eq!(items.len(), 2);
                this.display_atom(items[0], f)?;
                write!(f, " {name} ")?;
                this.display_atom(items[1], f)
            }
        };

        let display_with_type = |this: &mut Self, f: &mut Formatter, name: &str, item: &'ast Atom, typ: &Type| {
            write!(f, "{name} ")?;
            this.display_atom(item, f)?;
            write!(f, " : {typ}")
        };

        match builtin {
            ir::Builtin::AddInt(lhs, rhs) => display(self, f, "+", &[lhs, rhs]),
            ir::Builtin::AddFloat(lhs, rhs) => display(self, f, "+.", &[lhs, rhs]),
            ir::Builtin::SubInt(lhs, rhs) => display(self, f, "-", &[lhs, rhs]),
            ir::Builtin::SubFloat(lhs, rhs) => display(self, f, "-.", &[lhs, rhs]),
            ir::Builtin::MulInt(lhs, rhs) => display(self, f, "*", &[lhs, rhs]),
            ir::Builtin::MulFloat(lhs, rhs) => display(self, f, "*.", &[lhs, rhs]),
            ir::Builtin::DivSigned(lhs, rhs) => display(self, f, "/s", &[lhs, rhs]),
            ir::Builtin::DivUnsigned(lhs, rhs) => display(self, f, "/u", &[lhs, rhs]),
            ir::Builtin::DivFloat(lhs, rhs) => display(self, f, "/.", &[lhs, rhs]),
            ir::Builtin::ModSigned(lhs, rhs) => display(self, f, "%s", &[lhs, rhs]),
            ir::Builtin::ModUnsigned(lhs, rhs) => display(self, f, "%u", &[lhs, rhs]),
            ir::Builtin::ModFloat(lhs, rhs) => display(self, f, "%.", &[lhs, rhs]),
            ir::Builtin::LessSigned(lhs, rhs) => display(self, f, "<s", &[lhs, rhs]),
            ir::Builtin::LessUnsigned(lhs, rhs) => display(self, f, "<u", &[lhs, rhs]),
            ir::Builtin::LessFloat(lhs, rhs) => display(self, f, "<.", &[lhs, rhs]),
            ir::Builtin::EqInt(lhs, rhs) => display(self, f, "==", &[lhs, rhs]),
            ir::Builtin::EqFloat(lhs, rhs) => display(self, f, "==.", &[lhs, rhs]),
            ir::Builtin::EqChar(lhs, rhs) => display(self, f, "==c", &[lhs, rhs]),
            ir::Builtin::EqBool(lhs, rhs) => display(self, f, "==b", &[lhs, rhs]),
            ir::Builtin::SignExtend(lhs, typ) => display_with_type(self, f, "sign_extend", lhs, typ),
            ir::Builtin::ZeroExtend(lhs, typ) => display_with_type(self, f, "zero_extend", lhs, typ),
            ir::Builtin::SignedToFloat(lhs, typ) => display_with_type(self, f, "signed_to_float", lhs, typ),
            ir::Builtin::UnsignedToFloat(lhs, typ) => display_with_type(self, f, "unsigned_to_float", lhs, typ),
            ir::Builtin::FloatToSigned(lhs, typ) => display_with_type(self, f, "float_to_signed", lhs, typ),
            ir::Builtin::FloatToUnsigned(lhs, typ) => display_with_type(self, f, "float_to_unsigned", lhs, typ),
            ir::Builtin::FloatPromote(lhs, typ) => display_with_type(self, f, "float_promote", lhs, typ),
            ir::Builtin::FloatDemote(lhs, typ) => display_with_type(self, f, "float_demote", lhs, typ),
            ir::Builtin::BitwiseAnd(lhs, rhs) => display(self, f, "bitwise_and", &[lhs, rhs]),
            ir::Builtin::BitwiseOr(lhs, rhs) => display(self, f, "bitwise_or", &[lhs, rhs]),
            ir::Builtin::BitwiseXor(lhs, rhs) => display(self, f, "bitwise_xor", &[lhs, rhs]),
            ir::Builtin::BitwiseNot(lhs) => display(self, f, "bitwise_not", &[lhs]),
            ir::Builtin::Truncate(lhs, typ) => display_with_type(self, f, "truncate", lhs, typ),
            ir::Builtin::Deref(lhs, typ) => display_with_type(self, f, "deref", lhs, typ),
            ir::Builtin::Offset(lhs, rhs, _) => display(self, f, "offset", &[lhs, rhs]),
            ir::Builtin::Transmute(lhs, typ) => display_with_type(self, f, "transmute", lhs, typ),
            ir::Builtin::StackAlloc(lhs) => display(self, f, "stack_allocate", &[lhs]),
        }
    }

    fn display_handle(&mut self, handle: &'ast ir::Handle, f: &mut Formatter) -> Result {
        write!(f, "handle ")?;
        self.display_ast_try_one_line(&handle.expression, f)?;

        self.next_line(f)?;
        write!(f, "| {}", handle.effect)?;

        for arg in &handle.branch_args {
            write!(f, " {arg}")?;
        }
        write!(f, " {} ->", handle.resume)?;

        self.display_ast_try_one_line(&handle.branch_body, f)
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}{}", self.name, self.definition_id.0)
    }
}
