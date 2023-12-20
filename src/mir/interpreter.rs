//! This MIR interpreter is used for debugging MIR programs which otherwise
//! can be difficult to follow since they are in CPS form.
use std::collections::HashMap;

use crate::{util::fmap, hir::{Literal, IntegerKind, PrimitiveType}, lexer::token::FloatKind};

use super::ir::{Mir, Atom, ParameterId, FunctionId, Type};

impl Mir {
    #[allow(unused)]
    pub fn interpret(&self) {
        let mut interpreter = Interpreter::new(self);
        let mut i = 0;

        while !interpreter.done && i < 100 {
            interpreter.call_current_function();

            let arg_tys = fmap(&interpreter.current_args, |arg| arg.approx_type(self));
            let function = &interpreter.mir.functions[&interpreter.current_function];
            let params = &function.argument_types;

            if params.len() != arg_tys.len() {
                eprintln!("  WARNING: Call to function {} with {} args when it takes {} params", 
                          interpreter.current_function, arg_tys.len(), params.len());
            }
            
            for (i, (param, arg)) in function.argument_types.iter().zip(arg_tys).enumerate() {
                if let Some(arg) = arg {
                    if *param != arg {
                        eprintln!("  WARNING: In function call to {}, parameter {} : {} where the argument : {}",
                                interpreter.current_function, i, param, arg);
                    }
                }
            }

            i += 1;
        }

        if i >= 100 {
            eprintln!("i = 100, early exit");
        }
    }
}

