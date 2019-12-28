#include "unification.h"
#include "types.h"
#include "trait.h"
#include "util.h"

namespace ante {
    size_t curTypeVar = 0;

    AnTypeVarType* nextTypeVar(){
        return AnTypeVarType::get('\'' + std::to_string(++curTypeVar));
    }

    template<typename T>
    std::vector<T*> copyWithNewTypeVars(std::vector<T*> tys,
            std::unordered_map<std::string, AnTypeVarType*> &map){

        return ante::applyToAll(tys, [&](T* type){
            return (T*)copyWithNewTypeVars(type, map);
        });
    }

    TraitImpl* copyWithNewTypeVars(TraitImpl* impl,
            std::unordered_map<std::string, AnTypeVarType*> &map){

        return new TraitImpl(impl->name, copyWithNewTypeVars(impl->typeArgs, map));
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
                    copyWithNewTypeVars(fn->paramTys, map),
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

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            auto it = map.find(tv->name);
            if(it != map.end()){
                return it->second;
            }else{
                auto newtv = nextTypeVar();
                if(tv->isRhoVar())
                    newtv = AnTypeVarType::get(newtv->name + "...");

                map[tv->name] = newtv;
                return newtv;
            }

        }else if(auto tup = try_cast<AnTupleType>(t)){
            return AnTupleType::getAnonRecord(copyWithNewTypeVars(tup->fields, map), tup->fieldNames);

        }else if(auto ptr = try_cast<AnPtrType>(t)){
            return AnPtrType::get(copyWithNewTypeVars(ptr->extTy, map));

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return AnArrayType::get(copyWithNewTypeVars(arr->extTy, map), arr->len);

        }else{
            std::cerr << "Unknown type: " << anTypeToColoredStr(t) << std::endl;
            ASSERT_UNREACHABLE();
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
            std::vector<T*> const& vec, int recursionLimit){

        return ante::applyToAll(vec, [&](T *elem){
            return (T*)substitute(u, subType, elem, recursionLimit - 1);
        });
    }

    TraitImpl* substitute(AnType *u, AnType* subType, TraitImpl *impl, int recursionLimit){
        return new TraitImpl(impl->name, substituteIntoAll(u, subType, impl->typeArgs, recursionLimit - 1));
    }

    AnType* substitute(AnType *u, AnType* subType, AnType *t, int recursionLimit){
        if(!t->isGeneric)
            return t;

        if(recursionLimit < 0){
            std::cerr << "u = " << anTypeToColoredStr(u)<< ", subType = " << anTypeToColoredStr(subType)
                      << ", t = " << anTypeToColoredStr(t) << '\n';
            ASSERT_UNREACHABLE("internal recursion limit (10,000) reached in ante::substitute");
        }

        if(t->isModifierType()){
            auto modTy = static_cast<AnModifier*>(t);
            return (AnType*)modTy->addModifiersTo(substitute(u, subType, (AnType*)modTy->extTy, recursionLimit - 1));
        }

        if(auto ptr = try_cast<AnPtrType>(t)){
            return AnPtrType::get(substitute(u, subType, ptr->extTy, recursionLimit - 1));

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return AnArrayType::get(substitute(u, subType, arr->extTy, recursionLimit - 1), arr->len);

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            return tv == subType ? u : t;

        }else if(auto dt = try_cast<AnProductType>(t)){
            auto exts = substituteIntoAll(u, subType, dt->fields, recursionLimit - 1);
            auto generics = substituteIntoAll(u, subType, dt->typeArgs, recursionLimit - 1);

            if(exts == dt->fields && generics == dt->typeArgs)
                return t;
            else
                return AnProductType::createVariant(dt, exts, generics);

        }else if(auto st = try_cast<AnSumType>(t)){
            auto exts = substituteIntoAll(u, subType, st->tags, recursionLimit - 1);
            auto generics = substituteIntoAll(u, subType, st->typeArgs, recursionLimit - 1);

            if(exts == st->tags && generics == st->typeArgs){
                return st;
            }else{
                auto ret = AnSumType::createVariant(st, exts, generics);
                setExtsParentUnionTypeIfNotSet(ret, exts);
                return ret;
            }

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto exts = substituteIntoAll(u, subType, fn->paramTys, recursionLimit - 1);
            auto rett = substitute(u, subType, fn->retTy, recursionLimit - 1);
            auto tcc  = substituteIntoAll(u, subType, fn->typeClassConstraints, recursionLimit - 1);
            return AnFunctionType::get(rett, exts, tcc, t->typeTag == TT_MetaFunction);

        }else if(auto tup = try_cast<AnTupleType>(t)){
            auto exts = substituteIntoAll(u, subType, tup->fields, recursionLimit - 1);
            return AnTupleType::getAnonRecord(exts, tup->fieldNames);

        }else{
            return t;
        }
    }


