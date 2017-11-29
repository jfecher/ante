#ifndef AN_ARGTUPLE_H
#define AN_ARGTUPLE_H
    
#include "compiler.h"

namespace ante {

    /** A data structure for translating between c++ values
     * and TypedValues while jitting. */
    class ArgTuple {

        public:
            /** Converts a tuple pointer into a TypedValue */
            TypedValue asTypedValue() const { return tval; }

            /** Returns the contained data without any conversions. */
            void* asRawData() const { return data; }

            /**
             * Constructs an ArgTuple from the given TypedValue arguments.
             *  - Assumes each Value* within each argument is a Constant*
             */
            ArgTuple(Compiler *c, std::vector<TypedValue> &val);

            /**
             * Constructs an ArgTuple of a single value from the given argument.
             *  - Assumes the Value* within val is a Constant*
             */
            ArgTuple(Compiler *c, TypedValue &val);

            /** Constructs an ArgTuple using the given pre-initialized data. */
            ArgTuple(Compiler *c, void *data, AnType *type);

            /** Constructs an empty ArgTuple representing a void literal. */
            ArgTuple();


        private:
            /** Pointer to a tuple of the given data types */
            void *data;

            /** The type and value of this data. */
            TypedValue tval;

            /** Stores pointer value of a constant pointer type */
            void storePtr(Compiler *c, TypedValue &tv);

            /** Allocates space then calls storeValue */
            void allocAndStoreValue(Compiler *c, TypedValue &tv);

            /** Stores a tuple value in data */
            void storeTuple(Compiler *c, TypedValue &tup);

            /**
             * Converts the given TypedValue into its corresponding
             * value in c++ and stores it in this->data
             */
            void storeValue(Compiler *c, TypedValue &tv);
    };
    
    
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

        TypedValue* operator()(Compiler *c);
        TypedValue* operator()(Compiler *c, TypedValue &tv);
        TypedValue* operator()(Compiler *c, TypedValue &tv1, TypedValue &tv2);
    };
}

#endif
