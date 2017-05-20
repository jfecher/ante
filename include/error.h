#ifndef AN_ERROR_H
#define AN_ERROR_H

struct Node;
#ifndef YYSTYPE
#  define YYSTYPE Node*
#endif
#include "yyparser.h"
#include "lazystr.h"

namespace ante {

    enum class ErrorType {
        Error, Warning, Note
    };

    struct CtError {};

    struct CompilationError : public CtError {
        lazy_printer msg;
        const yy::location loc;
        CompilationError(lazy_printer m, const yy::location l) : msg(m), loc(l){}
    };

    struct IncompleteTypeError : public CtError {};

    struct TypeVarError : public CtError {};

    /* General error function */
    void error(const char* msg, const yy::location& loc, ErrorType t = ErrorType::Error);

}

#endif
