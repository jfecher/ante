# Ante
The compile-time language

## Features
* Compile time execution by default, no more constexpr
* Entire program dead code elimation as a result of compile time execution.
* Systems language that feels like an interpreted language.
* Support for imperative, functional, and object-oriented paradigms.
* Strongly typed with a detailed algebraic type system and type inferencing
```go
var i = 55        ~create i, a mutable 32-bit integer

~Create j, an immutable integer
let j = 0

let myTuple = (5, 5.0, "five")

~tuples can also be destructured and stored into multiple variables
i32 x y = (4, 5)

~Arrays:
var myArray = [0, 1, 2, 3, 4]

~Function pointers:
let myFunctionPtr = lambda x y -> x * y

~Sum types:
data Maybe
    Some('t) | None

var f = Some 4
```
* Significant whitespace after newlines; no tabs allowed in significant whitespace.
```go
fun myFunction
    if 3 > 2
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
data Point
    pri i32 x y

ext Point
    fun cast(): i32 x, i32 y => Point
        return Point (x, y)

    fun scale: self*, i32 sx sy
        self.x *= sx
        self.y *= sy

    fun getx => i32
        return x

var p = Point(2, 3)

~mutator functions are accessed with ':' instead of '.'
p:scale(3, 4)
if p.getx() == 6
    print("Hello World!")


~All of the above is compiled to
print("Hello World!")
```
* Explicit yet concise currying support
```
let increment = _ + 1

print(increment(4)) ~prints 5


~filter out all numbers that aren't divisible by 7
var l = List(0..100).filter(_ % 7 == 0)

```

* For more information, check out tests/language.an for all planned features.
