#ifndef JITLINKER_H
#define JITLINKER_H

#include "compiler.h"

namespace ante {
    unique_ptr<Compiler> wrapFnInModule(Compiler *c, string &basename, string &mangledName);
}

#endif
