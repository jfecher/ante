#include "antype.h"
#include "types.h"

using namespace std;
using namespace ante::parser;

namespace ante {

    AnTypeContainer typeArena;

    void AnType::dump() const{
        if(auto *dt = llvm::dyn_cast<AnDataType>(this)){
            cout << dt->name;
            if(!dt->generics.empty()){
                cout << "[";
                for(auto &t : dt->generics){
                    if(&t != &dt->generics.back())
                        cout << anTypeToStr(t) << ", ";
                    else
                        cout << anTypeToStr(t) << "]";
                }
            }
            if(!dt->boundGenerics.empty()){
                cout << "<";
                for(auto &p : dt->boundGenerics){
                    if(&p != &dt->boundGenerics.back())
                        cout << p.first << " -> " << anTypeToStr(p.second) << ", ";
                    else
                        cout << p.first << " -> " << anTypeToStr(p.second) << ">";
                }
            }
            cout << " = " << anTypeToStr(AnAggregateType::get(TT_Tuple, dt->extTys));
        }else{
            cout << anTypeToStr(this);
        }
        cout << endl;
    }

    bool isGeneric(const std::vector<AnType*> &vec){
        for(auto *t : vec)
            if(t->isGeneric)
                return true;
        return false;
    }

    bool isGeneric(const std::vector<std::pair<std::string, AnType*>> &vec){
        for(auto &p : vec)
            if(p.second->isGeneric)
                return true;
        return false;
    }


    bool AnType::hasModifier(TokenType m) const{
        if(!mods) return false;
        return std::find(mods->modifiers.cbegin(), mods->modifiers.cend(), m) != mods->modifiers.end();
    }

    AnType* AnType::addModifier(TokenType m){
        if(mods){
            if(hasModifier(m)){
                return this;
            }else{
                auto modifiers = mods->modifiers;
                modifiers.push_back(m);
                return AnType::getPrimitive(typeTag, AnModifier::get(modifiers));
            }
        }
        return AnType::getPrimitive(typeTag, AnModifier::get({m}));
    }

    unsigned short AnDataType::getTagVal(std::string &name){
        for(auto& tag : tags){
            if(tag->name == name){
                return tag->tag;
            }
        }

        std::cerr << "No value found for tag " << name
                    << " of type " << this->name << std::endl;
        throw new CtError();
    }

    string modifiersToStr(const AnModifier *m){
        string ret = "";
        if(m)
            for(auto tok : m->modifiers)
                ret += Lexer::getTokStr(tok) + " ";
        return ret;
    }

    string typeTagToStrWithModifiers(TypeTag tag, AnModifier *m){
        return modifiersToStr(m) + typeTagToStr(tag);
    }

    template<typename T>
    T* search(llvm::StringMap<unique_ptr<T>> &map, string &key){
        auto it = map.find(key);
        if(it != map.end())
            return it->getValue().get();
        return nullptr;
    }

    template<typename T>
    void addKVPair(llvm::StringMap<unique_ptr<T>> &map, string const& key, T* val){
        auto entry = llvm::StringMapEntry<unique_ptr<T>>::Create(key, unique_ptr<T>(val));
        map.insert(entry);
    }

    AnType* AnType::getPrimitive(TypeTag tag, AnModifier *m){
        if(!m){
            switch(tag){
                case TT_I8:           return typeArena.primitiveTypes[tag].get();
                case TT_I16:          return typeArena.primitiveTypes[tag].get();
                case TT_I32:          return typeArena.primitiveTypes[tag].get();
                case TT_I64:          return typeArena.primitiveTypes[tag].get();
                case TT_Isz:          return typeArena.primitiveTypes[tag].get();
                case TT_U8:           return typeArena.primitiveTypes[tag].get();
                case TT_U16:          return typeArena.primitiveTypes[tag].get();
                case TT_U32:          return typeArena.primitiveTypes[tag].get();
                case TT_U64:          return typeArena.primitiveTypes[tag].get();
                case TT_Usz:          return typeArena.primitiveTypes[tag].get();
                case TT_F16:          return typeArena.primitiveTypes[tag].get();
                case TT_F32:          return typeArena.primitiveTypes[tag].get();
                case TT_F64:          return typeArena.primitiveTypes[tag].get();
                case TT_C8:           return typeArena.primitiveTypes[tag].get();
                case TT_C32:          return typeArena.primitiveTypes[tag].get();
                case TT_Bool:         return typeArena.primitiveTypes[tag].get();
                case TT_Void:         return typeArena.primitiveTypes[tag].get();
                case TT_Type:         return typeArena.primitiveTypes[tag].get();
                case TT_FunctionList: return typeArena.primitiveTypes[tag].get();
                default:
                    cerr << "error: AnType::getPrimitive: TypeTag " << typeTagToStr(tag) << " is not primitive!\n";
                    throw new CtError();
            }
        }else{
            string key = typeTagToStrWithModifiers(tag, m);

            auto existing_ty = search(typeArena.otherTypes, key);
            if(existing_ty) return existing_ty;

            auto *ty = new AnType(tag, false, 1, m);
            addKVPair(typeArena.otherTypes, key, ty);
            return ty;
        }
    }