    bool containsTypeVarHelper(const AnType *t, AnTypeVarType *typeVar){
        if(!t->isGeneric)
            return false;

        if(t->isModifierType()){
            auto modTy = (AnModifier*)t;
            return containsTypeVarHelper(modTy->extTy, typeVar);
        }

        if(auto ptr = try_cast<AnPtrType>(t)){
            return containsTypeVarHelper(ptr->extTy, typeVar);

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return containsTypeVarHelper(arr->extTy, typeVar);

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            return tv == typeVar;

        }else if(auto dt = try_cast<AnProductType>(t)){
            if(ante::any(dt->typeArgs, [&](AnType *f){ return containsTypeVarHelper(f, typeVar); }))
                return true;

            return ante::any(dt->fields, [&](AnType *f){ return containsTypeVarHelper(f, typeVar); });

        }else if(auto st = try_cast<AnSumType>(t)){
            if(ante::any(st->typeArgs, [&](AnType *f){ return containsTypeVarHelper(f, typeVar); }))
                return true;

            return ante::any(st->tags, [&](AnProductType *f){ return containsTypeVarHelper(f, typeVar); });

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            if(ante::any(fn->paramTys, [&](AnType *f){ return containsTypeVarHelper(f, typeVar); }))
                return true;

            if(containsTypeVarHelper(fn->retTy, typeVar))
                return true;

            auto tccContainsTypeVar = [&](TraitImpl *t){
                return ante::any(t->typeArgs, [&](AnType *t){ return containsTypeVarHelper(t, typeVar); });
            };

            return ante::any(fn->typeClassConstraints, tccContainsTypeVar);

        }else if(auto tup = try_cast<AnTupleType>(t)){
            return ante::any(tup->fields, [&](AnType *f){ return containsTypeVarHelper(f, typeVar); });

        }else{
            return false;
        }
    }

    bool containsTypeVar(AnType *t, AnTypeVarType *typeVar){
        if(t == typeVar) return false;
        return containsTypeVarHelper(t, typeVar);
    }

    bool hasTypeVarNotInMap(const AnType *t, llvm::StringMap<const AnTypeVarType*> &map){
        if(!t->isGeneric)
            return false;

        if(t->isModifierType()){
            auto modTy = (AnModifier*)t;
            return hasTypeVarNotInMap(modTy->extTy, map);
        }

        if(auto tv = try_cast<AnTypeVarType>(t)){
            return map.find(tv->name) == map.end();

        }else if(auto ptr = try_cast<AnPtrType>(t)){
            return hasTypeVarNotInMap(ptr->extTy, map);

        }else if(auto arr = try_cast<AnArrayType>(t)){
            return hasTypeVarNotInMap(arr->extTy, map);

        }else if(auto dt = try_cast<AnProductType>(t)){
            return ante::any(dt->typeArgs, [&](AnType *f){ return hasTypeVarNotInMap(f, map); })
                || ante::any(dt->fields, [&](AnType *f){ return hasTypeVarNotInMap(f, map); });

        }else if(auto st = try_cast<AnSumType>(t)){
            return ante::any(st->typeArgs, [&](AnType *f){ return hasTypeVarNotInMap(f, map); })
                || ante::any(st->tags, [&](AnProductType *f){ return hasTypeVarNotInMap(f, map); });

        }else if(auto fn = try_cast<AnFunctionType>(t)){
            auto tccContainsTypeVar = [&](TraitImpl *t){
                return ante::any(t->typeArgs, [&](AnType *t){ return hasTypeVarNotInMap(t, map); });
            };

            return ante::any(fn->paramTys, [&](AnType *f){ return hasTypeVarNotInMap(f, map); })
                || hasTypeVarNotInMap(fn->retTy, map)
                || ante::any(fn->typeClassConstraints, tccContainsTypeVar);

        }else if(auto tup = try_cast<AnTupleType>(t)){
            return ante::any(tup->fields, [&](AnType *f){ return hasTypeVarNotInMap(f, map); });

        }else{
            return false;
        }
    }

    void getAllContainedTypeVarsHelper(const AnType *t, llvm::StringMap<const AnTypeVarType*> &map);

    void getAllContainedTypeVarsHelper(const TraitImpl *impl, llvm::StringMap<const AnTypeVarType*> &map){
        for(AnType *t : impl->typeArgs){
            getAllContainedTypeVarsHelper(t, map);
        }
    }

