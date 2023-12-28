//! This MIR interpreter is used for debugging MIR programs which otherwise
//! can be difficult to follow since they are in CPS form.
use std::collections::HashMap;

use crate::{util::fmap, hir::{Literal, IntegerKind, PrimitiveType}, lexer::token::FloatKind};

use super::ir::{Mir, Expr, ParameterId, FunctionId, Type};

impl Mir {
    #[allow(unused)]
    pub fn interpret(&self) {
        let mut interpreter = Interpreter::new(self);
        let mut i = 0;

        while !interpreter.done && i < 100 {
            interpreter.call_current_function();

            let arg_tys = fmap(&interpreter.current_args, |arg| arg.approx_type(self));
            let function = &interpreter.mir.functions[&interpreter.current_function];
            // let params = &function.argument_types;

            // if params.len() != arg_tys.len() {
            //     eprintln!("  WARNING: Call to function {} with {} args when it takes {} params", 
            //               interpreter.current_function, arg_tys.len(), params.len());
            // }
            
            // for (i, (param, arg)) in function.argument_types.iter().zip(arg_tys).enumerate() {
            //     if let Some(arg) = arg {
            //         if *param != arg {
            //             eprintln!("  WARNING: In function call to {}, parameter {} : {} where the argument : {}",
            //                     interpreter.current_function, i, param, arg);
            //         }
            //     }
            // }

            i += 1;
        }

        if i >= 100 {
            eprintln!("i = 100, early exit");
        }
    }
}

struct Interpreter<'mir> {
    mir: &'mir Mir,
    memory: HashMap<ParameterId, Expr>,
    done: bool,
    current_function: FunctionId,
    current_args: Vec<Expr>,
}

impl<'mir> Interpreter<'mir> {
    fn new(mir: &'mir Mir) -> Self {
        Self {
            mir,
            memory: HashMap::new(),
            done: false,
            current_function: Mir::main_id(),
            current_args: Vec::new(),
        }
    }

    fn define(&mut self, parameter: ParameterId, value: Expr) {
        let value = match value {
            Expr::Parameter(value_param_id) => self.memory[&value_param_id].clone(),
            other => other,
        };
        eprintln!("{} <- {}", parameter, value);
        self.memory.insert(parameter, value);
    }

    fn call_current_function(&mut self) {
        let function = &self.mir.functions[&self.current_function];

        let args = std::mem::take(&mut self.current_args);
        for (parameter, arg) in function.parameters().zip(args) {
            self.define(parameter, arg);
        }

        // self.evaluate_expression(&function.body)
    }

    fn evaluate_expression(&mut self, body: &Expr) {
        match self.evaluate(body) {
            Expr::Function(_function_id) => (),
            Expr::If(condition, then, otherwise) => {
                let condition = self.evaluate(&condition);
                let then = self.evaluate(&then);
                let otherwise = self.evaluate(&otherwise);
                eprintln!("if {} then {} else {}", condition, then, otherwise);

                let k = if matches!(condition, Expr::Literal(Literal::Bool(true))) {
                    then
                } else {
                    otherwise
                };
                self.current_function = self.evaluate_function(&k);
                self.current_args.clear();
            },
            Expr::Switch(expr, cases, else_case) => {
                let int = self.evaluate_int(&expr).0;
                eprintln!("switch to case {}", int);

                if let Some((_, case_fn)) = cases.into_iter().find(|(case_int, _)| *case_int == int as u32) {
                    self.current_function = case_fn;
                } else {
                    self.current_function = else_case.unwrap();
                }
                self.current_args.clear();
            },
            Expr::Literal(Literal::Unit) => {
                // The program always ends in a call to ()
                self.done = true;
            }
            Expr::Assign => {
                eprintln!(":=");
            }
            other => unreachable!("evaluate_call_body expected function, found {}", other),
        }
    }

