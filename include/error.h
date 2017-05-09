#ifndef AN_ERROR_H
#define AN_ERROR_H

struct Node;
#ifndef YYSTYPE
#  define YYSTYPE Node*
#endif
#include "yyparser.h"
#include "lazystr.h"

namespace ante {

    enum ErrorType {
        Error, Warning, Note
    };

    struct CompilationError {
        lazy_printer msg;
        const yy::location loc;
        CompilationError(lazy_printer m, const yy::location l) : msg(m), loc(l){}
    };

    /* General error function */
    void error(const char* msg, const yy::location& loc, ErrorType t = Error);

}

#endif
