#ifndef AN_TYPEDVALUE_H
#define AN_TYPEDVALUE_H

#include "llvm/IR/Value.h"

namespace ante {
    class AnType;

    /**
    * @brief A Value* and TypeNode* pair
    *
    * This is the main type used to represent a value in Ante
    */
    struct TypedValue {
        llvm::Value *val;
        AnType *type;

        TypedValue() : val(nullptr), type(nullptr){}
        TypedValue(llvm::Value *v, AnType *ty) : val(v), type(ty){}

        bool operator!() const{ return !type || !val; }

        explicit operator bool() const{ return type; }

        llvm::Type* getType() const{ return val->getType(); }

        /**
        * @brief Prints type and value to stdout
        */
        void dump() const;
    };
}

#endif
