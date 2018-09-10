#ifndef AN_COMPAPI_H
#define AN_COMPAPI_H

#include "compiler.h"
#include "antevalue.h"

namespace ante {
    // namespace for compiler-api handling functions.
    // Actual compiler api functions such as Ante_eval are in the global namespace with the Ante_ prefix.
    namespace capi {
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

            using Arg = AnteValue const&;

            TypedValue* operator()(Compiler *c);
            TypedValue* operator()(Compiler *c, Arg tv);
            TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2);
            TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3);
            TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4);
            TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4, Arg tv5);
            TypedValue* operator()(Compiler *c, Arg tv1, Arg tv2, Arg tv3, Arg tv4, Arg tv5, Arg tv6);
        };

        /**
         * Initialize functions contained within the internal map.
         * This should be called before capi::lookup.
         */
        void init();

        /**
        * Lookup the name of a function in the list of compiler api functions.
        * If no function is found, nullptr is returned.
        */
        CtFunc* lookup(std::string const& fn);

        TypedValue create_wrapper(Compiler *c, FuncDecl *fd);
    }
}

#endif
