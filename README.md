# Ante

[![Build Status](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Factions-badge.atrox.dev%2Fjfecher%2Fante%2Fbadge&style=flat)](https://actions-badge.atrox.dev/jfecher/ante/goto)

---

Ante is a low-level functional language for exploring safe, shared mutability, algebraic
effects, and other fun features. Here's a quick taste:

```scala
foo (x: mut Bar) (y: ref a) {Clone a} {Fail}: a =
    // The `Fail` capability above lets us call `fail`
    if not valid x then fail ()

    // Safe, aliasable, borrowed mutable references
    baz x x

    // Traits via implicits (no more forced newtype wrappers)
    clone y
```

Ante is built upon a core of ownership and borrowing rules similar to rust but aims to
be as readable as possible by encouraging high-level approaches that can be optimized with
low-level details later on. Traits and effects are merged cleanly into one feature: abilities.

See the [website](https://antelang.org), [language tour](https://antelang.org/docs/language/),
and [roadmap](https://antelang.org/docs/roadmap) for more information.

---

### Contributing

The compiler is still in a rather early state so any contributors are greatly welcome.
Feel free to contribute to either any known issues/improvements or any standard library
additions you think may be useful.

Each file in the codebase is prefixed with a module comment explaining the purpose of
the file and any algorithms used. `src/main.rs` is a good place to start reading.

Make sure any PRs pass the tests in the `examples` directory. These tests have commands
in them which the [goldentests](https://github.com/jfecher/golden-tests) library uses
to run the ante compiler and check its output for each file against the expected output
contained within comments of that file. Run them with `cargo test --test goldentests`.

[**Good first issues**](https://github.com/jfecher/ante/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22)
to contribute to

---

### Community

The best place to follow ante's development is in the official discord: https://discord.gg/BN97fKnEH2.
There is also the mostly inactive subreddit at [/r/ante](https://reddit.com/r/ante) which is mainly used for questions about the language rather
than development updates. You can also feel free to file issues or ask questions on this repository.

---

### Building

When cloning, make sure to clone submodules as well:

```bash
$ git clone --recurse-submodules https://github.com/jfecher/ante
# or
$ git clone https://github.com/jfecher/ante
$ cd ante
$ git submodule update --init
```

If you do not do this, you will get an error when compiling an Ante program from clang complaining it cannot find `aminicoro.c`.

Ante requires LLVM 21.1 to build. If you already have it installed with sources,
`cargo install --path .` should work directly. Otherwise, install LLVM 21.1 through your
package manager (Linux/Mac) or build it from source via [CMake](#CMake).

Older LLVM versions are not supported.

#### Linux and Mac

The easiest method to install LLVM 21.1 is through your package manager, making sure to install any `-dev` packages
if they are available for your distro. Once installed, if `cargo b` still cannot find the right version of LLVM, you may
need to set `LLVM_SYS_211_PREFIX` to the path LLVM was installed to:

```bash
$ LLVM_SYS_211_PREFIX=$(llvm-config --obj-root)
```

If your distro does not ship LLVM 21.1, build it from source via [CMake](#CMake).

##### Nix

Ante is available in the unstable branch of the [nixpkgs repository](https://search.nixos.org/packages?channel=unstable&show=ante&type=packages&query=ante).

The project itself provides build instructions for the [Nix package manager](https://nixos.org/).
Those can be used for the most recent version of the compiler, or for working on it.

To enter the development environment, run either `nix-shell` or `nix develop` depending on whether you are using nix
with [flakes](https://wiki.nixos.org/wiki/Flakes) and [nix command](https://wiki.nixos.org/wiki/Nix_command) enabled or not.
Then you can build and run the project with `cargo` as described at the top of this section.

Beyond that, the project will also build with `nix-build` / `nix build`, meaning you can install it on your system using
the provided overlay or play around with the compiler via `nix shell github:jfecher/ante`.

#### Windows

Note: LLVM is notoriously difficult to build on Windows. Since the LLVM binaries do not ship
with the appropriate library files on Windows, you will have to build LLVM 21.1 from source via
[CMake](#CMake).

##### CMake

If the above steps don't work for you, you can try [building llvm from source
with cmake](https://www.llvm.org/docs/CMake.html). If you're on windows, this
requires you to have Visual Studio 2017 or later installed already.

```
$ git clone https://github.com/llvm/llvm-project --branch=release/21.x
$ mkdir llvm-build
$ cd llvm-build
$ cmake ../llvm-project/llvm
```

At this point, cmake may show an error that it failed to find z3 or the windows SDK, in
which case you may need to install them. For the windows SDK, you can install it
via the Visual Studio Installer (under **Modify -> Individual Components**). I used
version 10.0.17763.0, though it is likely newer versions will work as well. Rerun
the last cmake command to test that everything is installed right. Once this is
done, move on to compiling llvm and ante:

```
$ cmake --build .
$ cmake --build . --target install
$ cd ..
$ set LLVM_SYS_211_PREFIX=/absolute/path/to/llvm-build
$ cargo build
```
