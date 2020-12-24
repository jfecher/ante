# Ante

[![Travis (.org)](https://img.shields.io/travis/jfecher/ante-rs)](https://travis-ci.org/github/jfecher/ante-rs)

---

Ante is a low-level mostly functional programming language targetted
at gamedev but is still applicable for most domains. Ante aims to
make it easier to write faster, safer code through region-based
memory management and refinement types.

```rs
get_fifth_elem array = array[4]

vec = Vec.of (0..4)
get_fifth_elem vec  // Compile-time error: get_fifth_elem requires
                    // `len vec >= 5` but `len vec == 4` here
```
---

### Features/Roadmap

- [x] Full type inference
    - [x] Traits with multiple parameters and a limited (friendlier) form of functional dependencies
    - [ ] Write untyped code and have the compiler write in the types for you after a successful compilation
- [x] LLVM Codegen
- [x] No Garbage Collector
    - [ ] Region-based deterministic memory management with region inference.
        - Easily write safe code without memory leaks all while having it compiled into
          fast pointer-bump allocators or even allocated on the stack for small regions.
    - [ ] Don't want to use a pointer-bump allocator? Use a different pointer type
          like `Rc t` or `Box t` to get reference-counted or uniquely owned pointer semantics.
- [ ] Refinement Types
- [ ] REPL
- [ ] Loops

Nice to have but not currently required:
- [ ] Multiple backends, possibly GCCJIT/cranelift for faster debug builds
- [ ] Reasonable C/C++ interop with clang api (stretch goal)
- [ ] Build system (stretch goal)

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
