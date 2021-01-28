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
- [ ] Language Documentation:
    - [ ] Article on interactions between `mut`, `ref`, and passing by reference.
    - [ ] Article on Ante's use of whitespace for line continuations.
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