    fn evaluate(&mut self, atom: &Expr) -> Expr {
        match atom {
            Expr::If(..)
            | Expr::Switch(..)
            | Expr::Literal(_)
            | Expr::Assign
            | Expr::Extern(_)
            | Expr::Call(..)
            | Expr::Function(_) => atom.clone(),
            Expr::Parameter(parameter_id) => {
                self.memory.get(parameter_id)
                    .cloned()
                    .unwrap_or_else(|| panic!("In function {}, Parameter {} not defined!", self.current_function, parameter_id))
            }
            Expr::Tuple(fields) => Expr::Tuple(fmap(fields, |field| self.evaluate(field))),
            Expr::MemberAccess(tuple, index, _typ) => {
                match self.evaluate(tuple) {
                    Expr::Tuple(fields) => {
                        let result = fields[*index as usize].clone();
                        self.evaluate(&result)
                    }
                    other => unreachable!("Atom::MemberAccess expected tuple, found {}", other),
                }
            },
            Expr::AddInt(lhs, rhs) => self.int_function(lhs, rhs, "+", |a, b| a + b),
            Expr::AddFloat(_, _) => todo!(),
            Expr::SubInt(lhs, rhs) => self.int_function(lhs, rhs, "-", |a, b| a - b),
            Expr::SubFloat(_, _) => todo!(),
            Expr::MulInt(lhs, rhs) => self.int_function(lhs, rhs, "*", |a, b| a * b),
            Expr::MulFloat(_, _) => todo!(),
            Expr::DivSigned(lhs, rhs) => self.int_function(lhs, rhs, "/s", |a, b| a / b),
            Expr::DivUnsigned(lhs, rhs) => self.int_function(lhs, rhs, "/u", |a, b| a / b),
            Expr::DivFloat(_, _) => todo!(),
            Expr::ModSigned(lhs, rhs) => self.int_function(lhs, rhs, "%s", |a, b| a + b),
            Expr::ModUnsigned(lhs, rhs) => self.int_function(lhs, rhs, "%u", |a, b| a + b),
            Expr::ModFloat(_, _) => todo!(),
            Expr::LessSigned(lhs, rhs) => self.bool_function(lhs, rhs, "<s", |a, b| a < b),
            Expr::LessUnsigned(lhs, rhs) => self.bool_function(lhs, rhs, "<u", |a, b| a < b),
            Expr::LessFloat(_, _) => todo!(),
            Expr::EqInt(lhs, rhs) => self.bool_function(lhs, rhs, "==", |a, b| a == b),
            Expr::EqFloat(_, _) => todo!(),
            Expr::EqChar(_, _) => todo!(),
            Expr::EqBool(_, _) => todo!(),
            Expr::SignExtend(atom, _) => self.evaluate(atom),
            Expr::ZeroExtend(atom, _) => self.evaluate(atom),
            Expr::SignedToFloat(int, _typ) => {
                self.map_literal(int, |literal| match literal {
                    Literal::Integer(x, _kind) => Literal::Float((x as f64).to_bits(), FloatKind::F64),
                    other => unreachable!("signed_to_float expected int, found {}", other),
                })
            }
            Expr::UnsignedToFloat(int, _typ) => {
                self.map_literal(int, |literal| match literal {
                    Literal::Integer(x, _kind) => Literal::Float((x as f64).to_bits(), FloatKind::F64),
                    other => unreachable!("signed_to_float expected int, found {}", other),
                })
            },
            Expr::FloatToSigned(_, _) => todo!(),
            Expr::FloatToUnsigned(_, _) => todo!(),
            Expr::FloatPromote(_, _) => todo!(),
            Expr::FloatDemote(_, _) => todo!(),
            Expr::BitwiseAnd(_, _) => todo!(),
            Expr::BitwiseOr(_, _) => todo!(),
            Expr::BitwiseXor(_, _) => todo!(),
            Expr::BitwiseNot(_) => todo!(),
            Expr::Truncate(atom, _typ) => self.evaluate(atom),
            Expr::Deref(atom, _typ) => {
                match self.evaluate(atom) {
                    Expr::Tuple(mut values) => values.remove(0),
                    other => unreachable!("Atom::Deref expected Atom::Tuple, found {}", other),
                }
            },
            Expr::Offset(_, _, _) => todo!(),
            Expr::Transmute(_, _) => todo!(),
            Expr::StackAlloc(value) => {
                let value = self.evaluate(value);
                Expr::Tuple(vec![value]) // Use tuples to emulate memory for now
            },
        }
    }

    fn map_literal(&mut self, atom: &Expr, f: impl FnOnce(Literal) -> Literal) -> Expr {
        match self.evaluate(atom) {
            Expr::Literal(literal) => Expr::Literal(f(literal)),
            other => unreachable!("map_literal expected literal, found {}", other),
        }
    }

    fn int_function(&mut self, lhs: &Expr, rhs: &Expr, name: &str, f: impl FnOnce(u64, u64) -> u64) -> Expr {
        let (lhs, kind) = self.evaluate_int(lhs);
        let (rhs, _) = self.evaluate_int(rhs);
        let result = f(lhs, rhs);
        eprintln!("{} {} {} = {}", lhs, name, rhs, result);
        Expr::Literal(Literal::Integer(result, kind))
    }

