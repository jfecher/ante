#include "antype.h"
#include "types.h"

using namespace std;
using namespace ante::parser;

namespace ante {

    AnTypeContainer typeArena;

    void AnType::dump() const{
        if(auto *dt = try_cast<AnDataType>(this)){
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
        return false;
    }


    bool BasicModifier::hasModifier(TokenType m) const {
        return this->mod == m;
    }


    const AnType* AnType::addModifier(TokenType m) const {
        if(m == Tok_Let) return this;
        return BasicModifier::get((AnType*)this, m);
    }

    //base case, generic AnType has no mods
    const AnType* AnType::addModifiersTo(const AnType* t) const {
        return t;
    }

    //base case, generic AnType has no mods
    const AnType* BasicModifier::addModifiersTo(const AnType* t) const {
        return extTy->addModifiersTo(t)->addModifier(this->mod);
    }

    //base case, generic AnType has no mods
    const AnType* CompilerDirectiveModifier::addModifiersTo(const AnType* t) const {
        return CompilerDirectiveModifier::get(extTy->addModifiersTo(t), directive);
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

    //string modifiersToStr(const AnModifier *m){
    //    string ret = "";
    //    if(m)
    //        for(auto tok : m->modifiers)
    //            ret += Lexer::getTokStr(tok) + " ";
    //    return ret;
    //}

    template<typename T>
    T* search(llvm::StringMap<unique_ptr<T>> &map, string const& key){
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

    AnType* AnType::getPrimitive(TypeTag tag){
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


    string getKey(const AnType *modifiedType, TokenType mod){
        return to_string(mod) + anTypeToStr(modifiedType);
    }

    BasicModifier* BasicModifier::get(const AnType *modifiedType, TokenType mod){
        auto key = getKey(modifiedType, mod);

        auto *existing_ty = search(typeArena.modifiers, key);
        if(existing_ty) return static_cast<BasicModifier*>(existing_ty);

        auto ret = new BasicModifier(modifiedType, mod);
        addKVPair(typeArena.modifiers, key, (AnModifier*)ret);
        return ret;
    }

    /** NOTE: this treats all directives as different and will break
     * reference equality for these types.  In practice this is not too
     * problematic as it is impossible to compare the arbitrary expressions
     * anyways. */
    string getKey(const AnType *modifiedType, Node* directive){
        return to_string((size_t)directive);
    }

    CompilerDirectiveModifier* CompilerDirectiveModifier::get(const AnType *modifiedType,
            const std::shared_ptr<Node> &directive){

        auto key = getKey(modifiedType, directive.get());

        auto *existing_ty = search(typeArena.modifiers, key);
        if(existing_ty) return static_cast<CompilerDirectiveModifier*>(existing_ty);

        auto ret = new CompilerDirectiveModifier(modifiedType, directive);
        addKVPair(typeArena.modifiers, key, (AnModifier*)ret);
        return ret;
    }

    CompilerDirectiveModifier* CompilerDirectiveModifier::get(const AnType *modifiedType, const Node *directive){
        shared_ptr<Node> dir{(Node*)directive};
        return CompilerDirectiveModifier::get(modifiedType, dir);
    }


    AnPtrType* AnType::getPtr(AnType* ext){ return AnPtrType::get(ext); }
    AnPtrType* AnPtrType::get(AnType* ext){
        try{
            auto *ptr = typeArena.ptrTypes.at(ext).get();
            return ptr;
        }catch(out_of_range &r){
            auto ptr = new AnPtrType(ext);
            typeArena.ptrTypes.emplace(ext, ptr);
            return ptr;
        }
    }

    AnArrayType* AnType::getArray(AnType* t, size_t len){ return AnArrayType::get(t,len); }
    AnArrayType* AnArrayType::get(AnType* t, size_t len){
        auto key = to_string(len) + anTypeToStr(t);

        auto existing_ty = search(typeArena.arrayTypes, key);
        if(existing_ty) return existing_ty;

        auto arr = new AnArrayType(t, len);
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

    AnAggregateType* AnAggregateType::get(TypeTag t, const std::vector<AnType*> exts){
        auto key = typeTagToStr(t) + getKey(exts);

        auto existing_ty = search(typeArena.aggregateTypes, key);
        if(existing_ty) return existing_ty;

        auto agg = new AnAggregateType(t, exts);
        addKVPair(typeArena.aggregateTypes, key, agg);
        return agg;
    }

    AnFunctionType* AnFunctionType::get(Compiler *c, AnType* retty, NamedValNode* params, bool isMetaFunction){
        vector<AnType*> extTys;

        while(params && params->typeExpr.get()){
            TypeNode *pty = (TypeNode*)params->typeExpr.get();
            auto *aty = toAnType(c, pty);
            extTys.push_back(aty);
            params = (NamedValNode*)params->next.get();
        }
        return AnFunctionType::get(retty, extTys, isMetaFunction);
    }


    AnFunctionType* AnFunctionType::get(AnType *retTy, const std::vector<AnType*> elems, bool isMetaFunction){
        auto key = (isMetaFunction ? "1":"0") + getKey(elems) + "->" + anTypeToStr(retTy);

        auto existing_ty = search(typeArena.functionTypes, key);
        if(existing_ty) return existing_ty;

        auto f = new AnFunctionType(retTy, elems, isMetaFunction);

        addKVPair(typeArena.functionTypes, key, f);
        return f;
    }


    AnTypeVarType* AnType::getTypeVar(std::string name){
        return AnTypeVarType::get(name);
    }

    AnTypeVarType* AnTypeVarType::get(std::string name){
        string &key = name;

        auto existing_ty = search(typeArena.typeVarTypes, key);
        if(existing_ty) return existing_ty;

        auto tvar = new AnTypeVarType(name);
        addKVPair(typeArena.typeVarTypes, key, tvar);
        return tvar;
    }

    AnDataType* AnType::getDataType(string name){
        return AnDataType::get(name);
    }


    AnDataType* AnDataType::get(string const& name){
        string const& key = name;

        auto existing_ty = search(typeArena.declaredTypes, key);
        if(existing_ty) return existing_ty;

        auto decl = new AnDataType(name, {}, false);
        addKVPair(typeArena.declaredTypes, key, decl);
        return decl;
    }

    /**
     * Returns the unique key for the given variant and modifier pair.
     */
    string variantKey(const AnDataType *variant){
        return anTypeToStr(variant);
    }

    AnDataType* AnDataType::getOrCreate(std::string const& name, std::vector<AnType*> const& elems, bool isUnion){
        string const& key = name;

        auto existing_ty = search(typeArena.declaredTypes, key);
        if(existing_ty) return existing_ty;

        //create declaration w/out definition
        return AnDataType::create(name, elems, isUnion, {});
    }

    AnDataType* AnDataType::getOrCreate(const AnDataType *dt){
        string key = anTypeToStr(dt);

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
            ret = new AnDataType(dt->unboundType->name, {}, false);
            addKVPair(typeArena.genericVariants, variantKey(dt), ret);
        }else{
            ret = AnDataType::create(dt->name, {}, dt->typeTag == TT_TaggedUnion, dt->generics);
        }

        ret->extTys = dt->extTys;
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
        auto ret = vecOf<AnType*>(bindings.size());
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
        if(AnDataType *dt = try_cast<AnDataType>(t)){
            return dt->generics;

        }else if(AnTypeVarType *tvt = try_cast<AnTypeVarType>(t)){
            return {tvt};

        }else if(AnPtrType *pt = try_cast<AnPtrType>(t)){
            return getGenerics(pt->extTy);

        }else if(AnArrayType *at = try_cast<AnArrayType>(t)){
            return getGenerics(at->extTy);

        }else if(AnFunctionType *ft = try_cast<AnFunctionType>(t)){
            vector<AnTypeVarType*> generics;
            for(auto *p : ft->extTys){
                auto p_generics = getGenerics(p);
                generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            }
            auto p_generics = getGenerics(ft->retTy);
            generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            return generics;

        }else if(AnAggregateType *agg = try_cast<AnAggregateType>(t)){
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
            AnType*>> &bindings, AnDataType *variant){

        auto boundExts = vecOf<AnType*>(unboundType->extTys.size());

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
            unionType = try_cast<AnDataType>(bindGenericToType(c, unionType, bindings));
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
    AnDataType* AnDataType::getVariant(Compiler *c, AnDataType *unboundType, vector<pair<string, AnType*>> const& boundTys){
        auto filteredBindings = filterMatchingBindings(unboundType, boundTys);

        filteredBindings = flatten(c, unboundType, filteredBindings);

        if(unboundType->unboundType)
            unboundType = unboundType->unboundType;

        AnDataType *variant = findMatchingVariant(unboundType, filteredBindings);

        //variant is already bound
        if(variant)
            return variant;

        variant = new AnDataType(unboundType->name, {}, false);

        addKVPair(typeArena.genericVariants, variantKey(variant), variant);
        return bindVariant(c, unboundType, filteredBindings, variant);
    }

    /*
     * Searches for the bound variant of the generic type
     * specified by name and creates it if it has not been
     * previously bound.  Will fail if the given name does
     * not correspond to any defined type.
     */
    AnDataType* AnDataType::getVariant(Compiler *c, string const& name, vector<pair<string, AnType*>> const& boundTys){
        auto *unboundType = AnDataType::get(name);
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

        variant = new AnDataType(unboundType->name, {}, false);
        addKVPair(typeArena.genericVariants, variantKey(variant), variant);
        return bindVariant(c, unboundType, filteredBindings, variant);
    }

    AnDataType* AnDataType::create(string const& name, vector<AnType*> const& elems, bool isUnion, vector<AnTypeVarType*> const& generics){
        string key = getBoundName(name, generics);

        AnDataType *dt = search(typeArena.declaredTypes, key);

        if(dt){
            if(!dt->isStub()){
                dt->extTys = elems;
                dt->isGeneric = !generics.empty();
                dt->generics = generics;
                return dt;
            }
        }else{
            dt = new AnDataType(name, {}, isUnion);
            addKVPair(typeArena.declaredTypes, key, dt);
        }

        dt->isGeneric = !generics.empty();
        dt->generics = generics;
        dt->extTys = elems;
        return dt;
    }

    //Constructor for AnTypeContainer, initializes all primitive types beforehand
    AnTypeContainer::AnTypeContainer(){
        primitiveTypes[TT_I8].reset(new AnType(TT_I8, false, 1));
        primitiveTypes[TT_I16].reset(new AnType(TT_I16, false, 1));
        primitiveTypes[TT_I32].reset(new AnType(TT_I32, false, 1));
        primitiveTypes[TT_I64].reset(new AnType(TT_I64, false, 1));
        primitiveTypes[TT_Isz].reset(new AnType(TT_Isz, false, 1));
        primitiveTypes[TT_U8].reset(new AnType(TT_U8, false, 1));
        primitiveTypes[TT_U16].reset(new AnType(TT_U16, false, 1));
        primitiveTypes[TT_U32].reset(new AnType(TT_U32, false, 1));
        primitiveTypes[TT_U64].reset(new AnType(TT_U64, false, 1));
        primitiveTypes[TT_Usz].reset(new AnType(TT_Usz, false, 1));
        primitiveTypes[TT_F16].reset(new AnType(TT_F16, false, 1));
        primitiveTypes[TT_F32].reset(new AnType(TT_F32, false, 1));
        primitiveTypes[TT_F64].reset(new AnType(TT_F64, false, 1));
        primitiveTypes[TT_Bool].reset(new AnType(TT_Bool, false, 1));
        primitiveTypes[TT_Void].reset(new AnType(TT_Void, false, 1));
        primitiveTypes[TT_C8].reset(new AnType(TT_C8, false, 1));
        primitiveTypes[TT_C32].reset(new AnType(TT_C32, false, 1));
        primitiveTypes[TT_Type].reset(new AnType(TT_Type, false, 1));
        primitiveTypes[TT_FunctionList].reset(new AnType(TT_FunctionList, false, 1));
    }


    AnType* AnType::getFunctionReturnType() const{
        return try_cast<AnFunctionType>(this)->retTy;
    }


    AnType* toAnType(Compiler *c, const TypeNode *tn){
        if(!tn) return AnType::getVoid();
        AnType *ret;

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
                ret = AnType::getPrimitive(tn->type);
                break;

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
                ret = AnFunctionType::get(ret, tys, tn->type == TT_MetaFunction);
                break;
            }
            case TT_Tuple: {
                TypeNode *ext = tn->extTy.get();
                vector<AnType*> tys;
                while(ext){
                    tys.push_back(toAnType(c, (TypeNode*)ext));
                    ext = (TypeNode*)ext->next.get();
                }
                ret = AnAggregateType::get(TT_Tuple, tys);
                break;
            }

            case TT_Array: {
                TypeNode *elemTy = tn->extTy.get();
                IntLitNode *len = (IntLitNode*)elemTy->next.get();
                ret = AnArrayType::get(toAnType(c, elemTy), len ? stoi(len->val) : 0);
                break;
            }
            case TT_Ptr:
                ret = AnPtrType::get(toAnType(c, tn->extTy.get()));
                break;
            case TT_Data:
            case TT_TaggedUnion: {
                if(!tn->params.empty()){
                    vector<AnType*> bindings;
                    for(auto &t : tn->params)
                        bindings.emplace_back(toAnType(c, t.get()));

                    auto *basety = AnDataType::get(tn->typeName);

                    ret = try_cast<AnDataType>(bindGenericToType(c, basety, bindings, basety));
                }else{
                    ret = AnDataType::get(tn->typeName);
                }
                break;
            }
            case TT_TypeVar:
                ret = AnTypeVarType::get(tn->typeName);
                break;
            default:
                cerr << "Unknown TypeTag " << typeTagToStr(tn->type) << endl;
                return nullptr;
        }

        for(auto &m : tn->modifiers){
            if(m->isCompilerDirective()){
                ret = CompilerDirectiveModifier::get(ret, m->directive);
            }else{
                ret = (AnType*)ret->addModifier((TokenType)m->mod);
            }
        }
        return ret;
    }

    const AnType* BasicModifier::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        if(this->mod == m or (this->mod == Tok_Const and m == Tok_Mut))
            return this;
        else
            return BasicModifier::get(extTy->addModifier(m), mod);
    }

    const AnType* CompilerDirectiveModifier::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return CompilerDirectiveModifier::get(extTy->addModifier(m), directive);
    }

    const AnType* AnAggregateType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }

    const AnType* AnArrayType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }

    const AnType* AnPtrType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }

    const AnType* AnTypeVarType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }

    const AnType* AnFunctionType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }

    const AnType* AnDataType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }
}
