#ifndef JITLINKER_H
#define JITLINKER_H

#include "compiler.h"

llvm::Module* wrapFnInModule(Compiler*, Function*);

#endif
