#define CATCH_CONFIG_MAIN
#include "unittest.h"

//Override some of the printing behaviour for the tests
namespace ante {
    //overide << for type bindings
    ostream& operator<<(ostream &out, pair<string, AnType*> const p){
        out << '"' << p.first << "\" -> " << anTypeToStr(p.second);
        return out;
    }

    //overide << for vectors of type bindings
    ostream& operator<<(ostream &out, vector<pair<string, AnType*>> const& vec){
        if(vec.empty()) return out << "{}";

        out << "{ " << vec[0];
        for(auto &i = ++begin(vec); i != end(vec); i++){
            out << ", " << *i;
        }
        return out << " }";
    }

    //overide << for TypeCheckResult
    ostream& operator<<(ostream &out, TypeCheckResult const& tcr){
        out << "TypeCheckResult(" << tcr.box->res << ", " << tcr.box->matches
            << ", " << tcr.box->bindings << ")" << endl;
        return out;
    }
}
