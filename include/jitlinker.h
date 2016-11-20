#ifndef JITLINKER_H
#define JITLINKER_H

#include "compiler.h"

Module* wrapFnInModule(Compiler*, Function*);
void linkFunction(Compiler *c, Function *f, Module *mod);


#endif
