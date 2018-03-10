#ifndef AN_UNITTEST_H
#define AN_UNITTEST_H

#define NO_MAIN
#include "compiler.h"

//Override some of the printing behaviour for the tests
namespace ante {
    std::string anTypeToStr(const AnType *t);

    //overide << for type bindings
    std::ostream& operator<<(std::ostream &out, std::pair<std::string, ante::AnType*> const p);

    //overide << for vectors of type bindings
    std::ostream& operator<<(std::ostream &out,
            std::vector<std::pair<std::string, ante::AnType*>> const& vec);

    //overide << for TypeCheckResult
    std::ostream& operator<<(std::ostream &out, TypeCheckResult const& tcr);

    template<typename T, typename U>
    bool contains(T const& container, U const& elem){
        return find(begin(container), end(container), elem) != end(container);
    }
}

#include "catch.hpp"

using namespace ante;
using namespace std;
#endif
