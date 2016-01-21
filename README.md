# Ante
The compile-time language

## Features
* The compiler can either compile normally, or function as a JIT.
* Support for imperative, functional, and object-oriented paradigms.
* Strongly typed with a detailed algebraic type system
```go
i32 i = 55        ~create i, a mutable 32-bit integer

~Types can also be inferred through let bindings:
~Create j, an immutable integer
let j = 0

let myTuple = 5, 5.0, "five"
~The following is the mutable version of the above:
i32,f32,c32[] myTuple = 5, 5.0, "five"

~tuples can also be destructured and stored into multiple variables
i32 x y = 4, 5

~Arrays:
u8[5] myArray = 0, 1, 2, 3, 4

~Function pointers:
i32(i32, i32) myFunctionPtr = lambda x y -> x * y

~Or-ing types:
File|None f = None
```
* Significant whitespace
```go
void myFunction:
    if 3 > 2
        print("3 is greater than 2")
    else
        print("Invalid laws of mathematics, please try again in an alternate universe")
```
* Reference counted smart pointers by default while keeping the ability to create raw pointers
```go
i32* intPtr = new 5
let strPtr = new "msg"

~Declaration of raw pointers is accomplished with the 'raw' modifier:
raw void* ptr = malloc(10)

~intPtr is automatically freed
~voidPtr is automatically freed
free(ptr) ~ptr must be manually freed
```
* Code is evaluated, by default, at compile time.  Only functions producing output,
user specified functions or variables, and necessary constructs are compiled into the binary.
```go
data Point
    pri i32 x y

    void new: i32 x, i32 y
        return Point (x, y)

    void scale: self*, i32 sx sy
        self.x *= sx
        self.y *= sy

    i32 getx:
        return x

Point p = Point(2, 3)
p.scale(3, 4)
if p.getx() == 6
    print("Hello World!")


~All of the above is compiled to
print("Hello World!")
```
* For more information, check out tests/language.an for all planned features.
