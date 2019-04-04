#ifndef AN_TRAIT_H
#define AN_TRAIT_H

#include "funcdecl.h"

namespace ante {
    class AnTypeVarType;

    /** A type with no definition that may be instantiated to any other type */
    struct TypeFamily {
        std::string name;
        std::vector<AnType*> typeArgs;
        TypeFamily(std::string const& name, std::vector<AnType*> typeArgs)
            : name{name}, typeArgs{typeArgs}{}
    };

    /**
    * @brief Holds the name of a trait and the functions needed to implement it
    */
    struct Trait {
        std::string name;
        std::vector<std::shared_ptr<FuncDecl>> funcs;
        std::vector<TypeFamily> typeFamilies;
    };
}


#endif /* end of include guard: AN_TRAIT_H */
