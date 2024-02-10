//! This file implements the pass to convert Hir into the initial Mir in 
//! capability passing style (not to be confused with continuation passing style).
//!
//! This is done following the "Translation of Statements" and "Translation of Expressions"
//! algorithms in https://se.cs.uni-tuebingen.de/publications/schuster19zero.pdf.
//!
//! Since Ante does not distinguish between expressions and statements however, both
//! the `E` and `S` functions in the paper correspond to the `cps_ast` function in this
//! file. Additionally, since expressions in Ante may themselves contain expressions that
//! the paper considers to be statements, almost all functions take an `EffectStack` parameter
//! instead of just functions operating on statement nodes.
//!
//! In addition to implementations for `E` and `S`, this file also implements `H` to
//! convert effect handlers. In this file, it is named `convert_effect`. Implementations
//! for `T` and `C` for converting types can be found in `src/mir/context.rs`.
//!
//! Where possible, functions in this file will document their corresponding case from the
//! paper, although since Ante is a larger language, many do not have corresponding cases.
//! Additionally, there are some notation changes from the linked paper as well:
//!
//! - Subscript arguments are converted into normal function arguments. E.g. `S(e)_ts` -> `S(e, ts)`.
//! - Since color cannot be used in doc comments, a different notation is used to distinguish
//!   compile-time terms from runtime terms:
//!
//!   - For function types, a runtime function type is denoted by `a -> b` where a
//!     compile-time function type uses `a => b`.
//!
//!   - For lambda values, `fn x -> e` is runtime, and `fn x => e` is a compile-time abstraction.
//!
//!   - For function calls, `f @ x` is runtime, and  `f @@ x` is a compile-time call.
//!
//!   - For the `C` function, an extra boolean parameter is added. This parameter is `true` if 
//!     `C` refers to the compile-time `C` rather than the runtime version. This parameter is in
//!     addition to the change of making the subscript effect stack a parameter to `C` as well.
//!     So a call to (red) `C[t]_ts` will translate to `C(t, ts, false)`, and a call to (blue)
//!     `C[t]_ts` will translate to `C(t, ts, true)`
//!
//!   Unless the term falls into one of the above cases, it is considered to be a runtime term.
use std::collections::{HashSet, VecDeque, BTreeMap};
use std::rc::Rc;

use crate::hir::{ self, Typed };
use crate::util::fmap;

#[macro_use]
mod ir;
mod evaluate;
mod cps;
mod printer;

pub use ir::*;

pub fn convert_to_mir(hir: hir::Ast, next_id: usize) -> ir::Mir {
    let mut context = Context::new(next_id);
    let main = hir.to_block(&mut context, Type::unit());

    let main_id = context.next_id();
    let mut functions = BTreeMap::new();
    functions.insert(main_id, (Rc::new("main".into()), main));

    let mut mir = ir::Mir { main: main_id, functions, next_id };

    while let Some(next_global) = context.definition_queue.pop_front() {
        let ast = next_global.definition.as_ref().unwrap();
        let ast = match ast.as_ref() { 
            hir::Ast::Definition(definition) => definition.expr.as_ref(),
            other => other,
        };
        let name = context.get_name(&next_global.name);
        let result = (name, ast.to_mir(&mut context));
        mir.functions.insert(next_global.definition_id, result);
    }

    mir.next_id = context.next_id;
    mir
}

struct Context {
    definition_queue: VecDeque<hir::Variable>,

    /// The set of already translated IDs
    translated: HashSet<DefinitionId>,

    /// The default name to give any variables or definitions with no name
    default_name: Rc<String>,

    /// Each time an ir::Ast needs to be converted to an Atom, we push a local
    /// definition here containing the Ast and a fresh DefinitionId, and return
    /// the DefinitionId as the atom. At the end of a function, these area all
    /// collected together in one Let binding for each.
    local_definitions: Vec<(DefinitionId, Rc<String>, Rc<Type>, ir::Ast)>,

    next_id: usize,
}

impl Context {
    fn new(next_id: usize) -> Self {
        Self {
            definition_queue: VecDeque::new(),
            translated: HashSet::new(),
            default_name: Rc::new("_".into()),
            local_definitions: Vec::new(),
            next_id,
        }
    }

