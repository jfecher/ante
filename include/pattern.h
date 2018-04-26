#ifndef AN_PATTERN_H
#define AN_PATTERN_H

#include "parser.h"
#include "compiler.h"

namespace ante {
    void handlePattern(CompilingVisitor &cv, parser::MatchNode *n, parser::Node *pattern,
            llvm::BasicBlock *jmpOnFail, TypedValue valToMatch);
}

#endif