struct Interpreter<'mir> {
    mir: &'mir Mir,
    memory: HashMap<ParameterId, Atom>,
    done: bool,
    current_function: FunctionId,
    current_args: Vec<Atom>,
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

    fn define(&mut self, parameter: ParameterId, value: Atom) {
        let value = match value {
            Atom::Parameter(value_param_id) => self.memory[&value_param_id].clone(),
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

        self.evaluate_call_body(&function.body_continuation, &function.body_args)
    }

    fn evaluate_call_body(&mut self, body_continuation: &Atom, body_args: &[Atom]) {
        match self.evaluate(body_continuation) {
            Atom::Function(function_id) => {
                let args = fmap(body_args, |arg| self.evaluate(arg));
                self.current_function = function_id.clone();
                self.current_args = args;
            },
            Atom::Branch => {
                let args = fmap(body_args, |arg| self.evaluate(arg));
                eprintln!("if {} then {} else {}", args[0], args[1], args[2]);

                let arg_i = 1 + matches!(args[0], Atom::Literal(Literal::Bool(true))) as usize;
                self.current_function = self.evaluate_function(&args[arg_i]);
                self.current_args.clear();
            },
            Atom::Switch(cases, else_case) => {
                assert_eq!(body_args.len(), 1);
                let int = self.evaluate_int(&body_args[0]).0;
                eprintln!("switch to case {}", int);

                if let Some((_, case_fn)) = cases.into_iter().find(|(case_int, _)| *case_int == int as u32) {
                    self.current_function = case_fn;
                } else {
                    self.current_function = else_case.unwrap();
                }
                self.current_args.clear();
            },
            Atom::Literal(Literal::Unit) => {
                // The program always ends in a call to ()
                self.done = true;
            }
            Atom::Assign => {
                eprintln!(":=");
                for arg in body_args {
                    let arg = self.evaluate(arg);
                    eprintln!("  {}", arg);
                }
                self.current_function = self.evaluate_function(&body_args[2]);
            }
            Atom::Extern(id) => {
                let symbol = self.mir.extern_symbols[&id].0.as_str();
                let args = fmap(body_args, |arg| self.evaluate(arg));

                match symbol {
                    "putchar" => {
                        match &args[0] {
                            Atom::Literal(Literal::Char(c)) => {
                                print!("{c}");
                            }
                            other => unreachable!("putchar: cannot put non-char '{}'", other),
                        }

                        self.current_function = self.evaluate_function(&args[1]);
                        self.current_args = vec![Atom::Literal(Literal::Unit)];
                    }
                    _ => unimplemented!("Evaluate extern '{symbol}'"),
                }
            }
            other => unreachable!("evaluate_call_body expected function, found {}", other),
        }
    }

    fn evaluate(&mut self, atom: &Atom) -> Atom {
        match atom {
            Atom::Branch
            | Atom::Switch(..)
            | Atom::Literal(_)
            | Atom::Assign
            | Atom::Extern(_)
            | Atom::Function(_) => atom.clone(),
            Atom::Parameter(parameter_id) => {
                self.memory.get(parameter_id)
                    .cloned()
                    .unwrap_or_else(|| panic!("In function {}, Parameter {} not defined!", self.current_function, parameter_id))
            }
            Atom::Tuple(fields) => Atom::Tuple(fmap(fields, |field| self.evaluate(field))),
            Atom::MemberAccess(tuple, index, _typ) => {
                match self.evaluate(tuple) {
                    Atom::Tuple(fields) => {
                        let result = fields[*index as usize].clone();
                        self.evaluate(&result)
                    }
                    other => unreachable!("Atom::MemberAccess expected tuple, found {}", other),
                }
            },
            Atom::Handle(_, _) => todo!("Handle"),
            Atom::Effect(_, _) => todo!("Effect"),
            Atom::AddInt(lhs, rhs) => self.int_function(lhs, rhs, "+", |a, b| a + b),
            Atom::AddFloat(_, _) => todo!(),
            Atom::SubInt(lhs, rhs) => self.int_function(lhs, rhs, "-", |a, b| a - b),
            Atom::SubFloat(_, _) => todo!(),
            Atom::MulInt(lhs, rhs) => self.int_function(lhs, rhs, "*", |a, b| a * b),
            Atom::MulFloat(_, _) => todo!(),
            Atom::DivSigned(lhs, rhs) => self.int_function(lhs, rhs, "/s", |a, b| a / b),
            Atom::DivUnsigned(lhs, rhs) => self.int_function(lhs, rhs, "/u", |a, b| a / b),
            Atom::DivFloat(_, _) => todo!(),
            Atom::ModSigned(lhs, rhs) => self.int_function(lhs, rhs, "%s", |a, b| a + b),
            Atom::ModUnsigned(lhs, rhs) => self.int_function(lhs, rhs, "%u", |a, b| a + b),
            Atom::ModFloat(_, _) => todo!(),
            Atom::LessSigned(lhs, rhs) => self.bool_function(lhs, rhs, "<s", |a, b| a < b),
            Atom::LessUnsigned(lhs, rhs) => self.bool_function(lhs, rhs, "<u", |a, b| a < b),
            Atom::LessFloat(_, _) => todo!(),
            Atom::EqInt(lhs, rhs) => self.bool_function(lhs, rhs, "==", |a, b| a == b),
            Atom::EqFloat(_, _) => todo!(),
            Atom::EqChar(_, _) => todo!(),
            Atom::EqBool(_, _) => todo!(),
            Atom::SignExtend(atom, _) => self.evaluate(atom),
            Atom::ZeroExtend(atom, _) => self.evaluate(atom),
            Atom::SignedToFloat(int, _typ) => {
                self.map_literal(int, |literal| match literal {
                    Literal::Integer(x, _kind) => Literal::Float((x as f64).to_bits(), FloatKind::F64),
                    other => unreachable!("signed_to_float expected int, found {}", other),
                })
            }
            Atom::UnsignedToFloat(int, _typ) => {
                self.map_literal(int, |literal| match literal {
                    Literal::Integer(x, _kind) => Literal::Float((x as f64).to_bits(), FloatKind::F64),
                    other => unreachable!("signed_to_float expected int, found {}", other),
                })
            },
            Atom::FloatToSigned(_, _) => todo!(),
            Atom::FloatToUnsigned(_, _) => todo!(),
            Atom::FloatPromote(_, _) => todo!(),
            Atom::FloatDemote(_, _) => todo!(),
            Atom::BitwiseAnd(_, _) => todo!(),
            Atom::BitwiseOr(_, _) => todo!(),
            Atom::BitwiseXor(_, _) => todo!(),
            Atom::BitwiseNot(_) => todo!(),
            Atom::Truncate(atom, _typ) => self.evaluate(atom),
            Atom::Deref(atom, _typ) => {
                match self.evaluate(atom) {
                    Atom::Tuple(mut values) => values.remove(0),
                    other => unreachable!("Atom::Deref expected Atom::Tuple, found {}", other),
                }
            },
            Atom::Offset(_, _, _) => todo!(),
            Atom::Transmute(_, _) => todo!(),
            Atom::StackAlloc(value) => {
                let value = self.evaluate(value);
                Atom::Tuple(vec![value]) // Use tuples to emulate memory for now
            },
        }
    }

    fn map_literal(&mut self, atom: &Atom, f: impl FnOnce(Literal) -> Literal) -> Atom {
        match self.evaluate(atom) {
            Atom::Literal(literal) => Atom::Literal(f(literal)),
            other => unreachable!("map_literal expected literal, found {}", other),
        }
    }

    fn int_function(&mut self, lhs: &Atom, rhs: &Atom, name: &str, f: impl FnOnce(u64, u64) -> u64) -> Atom {
        let (lhs, kind) = self.evaluate_int(lhs);
        let (rhs, _) = self.evaluate_int(rhs);
        let result = f(lhs, rhs);
        eprintln!("{} {} {} = {}", lhs, name, rhs, result);
        Atom::Literal(Literal::Integer(result, kind))
    }

    fn bool_function(&mut self, lhs: &Atom, rhs: &Atom, name: &str, f: impl FnOnce(u64, u64) -> bool) -> Atom {
        let (lhs, _) = self.evaluate_int(lhs);
        let (rhs, _) = self.evaluate_int(rhs);
        let result = f(lhs, rhs);
        eprintln!("{} {} {} = {}", lhs, name, rhs, result);
        Atom::Literal(Literal::Bool(result))
    }

    fn evaluate_int(&mut self, atom: &Atom) -> (u64, IntegerKind) {
        match self.evaluate(atom) {
            Atom::Literal(Literal::Integer(int, kind)) => (int, kind),
            other => unreachable!("evaluate_int expected int, found {}", other),
        }
    }

    fn evaluate_function(&mut self, atom: &Atom) -> FunctionId {
        match self.evaluate(atom) {
            Atom::Function(function_id) => function_id,
            other => unreachable!("evaluate_function expected function, found {}", other),
        }
    }
}

