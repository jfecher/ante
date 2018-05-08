#ifndef AN_ARGTUPLE_H
#define AN_ARGTUPLE_H

#include "typedvalue.h"
#include "error.h"
#include <vector>

namespace ante {
    struct Compiler;

    /** A data structure for translating between runtime
     * values and TypedValues while jitting. */
    class ArgTuple {

        public:
            /** Converts a tuple pointer into a TypedValue */
            TypedValue asTypedValue(Compiler *c) const;

            /** Returns the contained data without any conversions. */
            void* asRawData() const { return data; }

            /** Return the AnType* of the contained value. */
            AnType *getType() const { return type; }

            template<typename T>
            T castTo() const {
                if(!data)
                    throw new CompilationError("Uninitialized data in cast");
                return *(T*)data;
            }

            /** Dump the contained value to the given stream. */
            void print(Compiler *c, std::ostream &os = std::cout) const;

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

            /** Construct an ArgTuple using the given pre-initialized data. */
            ArgTuple(void *d, AnType *t) : data(d), type(t){}

            /** Constructs an empty ArgTuple representing a void literal. */
            ArgTuple() = default;


        private:
            /** Pointer to a tuple of the given data types */
            void *data;

            /** The type and value of this data. */
            AnType *type;

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

            /** Implementation of print function. */
            void printCtVal(Compiler *c, std::ostream &os) const;

            /** Implementation of print function. */
            void printTupleOrData(Compiler *c, std::ostream &os) const;

            /** Implementation of print function. */
            void printUnion(Compiler *c, std::ostream &os) const;

    }__attribute__((__packed__));
}

#endif
