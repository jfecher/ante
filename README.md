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

See the [website](https://antelang.org), [language tour](https://antelang.org/docs/language/),
and [roadmap](https://antelang.org/docs/roadmap) for more information.

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

The best place to follow ante's development is in the official discord: https://discord.gg/BN97fKnEH2.
There is also the mostly inactive subreddit at [/r/ante](https://reddit.com/r/ante) which is mainly used for questions about the language rather
than development updates. You can also feel to file issues or ask questions on this repository.

---

### Building

Ante currently optionally requires llvm 13.0 while building. If you already have this installed with
sources, you may be fine building with `cargo install --path .` alone. If cargo complains
about not finding any suitable llvm version, you can either choose to build ante without
the llvm backend via `cargo install --path . --no-default-features` or you can build llvm from
source, either via `llvmenv` or `cmake` as covered in the next sections.

#### Linux and Mac

```bash
$ cargo install llvmenv
$ llvmenv init
$ llvmenv build-entry -G Makefile -j7 13.0.0
$ llvmenv global 13.0.0
$ LLVM_SYS_130_PREFIX=$(llvmenv prefix)
$ cargo build
```

If `llvmenv prefix` defaults to a path with spaces in it, you may get an error during `cargo build`
complaining it cannot find the path to llvm. If this happens, try manually moving the installation
in `llvmenv prefix` to a new directory without spaces, updating `LLVM_SYS_130_PREFIX` to this new
location and re-running `cargo build`.

##### Nix

Ante is available in the unstable branch of the [nixpkgs repository](https://search.nixos.org/packages?channel=unstable&show=ante&type=packages&query=ante).

The project itself provides build instructions for the [Nix package manager](https://nixos.org/).
Those can be used for the most recent version of the compiler, or for working on it.

To enter the development environment, run either `nix-shell` or `nix develop` depending on whether you are using nix
with [flakes](https://nixos.wiki/wiki/Flakes) and [nix command](https://nixos.wiki/wiki/Nix_command) enabled or not.
Then you can build and run the project with `cargo` as described at the top of this section.

Beyond that, the project will also build with `nix-build` / `nix build`, meaning you can install it on your system using
the provided overlay or play around with the compiler via `nix shell github:jfecher/ante`.

#### Windows

Note: LLVM is notoriously difficult to build on windows. If you're a windows user who has tried
the following and still cannot build llvm, I highly recommend trying out ante without the llvm
backend via `cargo install --path . --no-default-features`.

That being said, here is one way to build llvm via llvmenv on windows:

```shell
$ cargo install llvmenv
$ llvmenv init
$ llvmenv build-entry -G VisualStudio -j7 13.0.0
$ llvmenv global 13.0.0
$ for /f "tokens=*" %a in ('llvmenv prefix') do (set LLVM_SYS_130_PREFIX=%a)
$ cargo build
```

You can confirm your current version of llvm by running `llvmenv version`
or `llvm-config`

##### CMake

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
$ set LLVM_SYS_130_PREFIX=/absolute/path/to/llvm-build
$ cargo build
```
