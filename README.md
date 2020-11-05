# Ante

### The compile-time language

[![Build Status](https://gitlab.com/jfecher/ante/badges/typeinference/build.svg)](https://gitlab.com/rndmprsn/ante/commits/typeinference)

Ante is a compiled systems language focusing on providing extreme extensibility through
the use of a compile-time API.  Using such an API, compiler extensions can be created
within the program itself, allowing for the addition of a garbage collector, ownership
system, automatic linters, etc, all in a normal library without requiring any changes
to the compiler itself.

Systems languages can traditionally be a pain to write.  To fix this, Ante provides high-level
solutions such as string interpolation, smart pointers, and pattern matching, while maintaining
the ability to interact at a lower level if needed.

## Community
- Join the official subreddit at [/r/ante](https://www.reddit.com/r/ante) for any and all discussion.  Everyone is welcome!
- Want to learn Ante?  Check out [the website](http://antelang.org/).
- Looking to contribute?  Check out [the documentation](http://antelang.org/doxygen/html/).

## Features
* Strong focus on readability
* Expression-based syntax
* Robust module system with an integrated build system
* Immutable by default
* Strongly typed with a detailed algebraic type system and type inferencing
* Compile-time execution combined with an extensible compiler API
    - Ability to write compiler plugins within the compiled program itself
    - Use compiler API to analyze or change type system, IR, macros, etc.
    - Programmers have just as much power over their program as the compiler does.  As an example,
    here is an implementation of the goto construct in Ante:

```haskell
//The 'ante' keyword declares compile-time values
ante
    labels = global mut empty Map

    goto lbl =
        label = lookup labels lbl ?
            None -> Ante.error "Cannot goto undefined label ${lbl}"

        Llvm.setInsertPoint (getCallSiteBlock ())
        Llvm.createBr label

    label name:Str =
        callingFn = getParentFn (getCallSiteBlock ())
        lbl = Llvm.BasicBlock(Ante.llvm_ctxt, callingFn)
        labels#name := lbl


//test it out
label "begin"
print "hello!"
goto "begin"
```

## Installation

### Requirements

 * `yacc`. This is normally provided by GNU Bison - to install Bison, install the `bison` package in your
distro's package manager.
 * (Optional) `llvm` version >= 8.0.  There is no need to install llvm manually.  If you do not have it
 installed already, cmake will automatically use the version in ante's git submodule.  If you wish to
 install llvm system-wide anyway, then make sure to check which version you have by running `$ lli --version`.
 To install a specific version of llvm, install the `llvm` package on your distro's package manager, eg. for
 Ubuntu: `$ sudo apt-get install llvm-8.0`.  Note that not all versions may be available on all systems
 without building from source.

### Steps

1. Install yacc/bison.

2. Run `$ git clone https://github.com/jfecher/ante.git`

3. Run `$ cd ante && cmake .` This will generate your platform-specific
build files.  Usually either a Makefile or Visual Studio solution file.
You can also specify which to make manually by passing the appropriate
arguments to cmake.

3. Run `$ cmake --build .`  This may take a while as it is also building llvm.

NOTE: If you are planning to develop ante in vim or a similar editor, make sure
to add include, llvm/include, and llvm_build/include to your include paths.

### Trying Ante in Docker

Alternatively, you can try Ante using Docker. You can build the image using:

```
docker build . -t ante
```

and then start it with:

```
docker run -it ante
```

At this point, you can install an editor and use the compiler/REPL (in /home/ante/ante) to write some code and run it.