    void getAllContainedTypeVarsHelper(const AnType *t, llvm::StringMap<const AnTypeVarType*> &map){
        if(!t->isGeneric)
            return;

        if(t->isModifierType()){
            getAllContainedTypeVarsHelper(static_cast<const AnModifier*>(t)->extTy, map);
            return;
        }

        if(auto fn = try_cast<AnFunctionType>(t)){
            getAllContainedTypeVarsHelper(fn->retTy, map);
            for(AnType *paramTy : fn->paramTys){ getAllContainedTypeVarsHelper(paramTy, map); }
            for(TraitImpl *tcc : fn->typeClassConstraints){ getAllContainedTypeVarsHelper(tcc, map); }

        }else if(auto pt = try_cast<AnProductType>(t)){
            for(AnType *fieldTy : pt->fields){ getAllContainedTypeVarsHelper(fieldTy, map); }
            for(AnType *typeArg : pt->typeArgs){ getAllContainedTypeVarsHelper(typeArg, map); }

        }else if(auto st = try_cast<AnSumType>(t)){
            for(AnType *tagTy : st->tags){ getAllContainedTypeVarsHelper(tagTy, map); }
            for(AnType *typeArg : st->typeArgs){ getAllContainedTypeVarsHelper(typeArg, map); }

        }else if(auto tv = try_cast<AnTypeVarType>(t)){
            map[tv->name] = tv;

        }else if(auto tup = try_cast<AnTupleType>(t)){
            for(AnType *extTy : tup->fields){ getAllContainedTypeVarsHelper(extTy, map); }

        }else if(auto ptr = try_cast<AnPtrType>(t)){
            getAllContainedTypeVarsHelper(ptr->extTy, map);

        }else if(auto arr = try_cast<AnArrayType>(t)){
            getAllContainedTypeVarsHelper(arr->extTy, map);

        }else{
            std::cerr << "Unknown type: " << anTypeToColoredStr(t) << std::endl;
            ASSERT_UNREACHABLE();
        }
    }

    llvm::StringMap<const AnTypeVarType*> getAllContainedTypeVars(const AnType *t){
        llvm::StringMap<const AnTypeVarType*> ret;
        getAllContainedTypeVarsHelper(t, ret);
        return ret;
    }


    AnFunctionType* cleanTypeClassConstraints(AnFunctionType *t){
        std::vector<TraitImpl*> c;
        c.reserve(t->typeClassConstraints.size());

        auto tEnd = t->typeClassConstraints.end();
        for(auto it1 = t->typeClassConstraints.begin(); it1 != tEnd; ++it1){
            auto elemit = std::find_if(it1 + 1, tEnd, [&](TraitImpl *elem){
                return *elem == **it1;
            });
            if(elemit == tEnd){
                c.push_back(*it1);
            }
        }

        return AnFunctionType::get(t->retTy, t->paramTys, c);
    }


