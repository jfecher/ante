#ifndef AN_COMPAPI_H
#define AN_COMPAPI_H

#include "antype.h"

namespace ante {
    /**
     * Holds a c++ function.
     *
     * Used to represent compiler API functions and call them
     * with compile-time constants as arguments
     */
    struct CtFunc {
        void *fn;
        std::vector<AnType*> params;
        AnType* retty;

        size_t numParams() const { return params.size(); }
        bool typeCheck(std::vector<AnType*> &args);
        bool typeCheck(std::vector<TypedValue&> &args);
        CtFunc(void* fn);
        CtFunc(void* fn, AnType *retTy);
        CtFunc(void* fn, AnType *retTy, std::vector<AnType*> params);

        ~CtFunc(){}

        using Arg = TypedValue const&;

        TypedValue* operator()(Compiler *c);
        TypedValue* operator()(Compiler *c, Arg tv);
        TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2);
        TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3);
        TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4);
        TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4, Arg tv5);
        TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4, Arg tv5, Arg tv6);
    };
}

#endif
