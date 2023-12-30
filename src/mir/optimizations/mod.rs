use std::collections::{HashSet, HashMap};

use crate::util::fmap;

use self::scope::Scopes;

use super::ir::{Mir, Function, Type, FunctionId, Expr, ParameterId, ExprMap};

mod scope;

impl Mir {
    // function lower2cff(p)
    //   repeat
    //     L ← {l ∈ dom(p) | l is bad}
    //     foreach l ∈ L do                              // for all bad functions
    //       foreach u ∈ uses(l) do mangle_uses(p, l, u) // mangle all uses
    //     end
    //     remove unreachable functions from p
    //   until |L| = 0
    // end
    #[allow(unused)]
    fn lower2cff(&mut self) {
        let mut i = 0;
        loop {
            eprintln!("\n=========================================");
            eprintln!(" lower2cff iteration {}:", i);
            eprintln!("=========================================");
            eprintln!("\n{}", self);
            i += 1;
            break;

            let scopes = self.find_scopes();

            let bad_functions = fmap(self.functions.iter().filter(|(_, f)| f.is_bad(&scopes)), |(id, _)| id.clone());
            let no_bad_functions = bad_functions.is_empty();

            for function in bad_functions {
                for u in self.uses_of(&function) {
                    self.mangle_uses(&function, u);
                }
            }

            self.remove_unreachable_functions();

            if no_bad_functions || i >= 5 {
                break;
            }
        }

        eprintln!("\n=========================================");
        eprintln!(" lower2cff finished");
        eprintln!("=========================================");
        eprintln!("\n{}", self);
    }

    // function mangle_uses(p, l, u)
    //   if p[u] = l(e1, . . . , en) ∧ u notin scope_p(l)
    //     χ ← {i | ei is higher-order}                  // indices corresponding to higher-order args
    //     t ← cut(t_l, χ)                               // cut higher-order part where p |- l : t_l
    //     foreach i ∈ χ do M[l_i] ← ei                  // map higher-order args to their parameters
    //     l′ ← mangle(p, l, t, M)                       // drop higher-order args
    //     args ← cut((e1, . . . , en), χ)               // get zeroth order args
    //     p[u] ← l′(args)                               // update call site
    //   end
    // end
    fn mangle_uses(&mut self, l: &FunctionId, u: FunctionId) {
        if let Expr::Call(u_body, u_args, _) = &self.functions[&u].body {
            let u_args = u_args.clone();

            if **u_body == Expr::Function(l.clone()) {
                let l_function = &self.functions[l];

                let x = l_function.argument_types.iter().enumerate()
                    .filter(|(_, typ)| matches!(typ, Type::Function(..)))
                    .map(|(i, _)| i)
                    .collect::<HashSet<_>>();

                let t = cut(&l_function.argument_types, &x);

                let parameter_map = x.iter().map(|i| {
                    let param = ParameterId { function: l.clone(), parameter_index: *i as u16 };
                    (param, u_args[*i].clone())
                }).collect::<HashMap<_, _>>();

                let mut m = ExprMap::default();
                m.parameters = parameter_map;

                let l2 = self.mangle(l, t, m);

                let args = cut(&u_args, &x);

                let call_site = self.functions.get_mut(&u).unwrap();

                call_site.body = Expr::rt_call(Expr::Function(l2), args);
            }
        }
    }

    /// Evaluate every argument of a function call
    pub fn evaluate_call(&mut self, l: &FunctionId, u_args: Vec<Expr>, modified: &mut HashSet<FunctionId>) -> Expr {
        // Each function only has 1 argument
        let mut x = HashSet::new();
        x.insert(0);

        self.substitute(l, u_args, modified)
    }

    fn substitute(&mut self, le: &FunctionId, args: Vec<Expr>, modified: &mut HashSet<FunctionId>) -> Expr {
        let mut m = ExprMap::default();

        for (i, arg) in args.into_iter().enumerate() {
            let parameter_id = ParameterId { function: le.clone(), parameter_index: i as u16 };
            m.parameters.insert(parameter_id, arg);
        }

        // TODO: Avoid re-finding scopes for all functions on each call
        let scopes = self.find_scopes();
        let scope_le = scopes.get_scope(le);

        let mutate = self.uses_of(le).len() == 1;

        if !mutate {
            for l in &scope_le.functions {
                if l != le {
                    let new_id = self.next_function_id(l.name.clone());
                    m.functions.insert(l.clone(), new_id.clone());
                }
            }
        }

        for l in &scope_le.functions {
            if l != le {
                if mutate {
                    modified.insert(l.clone());

                    self.functions.get_mut(l).unwrap_or_else(|| {
                        unreachable!("substitute: No function {}", l)
                    }).map_functions(&m);
                } else {
                    let new_id = m.functions[&l].clone();

                    let mut new_function = self.functions.get(l).unwrap_or_else(|| {
                        unreachable!("substitute: No function {}", l)
                    }).clone();

                    new_function.id = new_id.clone();
                    new_function.map_functions(&m);
                    self.functions.insert(new_id, new_function);
                }
            }
        }

        let mut new_body = self.functions[le].body.clone();
        new_body.map_functions(&m);
        new_body
    }

