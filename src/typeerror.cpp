#include "typeerror.h"
#include "types.h"
#include <tuple>

namespace ante {
    // Helper type to store whether a split happened and store each split part
    struct Split {
        bool split_occurred;
        lazy_str a, b, c, d, e;

        Split(lazy_str const& a, lazy_str const& b_, lazy_str const& c,
                lazy_str const& d, lazy_str const& e)
            : split_occurred{!b_.s.empty()}, a{a}, b{b_}, c{c}, d{d}, e{e}{}
    };

    Split replace(lazy_str const& str, lazy_str const& replacement1, lazy_str const& replacement2){
        size_t i = str.s.find("$1");
        size_t j = str.s.find("$2");
        size_t npos = std::string::npos;

        if(i == npos && j == npos){
            return {str, "", "", "", ""};
        }else if(i != npos && j == npos){
            std::string prefix = str.s.substr(0, i);
            std::string suffix = str.s.substr(i+2);
            return {prefix, replacement1, suffix, "", ""};
        }else if(i == npos && j != npos){
            std::string prefix = str.s.substr(0, j);
            std::string suffix = str.s.substr(j+2);
            return {prefix, replacement2, suffix, "", ""};
        }else if(i < j){
            std::string prefix = str.s.substr(0, i);
            std::string mid = str.s.substr(i+2, j - (i+2));
            std::string suffix = str.s.substr(j+2);
            return {prefix, replacement1, mid, replacement2, suffix};
        }else{ // i > j, they should never be equal
            std::string prefix = str.s.substr(0, j);
            std::string mid = str.s.substr(j+2, i - (j+2));
            std::string suffix = str.s.substr(i+2);
            return {prefix, replacement2, mid, replacement1, suffix};
        }
    }

    lazy_printer TypeError::decode(const AnType *a, const AnType *b) const {
        lazy_printer ret;
        lazy_str astr = anTypeToColoredStr(a);
        lazy_str bstr = anTypeToColoredStr(b);
        for(auto &s : this->encoded_msg.strs){
            Split split = replace(s, astr, bstr);
            if(split.split_occurred){
                ret.strs.push_back(split.a);
                ret.strs.push_back(split.b);
                ret.strs.push_back(split.c);
                ret.strs.push_back(split.d);
                ret.strs.push_back(split.e);
            }else{
                ret.strs.push_back(split.a);
            }
        }
        return ret;
    }
}
