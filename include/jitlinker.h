#ifndef JITLINKER_H
#define JITLINKER_H

#include <llvm/ExecutionEngine/GenericValue.h>
#include "compiler.h"

namespace ante {
    unique_ptr<Compiler> wrapFnInModule(Compiler *c, string &basename, string &mangledName);

    GenericValue typedValueToGenericValue(Compiler *c, TypedValue *tv);
    vector<GenericValue> typedValuesToGenericValues(Compiler *c, vector<TypedValue*> &typedArgs, LOC_TY loc, string fnname);
    TypedValue* genericValueToTypedValue(Compiler *c, GenericValue gv, TypeNode *tn);
}

#endif