    // function mangle(p, le, t, M)
    //   foreach l ∈ scope_p(le) \ le do M[l] ← new label
    //   foreach l ∈ scope_p(le) \ le do
    //     fn(t): b ← p[l]              // get l’s signature and body
    //     l′ = M[l]                      // get l`’s associated new label
    //     p[l′] ← fn(t): b′              // insert new l′ where M, b |> b′
    //   end
    //   l′e ← new label                  // now deal with entry: create new label
    //   _: be ← p[le]                    // get `e’s body
    //   p[l′e] ← t: b′e                  // insert new entry where M, be |> b′e
    //   return l′e                       // return entry to new mangled region
    // end
    fn mangle(&mut self, le: &FunctionId, ts: Vec<Type>, mut m: ExprMap) -> FunctionId {
        // TODO: Avoid re-finding scopes for all functions on each mangle call
        let scopes = self.find_scopes();
        let scope_le = scopes.get_scope(le);

        // Deviation from the above algorithm: If any parameter of `le` is not
        // present in the map we need to update it to map to the same parameter
        // of the new function. Note that we do not update the function itself
        // so that any recursive calls are left in place.
        let new_le_id = self.next_function_id(le.name.clone());
        let mut next_parameter_index = 0;

        for parameter in self.functions[le].parameters() {
            if !m.parameters.contains_key(&parameter) {
                let parameter_id = ParameterId { function: new_le_id.clone(), parameter_index: next_parameter_index };
                m.parameters.insert(parameter, Expr::Parameter(parameter_id));
                next_parameter_index += 1;
            }
        }

        for l in &scope_le.functions {
            if l != le {
                let new_id = self.next_function_id(l.name.clone());
                m.functions.insert(l.clone(), new_id.clone());
            }
        }

        for l in &scope_le.functions {
            if l != le {
                let new_id = m.functions[&l].clone();

                let mut new_function = self.functions[l].clone();
                new_function.id = new_id.clone();
                new_function.map_functions(&m);
                self.functions.insert(new_id, new_function);
            }
        }

        let mut new_function = self.functions[le].clone();
        new_function.id = new_le_id.clone();

        new_function.argument_types = ts;
        new_function.map_functions(&m);

        println!("Inserting new function {}", new_le_id);
        self.functions.insert(new_le_id.clone(), new_function);
        new_le_id
    }

    fn uses_of(&self, target: &FunctionId) -> HashSet<FunctionId> {
        let mut uses = HashSet::new();

        for function in self.functions.values() {
            function.for_each_id(&mut (), |_, cont| {
                if cont == target {
                    uses.insert(function.id.clone());
                }
            }, |_, _| ());
        }

        uses
    }

    pub fn remove_unreachable_functions(&mut self) {
        let reachable = self.reachable_functions();
        self.functions.retain(|id, _| reachable.contains(id));
    }

    pub fn reachable_functions(&self) -> HashSet<FunctionId> {
        let mut queue = vec![Self::main_id()];
        let mut visited = HashSet::new();

        while let Some(function) = queue.pop() {
            if !visited.contains(&function) {
                visited.insert(function.clone());

                self.functions[&function].for_each_id(&mut (), |_, used| {
                    queue.push(used.clone());
                }, |_, _| ());
            }
        }

        visited
    }
}

/// Cut out the given indices from the array
fn cut<T: Clone>(array: &[T], indices: &HashSet<usize>) -> Vec<T> {
    let mut result = Vec::with_capacity(indices.len());

    for (i, elem) in array.iter().enumerate() {
        if !indices.contains(&i) {
            result.push(elem.clone());
        }
    }

    result
}

#[derive(Debug, PartialEq, Eq)]
enum Order {
    /// A first-order function (one with no function parameters)
    BasicBlock,

    /// A second-order function with exactly 1 function parameter which itself is first-order
    Returning,

    /// A second-order function with > 1 function parameter, or a third-order or higher function.
    HigherOrder,
}

#[derive(Debug, PartialEq, Eq)]
enum FunctionLevel {
    /// A function containing no free variables in scope
    TopLevel,

    /// A function containing at least one free variable in scope
    NonTopLevel,
}

impl Function {
    fn order(&self) -> Order {
        let mut function_parameters = self.argument_types.iter()
            .filter_map(|arg| match arg {
                Type::Function(params, _, _) => Some(params),
                _ => None,
            });

        let first = function_parameters.next();
        let second = function_parameters.next();

        match (first, second) {
            (None, None) => Order::BasicBlock,
            (Some(args), None) if !args.iter().any(|arg| arg.contains_function()) => Order::Returning,
            _ => Order::HigherOrder,
        }
    }

    fn level(&self, scopes: &Scopes) -> FunctionLevel {
        let scope = scopes.get_scope(&self.id);

        if scope.entry_point == self.id {
            FunctionLevel::TopLevel
        } else {
            FunctionLevel::NonTopLevel
        }
    }

    /// A 'bad' function is one that is neither a BasicBlock, nor top-level and returning.
    fn is_bad(&self, scopes: &Scopes) -> bool {
        let order = self.order();
        order != Order::BasicBlock && !(order == Order::Returning && self.level(scopes) == FunctionLevel::TopLevel)
    }
}
