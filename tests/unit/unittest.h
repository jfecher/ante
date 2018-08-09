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

    std::string to_string(ante::TypeBinding const& b);

    //overide << for vectors of type bindings
    std::ostream& operator<<(std::ostream &out,
            std::vector<std::pair<std::string, ante::AnType*>> const& vec);

    //overide << for TypeCheckResult
    std::ostream& operator<<(std::ostream &out, ante::TypeCheckResult const& tcr);
}


#include "catch.hpp"

using namespace ante;
using namespace std;
#endif
