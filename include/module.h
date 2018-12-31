#ifndef AN_MODULE_H
#define AN_MODULE_H

#include <string>
#include <memory>
#include "trait.h"
#include "antype.h"

namespace ante {
    /**
     * A virtual filesystem tree node containing information on
     * types, functions, imports, and traits of the current module.
     *
     * Unlike the actual file system, this tree has several roots,
     * including the PWD as well as the location of any included
     * libraries, including Ante's standard library.  This has the
     * effect of merging each of these directories into a single
     * larger root directory.  In addition, not all modules within
     * the tree encompass a whole file/directory.  Instead, some
     * may be submodules within the same file as their parent module.
     */
    struct Module {
        std::string name;

        /**
         * @brief All imports of the module
         */
        std::vector<Module*> imports;

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

        private:
        /** The submodules of the current node */
        llvm::StringMap<Module> children;

        public:
            Module(std::string const& name) : name{name} {}
            ~Module() = default;

            /** Return the root of the virtual file/module system. */
            static Module& getRoot();

            /** Find a single direct child with the given name */
            llvm::StringMap<Module>::iterator findChild(std::string const& name);


            /**
             * Return children.end().
             *
             * Can be used to check if returned iterators from the find methods are valid.
             */
            llvm::StringMap<Module>::iterator childrenEnd();

            /** Add a single direct child with the given name. */
            Module& addChild(std::string const& childName);

            /** Find a child with the given relative path from the current node. */
            template<class StringIt>
            llvm::StringMap<Module>::iterator findPath(StringIt path) {
                Module *node = this;
                llvm::StringMap<Module>::iterator ret = children.end();

                for(std::string const& name : path){
                    ret = node->findChild(name);
                    if(ret == node->childrenEnd()){
                        return children.end();
                    }
                }
                return ret;
            }

            /** Add a child at the given relative path, adding any intermediate children as necessary. */
            template<class StringIt>
            Module& addPath(StringIt path){
                Module *node = this;
                for(std::string const& name : path){
                    auto child = node->findChild(name);
                    if(child == node->childrenEnd()){
                        node = &node->addChild(name);
                    }else{
                        node = &child->getValue();
                    }
                }
                return *node;
            }
    };


    /**
     * Provides an iterator over a substrings of a file path.
     * each item is separated by the OS-specific file separater character.
     */
    struct ModulePath
    {
        typedef ModulePath iterator;
        typedef std::string const& reference;
        typedef std::string const* pointer;

#ifdef _WIN32
    #define AN_PATH_SEPARATOR '\\'
#else
    #define AN_PATH_SEPARATOR '/'
#endif

        std::string const& s;
        std::string substr;
        std::string::size_type prev, cur;

    public:
        ModulePath(std::string const& s)
            : s{s}, prev{0}, cur{0} {}
        ~ModulePath() = default;

        iterator begin() const { return *this; }
        iterator end() const { ModulePath r{s}; r.cur = s.length(); return r; }

        iterator  operator++(int) /* postfix */         {
            iterator i = *this;
            prev = cur;
            while(cur < s.length() && s[cur] != AN_PATH_SEPARATOR)
                ++cur;
            substr = s.substr(prev, cur - prev);
            return i;
        }
        iterator& operator++()    /* prefix */          {
            prev = cur;
            while(cur < s.length() && s[cur] != AN_PATH_SEPARATOR)
                ++cur;
            substr = s.substr(prev, cur - prev);
            return *this;
        }
        reference operator* () const                    { return substr; }
        pointer   operator->() const                    { return &substr; }
        bool      operator==(const iterator& rhs) const { return s == rhs.s && cur == rhs.cur; }
        bool      operator!=(const iterator& rhs) const { return !(*this == rhs); }
    };


}


#endif /* end of include guard: AN_MODULE_H */

