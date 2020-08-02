- Kind checking!
- `llvm::Generator::convert_type` needs be fixed for generic types and possibly sum types as well
- Locations should be stored in a `types::traits::Impl` for better error messages for trait errors
- tuples! (parsing -> name resolution -> typechecking -> codegen)
- Audit uses of `typechecker::unify` to maybe specialize them to improve error messages for type errors
- Pattern matching completeness checking
- We have parsing for irrefutable patterns but the refutable patterns used in match expressions
  are just normal expressions. Ideally, invalid cases like an `a + b` pattern wouldn't be allowed
  by the parser at all.
- Allocate all ast nodes in a pool, and change them to store node IDs instead of hard references
- Variadic functions. Goal: support `extern printf: (ref char) ... -> int`
- cleanup `resolve_definitions` and friends in name resolution. Their use of DefinitionNodes is
  one of the less satisfying uses of `unsafe` in this codebase. This would be trivial if we
  allocated nodes in a pool since we could store the node ID instead and wouldn't have to worry
  about storing the mutable reference to a node.
- cleanup `ast::If` codegen
- cleanup `required_definitions` in name resolution (is it still needed?)
- Move towards using the `salsa` library and possibly removing ModuleCache