    AnType* AnType::getI8(){
        return typeArena.primitiveTypes[TT_I8].get();
    }

    AnType* AnType::getI16(){
        return typeArena.primitiveTypes[TT_I16].get();
    }

    AnType* AnType::getI32(){
        return typeArena.primitiveTypes[TT_I32].get();
    }

    AnType* AnType::getI64(){
        return typeArena.primitiveTypes[TT_I64].get();
    }

    AnType* AnType::getIsz(){
        return typeArena.primitiveTypes[TT_Isz].get();
    }

    AnType* AnType::getU8(){
        return typeArena.primitiveTypes[TT_U8].get();
    }

    AnType* AnType::getU16(){
        return typeArena.primitiveTypes[TT_U16].get();
    }

    AnType* AnType::getU32(){
        return typeArena.primitiveTypes[TT_U32].get();
    }

    AnType* AnType::getU64(){
        return typeArena.primitiveTypes[TT_U64].get();
    }

    AnType* AnType::getUsz(){
        return typeArena.primitiveTypes[TT_Usz].get();
    }

    AnType* AnType::getF16(){
        return typeArena.primitiveTypes[TT_F16].get();
    }

    AnType* AnType::getF32(){
        return typeArena.primitiveTypes[TT_F32].get();
    }

    AnType* AnType::getF64(){
        return typeArena.primitiveTypes[TT_F64].get();
    }

    AnType* AnType::getBool(){
        return typeArena.primitiveTypes[TT_Bool].get();
    }

    AnType* AnType::getVoid(){
        return typeArena.primitiveTypes[TT_Void].get();
    }


    string getKey(const std::vector<TokenType> &mods){
        string ret = "";
        for(auto m : mods){
            ret += Lexer::getTokStr(m) + " ";
        }
        return ret;
    }

    AnModifier* AnModifier::get(const std::vector<TokenType> modifiers){
        auto key = getKey(modifiers);

        auto *existing_ty = search(typeArena.modifiers, key);
        if(existing_ty) return existing_ty;

        auto mod = new AnModifier(modifiers);
        addKVPair(typeArena.modifiers, key, mod);
        return mod;
    }


    AnPtrType* AnType::getPtr(AnType* ext){ return AnPtrType::get(ext); }
    AnPtrType* AnPtrType::get(AnType* ext, AnModifier *m){
        if(!m){
            try{
                auto *ptr = typeArena.ptrTypes.at(ext).get();
                return ptr;
            }catch(out_of_range r){
                auto ptr = new AnPtrType(ext, nullptr);
                typeArena.ptrTypes.emplace(ext, ptr);
                return ptr;
            }
        }else{
            string key = modifiersToStr(m) + anTypeToStr(ext) + "*";

            auto *existing_ty = search(typeArena.otherTypes, key);
            if(existing_ty) return (AnPtrType*)existing_ty;

            auto ptr = new AnPtrType(ext, m);
            addKVPair(typeArena.otherTypes, key, (AnType*)ptr);
            return ptr;
        }
    }

    AnArrayType* AnType::getArray(AnType* t, size_t len){ return AnArrayType::get(t,len); }
    AnArrayType* AnArrayType::get(AnType* t, size_t len, AnModifier *m){
        auto key = modifiersToStr(m) + to_string(len) + anTypeToStr(t);

        auto existing_ty = search(typeArena.arrayTypes, key);
        if(existing_ty) return existing_ty;

        auto arr = new AnArrayType(t, len, m);
        addKVPair(typeArena.arrayTypes, key, arr);
        return arr;
    }

