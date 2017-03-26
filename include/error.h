#ifndef AN_ERROR_H
#define AN_ERROR_H

struct Node;
#ifndef YYSTYPE
#  define YYSTYPE Node*
#endif
#include "yyparser.h"

namespace ante {

    enum ErrorType {
        Error, Warning, Note
    };

    /* General error function */
    void error(const char* msg, const yy::location& loc, ErrorType t = Error);

}

#endif