    fn bool_function(&mut self, lhs: &Expr, rhs: &Expr, name: &str, f: impl FnOnce(u64, u64) -> bool) -> Expr {
        let (lhs, _) = self.evaluate_int(lhs);
        let (rhs, _) = self.evaluate_int(rhs);
        let result = f(lhs, rhs);
        eprintln!("{} {} {} = {}", lhs, name, rhs, result);
        Expr::Literal(Literal::Bool(result))
    }

    fn evaluate_int(&mut self, atom: &Expr) -> (u64, IntegerKind) {
        match self.evaluate(atom) {
            Expr::Literal(Literal::Integer(int, kind)) => (int, kind),
            other => unreachable!("evaluate_int expected int, found {}", other),
        }
    }

    fn evaluate_function(&mut self, atom: &Expr) -> FunctionId {
        match self.evaluate(atom) {
            Expr::Function(function_id) => function_id,
            other => unreachable!("evaluate_function expected function, found {}", other),
        }
    }
}

impl Expr {
    pub(crate) fn approx_type(&self, mir: &Mir) -> Option<Type> {
        match self {
            Expr::If(..) => None,
            Expr::Switch(..) => None,
            Expr::Call(..) => None,
            Expr::Literal(literal) => {
                match literal {
                    Literal::Integer(_, kind) => Some(Type::Primitive(PrimitiveType::Integer(*kind))),
                    Literal::Float(_, kind) => Some(Type::Primitive(PrimitiveType::Float(*kind))),
                    Literal::CString(_) => Some(Type::Primitive(PrimitiveType::Pointer)),
                    Literal::Char(_) => Some(Type::Primitive(PrimitiveType::Char)),
                    Literal::Bool(_) => Some(Type::Primitive(PrimitiveType::Boolean)),
                    Literal::Unit => Some(Type::Primitive(PrimitiveType::Unit)),
                }
            },
            Expr::Parameter(id) => {
                let function = &mir.functions[&id.function];
                None
                // Some(function.argument_types[id.parameter_index as usize].clone())
            },
            Expr::Function(id) => {
                let function = &mir.functions[id];
                None
                // Some(Type::Function(function.argument_types.clone()))
            },
            Expr::Tuple(fields) => {
                let fields = fields.iter().map(|field| field.approx_type(mir)).collect::<Option<Vec<_>>>()?;
                Some(Type::Tuple(fields))
            },
            Expr::MemberAccess(_, _, _) => None,
            Expr::Assign => None,
            Expr::Extern(_) => None,
            Expr::AddInt(_, _) => None,
            Expr::AddFloat(_, _) => None,
            Expr::SubInt(_, _) => None,
            Expr::SubFloat(_, _) => None,
            Expr::MulInt(_, _) => None,
            Expr::MulFloat(_, _) => None,
            Expr::DivSigned(_, _) => None,
            Expr::DivUnsigned(_, _) => None,
            Expr::DivFloat(_, _) => None,
            Expr::ModSigned(_, _) => None,
            Expr::ModUnsigned(_, _) => None,
            Expr::ModFloat(_, _) => None,
            Expr::LessSigned(_, _) => None,
            Expr::LessUnsigned(_, _) => None,
            Expr::LessFloat(_, _) => None,
            Expr::EqInt(_, _) => None,
            Expr::EqFloat(_, _) => None,
            Expr::EqChar(_, _) => None,
            Expr::EqBool(_, _) => None,
            Expr::SignExtend(_, _) => None,
            Expr::ZeroExtend(_, _) => None,
            Expr::SignedToFloat(_, _) => None,
            Expr::UnsignedToFloat(_, _) => None,
            Expr::FloatToSigned(_, _) => None,
            Expr::FloatToUnsigned(_, _) => None,
            Expr::FloatPromote(_, _) => None,
            Expr::FloatDemote(_, _) => None,
            Expr::BitwiseAnd(_, _) => None,
            Expr::BitwiseOr(_, _) => None,
            Expr::BitwiseXor(_, _) => None,
            Expr::BitwiseNot(_) => None,
            Expr::Truncate(_, _) => None,
            Expr::Deref(_, _) => None,
            Expr::Offset(_, _, _) => None,
            Expr::Transmute(_, _) => None,
            Expr::StackAlloc(_) => None,
        }
    }
}
