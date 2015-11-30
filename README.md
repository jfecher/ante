# Ante
A compiled, optionally typed language

## Features
* Ante can either compile normally, or function as a JIT.
* Ante is optionally typed
```go
dyn myVar = 32    ~create a dynamic variable myVar, and give it the value 32  
myVar = "Test 1"  ~set dyn to equal the string "Test 1" 
int i = 55        ~create i, an integer
i = "Test 2"      ~This line triggers a compile-time error since i has a static typing  
```
* For more information, check out tests/language.an for all planned features.