    AnType* applySubstitutions(Substitutions const& substitutions, AnType *t){
        for(auto it = substitutions.rbegin(); it != substitutions.rend(); ++it){
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

    TraitImpl* applySubstitutions(Substitutions const& substitutions, TraitImpl *t){
        auto ret = new TraitImpl(t->name, t->typeArgs);
        for(auto it = substitutions.rbegin(); it != substitutions.rend(); ++it){
            ret->typeArgs = ante::applyToAll(ret->typeArgs, [it](AnType *type){
                return substitute(it->second, it->first, type);
            });
        }
        return ret;
    }

    enum TypeErrorKind {
        Mismatch, InfRecursion1, InfRecursion2
    };

    struct TypeErrorContext : public std::exception {
        const AnType *t1, *t2;
        const TypeErrorKind kind;
        Substitutions subs;

        TypeErrorContext(const AnType *t1, const AnType *t2, TypeErrorKind kind)
            : t1{t1}, t2{t2}, kind{kind}, subs{}{}
    };

    Substitutions unify(UnificationList const& list, UnificationList::const_reverse_iterator cur, bool isTopLevel);

    /**
     * Helper for a recursive call to unify.
     * Supplies arguments needed for good error messages.
     */
    Substitutions unifyRecursive(UnificationList const& list){
        return unify(list, list.rbegin(), false);
    }

    template<class T>
    Substitutions unifyExts(std::vector<T*> const& exts1, std::vector<T*> const& exts2,
            const AnType *t1, const AnType *t2, TypeError const& err){

        if(exts1.size() != exts2.size()){
            throw TypeErrorContext(t1, t2, Mismatch);
        }

        UnificationList ret;
        for(size_t i = 0; i < exts1.size(); i++)
            ret.emplace_back(exts1[i], exts2[i], err);
        return unifyRecursive(ret);
    }

    Substitutions unifyTuple(AnTupleType *tup1, AnTupleType *tup2, TypeError const& err){
        auto len1 = tup1->fields.size();
        auto len2 = tup2->fields.size();
        if(tup1->hasRhoVar()) len1--;
        if(tup2->hasRhoVar()) len2--;

        if(len1 != len2){
            if(len1 < len2){
                if(tup1->hasRhoVar()){
                    len2 = len1;
                }else{
                    throw TypeErrorContext(tup1, tup2, Mismatch);
                }
            }else{
                if(tup2->hasRhoVar()){
                    len1 = len2;
                }else{
                    throw TypeErrorContext(tup1, tup2, Mismatch);
                }
            }
        }

        UnificationList ret;
        for(size_t i = 0; i < len1; i++)
            ret.emplace_back(tup1->fields[i], tup2->fields[i], err);
        return unifyRecursive(ret);
    }


    Substitutions unifyOne(AnType *t1, AnType *t2, TypeError const& err){
        auto tv1 = try_cast<AnTypeVarType>(t1);
        auto tv2 = try_cast<AnTypeVarType>(t2);

        if(tv1){
            if(containsTypeVar(t2, tv1)){
                throw TypeErrorContext(t1, t2, InfRecursion1);
            }
            return {{tv1, t2}};
        }else if(tv2){
            if(containsTypeVar(t1, tv2)){
                throw TypeErrorContext(t1, t2, InfRecursion2);
            }
            return {{tv2, t1}};
        }

        if(t1->typeTag != t2->typeTag){
            throw TypeErrorContext(t1, t2, Mismatch);
        }

        UnificationList ret;

        if(!t1->isGeneric && !t2->isGeneric){
            if(!t1->approxEq(t2)){
                throw TypeErrorContext(t1, t2, Mismatch);
            }
            return {};
        }

        if(auto ptr1 = try_cast<AnPtrType>(t1)){
            auto ptr2 = try_cast<AnPtrType>(t2);
            ret.emplace_back(ptr1->extTy, ptr2->extTy, err);
            return unifyRecursive(ret);

        }else if(auto arr1 = try_cast<AnArrayType>(t1)){
            auto arr2 = try_cast<AnArrayType>(t2);
            ret.emplace_back(arr1->extTy, arr2->extTy, err);
            return unifyRecursive(ret);

        }else if(auto pt1 = try_cast<AnProductType>(t1)){
            auto pt2 = try_cast<AnProductType>(t2);
            return unifyExts(pt1->typeArgs, pt2->typeArgs, pt1, pt2, err);

        }else if(auto st1 = try_cast<AnSumType>(t1)){
            auto st2 = try_cast<AnSumType>(t2);
            return unifyExts(st1->typeArgs, st2->typeArgs, st1, st2, err);

        }else if(auto fn1 = try_cast<AnFunctionType>(t1)){
            auto fn2 = try_cast<AnFunctionType>(t2);
            if(fn1->paramTys.size() != fn2->paramTys.size()){
                throw TypeErrorContext(t1, t2, Mismatch);
            }

            for(size_t i = 0; i < fn1->paramTys.size(); i++)
                ret.emplace_back(fn1->paramTys[i], fn2->paramTys[i], err);

            ret.emplace_back(fn1->retTy, fn2->retTy, err);
            return unifyRecursive(ret);

        }else if(auto tup1 = try_cast<AnTupleType>(t1)){
            auto tup2 = try_cast<AnTupleType>(t2);
            return unifyTuple(tup1, tup2, err);

        }else{
            return {};
        }
    }


    Substitutions unify(UnificationList const& list, UnificationList::const_reverse_iterator cur, bool isTopLevel){
        if(cur == list.rend()){
            return {};
        }else{
            auto &p = *cur;
            auto t2 = unify(list, ++cur, isTopLevel);

            if(!p.isEqConstraint()){
                return t2;
            }

            try{
                auto eq = p.asEqConstraint();

                Substitutions t1;
                try {
                    t1 = unifyOne(applySubstitutions(t2, eq.first),
                                  applySubstitutions(t2, eq.second), p.error);
                }catch(TypeErrorContext& e){
                    e.subs.insert(e.subs.end(), t2.begin(), t2.end());
                    if(!isTopLevel){
                        throw e;
                    }

                    p.error.show(applySubstitutions(e.subs, eq.first), applySubstitutions(e.subs, eq.second));
                    if(e.kind == InfRecursion1)
                        showError(anTypeToColoredStr(e.t1) + " occurs inside " + anTypeToColoredStr(e.t2), p.error.loc, ErrorType::Note);
                    if(e.kind == InfRecursion2)
                        showError(anTypeToColoredStr(e.t2) + " occurs inside " + anTypeToColoredStr(e.t1), p.error.loc, ErrorType::Note);
                    return {};
                }

                Substitutions ret = t1;
                ret.insert(ret.end(), t2.begin(), t2.end());
                return ret;
            }catch(CtError e){
                return t2;
            }
        }
    }

    Substitutions unify(UnificationList const& cur){
        return unify(cur, cur.rbegin(), true);
    }
}
