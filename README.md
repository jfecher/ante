# ante-rs

A WIP rewrite of Ante's compiler in rust.

---

### Why?

The original c++ compiler was written as ante's
design was evolving quickly. In fact, the repository
started as an interpreter for a gradually-typed scripting
language by the name of Zy. Over the years, the
compiler has had many features bolted on it wasn't defined
for - including hindley-milner type inference, a REPL,
and functional dependencies. This has resulted in not
only many bugs but also the complexity has increased
the difficulty of adding new features and fixing existing
bugs.

This new compiler to be built from the ground up with these
features in mind - hopefully enabling a cleaner codebase.

---

### Progress

Steps needed to get back in line with the C++ compiler:

- [x] Lexer
- [x] Error reporting
- [ ] Parser
  - [x] Most things, except for:
  - [ ] Tuples
  - [ ] Loops
- [x] Name resolution
  - [x] Variable definitions
  - [x] Function definitions
  - [x] Type definitions
  - [x] Trait definitions
  - [x] Redefined warning
  - [x] Never used warning
- [ ] HM-type inference
- [ ] Code generation (llvm)
- [ ] REPL

Future goals still unimplemented in the C++ compiler:

- [ ] Refinement Types
- [ ] More general trait support (C++ compiler has trait inference bugs)
- [ ] Commit to having deterministic destruction w/ destructors
    - get rid of plans for optional GC as it would likely poison any libraries it touches.
- [ ] Better compiler tests e.g. golden tests

Nice to have but not currently required:
- [ ] Multiple backends, possibly GCCJIT/cranelift for faster debug builds?
- [ ] Possibly re-add UFCS since it reduces the need to import everything.
    - need to workout how this interacts when the types aren't known
- [ ] Reasonable C/C++ interop with clang api (stretch goal)
- [ ] Build system (stretch goal)
