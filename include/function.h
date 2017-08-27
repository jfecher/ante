#ifndef AN_FUNCTION_H
#define AN_FUNCTION_H

#include "types.h"
#include "compiler.h"

namespace ante {
    typedef std::vector<std::pair<TypeCheckResult&,FuncDecl*>> FunctionListTCResults;

    FunctionListTCResults filterBestMatches(Compiler *c, std::vector<std::shared_ptr<FuncDecl>> &candidates, std::vector<AnType*> args);
    TypedValue compFnWithArgs(Compiler *c, FuncDecl *fd, std::vector<AnType*> args);
}

#endif
