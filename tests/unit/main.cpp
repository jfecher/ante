#define CATCH_CONFIG_MAIN
#include "unittest.h"

//Override some of the printing behaviour for the tests
namespace ante {
    //overide << for vectors of type bindings
    std::ostream& operator<<(std::ostream &out, std::vector<ante::TypeBinding> const& vec){
        if(vec.empty()) return out << "{}";

        out << "{ " << vec[0];
        for(auto &i = ++begin(vec); i != end(vec); i++){
            out << ", " << *i;
        }
        return out << " }";
    }

    //overide << for TypeCheckResult
    std::ostream& operator<<(std::ostream &out, ante::TypeCheckResult const& tcr){
        out << "TypeCheckResult(" << tcr.box->res << ", " << tcr.box->matches
            << ", " << tcr.box->bindings << ")" << std::endl;
        return out;
    }
}
