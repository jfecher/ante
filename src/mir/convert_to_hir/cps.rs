//! This module contains a pass to convert the Mir into continuation passing style.
//! It assumes the Mir has already been created (initially in capability passing style)
//! then evaluated to remove capabilities from the program.

use std::{rc::Rc, collections::HashMap};

use crate::mir::ir::{Mir, Function, FunctionId, Expr, ParameterId};

impl Mir {
    pub(super) fn cps_convert(&mut self) {
        let mut context = Context::new(self.next_function_id);

        while let Some((old_id, new_id)) = context.queue.pop() {
            let function = self.functions.get_mut(&old_id).unwrap();
            context.cps_convert_function(function, new_id);
        }

        self.next_function_id = context.next_function_id;
    }
}

struct Context {
    next_function_id: u32,
    new_functions: HashMap<FunctionId, FunctionId>,
    queue: Vec<(FunctionId, FunctionId)>,
}

impl Context {
    fn new(next_function_id: u32) -> Self {
        let mut context = Context { next_function_id, new_functions: HashMap::new(), queue: Vec::new() };
        let new_main_id = context.next_function_id(Mir::main_id().name);
        context.queue.push((Mir::main_id(), new_main_id));
        context
    }

    fn next_function_id(&mut self, name: Rc<String>) -> FunctionId {
        let id = FunctionId { id: self.next_function_id, name };
        self.next_function_id += 1;
        id
    }

    /// CPS convert a function. Each CPS conversion returns a function term.
    ///
    /// CPS(fn x -> e) = fn k -> k (fn x k' -> CPS(e) k')
    fn cps_convert_function(&mut self, function: &Function, new_id: FunctionId) -> FunctionId {
        let k = Expr::Parameter(ParameterId::new(new_id, 0));
        let body = Expr::rt_call(k, todo!());
    }

    fn cps_convert_expression(&mut self, expression: &Expr) -> FunctionId {
        todo!()
    }
}
