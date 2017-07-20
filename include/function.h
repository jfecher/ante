#ifndef AN_FUNCTION_H
#define AN_FUNCTION_H

#include "types.h"
#include "compiler.h"

namespace ante {
    typedef vector<pair<TypeCheckResult,FuncDecl*>> FunctionListTCResults;

    FunctionListTCResults filterBestMatches(Compiler *c, vector<shared_ptr<FuncDecl>> candidates, vector<TypeNode*> args);
    TypedValue* compFnWithArgs(Compiler *c, FuncDecl *fd, vector<TypeNode*> args);
}

#endif