    /// Returns the given name, or a default name if `name` is `None`.
    fn get_name(&self, name: &Option<String>) -> Rc<String> {
        match name {
            Some(name) => Rc::new(name.clone()),
            None => self.default_name.clone(),
        }
    }

    /// Convert a hir::Variable to a mir::ir::Variable.
    /// This will not add the variable's definition_id to `self.definition_queue`.
    fn convert_variable(&self, variable: &hir::Variable) -> ir::Variable {
        ir::Variable {
            definition_id: variable.definition_id,
            typ: variable.typ.clone(),
            name: self.get_name(&variable.name),
        }
    }

    fn next_id(&mut self) -> DefinitionId {
        let id = self.next_id;
        self.next_id += 1;
        DefinitionId(id)
    }

    /// Push a local definition to create a Let binding later,
    /// and return a variable referencing the new definition.
    fn push_local_definition(&mut self, id: DefinitionId, name: Option<String>, ir: ir::Ast, typ: Type) -> Atom {
        let name = name.map_or_else(|| self.default_name.clone(), Rc::new);
        let typ = Rc::new(typ);

        self.local_definitions.push((id, name.clone(), typ.clone(), ir));
        Atom::Variable(ir::Variable { definition_id: id, name, typ })
    }

    /// Finish the current block, collecting all the previous local definitions
    /// into a let binding for each to properly sequence each statement.
    ///
    /// If `return_type` is set, this will wrap the last expression in a `return`
    /// node of the given type.
    fn finish_block(&mut self, last_expression: ir::Ast, typ: Type) -> ir::Ast {
        let mut result = match last_expression {
            ir::Ast::Atom(expression) => ir::Ast::Return(ir::Return { expression, typ }),
            ir::Ast::Return(return_expr) => ir::Ast::Return(return_expr),
            other => {
                let fresh_id = self.next_id();
                let name = self.default_name.clone();
                let rc_type = Rc::new(typ.clone());
                self.local_definitions.push((fresh_id, name, rc_type.clone(), other));
                let expression = Atom::Variable(ir::Variable {
                    definition_id: fresh_id,
                    typ: rc_type,
                    name: self.default_name.clone(),
                });
                ir::Ast::Return(ir::Return { expression, typ })
            }
        };

        let definitions = std::mem::take(&mut self.local_definitions);

        for (variable, name, typ, definition_rhs) in definitions.into_iter().rev() {
            let expr = Box::new(definition_rhs);
            let body = Box::new(result);
            result = ir::Ast::Let(ir::Let { variable, name, expr, body, typ });
        }

        result
    }

    /// Convert a Builtin::Transmute or ReinterpretCast to a transmute instruction.
    /// Contains the minor optimization that `transmute x as T == x` iff `x: T`
    fn convert_transmute(&mut self, transmute_lhs: &hir::Ast, typ: Type) -> Ast {
        let lhs = transmute_lhs.to_atom(self, typ.clone());

        if transmute_lhs.get_type() == typ {
            Ast::Atom(lhs)
        } else {
            ir::Ast::Builtin(ir::Builtin::Transmute(lhs, typ))
        }
    }
}

trait ToMir {
    fn to_mir(&self, context: &mut Context) -> ir::Ast;

    fn to_atom(&self, context: &mut Context, typ: Type) -> ir::Atom {
        match self.to_mir(context) {
            ir::Ast::Atom(atom) => atom,
            other => {
                let id = context.next_id();
                context.push_local_definition(id, None, other, typ)
            }
        }
    }

    /// Translate a block of statements. This is preferred when a ir::Ast
    /// is needed over the more primitive `to_mir` since `to_block` will also
    /// collect all the `local_definitions` into let bindings.
    fn to_block(&self, context: &mut Context, typ: Type) -> ir::Ast {
        let old_local_definitions = std::mem::take(&mut context.local_definitions);

        let block = self.to_mir(context);
        let block = context.finish_block(block, typ);

        context.local_definitions = old_local_definitions;
        block
    }
}

