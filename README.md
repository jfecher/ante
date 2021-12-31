# Ante

[![Build Status](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fjfecher%2Fante%2Fbadge&style=flat)](https://actions-badge.atrox.dev/jfecher/ante/goto)

---

Ante is a low-level functional language for exploring refinement types, lifetime inference, and
other fun features. Here's a quick taste:

```scala
type Person = name: string, job: ref string

// Infer that the data referenced via `&` should not be freed inside this function
make_person name =
    Person name &"programmer"

// bob is only used at this scope, so it can be safely freed afterward
bob = make_person "bob"

// unlike ownership systems, aliasing is allowed with lifetime inference
bob_twin = bob
assert (bob.name == bob_twin.name)
```

In general, ante is low-level (no GC, values aren't boxed by default) while also trying to
be as readable as possible by encouraging high-level approaches that can be optimized with
low-level details later on.

See the [website](https://antelang.org) and [language tour](https://antelang.org/docs/language/) for more information.

---

### Roadmap

- [x] Whitespace-sensitive lexer
- [x] Parser
- [x] Name Resolution
- [x] Full type inference
    - [x] Traits with multiple parameters and limited functional dependencies
    - [ ] Compiler option to write inferred types into program source after successful compilation
- [x] LLVM Codegen
- [x] No Garbage Collector
    - [ ] Region Inference for `ref`s
    - [ ] RAII to allow `Rc t` or `Box t` when necessary
- [x] Language [Documentation](https://antelang.org/docs/language/):
    - [x] [Article on Ante's use of whitespace for line continuations](https://antelang.org/docs/language/#line-continuations)
    - [x] [Article on the sugar for immediately invoked recursive functions (loop/recur)](https://antelang.org/docs/language/#loops)
    - [ ] Article on interactions between `mut`, `ref`, and passing by reference
    - [ ] Article on autoboxing recursive types for polymorphic pointer types
- [ ] Refinement Types
- [ ] Cranelift backend for faster debug builds
- [ ] Incremental compilation metadata
- [ ] REPL

Nice to have but not currently required:
- [ ] Reasonable automatic C/C++ interop with clang api
- [ ] Build system built into standard library
    - Ante should always be able to build itself along with any required libraries, the main question is how should a build system facilitate the more complex tasks of building other languages or running arbitrary programs like yacc/bison.

---

### Contributing

The compiler is still in a rather early state so any contributors are greatly welcome.
Feel free to contribute to either any known issues/improvements (some are listed in the
"Quick Tasks" list below) or any standard library functions you think may be useful.

Each file in the codebase is prefixed with a module comment explaining the purpose of
the file and any algorithms used. `src/main.rs` is a good place to start reading.

Make sure any PRs pass the tests in the `examples` directory. These tests have commands
in them which the [goldentests](https://github.com/jfecher/golden-tests) library uses
to run the ante compiler and check its output for each file against the expected output
contained within comments of that file.

[**Quick Tasks**](https://github.com/jfecher/ante/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)
to contribute to

---

### Community

The best place to follow ante's development is in the #ante channel of the Programming Languages discord: https://discord.gg/4Kjt3ZE.
There is also the subreddit at [/r/ante](https://reddit.com/r/ante) which is mainly used for questions about the language rather
than development updates.

---

### Building

Ante currently requires llvm 12.0 while building. If you already have this installed with
sources, you may be fine building with `cargo build` alone. If `cargo build` complains
about not finding any suitable llvm version, the easiest way to build llvm is through `llvmenv`.
In that case, you can build from source using the following:

```bash
$ cargo install llvmenv
$ llvmenv init
$ llvmenv build-entry -G Makefile -j7 12.0.1
$ llvmenv global 12.0.1
$ LLVM_SYS_120_PREFIX=$(llvmenv prefix)
$ cargo build
```

or on windows:

```shell
$ cargo install llvmenv
$ llvmenv init
$ llvmenv build-entry -G VisualStudio -j7 12.0.1
$ llvmenv global 12.0.1
$ for /f "tokens=*" %a in ('llvmenv prefix') do (set LLVM_SYS_120_PREFIX=%a)
$ cargo build
```

You can confirm your current version of llvm by running `llvmenv version`
or `llvm-config`

If the above steps don't work for you, you can try [building llvm from source
with cmake](https://www.llvm.org/docs/CMake.html). If you're on windows this
requires you to have Visual Studio 2017 or later installed already.

```
$ git clone https://github.com/llvm/llvm-project --branch=release/10.x
$ mkdir llvm-build
$ cd llvm-build
$ cmake ../llvm-project/llvm
```

At this point, cmake may error that it failed to find z3, or the windows SDK in
which case you may need to install them. For the windows SDK, you can install it
via the Visual Studio Installer (under Modify -> Individual Components). I used
version 10.0.17763.0, though it is likely newer versions will work as well. Rerun
the last cmake command to test that everything is installed right. Once this is
done, move on to compiling llvm and ante:

```
$ cmake --build .
$ cmake --build . --target install
$ cd ..
$ set LLVM_SYS_120_PREFIX=/absolute/path/to/llvm-build
$ cargo build
```
