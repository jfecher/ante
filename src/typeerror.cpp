#include "typeerror.h"
#include "types.h"
#include "trait.h"
#include <tuple>

namespace ante {
    // Helper type to store whether a split happened and store each split part
    struct Split {
        bool split_occurred;
        lazy_str a;
        lazy_printer b;
        lazy_str c;
        lazy_printer d;
        lazy_str e;

        Split(lazy_str const& a, lazy_printer const& b_, lazy_str const& c,
                lazy_printer const& d, lazy_str const& e)
            : split_occurred{!b_.strs.empty()}, a{a}, b{b_}, c{c}, d{d}, e{e}{}
    };

    Split replace(lazy_str const& str, lazy_printer const& replacement1, lazy_printer const& replacement2){
        size_t i = str.s.find("$1");
        size_t j = str.s.find("$2");
        size_t npos = std::string::npos;

        if(i == npos && j == npos){
            return {str, {}, "", {}, ""};
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

    TraitImpl* sanitize(TraitImpl* v, std::unordered_map<AnTypeVarType*, AnTypeVarType*> &map, std::string &nextName){
        return new TraitImpl(v->name, sanitizeAll(v->typeArgs, map, nextName));
    }

    std::vector<TraitImpl*> sanitizeAll(std::vector<TraitImpl*> v, std::unordered_map<AnTypeVarType*, AnTypeVarType*> &map, std::string &nextName){
        for(size_t i = 0; i < v.size(); ++i){
            v[i] = sanitize(v[i], map, nextName);
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
                auto newTv = AnTypeVarType::get(tv->isRhoVar() ? (curName + "...") : curName);
                map[tv] = newTv;
                curName = nextLetter(curName);
                return newTv;
            }

        }else if(auto dt = try_cast<AnProductType>(t)){
            auto exts = sanitizeAll(dt->fields, map, curName);
            auto generics = sanitizeAll(dt->typeArgs, map, curName);
            return AnProductType::createVariant(dt, exts, generics);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto exts = sanitizeAll(st->tags, map, curName);
            auto generics = sanitizeAll(st->typeArgs, map, curName);
            return AnSumType::createVariant(st, exts, generics);

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto exts = sanitizeAll(fn->paramTys, map, curName);
            auto rett = sanitize(fn->retTy, map, curName);
            auto tcc  = sanitizeAll(fn->typeClassConstraints, map, curName);
            return AnFunctionType::get(rett, exts, tcc, t->typeTag == TT_MetaFunction);

        }else if(auto tup = try_cast<AnTupleType>(t)){
            auto exts = sanitizeAll(tup->fields, map, curName);
            return AnTupleType::getAnonRecord(exts, tup->fieldNames);

        }else{
            return t;
        }
    }

    TraitImpl* sanitize(TraitImpl* impl){
        std::unordered_map<AnTypeVarType*, AnTypeVarType*> map;
        std::string cur = "'a";
        return sanitize(impl, map, cur);
    }

    AnType* sanitize(AnType *t){
        std::unordered_map<AnTypeVarType*, AnTypeVarType*> map;
        std::string cur = "'a";
        return sanitize(t, map, cur);
    }

    std::pair<lazy_printer, lazy_printer>
    anTypesToErrorStrs(const AnType *t1, const AnType *t2){
        return {anTypeToColoredStr(t1), anTypeToColoredStr(t2)};
        /*
        if(!t) return "(null)";

        bool exists = path.exists;
        lazy_printer s;
        auto color = path.here() ? AN_ERR_COLOR : AN_TYPE_COLOR;

        if(t->isModifierType()){
            if(auto *mod = dynamic_cast<const BasicModifier*>(t)){
                return Lexer::getTokStr(mod->mod) + ' ' + anTypeToErrorStr(mod->extTy, path);

            }else if(auto *cdmod = dynamic_cast<const CompilerDirectiveModifier*>(t)){
                //TODO: modify printingvisitor to print to streams
                // PrintingVisitor::print(cdmod->directive.get());
                return anTypeToErrorStr(cdmod->extTy, path);
            }else{
                return "(unknown modifier type)";
            }
        }else if(auto *dt = try_cast<AnDataType>(t)){
            s.strs.emplace_back(dt->name, color);

            size_t i = 0;
            for(auto &a : dt->typeArgs){
                if(shouldWrapInParenthesis(a)){
                    s += lazy_str(" (", color) + anTypeToErrorStr(a, path.nextAt(exists, i)) + lazy_str(")", color);
                }else{
                    s += lazy_str(" ", color) + anTypeToErrorStr(a, path.nextAt(exists, i));
                }
                i++;
            }
            return s;
        }else if(auto *tvt = try_cast<AnTypeVarType>(t)){
            return s + lazy_str(tvt->name, color);
        }else if(auto *f = try_cast<AnFunctionType>(t)){
            size_t i = 0;
            for(auto &param : f->paramTys){
                auto pstr = anTypeToErrorStr(param, path.nextAt(exists, i));
                s += (shouldWrapInParenthesis(param) ? '(' + pstr + ')' : pstr) + ' ';
                i++;
            }

            lazy_printer retTy = anTypeToErrorStr(f->retTy, path.nextAt(exists, i));

            string tcConstraints = f->typeClassConstraints.empty() ? ""
                : " given " + commaSeparated(f->typeClassConstraints);

            return s + lazy_str("-> ", color) + retTy + lazy_str(tcConstraints, color);
        }else if(auto *tup = try_cast<AnTupleType>(t)){
            s += lazy_str("(", color);
            size_t i = 0;
            for(const auto &ext : tup->fields){
                s += anTypeToErrorStr(ext, path.nextAt(exists, i));

                if(&ext != &tup->fields.back()){
                    s += lazy_str(", ", color);
                }else if(tup->fields.size() == 1){
                    s += lazy_str(",", color);
                }
                i++;
            }
            return s + lazy_str(")");
        }else if(auto *arr = try_cast<AnArrayType>(t)){
            return lazy_str("[" + to_string(arr->len) + " ", color)
                + anTypeToErrorStr(arr->extTy, path.nextAt(exists, 0)) + lazy_str("]", color);
        }else if(auto *ptr = try_cast<AnPtrType>(t)){
            return lazy_str("ref ", color) + anTypeToErrorStr(ptr->extTy, path.nextAt(exists, 0));
        }else{
            return lazy_str(typeTagToStr(t->typeTag), color);
        }
        */
    }

    /**
     * Replace $1 and $2 in the lazy_printer with the relevant types from
     * the error. This will also format the types to make the typevars
     * more human readable, eg changing '1382 to 'a
     */
    lazy_printer TypeError::decode(const AnType *a, const AnType *b) const {
        lazy_printer ret;

        auto strs = anTypesToErrorStrs(sanitize((AnType*)a), sanitize((AnType*)b));

        for(auto &s : this->encoded_msg.strs){
            Split split = replace(s, strs.first, strs.second);
            if(split.split_occurred){
                ret += split.a + split.b + split.c + split.d + split.e;
            }else{
                ret += split.a;
            }
        }
        return ret;
    }

    void TypeError::show(const AnType *a, const AnType *b) const {
        ante::showError(decode(a, b), loc);
    }
}