impl ToMir for hir::Ast {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        dispatch_on_hir!(self, ToMir::to_mir, context)
    }
}

impl ToMir for hir::Literal {
    fn to_mir(&self, _: &mut Context) -> ir::Ast {
        ir::Ast::Atom(Atom::Literal(self.clone()))
    }
}

impl ToMir for hir::Variable {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        if !context.translated.contains(&self.definition_id) {
            context.translated.insert(self.definition_id);
            context.definition_queue.push_back(self.clone());
        }

        ir::Ast::Atom(Atom::Variable(context.convert_variable(self)))
    }
}

impl ToMir for Rc<hir::Lambda> {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let args = fmap(&self.args, |arg| {
            context.translated.insert(arg.definition_id);
            context.convert_variable(arg)
        });

        let body_type = self.typ.return_type.as_ref().clone();
        let body = Box::new(self.body.to_block(context, body_type));

        let typ = self.typ.clone();
        ir::Ast::Atom(Atom::Lambda(ir::Lambda { args, body, typ, compile_time: false }))
    }
}

impl ToMir for hir::FunctionCall {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let function = self.function.to_atom(context, Type::Function(self.function_type.clone()));

        let args = fmap(self.args.iter().zip(&self.function_type.parameters), |(arg, typ)| {
            arg.to_atom(context, typ.clone())
        });

        let function_type = self.function_type.clone();
        ir::Ast::FunctionCall(ir::FunctionCall { function, args, function_type, compile_time: false })
    }
}

impl ToMir for hir::Definition {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        context.translated.insert(self.variable);
        let rhs = self.expr.to_mir(context);
        let name = self.name.clone();
        ir::Ast::Atom(context.push_local_definition(self.variable, name, rhs, self.typ.clone()))
    }
}

impl ToMir for hir::If {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let condition = self.condition.to_atom(context, Type::Primitive(PrimitiveType::Boolean));

        let then = Box::new(self.then.to_block(context, self.result_type.clone()));
        let otherwise = Box::new(self.otherwise.to_block(context, self.result_type.clone()));

        let result_type = self.result_type.clone();
        ir::Ast::If(ir::If { condition, then, otherwise, result_type })
    }
}

impl ToMir for hir::Match {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let decision_tree = decision_tree_to_mir(&self.decision_tree, context);
        let result_type = self.result_type.clone();

        // Optimization: if the match is a single case we can remove it entirely
        if let DecisionTree::Leaf(index) = &decision_tree {
            return self.branches[*index].to_block(context, result_type);
        }

        let branches = fmap(&self.branches, |branch| branch.to_block(context, result_type.clone()));
        ir::Ast::Match(ir::Match { branches, decision_tree, result_type })
    }
}

fn decision_tree_to_mir(tree: &hir::DecisionTree, context: &mut Context) -> ir::DecisionTree {
    match tree {
        hir::DecisionTree::Leaf(leaf_index) => ir::DecisionTree::Leaf(*leaf_index),
        hir::DecisionTree::Definition(definition, rest) => {
            let variable = definition.variable;
            context.translated.insert(variable);
            let name = context.get_name(&definition.name);
            let expr = Box::new(definition.expr.to_block(context, definition.typ.clone()));
            let body = Box::new(decision_tree_to_mir(rest, context));
            let typ = Rc::new(definition.typ.clone());
            ir::DecisionTree::Let(ir::Let { variable, name, expr, body, typ })
        },
        hir::DecisionTree::Switch { int_to_switch_on, cases, else_case } => {
            let int_to_switch_on = int_to_switch_on.to_atom(context, Type::tag_type());
            let cases = fmap(cases, |(tag, case)| (*tag, decision_tree_to_mir(case, context)));
            let else_case = else_case.as_ref().map(|case| Box::new(decision_tree_to_mir(case, context)));
            ir::DecisionTree::Switch { int_to_switch_on, cases, else_case }
        },
    }
}

impl ToMir for hir::Return {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let expression = self.expression.to_atom(context, self.typ.clone());
        let typ = self.typ.clone();
        ir::Ast::Return(ir::Return { expression, typ })
    }
}

