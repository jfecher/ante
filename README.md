# Ante

[![Travis (.org)](https://img.shields.io/travis/jfecher/ante)](https://travis-ci.org/github/jfecher/ante)

---

Ante is a low-level mostly functional programming language targetted
at gamedev but is still applicable for most domains. Ante aims to
make it easier to write faster, safer code through region-based
memory management and refinement types.

```scala
type Person =
    job: string
    name: ref string

// Ante uses region inference to infer the data allocated
// via `new` should not be freed inside this function
make_person job =
    Person job (new "bob")

// bob is only used at this scope, so it can be safely freed afterward.
bob = make_person "programmer"

// unlike ownership systems, aliasing is allowed in region inference
bob_twin = bob
assert (bob.name == bob_twin.name)
```

Ideally, idiomatic code should be easy to read _and_ run fast so that
developers can spend as little time as possible optimizing and more time implementing new features. This
is accomplished primarily through region inference, which automatically infers the lifetime of pointers
and ensures you will never run into a use-after-free, double-free, or forget-to-free unless you explicitly
opt-out by using a different pointer type. Moreover, because lifetimes are completely inferred, you don't
have to be aware of them while programming, making ante approachable even for developers used to
garbage-collected languages. Memory within a region is allocated in a pool for the best performance, and
regions tend to be small which helps reduce memory fragmentation.

---

### Features/Roadmap

- [x] Whitespace-sensitive lexer
- [x] Parser
- [x] Name Resolution
- [x] Full type inference
    - [x] Traits with multiple parameters and a limited (friendlier) form of functional dependencies
    - [ ] Write untyped code and have the compiler write in the types for you after a successful compilation
- [x] LLVM Codegen
- [x] No Garbage Collector
    - [ ] Region-based deterministic memory management with region inference.
        - Easily write safe code without memory leaks all while having it compiled into
          fast pointer-bump allocators or even allocated on the stack for small regions.
    - [ ] Opt-out of region inference by using a different pointer type
          like `Rc t` or `Box t` to get reference-counted or uniquely owned pointer semantics.
- [x] Language [Documentation](https://antelang.org/docs/language/):
    - [x] [Article on Ante's use of whitespace for line continuations.](https://antelang.org/docs/language/#line-continuations)
    - [ ] Article on interactions between `mut`, `ref`, and passing by reference.
- [ ] Refinement Types
- [ ] REPL
- [ ] Loops

Nice to have but not currently required:
- [ ] Multiple backends, possibly GCCJIT/cranelift for faster debug builds
- [ ] Reasonable C/C++ interop with clang api
- [ ] Build system built into standard library

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

Quick Tasks:
- [ ] Change the lambda syntax from `\a b.y` to `fn a b -> y` ([#67](https://github.com/jfecher/ante/issues/67))
- [ ] Update the syntax for specifying the return type of a function from `->` to `:` ([#68](https://github.com/jfecher/ante/issues/68))
- [ ] Desugar parsing for <| (Token::ApplyLeft) and |> (Token::ApplyRight) directly into function calls ([#65](https://github.com/jfecher/ante/issues/65))
- [ ] Add support for explicit currying via `_` ([#66](https://github.com/jfecher/ante/issues/66))

---

### Building

Ante currently requires llvm 10.0 while building. If you already have this installed with
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
$ set LLVM_SYS_100_PREFIX=/absolute/path/to/llvm-build
$ cargo build
```
