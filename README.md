# Ante
The compile-time language

## Features
* Compile time execution by default, no more constexpr
* Entire program dead code elimation as a result of compile time execution
* Systems language that feels like an interpreted language
* Expression-based syntax, no statements
* Support for imperative, functional, and object-oriented paradigms
* Strongly typed with a detailed algebraic type system and type inferencing
```go
var i = 55        ~create i, a mutable 32-bit integer

~Create j, an immutable integer
let j = 0

let myTuple = (5, 5.0, "five")

~tuples can also be destructured and stored into multiple variables
let (x, y) = (4, 5)

~Arrays:
var myArray = [0, 1, 2, 3, 4]

~Function pointers:
let myFunctionPtr =
    fun x y = x * y

~Sum types:
type Maybe =
    Some 't | None

var f = Some 4
```
* Significant whitespace after newlines; no tabs allowed in significant whitespace.
```go
fun myFunction:
    if 3 > 2 then
        print("3 is greater than 2")
    else
        print("Invalid laws of mathematics, please try again in an alternate universe")
```
* Reference counted smart pointers by default while keeping the ability to create raw pointers
* Unique pointers used whenever possible automatically
* No more memory hassle trying to find cycles with pointers, everything is done by the compiler
* No garbage collector
```go
let intPtr = new 5
let strPtr = new "msg"

~Declaration of raw pointers is accomplished with the 'raw' modifier:
let raw myPtr = malloc(10)

~intPtr is automatically freed
~strPtr is automatically freed
free(myPtr) ~myPtr must be manually freed
```
* Code is evaluated, by default, at compile time.  Only functions producing output,
user specified functions or variables, and necessary constructs are compiled into the binary.
```go
type Point = i32 x y

ext Point
    fun scale: self*, i32 sx sy
        self.x *= sx
        self.y *= sy

    ~Return type inference with = 
    fun getx: = x


var p = Point(2, 3)

~mutator functions are accessed with ':' instead of '.'
p:scale(3, 4)
if p.getx() == 6 then
    print("Hello World!")


~All of the above is compiled to
print("Hello World!")
```
* Explicit yet concise currying support
```go
let increment = _ + 1

print(increment(4)) ~prints 5


~filter out all numbers that aren't divisible by 7
let l = List(0..100).filter(_ % 7 == 0)

```

* For more information, check out tests/language.an for all planned features.


## Installation
1. Make sure to have `llvm` version >= 3.6 installed.  To check which version you have, run `$ lli --version`.  To install llvm, install the `llvm` package on your distro's package manager, eg. for Ubuntu: `$ sudo apt-get install llvm`

2. Run `$ git clone https://github.com/jfecher/ante.git`

3. Run `$ cd ante && make && sudo make stdlib`

    - NOTE: root permissions are only needed to export the standard library.  To export it manually, execute the following command as root:

        `# mkdir -p /usr/include/ante && cp stdlib/*.an /usr/include/ante`