impl ToMir for hir::Sequence {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let exclude_last_element = self.statements.len().saturating_sub(1);
        let first_statements = &self.statements[..exclude_last_element];

        for statement in first_statements {
            statement.to_atom(context, statement.get_type());
        }

        match self.statements.last() {
            Some(last) => last.to_mir(context),
            None => ir::Ast::Atom(Atom::Literal(hir::Literal::Unit)),
        }
    }
}

impl ToMir for hir::Extern {
    fn to_mir(&self, _: &mut Context) -> ir::Ast {
        ir::Ast::Atom(Atom::Extern(self.clone()))
    }
}

impl ToMir for hir::Assignment {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let lhs = self.lhs.to_atom(context, self.lhs.get_type());
        let rhs = self.rhs.to_atom(context, self.rhs.get_type());
        ir::Ast::Assignment(ir::Assignment { lhs, rhs })
    }
}

impl ToMir for hir::MemberAccess {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let lhs = self.lhs.to_atom(context, self.lhs.get_type());
        let typ = self.typ.clone();
        ir::Ast::MemberAccess(ir::MemberAccess { lhs, typ, member_index: self.member_index })
    }
}

impl ToMir for hir::Tuple {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let fields = fmap(&self.fields, |field| field.to_atom(context, field.get_type()));
        ir::Ast::Tuple(ir::Tuple { fields })
    }
}

impl ToMir for hir::ReinterpretCast {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        context.convert_transmute(&self.lhs, self.target_type.clone())
    }
}

impl ToMir for hir::Builtin {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        let both = |context: &mut _, f: fn(_, _) -> _, lhs: &hir::Ast, rhs: &hir::Ast| {
            let lhs = lhs.to_atom(context, lhs.get_type());
            let rhs = rhs.to_atom(context, rhs.get_type());
            ir::Ast::Builtin(f(lhs, rhs))
        };

        let one_with_type = |context, f: fn(_, _) -> _, lhs: &hir::Ast, typ: &Type| {
            let lhs = lhs.to_atom(context, lhs.get_type());
            ir::Ast::Builtin(f(lhs, typ.clone()))
        };

        let one = |context, f: fn(_) -> _, lhs: &hir::Ast| {
            let lhs = lhs.to_atom(context, lhs.get_type());
            ir::Ast::Builtin(f(lhs))
        };