    string getKey(const std::vector<AnType*> &exts){
        string ret = "";
        for(auto &ext : exts){
            ret += anTypeToStr(ext);
            if(&ext != &exts.back())
                ret += ", ";
        }
        return ret;
    }

    AnAggregateType* AnType::getAggregate(TypeTag t, const std::vector<AnType*> exts){
        return AnAggregateType::get(t, exts);
    }

    AnAggregateType* AnAggregateType::get(TypeTag t, const std::vector<AnType*> exts, AnModifier *m){
        auto key = modifiersToStr(m) + typeTagToStr(t) + getKey(exts);

        auto existing_ty = search(typeArena.aggregateTypes, key);
        if(existing_ty) return existing_ty;

        auto agg = new AnAggregateType(t, exts, m);
        addKVPair(typeArena.aggregateTypes, key, agg);
        return agg;
    }

    AnFunctionType* AnFunctionType::get(Compiler *c, AnType* retty, NamedValNode* params, bool isMetaFunction, AnModifier *m){
        vector<AnType*> extTys;

        while(params && params->typeExpr.get()){
            TypeNode *pty = (TypeNode*)params->typeExpr.get();
            auto *aty = toAnType(c, pty);
            extTys.push_back(aty);
            params = (NamedValNode*)params->next.get();
        }
        return AnFunctionType::get(retty, extTys, isMetaFunction, m);
    }


    AnFunctionType* AnFunctionType::get(AnType *retTy, const std::vector<AnType*> elems, bool isMetaFunction, AnModifier *m){
        auto key = modifiersToStr(m) + (isMetaFunction ? "1":"0") + getKey(elems) + "->" + anTypeToStr(retTy);

        auto existing_ty = search(typeArena.functionTypes, key);
        if(existing_ty) return existing_ty;

        auto f = new AnFunctionType(retTy, elems, isMetaFunction, m);

        addKVPair(typeArena.functionTypes, key, f);
        return f;
    }


    AnTypeVarType* AnType::getTypeVar(std::string name){
        return AnTypeVarType::get(name);
    }

    AnTypeVarType* AnTypeVarType::get(std::string name, AnModifier *m){
        string key = modifiersToStr(m) + name;

        auto existing_ty = search(typeArena.typeVarTypes, key);
        if(existing_ty) return existing_ty;

        auto tvar = new AnTypeVarType(name, m);
        addKVPair(typeArena.typeVarTypes, key, tvar);
        return tvar;
    }

    AnDataType* AnType::getDataType(string name){
        return AnDataType::get(name);
    }


    AnDataType* AnDataType::get(string const& name, AnModifier *m){
        string key = modifiersToStr(m) + name;

        auto existing_ty = search(typeArena.declaredTypes, key);
        if(existing_ty) return existing_ty;

        if(m){
            auto dt = AnDataType::get(name, nullptr);
            return dt->setModifier(m);
        }else{
            auto decl = new AnDataType(name, {}, false, m);
            addKVPair(typeArena.declaredTypes, key, decl);
            return decl;
        }
    }

    /**
     * Returns the unique key for the given variant and modifier pair.
     */
    string variantKey(const AnDataType *variant, AnModifier *m){
        return modifiersToStr(m) + anTypeToStr(variant);
    }

    AnDataType* AnDataType::getOrCreate(std::string const& name, std::vector<AnType*> const& elems, bool isUnion, AnModifier *m){
        string key = modifiersToStr(m) + name;

        auto existing_ty = search(typeArena.declaredTypes, key);
        if(existing_ty) return existing_ty;

        //create declaration w/out definition
        return AnDataType::create(name, elems, isUnion, {}, m);
    }

