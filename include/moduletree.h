#ifndef AN_MODULE_TREE
#define AN_MODULE_TREE

#include <string>
#include <memory>
#include <optional>
#include <llvm/ADT/StringMap.h>
#include "module.h"

namespace ante {

    /**
     * A virtual filesystem tree data structure for modules.
     *
     * Unlike the actual file system, this tree has several roots,
     * including the PWD as well as the location of any included
     * libraries, including Ante's standard library.  This has the
     * effect of merging each of these directories into a single
     * larger root directory.  In addition, not all modules within
     * the tree encompass a whole file/directory.  Instead, some
     * may be submodules within the same file as their parent module.
     */
    class ModuleTree {
        const std::string name;

        /** The compiled module.  Will be null if the module
         *  has not yet been compiled or is a directory. */
        std::unique_ptr<Module> module;

        /** The submodules of the current node */
        llvm::StringMap<ModuleTree> children;

        public:
            ModuleTree(std::string const& name) : name{name} {}
            ~ModuleTree() = default;

            /** Return the root of the virtual file/module system. */
            static ModuleTree& getRoot();

            std::string const& getName() const;

            /** Return the optional if available, or nullptr otherwise. */
            Module* getModule() const;

            /** Set the module for the current node.  Overrides any pre-existing module. */
            void setModule(Module *module);

            /** Find a single direct child with the given name */
            llvm::StringMap<ModuleTree>::iterator findChild(std::string const& name);

            /** Find a child with the given relative path from the current node. */
            template<class StringIt>
            llvm::StringMap<ModuleTree>::iterator findPath(StringIt path);

            /**
             * Return children.end().
             *
             * Can be used to check if returned iterators from the find methods are valid.
             */
            llvm::StringMap<ModuleTree>::iterator childrenEnd();

            /** Add a single direct child with the given name. */
            ModuleTree& addChild(std::string const& childName);

            /** Add a child at the given relative path, adding any intermediate children as necessary. */
            template<class StringIt>
            ModuleTree& addPath(StringIt childPath);
    };
}

#endif /* ifndef AN_MODULE_TREE */