        match self {
            hir::Builtin::AddInt(lhs, rhs) => both(context, ir::Builtin::AddInt, lhs, rhs),
            hir::Builtin::AddFloat(lhs, rhs) => both(context, ir::Builtin::AddFloat, lhs, rhs),
            hir::Builtin::SubInt(lhs, rhs) => both(context, ir::Builtin::SubInt, lhs, rhs),
            hir::Builtin::SubFloat(lhs, rhs) => both(context, ir::Builtin::SubFloat, lhs, rhs),
            hir::Builtin::MulInt(lhs, rhs) => both(context, ir::Builtin::MulInt, lhs, rhs),
            hir::Builtin::MulFloat(lhs, rhs) => both(context, ir::Builtin::MulFloat, lhs, rhs),
            hir::Builtin::DivSigned(lhs, rhs) => both(context, ir::Builtin::DivSigned, lhs, rhs),
            hir::Builtin::DivUnsigned(lhs, rhs) => both(context, ir::Builtin::DivUnsigned, lhs, rhs),
            hir::Builtin::DivFloat(lhs, rhs) => both(context, ir::Builtin::DivFloat, lhs, rhs),
            hir::Builtin::ModSigned(lhs, rhs) => both(context, ir::Builtin::ModSigned, lhs, rhs),
            hir::Builtin::ModUnsigned(lhs, rhs) => both(context, ir::Builtin::ModUnsigned, lhs, rhs),
            hir::Builtin::ModFloat(lhs, rhs) => both(context, ir::Builtin::ModFloat, lhs, rhs),
            hir::Builtin::LessSigned(lhs, rhs) => both(context, ir::Builtin::LessSigned, lhs, rhs),
            hir::Builtin::LessUnsigned(lhs, rhs) => both(context, ir::Builtin::LessUnsigned, lhs, rhs),
            hir::Builtin::LessFloat(lhs, rhs) => both(context, ir::Builtin::LessFloat, lhs, rhs),
            hir::Builtin::EqInt(lhs, rhs) => both(context, ir::Builtin::EqInt, lhs, rhs),
            hir::Builtin::EqFloat(lhs, rhs) => both(context, ir::Builtin::EqFloat, lhs, rhs),
            hir::Builtin::EqChar(lhs, rhs) => both(context, ir::Builtin::EqChar, lhs, rhs),
            hir::Builtin::EqBool(lhs, rhs) => both(context, ir::Builtin::EqBool, lhs, rhs),
            hir::Builtin::SignExtend(lhs, rhs) => one_with_type(context, ir::Builtin::SignExtend, lhs, rhs),
            hir::Builtin::ZeroExtend(lhs, rhs) => one_with_type(context, ir::Builtin::ZeroExtend, lhs, rhs),
            hir::Builtin::SignedToFloat(lhs, rhs) => one_with_type(context, ir::Builtin::SignedToFloat, lhs, rhs),
            hir::Builtin::UnsignedToFloat(lhs, rhs) => one_with_type(context, ir::Builtin::UnsignedToFloat, lhs, rhs),
            hir::Builtin::FloatToSigned(lhs, rhs) => one_with_type(context, ir::Builtin::FloatToSigned, lhs, rhs),
            hir::Builtin::FloatToUnsigned(lhs, rhs) => one_with_type(context, ir::Builtin::FloatToUnsigned, lhs, rhs),
            hir::Builtin::FloatPromote(lhs, rhs) => one_with_type(context, ir::Builtin::FloatPromote, lhs, rhs),
            hir::Builtin::FloatDemote(lhs, rhs) => one_with_type(context, ir::Builtin::FloatDemote, lhs, rhs),
            hir::Builtin::BitwiseAnd(lhs, rhs) => both(context, ir::Builtin::BitwiseAnd, lhs, rhs),
            hir::Builtin::BitwiseOr(lhs, rhs) => both(context, ir::Builtin::BitwiseOr, lhs, rhs),
            hir::Builtin::BitwiseXor(lhs, rhs) => both(context, ir::Builtin::BitwiseXor, lhs, rhs),
            hir::Builtin::BitwiseNot(lhs) => one(context, ir::Builtin::BitwiseNot, lhs),
            hir::Builtin::Truncate(lhs, rhs) => one_with_type(context, ir::Builtin::Truncate, lhs, rhs),
            hir::Builtin::Deref(lhs, rhs) => one_with_type(context, ir::Builtin::Deref, lhs, rhs),
            hir::Builtin::StackAlloc(lhs) => one(context, ir::Builtin::StackAlloc, lhs),
            hir::Builtin::Transmute(lhs, rhs) => context.convert_transmute(lhs, rhs.clone()),
            hir::Builtin::Offset(lhs, rhs, typ) => {
                let lhs = lhs.to_atom(context, lhs.get_type());
                let rhs = rhs.to_atom(context, rhs.get_type());
                ir::Ast::Builtin(ir::Builtin::Offset(lhs, rhs, typ.clone()))
            },
        }
    }
}

impl ToMir for hir::Effect {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        context.translated.insert(self.id);
        ir::Ast::Atom(Atom::Effect(self.clone()))
    }
}

impl ToMir for hir::Handle {
    fn to_mir(&self, context: &mut Context) -> ir::Ast {
        context.translated.insert(self.resume.definition_id);
        let expression = Box::new(self.expression.to_block(context, self.result_type.clone()));

        let branch_args = fmap(&self.branch_body.args, |arg| {
            context.translated.insert(arg.definition_id);
            context.convert_variable(arg)
        });

        let branch_body = Box::new(self.branch_body.body.to_block(context, self.result_type.clone()));

        ir::Ast::Handle(ir::Handle {
            expression,
            effect: self.effect.clone(),
            resume: context.convert_variable(&self.resume),
            result_type: self.result_type.clone(),
            branch_args,
            branch_body,
        })
    }
}
