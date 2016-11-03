# Ante
The compile-time language

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
```go
//create i, a mutable integer
var i = 55

//Create j, an immutable integer
let j = 0

let myTuple = (5, 5.0, "five")

//tuples can also be destructured and stored into multiple variables
let (x, y) = (4, 5)

//Arrays:
var myArray = [0, 1, 2, 3, 4]

//Return type inference:
fun add: i32 x y = x + y

//Sum types:
type Maybe =
   | Some 't
   | None

var f = Some 4
f = None


//pattern matching
match parse_int "0g" with
| Some n -> print n
| None -> ()

```
* Significant whitespace after newlines; no tabs allowed in significant whitespace.
```go
fun myFunction:
    if 3 > 2 then
        print "3 is greater than 2"
    else
        print "Invalid laws of mathematics, please try again in an alternate universe"
```
* Reference counted smart pointers by default while keeping the ability to create raw pointers
* Unique pointers used whenever possible automatically
* No more memory hassle trying to find cycles with pointers, everything is done by the compiler
* No garbage collector
```go
let intPtr = new 5
let strPtr = new "msg"

//Declaration of raw pointers is accomplished with the 'raw' modifier:
let raw myPtr = malloc 10

//intPtr is automatically freed
//strPtr is automatically freed
free myPtr //myPtr must be manually freed
```
* API designers given full reign to implement custom rules for their types, full access to the
parse tree is provided, along with a quick list of the uses of the variable in question.
* Programmers have just as much power over their program as the compiler does.  As an example,
here is an implementation of the goto construct in Ante
```go
#![macro]
fun goto: VarNode vn
    let label = ctLookup vn ?
        None -> compErr "Cannot goto undefined label {vn.name}"

    LLVM.builder.SetInsertPoint <| getCallSiteBlock()
    LLVM.builder.CreateBr label

#![macro]
fun label: VarNode vn
    let ctxt = LLVM.getGlobalContext()
    let callingFn = getCallSiteBlock().getParentFn()
    let lbl = LLVM.BasicBlock ctxt callingFn
    ctStore vn lbl



//test it out
label begin
print "hello!"
goto begin
```

* Here is an example implementation of a thread that 'owns' the objects inside its function
```Rust
type MyThread = 'f fn, Pid pid

ext MyThread
    fun run: self*
        self.pid = Thread.exec self.fn


    //Compile time function that runs whenever MyThread is created
    pri fun handleInputs(onCreation): self
        //get a list of all mutable variables used
        let vars = 
            Ante.getVarsInFn self.fn 
            .unwrap()
            .filter _.isMutable

        //Store them compile-time for later use in the cleanup function
        Ante.ctStore vars
        
        //Iterate through each variable and invalidate them
        vars.iter Ante.invalidate


    pri fun cleanup(onDeletion): self
        let vars = (Ante.ctLookup "vars").unwrap()
        vars.iter Ante.revalidate
```
* Explicit yet concise currying support
```go
let increment = _ + 1

print(increment 4) //prints 5

let f = _ + increment _

f 3 |> print
//output: 7

//filter out all numbers that aren't divisible by 7
let l = List(0..100):filter(_ % 7 == 0)

```

* For more information, check out tests/language.an for all planned features.


## Installation
1. Make sure to have `llvm` version >= 3.6 installed.  To check which version you have, run `$ lli --version`.  To install llvm, install the `llvm` package on your distro's package manager, eg. for Ubuntu: `$ sudo apt-get install llvm`

2. Run `$ git clone https://github.com/jfecher/ante.git`

3. Run `$ cd ante && make && sudo make stdlib`

    - NOTE: root permissions are only needed to export the standard library.  To export it manually, execute the following command as root:

        `# mkdir -p /usr/include/ante && cp stdlib/*.an /usr/include/ante`
