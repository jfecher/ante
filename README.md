# ante-rs

[![Travis (.org)](https://img.shields.io/travis/jfecher/ante-rs)](https://travis-ci.org/github/jfecher/ante-rs)

A WIP rewrite of Ante's compiler in rust.

---

### Why?

The original c++ compiler was written as ante's
design was evolving quickly. In fact, the repository
started as an interpreter for a gradually-typed scripting
language by the name of Zy. Over the years, the
compiler has had many features bolted on it wasn't designed
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
- [x] HM-type inference
  - [x] Basic inference
  - [x] Let generalization
  - [x] Trait inference
- [ ] Code generation (llvm)
  - [x] Builtin functions/operators
  - [x] Monomorphisation
  - [x] Extern functions
  - [x] Trait Impls
  - [ ] Match expressions
  - [ ] Tuples and loops
- [ ] REPL

Future goals still unimplemented in the C++ compiler:

- [ ] `given` clauses in traits/impls
- [ ] Refinement Types
- [x] More general trait support (C++ compiler has trait inference bugs)
- [ ] Commit to having deterministic destruction w/ destructors
    - get rid of plans for optional GC as it would likely poison any libraries it touches.
- [x] Better compiler tests e.g. golden tests

Nice to have but not currently required:
- [ ] Multiple backends, possibly GCCJIT/cranelift for faster debug builds?
- [ ] Possibly re-add UFCS since it reduces the need to import everything.
    - need to workout how this interacts when the types aren't known
- [ ] Reasonable C/C++ interop with clang api (stretch goal)
- [ ] Build system (stretch goal)

---

### Building

ante currently requires llvm 10.0 while building. If you already have this installed with
sources, you may be fine building with `cargo build` alone. If `cargo build` complains
about not finding any suitable llvm version, the easiest way to build llvm is through `llvmenv`.
In that case, you can build from source using the following:

```bash
$ cargo install llvmenv
$ llvmenv init
$ llvmenv build-entry -G Makefile -j7 10.0.0
$ llvmenv global 10.0.0
$ LLVM_SYS_100_PREFIX=$(llvmenv prefix)
$ cargo build
```

or on windows:

```shell
$ cargo install llvmenv
$ llvmenv init
$ llvmenv build-entry -G VisualStudio -j7 10.0.0
$ llvmenv global 10.0.0
$ for /f "tokens=*" %a in ('llvmenv prefix') do (set LLVM_SYS_100_PREFIX=%a)
$ cargo build
```

You can confirm your current version of llvm by running `llvmenv version`
or `llvm-config`
