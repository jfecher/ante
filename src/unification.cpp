#include "unification.h"
#include "types.h"

namespace ante {
    size_t curTypeVar = 0;

    AnTypeVarType* nextTypeVar(){
        return AnTypeVarType::get("'" + std::to_string(++curTypeVar));
    }

    template<typename T>
    std::vector<T*> copyWithNewTypeVars(std::vector<T*> tys,
            std::unordered_map<std::string, AnTypeVarType*> &map){

        std::vector<T*> ret;
        ret.reserve(tys.size());
        for(auto &t : tys){
            ret.push_back((T*)copyWithNewTypeVars(t, map));
        }
        return ret;
    }

    void setExtsParentUnionTypeIfNotSet(AnSumType *parentUnion, std::vector<AnProductType*> &exts){
        for(auto e : exts){
            if(!e->parentUnionType){
                e->parentUnionType = parentUnion;
            }
        }
    }

    AnType* copyWithNewTypeVars(AnType *t, std::unordered_map<std::string, AnTypeVarType*> &map){
        if(!t->isGeneric)
            return t;

        if(t->isModifierType()){
            auto modTy = static_cast<AnModifier*>(t);
            return (AnType*)modTy->addModifiersTo(copyWithNewTypeVars((AnType*)modTy->extTy, map));
        }

        if(auto fn = try_cast<AnFunctionType>(t)){
            return AnFunctionType::get(copyWithNewTypeVars(fn->retTy, map),
                    copyWithNewTypeVars(fn->extTys, map),
                    copyWithNewTypeVars(fn->typeClassConstraints, map));

        }else if(auto pt = try_cast<AnProductType>(t)){
            auto exts = copyWithNewTypeVars(pt->fields, map);
            auto typeVars = copyWithNewTypeVars(pt->typeArgs, map);
            if(exts == pt->fields && typeVars == pt->typeArgs){
                return pt;
            }else{
                return AnProductType::createVariant(pt, exts, typeVars);
            }

        }else if(auto st = try_cast<AnSumType>(t)){
            auto exts = copyWithNewTypeVars(st->tags, map);
            auto typeVars = copyWithNewTypeVars(st->typeArgs, map);
            if(exts == st->tags && typeVars == st->typeArgs){
                return st;
            }else{
                auto ret = AnSumType::createVariant(st, exts, typeVars);
                setExtsParentUnionTypeIfNotSet(ret, exts);
                return ret;
            }

        }else if(auto tt = try_cast<AnTraitType>(t)){
            auto typeVars = copyWithNewTypeVars(tt->typeArgs, map);
            auto selfType = copyWithNewTypeVars(tt->selfType, map);
            if(selfType == tt->selfType && typeVars == tt->typeArgs)
                return tt;
            else
                return AnTraitType::createVariant(tt, selfType, typeVars);

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
        auto variant = try_cast<AnProductType>(t);
        if(variant && variant->parentUnionType){
            auto st = (AnSumType*)copyWithNewTypeVars(variant->parentUnionType, map);
            return st->getTagByName(variant->name);
        }else{
            return copyWithNewTypeVars(t, map);
        }
    }


    template<class T>
    std::vector<T*> substituteIntoAll(AnType *u, AnType *subType,
            std::vector<T*> const& vec){

        std::vector<T*> ret;
        ret.reserve(vec.size());
        for(auto &elem : vec){
            ret.push_back(static_cast<T*>(substitute(u, subType, elem)));
        }
        return ret;
    }

    AnType* substitute(AnType *u, AnType* subType, AnType *t){
        if(!t->isGeneric)
            return t;

        if(t->isModifierType()){
            auto modTy = static_cast<AnModifier*>(t);
            return (AnType*)modTy->addModifiersTo(substitute(u, subType, (AnType*)modTy->extTy));
        }

        if(auto ptr = try_cast<AnPtrType>(t)){
            return AnPtrType::get(substitute(u, subType, ptr->extTy));

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return AnArrayType::get(substitute(u, subType, arr->extTy), arr->len);

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            return tv == subType ? u : t;

        }else if(auto dt = try_cast<AnProductType>(t)){
            auto exts = substituteIntoAll(u, subType, dt->fields);;
            auto generics = substituteIntoAll(u, subType, dt->typeArgs);;

            if(exts == dt->fields && generics == dt->typeArgs)
                return t;
            else
                return AnProductType::createVariant(dt, exts, generics);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto exts = substituteIntoAll(u, subType, st->tags);;
            auto generics = substituteIntoAll(u, subType, st->typeArgs);;

            if(exts == st->tags && generics == st->typeArgs){
                return st;
            }else{
                auto ret = AnSumType::createVariant(st, exts, generics);
                setExtsParentUnionTypeIfNotSet(ret, exts);
                return ret;
            }

        }else if(auto tt = try_cast<AnTraitType>(t)){
            if(tt == subType){
                return u;
            }

            auto generics = substituteIntoAll(u, subType, tt->typeArgs);;
            auto selfType = substitute(u, subType, tt->selfType);

            if(selfType == tt->selfType && generics == tt->typeArgs)
                return tt;
            else
                return AnTraitType::createVariant(tt, selfType, generics);

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto exts = substituteIntoAll(u, subType, fn->extTys);;
            auto rett = substitute(u, subType, fn->retTy);
            auto tcc  = substituteIntoAll(u, subType, fn->typeClassConstraints);
            return AnFunctionType::get(rett, exts, tcc, t->typeTag == TT_MetaFunction);

        }else if(auto tup = try_cast<AnAggregateType>(t)){
            auto exts = substituteIntoAll(u, subType, tup->extTys);;
            return AnAggregateType::get(TT_Tuple, exts);

        }else{
            return t;
        }
    }


    AnFunctionType* cleanTypeClassConstraints(AnFunctionType *t){
        std::vector<AnTraitType*> c;
        c.reserve(t->typeClassConstraints.size());

        auto tEnd = t->typeClassConstraints.end();
        for(auto it1 = t->typeClassConstraints.begin(); it1 != tEnd; ++it1){
            auto elemit = std::find_if(it1 + 1, tEnd, [&](AnTraitType *elem){
                return *elem == **it1;
            });
            if(elemit == tEnd && !(*it1)->implemented())
                c.push_back(*it1);
        }

        return AnFunctionType::get(t->retTy, t->extTys, c);
    }


    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t){
        for(auto it = substitutions.rbegin(); it != substitutions.rend(); it++){
            auto variant = try_cast<AnProductType>(t);
            if(variant && variant->parentUnionType){
                auto st = (AnSumType*)applySubstitutions(substitutions, variant->parentUnionType);
                t = st->getTagByName(variant->name);
            }else{
                t = substitute(it->second, it->first, t);
            }
        }
        return t;
    }

