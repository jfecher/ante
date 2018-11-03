#include "unification.h"
#include "types.h"

namespace ante {
    size_t curTypeVar = 0;

    AnTypeVarType* nextTypeVar(){
        return AnTypeVarType::get("'" + std::to_string(++curTypeVar));
    }

    std::vector<AnType*> copyWithNewTypeVars(std::vector<AnType*> tys,
            std::unordered_map<std::string, AnTypeVarType*> &map){

        std::vector<AnType*> ret;
        ret.reserve(tys.size());
        for(auto &t : tys){
            ret.push_back(copyWithNewTypeVars(t, map));
        }
        return ret;
    }

    std::vector<AnProductType*> copyWithNewTypeVars(std::vector<AnProductType*> tys,
            std::unordered_map<std::string, AnTypeVarType*> &map){

        std::vector<AnProductType*> ret;
        ret.reserve(tys.size());
        for(auto &t : tys){
            ret.push_back(static_cast<AnProductType*>(copyWithNewTypeVars(t, map)));
        }
        return ret;
    }


    AnType* copyWithNewTypeVars(AnType *t, std::unordered_map<std::string, AnTypeVarType*> &map){
        if(!t->isGeneric)
            return t;

        if(auto fn = try_cast<AnFunctionType>(t)){
            return AnFunctionType::get(copyWithNewTypeVars(fn->retTy, map), copyWithNewTypeVars(fn->extTys, map));

        }else if(auto pt = try_cast<AnProductType>(t)){
            auto exts = copyWithNewTypeVars(pt->fields, map);
            auto typeVars = copyWithNewTypeVars(pt->typeArgs, map);
            if(exts == pt->fields && typeVars == pt->typeArgs)
                return pt;
            else
                return AnProductType::getOrCreateVariant(pt, exts, typeVars);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto exts = copyWithNewTypeVars(st->tags, map);
            auto typeVars = copyWithNewTypeVars(st->typeArgs, map);
            if(exts == st->tags && typeVars == st->typeArgs)
                return st;
            else
                return AnSumType::getOrCreateVariant(st, exts, typeVars);

        }else if(auto tt = try_cast<AnTraitType>(t)){
            auto typeVars = copyWithNewTypeVars(tt->typeArgs, map);
            if(typeVars == tt->typeArgs)
                return tt;
            else
                return AnTraitType::getOrCreateVariant(tt, typeVars);

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            auto it = map.find(tv->name);
            if(it != map.end()){
                return it->second;
            }else{
                auto newtv = nextTypeVar();
                if(tv->isVarArgs())
                    newtv = AnTypeVarType::get(newtv->name + "...");

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


    template<class T>
    std::vector<T*> substituteIntoAll(AnType *u, std::string const& name,
            std::vector<T*> const& vec){

        std::vector<T*> ret;
        ret.reserve(vec.size());
        for(auto &elem : vec){
            ret.push_back(static_cast<T*>(substitute(u, name, elem)));
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

        }else if(auto dt = try_cast<AnProductType>(t)){
            auto exts = substituteIntoAll(u, name, dt->fields);;
            auto generics = substituteIntoAll(u, name, dt->typeArgs);;

            if(exts == dt->fields && generics == dt->typeArgs)
                return t;
            else
                return AnProductType::getOrCreateVariant(dt, exts, generics);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto exts = substituteIntoAll(u, name, st->tags);;
            auto generics = substituteIntoAll(u, name, st->typeArgs);;

            if(exts == st->tags && generics == st->typeArgs)
                return st;
            else
                return AnSumType::getOrCreateVariant(st, exts, generics);

        }else if(auto tt = try_cast<AnTraitType>(t)){
            auto generics = substituteIntoAll(u, name, tt->typeArgs);;

            if(generics == tt->typeArgs)
                return tt;
            else
                return AnTraitType::getOrCreateVariant(tt, generics);

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

    template<class T>
    Substitutions unifyExts(std::vector<T*> const& exts1, std::vector<T*> const& exts2, LOC_TY &loc){
        if(exts1.size() != exts2.size()){
            std::vector<AnType*> lt{exts1.begin(), exts1.end()};
            std::vector<AnType*> rt{exts2.begin(), exts2.end()};
            auto l = AnAggregateType::get(TT_Tuple, lt);
            auto r = AnAggregateType::get(TT_Tuple, rt);
            error("Types are of varying sizes: " + anTypeToColoredStr(l)
                    + " vs " + anTypeToColoredStr(r), loc);
        }

        std::list<std::tuple<AnType*, AnType*, LOC_TY&>> ret;
        for(size_t i = 0; i < exts1.size(); i++)
            ret.emplace_back(exts1[i], exts2[i], loc);
        return unify(ret);
    }


    bool implements(AnType *type, AnTraitType *trait){
        return true;
    }


    Substitutions unifyOne(AnType *t1, AnType *t2, LOC_TY &loc){
        auto tv1 = try_cast<AnTypeVarType>(t1);
        auto tv2 = try_cast<AnTypeVarType>(t2);

        if(tv1){
            return {{tv1->name, t2}};
        }else if(tv2){
            return {{tv2->name, t1}};
        }

        if(t1->typeTag != t2->typeTag){
            auto trait = try_cast<AnTraitType>(t1);
            if(trait && implements(t2, trait)){
                return {};
            }

            trait = try_cast<AnTraitType>(t2);
            if(trait && implements(t1, trait)){
                return {};
            }
            error("Mismatched types " + anTypeToColoredStr(t1) + " and " + anTypeToColoredStr(t2), loc);
            return {};
        }

        std::list<std::tuple<AnType*, AnType*, LOC_TY&>> ret;

        if(!t1->isGeneric && !t2->isGeneric)
            return {};

        if(auto ptr1 = try_cast<AnPtrType>(t1)){
            auto ptr2 = try_cast<AnPtrType>(t2);
            ret.emplace_back(ptr1->extTy, ptr2->extTy, loc);
            return unify(ret);

        }else if(auto arr1 = try_cast<AnArrayType>(t1)){
            auto arr2 = try_cast<AnArrayType>(t2);
            ret.emplace_back(arr1->extTy, arr2->extTy, loc);
            return unify(ret);

        }else if(auto pt1 = try_cast<AnProductType>(t1)){
            auto pt2 = try_cast<AnProductType>(t2);
            return unifyExts(pt1->fields, pt2->fields, loc);

        }else if(auto st1 = try_cast<AnSumType>(t1)){
            auto st2 = try_cast<AnSumType>(t2);
            return unifyExts(st1->tags, st2->tags, loc);

        }else if(auto fn1 = try_cast<AnFunctionType>(t1)){
            auto fn2 = try_cast<AnFunctionType>(t2);
            if(fn1->extTys.size() != fn2->extTys.size()){
                error("Types are of varying sizes", loc);
                return {};
            }

            std::list<std::tuple<AnType*, AnType*, LOC_TY&>> ret;
            for(size_t i = 0; i < fn1->extTys.size(); i++)
                ret.emplace_back(fn1->extTys[i], fn2->extTys[i], loc);

            ret.emplace_back(fn1->retTy, fn2->retTy, loc);
            return unify(ret);

        }else if(auto tup1 = try_cast<AnAggregateType>(t1)){
            auto tup2 = try_cast<AnAggregateType>(t2);
            return unifyExts(tup1->extTys, tup2->extTys, loc);

        }else{
            return {};
        }

    }


    Substitutions unify(std::list<std::tuple<AnType*, AnType*, LOC_TY&>> const& list,
            std::list<std::tuple<AnType*, AnType*, LOC_TY&>>::iterator cur){

        if(cur == list.end()){
            return {};
        }else{
            auto &p = *cur;
            auto t2 = unify(list, ++cur);

            try{
                auto t1 = unifyOne(applySubstitutions(t2, std::get<0>(p)),
                        applySubstitutions(t2, std::get<1>(p)), std::get<2>(p));

                Substitutions ret = t1;
                ret.insert(ret.end(), t2.begin(), t2.end());
                return ret;
            }catch(CompilationError *e){
                delete e;
                return t2;
            }
        }
    }

    Substitutions unify(std::list<std::tuple<AnType*, AnType*, LOC_TY&>>& cur){
        return unify(cur, cur.begin());
    }
}
