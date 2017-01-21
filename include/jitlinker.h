#ifndef JITLINKER_H
#define JITLINKER_H

#include "compiler.h"

unique_ptr<Compiler> wrapFnInModule(Compiler *c, Function *f);

#endif