    template<class T>
    Substitutions unifyExts(std::vector<T*> const& exts1, std::vector<T*> const& exts2,
            LOC_TY const& loc, AnType *t1, AnType *t2){

        if(exts1.size() != exts2.size()){
            showError("Mismatched types " + anTypeToColoredStr(t1)
                    + " and " + anTypeToColoredStr(t2), loc);
            return {};
        }

        UnificationList ret;
        for(size_t i = 0; i < exts1.size(); i++)
            ret.emplace_back(exts1[i], exts2[i], loc);
        return unify(ret);
    }


    bool implements(AnType *type, AnTraitType *trait){
        return true;
    }

    Substitutions unifyOne(AnType *t1, AnType *t2, LOC_TY const& loc){
        auto tv1 = try_cast<AnTypeVarType>(t1);
        auto tv2 = try_cast<AnTypeVarType>(t2);

        if(tv1){
            return {{tv1, t2}};
        }else if(tv2){
            return {{tv2, t1}};
        }

        if(t1->typeTag != t2->typeTag){
            auto trait = try_cast<AnTraitType>(t1);
            if(trait){
                return unifyOne(trait->selfType, t2, loc);
            }

            trait = try_cast<AnTraitType>(t2);
            if(trait){
                return unifyOne(t1, trait->selfType, loc);
            }
            showError("Mismatched types " + anTypeToColoredStr(t1) + " and " + anTypeToColoredStr(t2), loc);
            return {};
        }

        UnificationList ret;

        if(!t1->isGeneric && !t2->isGeneric){
            if(t1 != t2){
                showError("Mismatched types " + anTypeToColoredStr(t1) + " and " + anTypeToColoredStr(t2), loc);
            }
            return {};
        }

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
            auto l1 = unifyExts(pt1->fields, pt2->fields, loc, pt1, pt2);
            auto l2 = unifyExts(pt1->typeArgs, pt2->typeArgs, loc, pt1, pt2);
            l1.merge(l2);
            return l1;

        }else if(auto st1 = try_cast<AnSumType>(t1)){
            auto st2 = try_cast<AnSumType>(t2);
            auto l1 = unifyExts(st1->tags, st2->tags, loc, st1, st2);
            auto l2 = unifyExts(st1->typeArgs, st2->typeArgs, loc, st1, st2);
            l1.merge(l2);
            return l1;

        }else if(auto tt1 = try_cast<AnTraitType>(t1)){
            auto tt2 = try_cast<AnTraitType>(t2);
            ret.emplace_back(tt1->selfType, tt2->selfType, loc);
            return unify(ret);

        }else if(auto fn1 = try_cast<AnFunctionType>(t1)){
            auto fn2 = try_cast<AnFunctionType>(t2);
            if(fn1->extTys.size() != fn2->extTys.size()){
                error("Types are of varying sizes", loc);
            }

            for(size_t i = 0; i < fn1->extTys.size(); i++)
                ret.emplace_back(fn1->extTys[i], fn2->extTys[i], loc);

            ret.emplace_back(fn1->retTy, fn2->retTy, loc);
            return unify(ret);

        }else if(auto tup1 = try_cast<AnAggregateType>(t1)){
            auto tup2 = try_cast<AnAggregateType>(t2);
            return unifyExts(tup1->extTys, tup2->extTys, loc, tup1, tup2);

        }else{
            return {};
        }
    }


    Substitutions unify(UnificationList const& list, UnificationList::const_iterator cur){
        if(cur == list.end()){
            return {};
        }else{
            auto &p = *cur;
            auto t2 = unify(list, ++cur);

            if(!p.isEqConstraint()){
                return t2;
            }

            try{
                auto eq = p.asEqConstraint();

                auto t1 = unifyOne(applySubstitutions(t2, eq.first),
                        applySubstitutions(t2, eq.second), p.loc);

                Substitutions ret = t1;
                ret.insert(ret.end(), t2.begin(), t2.end());
                return ret;
            }catch(CtError e){
                return t2;
            }
        }
    }

    Substitutions unify(UnificationList const& cur){
        return unify(cur, cur.begin());
    }
}
