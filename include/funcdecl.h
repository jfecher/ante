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
        /** object of this method, if available.  Currently unused */
        AnType *obj;

        /** The type of this function */
        AnFunctionType *type;

        /** The module this function was declared in */
        Module *module;

        /** Each return of this function.  TODO: remove */
        std::vector<std::pair<TypedValue,LOC_TY>> returns;

        /** True if this is a decl from a trait, used as a flag to swap with impl later */
        bool traitFuncDecl = false;

        parser::FuncDeclNode* getFDN() const noexcept {
            return static_cast<parser::FuncDeclNode*>(this->definition);
        }

        LOC_TY& getLoc() const noexcept {
            return getFDN()->loc;
        }

        /** Gets the unmangled name. */
        const std::string& getName() const noexcept {
            return name;
        }

        /**
         * True if this is a declaration with no definition.
         *
         * This indicates the function is extern and (usually) C FFI.
         */
        bool isDecl() const noexcept {
            return getFDN()->name.back() == ';' || this->name == getFDN()->name;
        }

        virtual bool isFuncDecl() const override {
            return true;
        }

        virtual bool isTraitFuncDecl() const override {
            return traitFuncDecl;
        }

        FuncDecl(parser::FuncDeclNode *fn, std::string const& n, Module *mod, TypedValue f)
            : Declaration(n, fn), type(0), module(mod), returns(){ tval = f; }
        FuncDecl(parser::FuncDeclNode *fn, std::string const& n, Module *mod)
            : Declaration(n, fn), type(0), module(mod), returns(){}
        virtual ~FuncDecl(){}
    };
}

#endif