    AnDataType* AnDataType::getOrCreate(const AnDataType *dt, AnModifier *m){
        string key = modifiersToStr(m) + anTypeToStr(dt);

        if(dt->isVariant()){
            auto existing_ty = search(typeArena.genericVariants, key);
            if(existing_ty) return existing_ty;
        }else{
            auto existing_ty = search(typeArena.declaredTypes, key);
            if(existing_ty) return existing_ty;
        }

        //create declaration w/out definition
        AnDataType *ret;

        //Store the new dt in genericVariants or the standard container depending
        //on if it is a generic variant or parent type / non generic type.
        if(dt->isVariant()){
            ret = new AnDataType(dt->unboundType->name, {}, false, m);
            addKVPair(typeArena.genericVariants, variantKey(dt, m), ret);
        }else{
            ret = AnDataType::create(dt->name, {}, dt->typeTag == TT_TaggedUnion, dt->generics, m);
        }

        vector<AnType*> elems;
        elems.reserve(dt->extTys.size());
        for(auto *ty : dt->extTys){
            auto *mod_type = ty->setModifier(m);
            elems.emplace_back(mod_type);
        }

        ret->extTys = elems;
        ret->isGeneric = dt->isGeneric;
        ret->fields = dt->fields;
        ret->tags = dt->tags;
        ret->traitImpls = dt->traitImpls;
        ret->unboundType = dt->unboundType;
        ret->boundGenerics = dt->boundGenerics;
        ret->generics = dt->generics;
        ret->llvmType = dt->llvmType;
        return ret;
    }
        
    /** Returns the type this type is aliased to */
    AnType* AnDataType::getAliasedType() const {
        if(isAlias){
            if(extTys.size() == 1){
                return extTys[0];
            }else{
                return AnAggregateType::get(TT_Tuple, extTys);
            }
        }else{
            return AnType::getVoid();
        }
    }

    string getBoundName(const string &baseName, const vector<pair<string, AnType*>> &typeArgs){
        if(typeArgs.empty())
            return baseName;

        string name = baseName + "<";
        for(auto &p : typeArgs){
            if(p.second->typeTag != TT_TypeVar)
                name += anTypeToStr(p.second);
            if(&p != &typeArgs.back())
                name += ",";
        }
        return name == baseName + "<" ? baseName : name+">";
    }

    /*
    * Returns the unique boundName of a generic type after it is bound
    * with the specified type arguments
    */
    string getBoundName(const string &baseName, const vector<AnTypeVarType*> &typeArgs){
        if(typeArgs.empty())
            return baseName;

        string name = baseName + "<";
        for(auto &arg : typeArgs){
            if(arg->typeTag != TT_TypeVar)
                name += anTypeToStr(arg);
            if(&arg != &typeArgs.back())
                name += ",";
        }
        return name == baseName + "<" ? baseName : name + ">";
    }

    vector<AnType*> extractTypes(const vector<pair<string, AnType*>> &bindings){
        vector<AnType*> ret;
        ret.reserve(bindings.size());
        for(auto &p : bindings){
            ret.emplace_back(p.second);
        }
        return ret;
    }

    void removeDuplicates(vector<AnTypeVarType*> &vec){
        vector<AnTypeVarType*> ret;

        /* the pos after the current element */
        auto pos = ++vec.begin();
        for(auto &tvt : vec){
            bool append = true;
            for(auto it = pos; it != vec.end(); ++it){
                if(tvt == *it)
                    append = false;
            }
            if(append)
                ret.push_back(tvt);

            ++pos;
        }
        vec.swap(ret);
    }

    /*
     * Returns a vector of all typevars used by a given type
     */
    vector<AnTypeVarType*> getGenerics(AnType *t){
        if(AnDataType *dt = llvm::dyn_cast<AnDataType>(t)){
            return dt->generics;

        }else if(AnTypeVarType *tvt = llvm::dyn_cast<AnTypeVarType>(t)){
            return {tvt};

        }else if(AnPtrType *pt = llvm::dyn_cast<AnPtrType>(t)){
            return getGenerics(pt->extTy);

        }else if(AnArrayType *at = llvm::dyn_cast<AnArrayType>(t)){
            return getGenerics(at->extTy);

        }else if(AnFunctionType *ft = llvm::dyn_cast<AnFunctionType>(t)){
            vector<AnTypeVarType*> generics;
            for(auto *p : ft->extTys){
                auto p_generics = getGenerics(p);
                generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            }
            auto p_generics = getGenerics(ft->retTy);
            generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            return generics;

        }else if(AnAggregateType *agg = llvm::dyn_cast<AnAggregateType>(t)){
            vector<AnTypeVarType*> generics;
            for(auto *p : agg->extTys){
                auto p_generics = getGenerics(p);
                generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            }
            return generics;

        }else{
            return {};
        }
    }

    void addGenerics(vector<AnTypeVarType*> &dest, vector<AnType*> &src){
        for(auto *t : src){
            if(t->isGeneric){
                auto g = getGenerics(t);
                dest.insert(dest.end(), g.begin(), g.end());
            }
        }
        removeDuplicates(dest);
    }

