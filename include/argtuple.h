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
            ArgTuple(Compiler *c, std::vector<TypedValue> const& val);

            /**
             * Constructs an ArgTuple of a single value from the given argument.
             *  - Assumes the Value* within val is a Constant*
             */
            ArgTuple(Compiler *c, TypedValue const& val);

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
            void storePtr(Compiler *c, TypedValue const& tv);

            /** Allocates space then calls storeValue */
            void allocAndStoreValue(Compiler *c, TypedValue const& tv);

            /** Stores a tuple value in data */
            void storeTuple(Compiler *c, TypedValue const& tup);

            /** Stores a primitive integer, character, or boolean. */
            void storeInt(Compiler *c, TypedValue const& tv);

            /** Stores a primitive floating-point value. */
            void storeFloat(Compiler *c, TypedValue const& tv);

            /**
             * Converts the given TypedValue into its corresponding
             * value in c++ and stores it in this->data
             */
            void storeValue(Compiler *c, TypedValue const& tv);
    };
}

#endif
