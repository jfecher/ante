# Ante

### The compile-time language

[![Build Status](https://travis-ci.org/jfecher/ante.svg?branch=master)](https://travis-ci.org/jfecher/ante)

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
- Want to learn Ante?  Checkout [the website](http://jfecher.github.io/ante/).
- Looking to contribute?  Checkout [the documentation](http://jfecher.github.io/ante/public/doxygen/html/).

## Features
* Lisp-esque compile-time execution combined with an extensible compiler API
* Systems language that feels like an interpreted language
* Expression-based syntax
* Robust module system with integrated build system
* Immutability by default
* Strongly typed with a detailed algebraic type system and type inferencing
* Ability to write compiler plugins within the compiled program itself
type and issue a compile-time error if it is invalidated
    -  Diverse and powerful compile-time analysis that can be custom programmed into
any datatype creating eg. iterator invalidation, pointer-autofree, or even an ownership system.
The implementation of these features resembles that of a compiler plugin, except that it is written
into the compiled module itself.

* Programmers have just as much power over their program as the compiler does.  As an example,
here is an implementation of the goto construct in Ante

```go
//The 'ante' keyword declares compile-time functions
ante
fun goto: VarNode vn
    let label = Ante.lookup vn ?
        None -> Ante.error "Cannot goto undefined label ${vn}"

    LLVM.setInsertPoint <| getCallSiteBlock ()
    LLVM.createBr label

ante
fun label: VarNode vn
    let ctxt = Ante.llvm_ctxt
    let callingFn = getParentFn <| getCallSiteBlock ()
    let lbl = LLVM.BasicBlock ctxt callingFn
    Ante.store vn lbl


//test it out
label begin
print "hello!"
goto begin
```

* For more information, check out tests/non_compiling/language.an for all planned features.
    - For implemented features, check out the tests directory

## Installation

### Requirements

 * `llvm` version >= 5.0.  To check which version you have, run `$ lli --version`.  To install llvm, install
the `llvm` package on your distro's package manager, eg. for Ubuntu: `$ sudo apt-get install llvm-5.0`
 * `yacc`. This is normally provided by GNU Bison - to install Bison, install the `bison` package in your
distro's package manager.

### Steps

1. Install any required packages.

2. Run `$ git clone https://github.com/jfecher/ante.git`

3. Run `$ cd ante && make && sudo make stdlib`

    - NOTE: root permissions are only needed to export the standard library.  To export it manually, execute the following command as root:

        `# mkdir -p /usr/include/ante && cp stdlib/*.an /usr/include/ante`