    void addGenerics(vector<AnTypeVarType*> &dest, vector<pair<string, AnType*>> &src){
        for(auto &p : src){
            if(p.second->isGeneric){
                auto g = getGenerics(p.second);
                dest.insert(dest.end(), g.begin(), g.end());
            }
        }
        removeDuplicates(dest);
    }


    bool AnDataType::isVariantOf(const AnDataType *dt) const {
        AnDataType *unbound = this->unboundType;
        while(unbound){
            if(unbound == dt)
                return true;
            unbound = unbound->unboundType;
        }
        return false;
    }

    /*
     * Helper function for AnDataType::getVariant functions.
     * Overwrites a given AnDataType to be a bound variant of
     * the given generic type specified by unboundType.
     */
    AnDataType* bindVariant(Compiler *c, AnDataType *unboundType, const std::vector<std::pair<std::string,
            AnType*>> &bindings, AnModifier *m, AnDataType *variant){

        vector<AnType*> boundExts;
        boundExts.reserve(unboundType->extTys.size());

        unboundType->variants.push_back(variant);

        if(unboundType->generics.empty()){
            cerr << "WARNING: empty generics for parent type " << anTypeToStr(unboundType) << endl;
            variant->boundGenerics = bindings;

            vector<pair<string, AnType*>> boundBindings;
            for(auto &p : unboundType->boundGenerics){
                boundBindings.emplace_back(p.first, bindGenericToType(c, p.second, bindings));
            }
        }

        variant->boundGenerics = filterMatchingBindings(unboundType, bindings);
        variant->numMatchedTys = variant->boundGenerics.size() + 1;

        addGenerics(variant->generics, variant->boundGenerics);

        for(auto *e : unboundType->extTys){
            auto *be = bindGenericToType(c, e, bindings);
            boundExts.push_back(be);
        }

        if(unboundType->isUnionTag()){
            auto *unionType = unboundType->parentUnionType;
            unionType = (AnDataType*)bindGenericToType(c, unionType, bindings);
            updateLlvmTypeBinding(c, unionType, unionType->isGeneric);
            variant->parentUnionType = unionType;
        }

        if(boundExts.empty()){
            variant->isGeneric = isGeneric(variant->boundGenerics);
        }else{
            bool extsGeneric = isGeneric(boundExts);
            variant->isGeneric = extsGeneric;
        }

        variant->typeTag = unboundType->typeTag;
        variant->fields = unboundType->fields;
        variant->unboundType = unboundType;
        variant->extTys = boundExts;
        variant->tags = unboundType->tags;
        variant->traitImpls = unboundType->traitImpls;
        updateLlvmTypeBinding(c, variant, variant->isGeneric);
        return variant;
    }


    /*
     * Returns a bound variant of an unbound type whose bound
     * types match the given map of boundTys.  Returns nullptr
     * if such a type is not found.
     */
    AnDataType* findMatchingVariant(AnDataType *unboundType, const vector<pair<string, AnType*>> &boundTys){
        auto filteredBindings = filterMatchingBindings(unboundType, boundTys);

        for(auto &v : unboundType->variants){
            if(v->boundGenerics == filteredBindings){
                return v;
            }
        }
        return nullptr;
    }

    vector<pair<string, AnType*>> flatten(const Compiler *c, const AnDataType *dt, const vector<pair<string, AnType*>> &bindings){
        vector<pair<string, AnType*>> ret;
        if(dt->unboundType){
            //initial bindings are the generics of the parent type
            ret.reserve(dt->unboundType->generics.size());
            for(auto *tv : dt->unboundType->generics){
                ret.emplace_back(tv->name, tv);
            }

            //once the entire branch is bound, bind the bindings
            for(auto &p : ret){
                p.second = bindGenericToType((Compiler*)c, p.second, dt->boundGenerics);
            }

            for(auto &p : ret){
                p.second = bindGenericToType((Compiler*)c, p.second, bindings);
            }
        }else{
            ret.reserve(dt->generics.size());
            for(auto *tv : dt->generics){
                ret.emplace_back(tv->name, tv);
            }

            for(auto &p : ret){
                p.second = bindGenericToType((Compiler*)c, p.second, bindings);
            }
        }

        return ret;
    }

