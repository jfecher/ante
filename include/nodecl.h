#ifndef AN_NODECL_H
#define AN_NODECL_H

#include "declaration.h"

namespace ante {
    /** Represents the absense of a concrete declaration, for
     *  example if a lambda is called it may not have had a
     *  declaration within the current scope. */
    struct NoDecl : public Declaration {
        NoDecl(parser::Node *n) : Declaration("(none)", n){}
        virtual ~NoDecl(){};
    };
}


#endif /* end of include guard: AN_NODECL_H */
