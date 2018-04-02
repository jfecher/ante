#ifndef JITLINKER_H
#define JITLINKER_H

#include "compiler.h"

namespace ante {
    std::unique_ptr<Compiler> wrapFnInModule(Compiler *c, std::string const& basename,
            std::string const& mangledName, std::vector<TypedValue> const& args);
}

#endif