    /*
     * Searches for the bound variant of the generic type
     * unboundType and creates it if it has not been
     * previously bound.
     */
    AnDataType* AnDataType::getVariant(Compiler *c, AnDataType *unboundType, vector<pair<string, AnType*>> const& boundTys, AnModifier *m){
        auto filteredBindings = filterMatchingBindings(unboundType, boundTys);

        filteredBindings = flatten(c, unboundType, filteredBindings);

        if(unboundType->unboundType)
            unboundType = unboundType->unboundType;

        AnDataType *variant = findMatchingVariant(unboundType, filteredBindings);

        //variant is already bound
        if(variant)
            return variant;

        variant = new AnDataType(unboundType->name, {}, false, unboundType->mods);

        addKVPair(typeArena.genericVariants, variantKey(variant, m), variant);
        return bindVariant(c, unboundType, filteredBindings, m, variant);
    }

    /*
     * Searches for the bound variant of the generic type
     * specified by name and creates it if it has not been
     * previously bound.  Will fail if the given name does
     * not correspond to any defined type.
     */
    AnDataType* AnDataType::getVariant(Compiler *c, string const& name, vector<pair<string, AnType*>> const& boundTys, AnModifier *m){
        auto *unboundType = AnDataType::get(name, m);
        if(unboundType->isStub()){
            cerr << "Warning: Cannot bind undeclared type " << name << endl;
            return unboundType;
        }

        auto filteredBindings = filterMatchingBindings(unboundType, boundTys);
        filteredBindings = flatten(c, unboundType, filteredBindings);

        if(unboundType->unboundType)
            unboundType = unboundType->unboundType;

        AnDataType *variant = findMatchingVariant(unboundType, filteredBindings);

        //variant is already bound
        if(variant)
            return variant;

        variant = new AnDataType(unboundType->name, {}, false, m);
        addKVPair(typeArena.genericVariants, variantKey(variant, m), variant);
        return bindVariant(c, unboundType, filteredBindings, m, variant);
    }

    AnDataType* AnDataType::create(string const& name, vector<AnType*> const& elems, bool isUnion, vector<AnTypeVarType*> const& generics, AnModifier *m){
        string key = modifiersToStr(m) + getBoundName(name, generics);

        AnDataType *dt = search(typeArena.declaredTypes, key);

        if(dt){
            if(!dt->isStub()){
                dt->extTys = elems;
                dt->isGeneric = !generics.empty();
                dt->generics = generics;
                return dt;
            }
        }else{
            dt = new AnDataType(name, {}, isUnion, m);
            addKVPair(typeArena.declaredTypes, key, dt);
        }

        dt->isGeneric = !generics.empty();
        dt->generics = generics;

        vector<AnType*> elemsWithMods;
        elemsWithMods.reserve(elems.size());
        for(auto *ty : elems){
            auto *mod_type = ty->setModifier(m);
            elemsWithMods.emplace_back(mod_type);
        }

        dt->extTys = elemsWithMods;
        return dt;
    }

    //Constructor for AnTypeContainer, initializes all primitive types beforehand
    AnTypeContainer::AnTypeContainer(){
        primitiveTypes[TT_I8].reset(new AnType(TT_I8, false, 1, nullptr));
        primitiveTypes[TT_I16].reset(new AnType(TT_I16, false, 1, nullptr));
        primitiveTypes[TT_I32].reset(new AnType(TT_I32, false, 1, nullptr));
        primitiveTypes[TT_I64].reset(new AnType(TT_I64, false, 1, nullptr));
        primitiveTypes[TT_Isz].reset(new AnType(TT_Isz, false, 1, nullptr));
        primitiveTypes[TT_U8].reset(new AnType(TT_U8, false, 1, nullptr));
        primitiveTypes[TT_U16].reset(new AnType(TT_U16, false, 1, nullptr));
        primitiveTypes[TT_U32].reset(new AnType(TT_U32, false, 1, nullptr));
        primitiveTypes[TT_U64].reset(new AnType(TT_U64, false, 1, nullptr));
        primitiveTypes[TT_Usz].reset(new AnType(TT_Usz, false, 1, nullptr));
        primitiveTypes[TT_F16].reset(new AnType(TT_F16, false, 1, nullptr));
        primitiveTypes[TT_F32].reset(new AnType(TT_F32, false, 1, nullptr));
        primitiveTypes[TT_F64].reset(new AnType(TT_F64, false, 1, nullptr));
        primitiveTypes[TT_Bool].reset(new AnType(TT_Bool, false, 1, nullptr));
        primitiveTypes[TT_Void].reset(new AnType(TT_Void, false, 1, nullptr));
        primitiveTypes[TT_C8].reset(new AnType(TT_C8, false, 1, nullptr));
        primitiveTypes[TT_C32].reset(new AnType(TT_C32, false, 1, nullptr));
        primitiveTypes[TT_Type].reset(new AnType(TT_Type, false, 1, nullptr));
        primitiveTypes[TT_FunctionList].reset(new AnType(TT_FunctionList, false, 1, nullptr));
    }


