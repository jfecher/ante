#ifndef COMPILER_H
#define COMPILER_H

#include "llvm/ADT/STLExtras.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/Verifier.h"
#include <cctype>
#include <cstdio>
#include <map>
#include <string>
#include <vector>
#include "types.h"

#define COMP_NDEF_ERR "Variable '%s' is used but never declared.\n"

bool compile(Token);

#endif