impl Atom {
    pub(crate) fn approx_type(&self, mir: &Mir) -> Option<Type> {
        match self {
            Atom::Branch => None,
            Atom::Switch(_, _) => None,
            Atom::Literal(literal) => {
                match literal {
                    Literal::Integer(_, kind) => Some(Type::Primitive(PrimitiveType::Integer(*kind))),
                    Literal::Float(_, kind) => Some(Type::Primitive(PrimitiveType::Float(*kind))),
                    Literal::CString(_) => Some(Type::Primitive(PrimitiveType::Pointer)),
                    Literal::Char(_) => Some(Type::Primitive(PrimitiveType::Char)),
                    Literal::Bool(_) => Some(Type::Primitive(PrimitiveType::Boolean)),
                    Literal::Unit => Some(Type::Primitive(PrimitiveType::Unit)),
                }
            },
            Atom::Parameter(id) => {
                let function = &mir.functions[&id.function];
                Some(function.argument_types[id.parameter_index as usize].clone())
            },
            Atom::Function(id) => {
                let function = &mir.functions[id];
                Some(Type::Function(function.argument_types.clone(), vec![]))
            },
            Atom::Tuple(fields) => {
                let fields = fields.iter().map(|field| field.approx_type(mir)).collect::<Option<Vec<_>>>()?;
                Some(Type::Tuple(fields))
            },
            Atom::MemberAccess(_, _, _) => None,
            Atom::Assign => None,
            Atom::Extern(_) => None,
            Atom::Handle(_, _) => None,
            Atom::Effect(_, _) => None,
            Atom::AddInt(_, _) => None,
            Atom::AddFloat(_, _) => None,
            Atom::SubInt(_, _) => None,
            Atom::SubFloat(_, _) => None,
            Atom::MulInt(_, _) => None,
            Atom::MulFloat(_, _) => None,
            Atom::DivSigned(_, _) => None,
            Atom::DivUnsigned(_, _) => None,
            Atom::DivFloat(_, _) => None,
            Atom::ModSigned(_, _) => None,
            Atom::ModUnsigned(_, _) => None,
            Atom::ModFloat(_, _) => None,
            Atom::LessSigned(_, _) => None,
            Atom::LessUnsigned(_, _) => None,
            Atom::LessFloat(_, _) => None,
            Atom::EqInt(_, _) => None,
            Atom::EqFloat(_, _) => None,
            Atom::EqChar(_, _) => None,
            Atom::EqBool(_, _) => None,
            Atom::SignExtend(_, _) => None,
            Atom::ZeroExtend(_, _) => None,
            Atom::SignedToFloat(_, _) => None,
            Atom::UnsignedToFloat(_, _) => None,
            Atom::FloatToSigned(_, _) => None,
            Atom::FloatToUnsigned(_, _) => None,
            Atom::FloatPromote(_, _) => None,
            Atom::FloatDemote(_, _) => None,
            Atom::BitwiseAnd(_, _) => None,
            Atom::BitwiseOr(_, _) => None,
            Atom::BitwiseXor(_, _) => None,
            Atom::BitwiseNot(_) => None,
            Atom::Truncate(_, _) => None,
            Atom::Deref(_, _) => None,
            Atom::Offset(_, _, _) => None,
            Atom::Transmute(_, _) => None,
            Atom::StackAlloc(_) => None,
        }
    }
}
