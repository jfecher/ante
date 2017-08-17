#ifndef JITLINKER_H
#define JITLINKER_H

#include <llvm/ExecutionEngine/GenericValue.h>
#include "compiler.h"

namespace ante {
    std::unique_ptr<Compiler> wrapFnInModule(Compiler *c, std::string &basename, std::string &mangledName);

    llvm::GenericValue typedValueToGenericValue(Compiler *c, TypedValue *tv);
    std::vector<llvm::GenericValue> typedValuesToGenericValues(Compiler *c, std::vector<TypedValue*> &typedArgs, LOC_TY loc, std::string fnname);
    TypedValue* genericValueToTypedValue(Compiler *c, llvm::GenericValue gv, TypeNode *tn);
}

#endif
