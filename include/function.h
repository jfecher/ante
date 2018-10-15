#ifndef AN_FUNCTION_H
#define AN_FUNCTION_H

#include "types.h"
#include "compiler.h"

namespace ante {
    TypedValue compFnWithArgs(Compiler *c, FuncDecl *fd, std::vector<AnType*> args);

    bool isCompileTimeFunction(TypedValue &tv);
    bool isCompileTimeFunction(AnType *fty);

    llvm::Type* parameterize(Compiler *c, AnType *t);
    bool implicitPassByRef(AnType* t);

    void moveFunctionBody(llvm::Function *src, llvm::Function *dest);
}

#endif
