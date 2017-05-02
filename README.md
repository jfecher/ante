# Ante
The compile-time language

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

## Features
* Systems language that feels like an interpreted language
* Expression-based syntax, no statements
* Support for functional, imperative, and object-oriented paradigms
* Robust module system
* Immutability by default
* Strongly typed with a detailed algebraic type system and type inferencing
* Full control given to users allowing them to specify specific requirements for a given
type and issue a compile-time error if it is invalidated
    -  Extremely diverse and powerful compile-time analysis that can be custom programmed into
any datatype creating eg. iterator invalidation, pointer-autofree, or even an ownership system.
The implementation of these features resembles that of a compiler plugin, except that it is written
into the compiled module itself.
    - These compile-time functions are checked at compile-time and not compiled into the binary.
    - Ability to write compiler plugins within the compiled program itself

* API designers given full reign to implement custom rules for their types, full access to the
parse tree is provided, along with a quick list of the uses of the variable in question.
* Programmers have just as much power over their program as the compiler does.  As an example,
here is an implementation of the goto construct in Ante

```go
![macro]
fun goto: VarNode vn
    let label = ctLookup vn ?
        None -> compErr "Cannot goto undefined label ${vn.name}"

    LLVM.setInsertPoint getCallSiteBlock{}
    LLVM.createBr label

![macro]
fun label: VarNode vn
    let ctxt = Ante.llvm_ctxt
    let callingFn = getCallSiteBlock().getParentFn()
    let lbl = LLVM.BasicBlock ctxt callingFn
    ctStore vn lbl


//test it out
label begin
print "hello!"
goto begin
```

* Extensivity is encouraged through type extensions, which allow adding additional static methods to pre-existing types.

* Universal Function Call Syntax

* For more information, check out tests/language.an for all planned features.
    - For implemented features, check out the tests/ directory


## Installation
1. Make sure to have `llvm` version 4.0 installed.  To check which version you have, run `$ lli --version`.  To install llvm, install
the `llvm` package on your distro's package manager, eg. for Ubuntu: `$ sudo apt-get install llvm-4.0`

2. Run `$ git clone https://github.com/jfecher/ante.git`

3. Run `$ cd ante && make && sudo make stdlib`

    - NOTE: root permissions are only needed to export the standard library.  To export it manually, execute the following command as root:

        `# mkdir -p /usr/include/ante && cp stdlib/*.an /usr/include/ante`
