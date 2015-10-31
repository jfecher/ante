# zy
An interpreted, statically typed language

## Features
* Zy can either interpret straight from the command line, or can be given a file
* While by default variables are dynamic, they can optionally be given a type  
```go
dyn myVar = 32    ~create a dynamic variable myVar, and give it the value 32  
myVar = "Test 1"  ~set dyn to equal the string "Test 1" 
int i = 55        ~create i, an integer
i = "Test 2"      ~This line triggers a runtime error since i has a static typing  
```
* All primitive variables can also have their type changed:  
```go
str i = "4"
int i
print i + 1
~output: 5
```
* Zy also supports arbitrary precision arithmetic through gmp: 
```go
int e = 2^16 + 1
~note that the ^ above is the pow operator, it is not a binary xor
print 999999^e
```
* For more information, check out tests/language.zy for all planned features.