    AnType* AnType::getFunctionReturnType() const{
        return ((AnFunctionType*)this)->retTy;
    }

    string typeNodeToStrWithModifiers(const TypeNode *tn){
        string ret = "";
        for(auto mod : tn->modifiers){
            ret += Lexer::getTokStr(mod) + " ";
        }
        return ret + typeNodeToStr(tn);
    }


    AnType* toAnType(Compiler *c, const TypeNode *tn){
        if(!tn) return AnType::getVoid();

        auto *mods = tn->modifiers.empty() ? nullptr : AnModifier::get(tn->modifiers);
        switch(tn->type){
            case TT_I8:
            case TT_I16:
            case TT_I32:
            case TT_I64:
            case TT_U8:
            case TT_U16:
            case TT_U32:
            case TT_U64:
            case TT_F16:
            case TT_F32:
            case TT_F64:
            case TT_Isz:
            case TT_Usz:
            case TT_C8:
            case TT_C32:
            case TT_Bool:
            case TT_Void:
                return AnType::getPrimitive(tn->type, mods);

            case TT_Function:
            case TT_MetaFunction:
            case TT_FunctionList: {
                TypeNode *ext = tn->extTy.get();
                AnType *ret = 0;
                vector<AnType*> tys;
                while(ext){
                    if(ret){
                        tys.push_back(toAnType(c, (TypeNode*)ext));
                    }else{
                        ret = toAnType(c, (TypeNode*)ext);
                    }
                    ext = (TypeNode*)ext->next.get();
                }
                return AnFunctionType::get(ret, tys, tn->type == TT_MetaFunction, mods);
            }
            case TT_Tuple: {
                TypeNode *ext = tn->extTy.get();
                vector<AnType*> tys;
                while(ext){
                    tys.push_back(toAnType(c, (TypeNode*)ext));
                    ext = (TypeNode*)ext->next.get();
                }
                return AnAggregateType::get(TT_Tuple, tys, mods);
            }

            case TT_Array: {
                TypeNode *elemTy = tn->extTy.get();
                IntLitNode *len = (IntLitNode*)elemTy->next.get();
                return AnArrayType::get(toAnType(c, elemTy), len ? stoi(len->val) : 0, mods);
            }
            case TT_Ptr:
                return AnPtrType::get(toAnType(c, tn->extTy.get()), mods);
            case TT_Data:
            case TT_TaggedUnion: {
                if(!tn->params.empty()){
                    vector<AnType*> bindings;
                    for(auto &t : tn->params)
                        bindings.emplace_back(toAnType(c, t.get()));

                    auto *basety = AnDataType::get(tn->typeName, mods);

                    return (AnDataType*)bindGenericToType(c, basety, bindings, basety);
                }else{
                    return AnDataType::get(tn->typeName, mods);
                }
            }
            case TT_TypeVar:
                return AnTypeVarType::get(tn->typeName, mods);
            default:
                cerr << "Unknown TypeTag " << typeTagToStr(tn->type) << endl;
                return nullptr;
        }
    }

    AnAggregateType* AnAggregateType::addModifier(TokenType m){
        if(mods){
            if(hasModifier(m)){
                return this;
            }else{
                auto modifiers = mods->modifiers;
                modifiers.push_back(m);
                auto *anmod = AnModifier::get(modifiers);

                vector<AnType*> modded_exts;
                modded_exts.reserve(extTys.size());
                for(auto &ext : extTys){
                    modded_exts.emplace_back(ext->setModifier(anmod));
                }

                return AnAggregateType::get(typeTag, modded_exts, anmod);
            }
        }

        auto *anmod = AnModifier::get({m});

        vector<AnType*> modded_exts;
        modded_exts.reserve(extTys.size());
        for(auto &ext : extTys){
            modded_exts.emplace_back(ext->setModifier(anmod));
        }
        return AnAggregateType::get(typeTag, modded_exts, anmod);
    }

