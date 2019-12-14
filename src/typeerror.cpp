#include "typeerror.h"
#include "types.h"
#include "trait.h"
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

    /**
     * Increment a string, say 'a to 'b then 'c ... 'z 'aa 'ab and so on
     */
    std::string nextLetter(std::string cur){
        // loop backwards through characters in the string to carry the 1 (a?) if needed
        // NOTE: never loops through index 0 as this is always assumed to be '\''
        for(size_t i = cur.length() - 1; i > 0; ++i){
            if(cur[i] == 'z'){
                cur[i] = 'a';
                if(i == 1)
                    return cur + 'a';
            }else{
                ++cur[i];
                break;
            }
        }
        return cur;
    }

    AnType* sanitize(AnType *t, std::unordered_map<AnTypeVarType*, AnTypeVarType*> &map, std::string &nextName);

    std::vector<AnType*> sanitizeAll(std::vector<AnType*> v, std::unordered_map<AnTypeVarType*, AnTypeVarType*> &map, std::string &nextName){
        for(size_t i = 0; i < v.size(); ++i){
            v[i] = sanitize(v[i], map, nextName);
        }
        return v;
    }

    std::vector<AnProductType*> sanitizeAll(std::vector<AnProductType*> v, std::unordered_map<AnTypeVarType*, AnTypeVarType*> &map, std::string &nextName){
        for(size_t i = 0; i < v.size(); ++i){
            v[i] = static_cast<AnProductType*>(sanitize(v[i], map, nextName));
        }
        return v;
    }

    std::vector<TraitImpl*> sanitizeAll(std::vector<TraitImpl*> v, std::unordered_map<AnTypeVarType*, AnTypeVarType*> &map, std::string &nextName){
        for(size_t i = 0; i < v.size(); ++i){
            v[i] = new TraitImpl(v[i]->name, sanitizeAll(v[i]->typeArgs, map, nextName));
        }
        return v;
    }

    /**
     * Replace numbered typevars, eg '1382 with proper names starting with 'a.
     * Keep track of already-seen typevars with map.
     */
    AnType* sanitize(AnType *t, std::unordered_map<AnTypeVarType*, AnTypeVarType*> &map, std::string &curName){
        if(!t->isGeneric)
            return t;

        if(t->isModifierType()){
            auto modTy = static_cast<AnModifier*>(t);
            return (AnType*)modTy->addModifiersTo(sanitize((AnType*)modTy->extTy, map, curName));
        }

        if(auto ptr = try_cast<AnPtrType>(t)){
            return AnPtrType::get(sanitize(ptr->extTy, map, curName));

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return AnArrayType::get(sanitize(arr->extTy, map, curName), arr->len);

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            auto it = map.find(tv);
            if(it != map.end()){
                return it->second;
            }else{
                auto newTv = AnTypeVarType::get(curName);
                map[tv] = newTv;
                curName = nextLetter(curName);
                if(tv->isRhoVar())
                    curName = curName + "...";
                return newTv;
            }

        }else if(auto dt = try_cast<AnProductType>(t)){
            auto exts = sanitizeAll(dt->fields, map, curName);;
            auto generics = sanitizeAll(dt->typeArgs, map, curName);;
            return AnProductType::createVariant(dt, exts, generics);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto exts = sanitizeAll(st->tags, map, curName);;
            auto generics = sanitizeAll(st->typeArgs, map, curName);;
            return AnSumType::createVariant(st, exts, generics);

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto exts = sanitizeAll(fn->paramTys, map, curName);;
            auto rett = sanitize(fn->retTy, map, curName);
            auto tcc  = sanitizeAll(fn->typeClassConstraints, map, curName);
            return AnFunctionType::get(rett, exts, tcc, t->typeTag == TT_MetaFunction);

        }else if(auto tup = try_cast<AnTupleType>(t)){
            auto exts = sanitizeAll(tup->fields, map, curName);;
            return AnTupleType::getAnonRecord(exts, tup->fieldNames);

        }else{
            return t;
        }
    }

    AnType* sanitize(AnType *t){
        std::unordered_map<AnTypeVarType*, AnTypeVarType*> map;
        std::string cur = "'a";
        return sanitize(t, map, cur);
    }

    /**
     * Replace $1 and $2 in the lazy_printer with the relevant types from
     * the error. This will also format the types to make the typevars
     * more human readable, eg changing '1382 to 'a
     */
    lazy_printer TypeError::decode(const AnType *a, const AnType *b) const {
        lazy_printer ret;
        lazy_str astr = anTypeToColoredStr(sanitize((AnType*)a));
        lazy_str bstr = anTypeToColoredStr(sanitize((AnType*)b));
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
