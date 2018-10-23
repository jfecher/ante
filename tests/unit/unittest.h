#ifndef AN_UNITTEST_H
#define AN_UNITTEST_H

#define NO_MAIN
#include "compiler.h"

//Override some of the printing behaviour for the tests
namespace ante {
    std::string anTypeToStr(const AnType *t);

    template<typename T, typename U>
    bool contains(T const& container, U const& elem){
        return find(begin(container), end(container), elem) != end(container);
    }

    //overide << for vectors of type bindings
    std::ostream& operator<<(std::ostream &out,
            std::vector<std::pair<std::string, ante::AnType*>> const& vec);
}


#include "catch.hpp"

#endif
