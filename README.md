# Ante
The compile-time language

Ante is a compiled systems language focusing on providing extreme extensibility through
the use of a compile-time API.  Using such an API, compiler extensions can be created
within the program itself, allowing for the addition of a garbage collector, ownership
system, etc, all in a normal library without requiring any changes to the compiler itself.

Systems languages can traditionally be a pain to write.  To fix this, Ante provides high-level
solutions such as string interpolation, smart pointers, and pattern matching, while maintaining
the ability to interact at a lower level if needed.

## Community
- Join the official subreddit at [/r/ante](https://www.reddit.com/r/ante) for any and all discussion.  Everyone is welcome!

## Features
* Systems language that feels like an interpreted language
* Expression-based syntax, no statements
* Support for functional, imperative, and object-oriented paradigms
* Strongly typed with a detailed algebraic type system and type inferencing
* Full control given to users allowing them to specify specific requirements for a given
type and issue a compile-time error if it is invalidated
    -  Extremely diverse and powerful compile-time analysis that can be custom programmed into
any datatype creating eg. iterator invalidation, pointer-autofree, or even an ownership system.
The implementation of these features resembles that of a compiler plugin, except that it is written
into the compiled module itself.
    - These compile-time functions are checked at compile-time and not compiled into the binary.
    - Ability to write compiler plugins within the compiled program itself
* Module system allowing the setting of compiler flags on a per-module basis.
* Immutability by default
* Type inferencing
* Significant whitespace after newlines
    - No tabs are allowed in significant whitespace
```go
fun myFunction:
    if 3 > 2 then
        print "3 is greater than 2"
    else
        print "Invalid laws of mathematics, please try again in an alternate universe"
```
* Reference counted smart pointers by default while keeping the ability to create raw pointers
    - Unique pointers used whenever possible automatically
    - No more memory hassle trying to find cycles with pointers, everything is done by the compiler
    - No garbage collector
```go
let intPtr = new 5
let strPtr = new "msg"

//Declaration of raw pointers is accomplished with the 'raw' modifier:
let raw myPtr = malloc 10

//intPtr is automatically freed
//strPtr is automatically freed
free myPtr //myPtr must be manually freed
```
* String interpolation
```
let person = "Joe"
let age = 44

print "${person} is ${age} years old."

//any expression can be within ${ }
print "Half of ${person}'s age is ${age / 2}"
```

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

* Here is an example implementation of a thread that 'owns' the mutable objects inside its function
```Rust
fun SafeThread.run: void->void fn
    //Import the Ante module for compile-time operations
    import Ante

    //Get a list of all mutable variables used in fn
    let vars =
        Ante.getVarsInFn fn
        .unwrap()
        .filter isMutable

    //Invalidate the further use of each variables
    //NOTE: this does not invalidate their use in fn since Ante
    //      uses eager evaluation and so fn is already compiled 
    //      by the time this function is called.
    vars.iter Ante.invalidate


    //actually run the function
    //all other operations in SafeThread.run, such as Ante.getVarsInFn,
    //are compile-time only and are not included in the binary.
    fn()

    //Function ran, variables are not revalidated because their
    //ownership was transfered to fn
```
* Extensivity is encouraged through type extensions, which allow adding additional static methods to pre-existing types.
* Universal Function Call Syntax
```rust
//add some methods to the Str type
ext Str
    fun reverse: Str s -> Str
        var ret = ""
        for i in reverse(0 .. s.len) do
            ret ++= s#i
        ret

print( "!dlrow olleh".reverse() ) //outputs hello world!
print( reverse "!dlrow olleh" )   //outputs hello world!

//Module Inferencing:
Str.reverse "my str" == reverse "my str"
```

* For more information, check out tests/language.an for all planned features.


## Installation
1. Make sure to have `llvm` version >= 3.9 installed.  To check which version you have, run `$ lli --version`.  To install llvm, install the `llvm` package on your distro's package manager, eg. for Ubuntu: `$ sudo apt-get install llvm`

2. Run `$ git clone https://github.com/jfecher/ante.git`

3. Run `$ cd ante && make && sudo make stdlib`

    - NOTE: root permissions are only needed to export the standard library.  To export it manually, execute the following command as root:

        `# mkdir -p /usr/include/ante && cp stdlib/*.an /usr/include/ante`
