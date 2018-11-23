#ifndef AN_FUNCDECL_H
#define AN_FUNCDECL_H

#include "parser.h"
#include "typedvalue.h"
#include "declaration.h"

namespace ante {
    struct Module;

    class AnFunctionType;

    /**
    * @brief Contains information about a function that is not contained
    * within its FuncDeclNode.
    *
    * Holds the scope the function was compiled in, the value of the function
    * so it is not recompiled, the object type if it is a method along with any
    * generic parameters of the object, the module compiled in, and each return
    * instance for type checking.
    */
    struct FuncDecl : public Declaration {
        AnType *obj;

        AnFunctionType *type;

        Module *module;
        std::vector<std::pair<TypedValue,LOC_TY>> returns;

        parser::FuncDeclNode* getFDN() const noexcept {
            return static_cast<parser::FuncDeclNode*>(this->definition);
        }

        LOC_TY& getLoc() const noexcept {
            return getFDN()->loc;
        }

        /** Gets the unmangled name. */
        const std::string& getName() const noexcept {
            return getFDN()->name;
        }

        const std::string& getMangledName() const noexcept {
            return name;
        }

        /**
         * True if this is a declaration with no definition.
         *
         * This indicates the function is extern and (usually) C FFI.
         */
        bool isDecl() const noexcept {
            return getFDN()->name.back() == ';' or this->name == getFDN()->name;
        }

        TypedValue getOrCompileFn(Compiler *c);

        bool isFuncDecl() override {
            return true;
        }

        FuncDecl(parser::FuncDeclNode *fn, std::string const& n, Module *mod, TypedValue f)
            : Declaration(n, fn), type(0), module(mod), returns(){ tval = f; }
        FuncDecl(parser::FuncDeclNode *fn, std::string &n, Module *mod)
            : Declaration(n, fn), type(0), module(mod), returns(){}
        virtual ~FuncDecl(){}
    };
}

#endif