    AnArrayType* AnArrayType::addModifier(TokenType m){
        if(mods){
            if(hasModifier(m)){
                return this;
            }else{
                auto modifiers = mods->modifiers;
                modifiers.push_back(m);
                auto *anmod = AnModifier::get(modifiers);
                return AnArrayType::get(extTy->setModifier(anmod), len, anmod);
            }
        }
        auto *anmod = AnModifier::get({m});
        return AnArrayType::get(extTy->setModifier(anmod), len, anmod);
    }

    AnPtrType* AnPtrType::addModifier(TokenType m){
        if(mods){
            if(hasModifier(m)){
                return this;
            }else{
                auto modifiers = mods->modifiers;
                modifiers.push_back(m);
                auto mods = AnModifier::get(modifiers);
                return AnPtrType::get(extTy->setModifier(mods), mods);
            }
        }
        auto mods = AnModifier::get({m});
        return AnPtrType::get(extTy->setModifier(mods), mods);
    }

    AnTypeVarType* AnTypeVarType::addModifier(TokenType m){
        if(mods){
            if(hasModifier(m)){
                return this;
            }else{
                auto modifiers = mods->modifiers;
                modifiers.push_back(m);
                return AnTypeVarType::get(name, AnModifier::get(modifiers));
            }
        }
        return AnTypeVarType::get(name, AnModifier::get({m}));
    }

    AnFunctionType* AnFunctionType::addModifier(TokenType m){
        if(mods){
            if(hasModifier(m)){
                return this;
            }else{
                auto modifiers = mods->modifiers;
                modifiers.push_back(m);
                return AnFunctionType::get(retTy, extTys,
                        typeTag == TT_MetaFunction, AnModifier::get(modifiers));
            }
        }
        return AnFunctionType::get(retTy, extTys,
                typeTag == TT_MetaFunction, AnModifier::get({m}));
    }

    AnDataType* AnDataType::addModifier(TokenType m){
        if(mods){
            if(hasModifier(m)){
                return this;
            }else{
                auto modifiers = mods->modifiers;
                modifiers.push_back(m);
                return AnDataType::getOrCreate(this, AnModifier::get(modifiers));
            }
        }
        return AnDataType::getOrCreate(this, AnModifier::get({m}));
    }

    AnType* AnType::setModifier(AnModifier *m){
        if(this->mods == m){
            return this;
        }else{
            return AnType::getPrimitive(typeTag, m);
        }
    }

    AnAggregateType* AnAggregateType::setModifier(AnModifier *m){
        if(this->mods == m){
            return this;
        }else{
            vector<AnType*> exts;
            exts.reserve(extTys.size());
            for(auto &ext : extTys){
                auto *mod_type = ext->setModifier(m);
                exts.emplace_back(mod_type);
            }
            return AnAggregateType::get(typeTag, exts, m);
        }
    }

    AnArrayType* AnArrayType::setModifier(AnModifier *m){
        if(this->mods == m){
            return this;
        }else{
            return AnArrayType::get(extTy->setModifier(m), len, m);
        }
    }

    AnPtrType* AnPtrType::setModifier(AnModifier *m){
        if(this->mods == m){
            return this;
        }else{
            return AnPtrType::get(extTy->setModifier(m), m);
        }
    }

    AnTypeVarType* AnTypeVarType::setModifier(AnModifier *m){
        if(this->mods == m){
            return this;
        }else{
            return AnTypeVarType::get(name, m);
        }
    }

    /*
     *  Set modifiers to an AnFunctionType, although unlike other AggregateTypes,
     *  the set modifiers do not apply to each of the extTys of the function as
     *  it would otherwise change the function signature when simply trying to
     *  make a mutable function pointer.
     */
    AnFunctionType* AnFunctionType::setModifier(AnModifier *m){
        if(this->mods == m){
            return this;
        }else{
            //vector<AnType*> exts(extTys.size());
            //for(auto &ext : extTys){
            //    exts.emplace_back(ext);
            //}
            return AnFunctionType::get(retTy, extTys, typeTag == TT_MetaFunction, m);
        }
    }

    AnDataType* AnDataType::setModifier(AnModifier *m){
        if(this->mods == m){
            return this;
        }else{
            return AnDataType::getOrCreate(this, m);
        }
    }
}
