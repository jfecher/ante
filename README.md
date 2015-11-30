# Ante
A compiled, optionally typed language

## Features
* Ante can either compile normally, or function as a JIT.
* Ante is optionally typed
```go
dyn myVar = 32    ~create a dynamic variable myVar, and give it the value 32  
myVar = "Test 1"  ~set dyn to equal the string "Test 1" 
i32 i = 55        ~create i, an integer
i = "Test 2"      ~This line triggers a compile-time error since i has a static typing  
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
```
class Point
    i32 x
    i32 y

    ~In the init function, instance data can be automatically set equal to
    ~parameters sharing the same identifier
    void _init: i32 x, i32 y

    void scale: i32 sx, i32 sy
        x *= sx
        y *= sy

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
