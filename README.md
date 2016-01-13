# Ante
A compiled, optionally typed language

## Features
* Ante can either compile normally, or function as a JIT.
* Ante is optionally typed
```go
~Static typing:
i32 i = 55        ~create i, an integer
i = "Test 2"      ~This line triggers a compile-time error since i has a static typing

~Let bindings:
let t = 0
~Note: t is immutable

```
* Spaces are significant after newlines, and indentation is required
```go
if 3 > 2
    print("3 is greater than 2")
else
    print("Invalid laws of mathematics, please try again in an alternate universe")
```
* Code is evaluated, by default, at compile time.  Only functions producing output
or user specified functions, and necessary constructs are compiled into the binary.
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
