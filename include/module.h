#ifndef AN_MODULE_H
#define AN_MODULE_H

#include <string>
#include <memory>
#include "trait.h"
#include "antype.h"

namespace ante {
    /**
     * @brief An Ante Module
     */
    struct Module {
        std::string name;

        /**
         * @brief The abstract syntax tree representing the contents of the module.
         */
        std::unique_ptr<parser::RootNode> ast;

        /**
         * @brief Each declared function in the module
         */
        llvm::StringMap<FuncDecl*> fnDecls;

        /**
         * @brief Each declared DataType in the module
         */
        llvm::StringMap<AnDataType*> userTypes;

        /**
         * @brief Map of all declared traits; not including their implementations for a given type
         * Each DataType is reponsible for holding its own trait implementations
         */
        llvm::StringMap<std::shared_ptr<Trait>> traits;

        /**
        * @brief Merges two modules
        *
        * @param m module to merge into this
        * @param loc The location of the import node in case a name is duplicated
        */
        void import(Module *m, LOC_TY &loc);
    };
}


#endif /* end of include guard: AN_MODULE_H */

