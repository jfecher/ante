#ifndef AN_ERROR_H
#define AN_ERROR_H

namespace ante { namespace parser { struct Node; } }
#ifndef YYSTYPE
#  define YYSTYPE ante::parser::Node*
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

        CompilationError(lazy_printer m) : msg(m), loc(){}
        CompilationError(lazy_printer m, const yy::location l) : msg(m), loc(l){}
    };

    struct IncompleteTypeError : public CtError {};

    struct TypeVarError : public CtError {};

    /* General error function.  Show an error and the line it is on, and throw an exception. */
    void error(const char* msg, const yy::location& loc, ErrorType t = ErrorType::Error);

    void error(lazy_printer msg, const yy::location& loc, ErrorType t = ErrorType::Error);

    /* Show an error and the line it is on, but do not throw an exception. */
    void showError(lazy_printer msg, const yy::location& loc, ErrorType t = ErrorType::Error);
}

#endif
