#ifndef AN_REPL_H
#define AN_REPL_H

#include "compiler.h"

namespace ante {
    /**
     * Merge the contents of rn into the current RootNode
     * of the compiler and update declarations accordingly
     */
    TypedValue mergeAndCompile(Compiler *c, parser::RootNode *rn,
            parser::ModNode *anteExpr);

    /**
     * Starts the read-eval printline loop.
     *
     * Expects the given Compiler to already contain a main
     * function or another valid insert point.
     */
    void startRepl(Compiler *c);
}

#endif
