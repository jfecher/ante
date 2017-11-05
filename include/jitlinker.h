#ifndef JITLINKER_H
#define JITLINKER_H

#include "compiler.h"

namespace ante {
    std::unique_ptr<Compiler> wrapFnInModule(Compiler *c, std::string &basename, std::string &mangledName);
}

#endif
