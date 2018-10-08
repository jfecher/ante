#ifndef AN_TRAIT_H
#define AN_TRAIT_H

#include "funcdecl.h"

namespace ante {
    /**
    * @brief Holds the name of a trait and the functions needed to implement it
    */
    struct Trait {
        std::string name;
        std::vector<std::shared_ptr<FuncDecl>> funcs;
    };
}


#endif /* end of include guard: AN_TRAIT_H */
