#ifndef AN_VARIABLE_H
#define AN_VARIABLE_H

#include <string>
#include <list>
#include <optional>
#include "llvm/IR/Value.h"

#include "parser.h"
#include "typedvalue.h"
#include "declaration.h"

namespace ante {
    /**
     * A variable assigned a certain value and restricted to a given scope.
     *
     * Variables may be mutable or immutable and all keep track of each time
     * they are assigned.
     */
    struct Variable : public Declaration {
        /**
        * @brief Set to true if this variable is an implicit pointer.
        * Used by mutable variables.
        */
        bool autoDeref;


        llvm::Value* getVal() const{
            return tval.val;
        }

        TypeTag getType() const;

        /**
        * @brief Variable constructor with initial value
        *
        * Although somewhat redundant, both declaration and def parameters
        * are required in case the variable was declared in an expression
        * without a rhs, such as a function parameter or for loop.
        *
        * @param n Name of variable
        * @param tv Value of variable
        * @param s Scope of variable
        * @param def The expression this was originally declared in.
        * @param ismutable True if the variable's alloca should be autotomatically dereferenced
        */
        Variable(std::string n, TypedValue const& tv,
                parser::Node *def, bool ismutable=false)
                : Declaration(n, def), autoDeref(ismutable){

            tval = tv;
        }

        /**
        * @brief Construct an empty variable
        *
        * Although somewhat redundant, both declaration and def parameters
        * are required in case the variable was declared in an expression
        * without a rhs, such as a function parameter or for loop.
        *
        * @param n Name of variable
        * @param tv Value of variable
        * @param s Scope of variable
        * @param def The expression this was originally declared in.
        * @param ismutable True if the variable's alloca should be autotomatically dereferenced
        */
        Variable(std::string n, parser::Node *def, bool ismutable=false)
                : Declaration(n, def), autoDeref(ismutable){}


        virtual ~Variable(){};
    };
}

#endif
