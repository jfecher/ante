# zy
An interpreted, statically typed language

## Features
* Zy can either interpret straight from the command line, or can be given a file
* While by default variables are dynamic, they can optionally be given a type  
```go
>dyn = 32       ~create a dynamic variable dyn, and give it the value 32  
dyn = "Test 1"  ~change dyn to a string.  
num> i = 55     ~create i, a number
i = "Test 2"    ~This line triggers a runtime error since i has a static typing  
```
* All variables can also have their type changed:  
```go
string> i = "4"  
num i    ~change i's type to int  
print i + 1  
~output: 5  
```
* Integers can also be of arbitrary length:  
```go
num>i = 2^256  
~note that the ^ above is the pow operator, it is not a binary xor  
```
* For more information, check out tests/language.zy for all planned features.
