#include "unification.h"
#include "types.h"

namespace ante {
    size_t curTypeVar = 0;

    AnTypeVarType* nextTypeVar(){
        return AnTypeVarType::get("'" + std::to_string(++curTypeVar));
    }

    std::vector<AnType*> copyWithNewTypeVars(std::vector<AnType*> tys,
            std::unordered_map<std::string, AnTypeVarType*> &map){
        auto ret = vecOf<AnType*>(tys.size());
        for(auto &t : tys){
            ret.push_back(copyWithNewTypeVars(t, map));
        }
        return ret;
    }


    AnType* copyWithNewTypeVars(AnType *t, std::unordered_map<std::string, AnTypeVarType*> &map){
        if(!t->isGeneric)
            return t;

        if(auto fn = try_cast<AnFunctionType>(t)){
            return AnFunctionType::get(copyWithNewTypeVars(fn->retTy, map), copyWithNewTypeVars(fn->extTys, map));

        }else if(auto dt = try_cast<AnDataType>(t)){
            // TODO
            return dt;

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            auto it = map.find(tv->name);
            if(it != map.end()){
                return it->second;
            }else{
                auto newtv = nextTypeVar();
                map[tv->name] = newtv;
                return newtv;
            }

        }else if(auto tup = try_cast<AnAggregateType>(t)){
            return AnAggregateType::get(t->typeTag, copyWithNewTypeVars(tup->extTys, map));

        }else if(auto ptr = try_cast<AnPtrType>(t)){
            return AnPtrType::get(copyWithNewTypeVars(ptr->extTy, map));

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return AnArrayType::get(copyWithNewTypeVars(arr->extTy, map), arr->len);

        }else{
            std::cerr << "Unknown type: " << anTypeToColoredStr(t) << std::endl;
            assert(false);
            return t;
        }
    }


    AnType* copyWithNewTypeVars(AnType *t){
        if(!t->isGeneric)
            return t;

        std::unordered_map<std::string, AnTypeVarType*> map;
        return copyWithNewTypeVars(t, map);
    }



    std::vector<AnType*> substituteIntoAll(AnType *u, std::string const& name,
            std::vector<AnType*> const& vec){
        std::vector<AnType*> ret;
        ret.reserve(vec.size());
        for(auto &elem : vec){
            ret.push_back(substitute(u, name, elem));
        }
        return ret;
    }

    AnType* substitute(AnType *u, std::string const& name, AnType *t){
        if(!t->isGeneric)
            return t;

        if(auto ptr = try_cast<AnPtrType>(t)){
            return AnPtrType::get(substitute(u, name, ptr->extTy));

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return AnArrayType::get(substitute(u, name, arr->extTy), arr->len);

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            return tv->name == name ? u : t;

        }else if(auto dt = try_cast<AnDataType>(t)){
            auto exts = substituteIntoAll(u, name, dt->extTys);;
            /* TODO */
            return t;

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto exts = substituteIntoAll(u, name, fn->extTys);;
            auto rett = substitute(u, name, fn->retTy);
            return AnFunctionType::get(rett, exts, t->typeTag == TT_MetaFunction);

        }else if(auto tup = try_cast<AnAggregateType>(t)){
            auto exts = substituteIntoAll(u, name, tup->extTys);;
            return AnAggregateType::get(TT_Tuple, exts);

        }else{
            return t;
        }
    }


    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t){
        for(auto it = substitutions.rbegin(); it != substitutions.rend(); it++){
            t = substitute(it->second, it->first, t);
        }
        return t;
    }


    Substitutions unifyExts(std::vector<AnType*> const& exts1, std::vector<AnType*> const& exts2){
        if(exts1.size() != exts2.size()){
            LOC_TY loc;
            error("Types are of varying sizes", loc);
        }

        std::list<std::pair<AnType*, AnType*>> ret;
        for(size_t i = 0; i < exts1.size(); i++)
            ret.emplace_back(exts1[i], exts2[i]);
        return unify(ret);
    }


    Substitutions unifyOne(AnType *t1, AnType *t2){
        auto tv1 = try_cast<AnTypeVarType>(t1);
        auto tv2 = try_cast<AnTypeVarType>(t2);
        if(tv1 && !tv2){
            return {{tv1->name, t2}};
        }else if(tv2 && !tv1){
            return {{tv2->name, t1}};
        }

        if(t1->typeTag != t2->typeTag){
            LOC_TY loc;
            error("Mismatched types " + anTypeToColoredStr(t1) + " and " + anTypeToColoredStr(t2), loc);
            return {};
        }

        std::list<std::pair<AnType*, AnType*>> ret;

        if(!t1->isGeneric && !t2->isGeneric)
            return {};

        if(auto ptr1 = try_cast<AnPtrType>(t1)){
            auto ptr2 = try_cast<AnPtrType>(t2);
            ret.emplace_back(ptr1->extTy, ptr2->extTy);
            return unify(ret);

        }else if(auto arr1 = try_cast<AnArrayType>(t1)){
            auto arr2 = try_cast<AnArrayType>(t2);
            ret.emplace_back(arr1->extTy, arr2->extTy);
            return unify(ret);

        }else if(auto dt1 = try_cast<AnDataType>(t1)){
            auto dt2 = try_cast<AnDataType>(t2);
            return unifyExts(dt1->extTys, dt2->extTys);

        }else if(auto fn1 = try_cast<AnFunctionType>(t1)){
            auto fn2 = try_cast<AnFunctionType>(t2);
            if(fn1->extTys.size() != fn2->extTys.size()){
                LOC_TY loc;
                error("Types are of varying sizes", loc);
            }

            std::list<std::pair<AnType*, AnType*>> ret;
            for(size_t i = 0; i < fn1->extTys.size(); i++)
                ret.emplace_back(fn1->extTys[i], fn2->extTys[i]);

            ret.emplace_back(fn1->retTy, fn2->retTy);
            return unify(ret);

        }else if(auto tup1 = try_cast<AnAggregateType>(t1)){
            auto tup2 = try_cast<AnAggregateType>(t2);
            return unifyExts(tup1->extTys, tup2->extTys);

        }else{
            return {};
        }

    }


    Substitutions unify(std::list<std::pair<AnType*, AnType*>> const& list,
            std::list<std::pair<AnType*, AnType*>>::iterator cur){

        if(cur == list.end()){
            return {};
        }else{
            auto &p = *cur;
            auto t2 = unify(list, ++cur);
            auto t1 = unifyOne(applySubstitutions(t2, p.first), applySubstitutions(t2, p.second));

            Substitutions ret = t1;
            ret.insert(ret.end(), t2.begin(), t2.end());
            return ret;
        }
    }

    Substitutions unify(std::list<std::pair<AnType*, AnType*>>& cur){
        return unify(cur, cur.begin());
    }
}
