#ifndef AN_NAMERESOLUTION_H
#define AN_NAMERESOLUTION_H

#include <llvm/ADT/StringMap.h>
#include "parser.h"
#include "variable.h"
#include "module.h"
#include "antype.h"

namespace ante {

    /**
     * Perform name resolution for modules.
     *
     * Annotates all VarNodes with their Variable* or their FuncDecl*.
     */
    struct NameResolutionVisitor : public NodeVisitor {
        /** The varTable is a stack of a set of scopes visible to each function.
         *  The current function is only allowed to see its own contained scopes
         *  which are all contained within the top of the stack. */
        std::stack<std::vector<llvm::StringMap<Variable*>>> varTable;

        /** Globals may be accessed from any scope but can be shadowed by any scope as well. */
        llvm::StringMap<std::unique_ptr<Variable>> globals;

        /** When this is set to true all VarNodes will be automatically declared as new variables.
         * This is used inside of match patterns. */
        bool autoDeclare = false;

        /** @brief functions and type definitions of current module */
        Module *compUnit;

        /** Construct a new NameResolutionVisitor */
        NameResolutionVisitor(std::string const& moduleName){
            compUnit = new Module(moduleName);
            enterFunction();
        }

        DECLARE_NODE_VISIT_METHODS();

        private:
            /** Declare a variable with its type unknown */
            void declare(std::string const& name, parser::VarNode *decl);
            void declare(std::string const& name, parser::NamedValNode *decl);

            /** Declare functions but do not define them */
            void declare(parser::FuncDeclNode *decl);
            void declare(parser::ExtNode *decl);

            /** Declare/Register a trait declaration */
            void declare(TraitDecl *decl, LOC_TY &loc);

            /** Declare a type with its contents unknown */
            void declareProductType(parser::DataDeclNode *n);
            void declareSumType(parser::DataDeclNode *n);

            /** Define a type with the given contents. */
            void define(std::string const& name, AnDataType *type, LOC_TY &loc);

            /** Lookup the variable name and return it if found or null otherwise */
            Variable* lookupVar(std::string const& name) const;

            /** Lookup the type by name and return it if found or null otherwise */
            TypeDecl* lookupType(std::string const& name) const;

            void searchForField(parser::BinOpNode *op);

            void validateType(const AnType *tn, const parser::DataDeclNode *decl);

            size_t getScope() const;

            void importFile(std::string const& fileName, LOC_TY &loc);

            void newScope();

            void exitScope();

            void enterFunction();

            void exitFunction();

            void visitUnionDecl(parser::DataDeclNode *decl);

            void visitTypeFamily(parser::DataDeclNode *n);

            /** A safe wrapper around toAnType that catches any exceptions and
             * sets error flags to let the compiler know it cannot continue to
             * the next phase. */
            AnType* tryToAnType(parser::TypeNode *tn);

            FuncDecl* getFunction(std::string const& name) const;

            Declaration* findCandidate(parser::Node *n) const;
    };
}

#endif
