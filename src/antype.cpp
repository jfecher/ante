#include "antype.h"
#include "types.h"
#include "uniontag.h"

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
                        cout << t << ", ";
                    else
                        cout << t << "]";
                }
            }
            if(dt->isVariant()){
                cout << "<";
                for(auto &b : dt->boundGenerics){
                    cout << b << ((&b != &dt->boundGenerics.back()) ? ", " : ">");
                }
            }
            cout << " = ";
            if(dt->extTys.empty()){
                cout << "()";
            }else{
                for(auto &ext : dt->extTys){
                    cout << anTypeToStr(ext);
                    if(&ext != &dt->extTys.back())
                        cout << (dt->typeTag == TT_TaggedUnion? " | " : ", ");
                }
            }
        }else{
            cout << anTypeToStr(this);
        }
        cout << endl;
    }

    bool isGeneric(vector<AnType*> const& vec){
        for(auto *t : vec)
            if(t->isGeneric)
                return true;
        return false;
    }

    bool isGeneric(vector<TypeBinding> const& vec){
        for(auto &p : vec)
            if(p.getBinding()->isGeneric)
                return true;
        return false;
    }


    bool AnType::hasModifier(TokenType m) const{
        return false;
    }


    bool BasicModifier::hasModifier(TokenType m) const {
        return mod == m or extTy->hasModifier(m);
    }


    bool CompilerDirectiveModifier::hasModifier(TokenType m) const {
        return extTy->hasModifier(m);
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

    size_t getNumMatchedTys(const vector<AnType*> &types){
        size_t ret = 0;
        for(auto *ty : types) ret += ty->numMatchedTys;
        return ret;
    }

    unsigned short AnDataType::getTagVal(std::string const& name){
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

    template<typename Key, typename Val>
    Val* search(std::unordered_map<Key, unique_ptr<Val>> &map, Key const& key){
        auto it = map.find(key);
        if(it != map.end())
            return it->second.get();
        return nullptr;
    }

    template<typename Key, typename Val>
    void addKVPair(std::unordered_map<Key, unique_ptr<Val>> &map, Key const& key, Val* val){
        if(map[key]){
            cout << lazy_str("WARNING", AN_WARN_COLOR) << ": Hash collision between "
                << anTypeToColoredStr(map[key].get()) << " and " << anTypeToColoredStr(val) << endl;
        }
        map[key] = unique_ptr<Val>(val);
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

    BasicModifier* BasicModifier::get(const AnType *modifiedType, TokenType mod){
        auto key = make_pair((AnType*)modifiedType, mod);

        auto *existing_ty = search(typeArena.basicModifiers, key);
        if(existing_ty) return static_cast<BasicModifier*>(existing_ty);

        auto ret = new BasicModifier(modifiedType, mod);
        addKVPair(typeArena.basicModifiers, key, (AnModifier*)ret);
        return ret;
    }

    /** NOTE: this treats all directives as different and will break
     * reference equality for these types.  In practice this is not too
     * problematic as it is impossible to compare the arbitrary expressions
     * anyways. */
    CompilerDirectiveModifier* CompilerDirectiveModifier::get(const AnType *modifiedType, Node *directive){
        auto key = make_pair((AnType*)modifiedType, (size_t)directive);

        auto *existing_ty = search(typeArena.cdModifiers, key);
        if(existing_ty) return static_cast<CompilerDirectiveModifier*>(existing_ty);

        auto ret = new CompilerDirectiveModifier(modifiedType, directive);
        addKVPair(typeArena.cdModifiers, key, (AnModifier*)ret);
        return ret;
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
        auto key = make_pair(t, len);

        auto existing_ty = search(typeArena.arrayTypes, key);
        if(existing_ty) return existing_ty;

        auto arr = new AnArrayType(t, len);
        addKVPair(typeArena.arrayTypes, key, arr);
        return arr;
    }

    AnAggregateType* AnType::getAggregate(TypeTag t, const std::vector<AnType*> exts){
        return AnAggregateType::get(t, exts);
    }

    AnAggregateType* AnAggregateType::get(TypeTag t, const std::vector<AnType*> exts){
        auto key = make_pair(t, exts);

        auto existing_ty = search(typeArena.aggregateTypes, key);
        if(existing_ty) return existing_ty;

        auto agg = new AnAggregateType(t, exts);
        addKVPair(typeArena.aggregateTypes, key, agg);
        return agg;
    }

    AnFunctionType* AnFunctionType::get(AnType* retty, NamedValNode* params, bool isMetaFunction){
        vector<AnType*> extTys;

        while(params && params->typeExpr.get()){
            TypeNode *pty = (TypeNode*)params->typeExpr.get();
            auto *aty = toAnType(pty);
            extTys.push_back(aty);
            params = (NamedValNode*)params->next.get();
        }
        return AnFunctionType::get(retty, extTys, isMetaFunction);
    }

    AnFunctionType* AnFunctionType::get(AnType *retTy, const std::vector<AnType*> elems, bool isMetaFunction){
        auto key = make_pair(retTy, make_pair(elems, isMetaFunction));

        auto existing_ty = search(typeArena.functionTypes, key);
        if(existing_ty) return existing_ty;

        auto f = new AnFunctionType(retTy, elems, isMetaFunction);

        addKVPair(typeArena.functionTypes, key, f);
        return f;
    }


    AnTypeVarType* AnType::getTypeVar(std::string const& name){
        return AnTypeVarType::get(name);
    }

    AnTypeVarType* AnTypeVarType::get(std::string const& name){
        auto key = name;

        auto existing_ty = search(typeArena.typeVarTypes, key);
        if(existing_ty) return existing_ty;

        auto tvar = new AnTypeVarType(name);
        addKVPair(typeArena.typeVarTypes, key, tvar);
        return tvar;
    }

    AnDataType* AnType::getDataType(string const& name){
        return AnDataType::get(name);
    }

    AnDataType* AnDataType::get(string const& name){
        auto key = name;

        auto existing_ty = search(typeArena.declaredTypes, key);
        if(existing_ty) return existing_ty;

        auto decl = new AnDataType(name, {}, false);
        addKVPair(typeArena.declaredTypes, key, decl);
        return decl;
    }

    /**
     * Returns the unique key for the given variant and modifier pair.
     */
    AnDataType* AnDataType::getOrCreate(std::string const& name, std::vector<AnType*> const& elems, bool isUnion){
        auto key = name;

        auto existing_ty = search(typeArena.declaredTypes, key);
        if(existing_ty) return existing_ty;

        //create declaration w/out definition
        return AnDataType::create(name, elems, isUnion, {});
    }

    AnDataType* AnDataType::getOrCreate(const AnDataType *dt){
        if(dt->isVariant()){
            auto key = make_pair(dt->name, dt->boundGenerics);
            auto existing_ty = search(typeArena.genericVariants, key);
            if(existing_ty) return existing_ty;
        }else{
            auto key = dt->name;
            auto existing_ty = search(typeArena.declaredTypes, key);
            if(existing_ty) return existing_ty;
        }

        //create declaration w/out definition
        AnDataType *ret;

        //Store the new dt in genericVariants or the standard container depending
        //on if it is a generic variant or parent type / non generic type.
        if(dt->isVariant()){
            ret = new AnDataType(dt->unboundType->name, {}, false);
            addKVPair(typeArena.genericVariants, make_pair(dt->name, dt->boundGenerics), ret);
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

    vector<AnType*> extractTypes(const vector<TypeBinding> &bindings){
        auto ret = vecOf<AnType*>(bindings.size());
        for(auto &p : bindings){
            ret.emplace_back(p.getBinding());
        }
        return ret;
    }

    void removeDuplicates(vector<GenericTypeParam> &vec){
        vector<GenericTypeParam> ret;

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
    vector<GenericTypeParam> getGenerics(AnType *t){
        if(AnDataType *dt = try_cast<AnDataType>(t)){
            return dt->generics;

        }else if(AnTypeVarType *tvt = try_cast<AnTypeVarType>(t)){
            return {tvt->name};

        }else if(AnPtrType *pt = try_cast<AnPtrType>(t)){
            return getGenerics(pt->extTy);

        }else if(AnArrayType *at = try_cast<AnArrayType>(t)){
            return getGenerics(at->extTy);

        }else if(AnFunctionType *ft = try_cast<AnFunctionType>(t)){
            vector<GenericTypeParam> generics;
            for(auto *p : ft->extTys){
                auto p_generics = getGenerics(p);
                generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            }
            auto p_generics = getGenerics(ft->retTy);
            generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            return generics;

        }else if(AnAggregateType *agg = try_cast<AnAggregateType>(t)){
            vector<GenericTypeParam> generics;
            for(auto *p : agg->extTys){
                auto p_generics = getGenerics(p);
                generics.insert(generics.end(), p_generics.begin(), p_generics.end());
            }
            return generics;

        }else{
            return {};
        }
    }

    void addGenerics(vector<GenericTypeParam> &dest, vector<AnType*> const& src){
        for(auto *t : src){
            if(t->isGeneric){
                auto g = getGenerics(t);
                dest.insert(dest.end(), g.begin(), g.end());
            }
        }
        removeDuplicates(dest);
    }

    void addGenerics(vector<GenericTypeParam> &dest, vector<TypeBinding> const& src){
        for(auto &p : src){
            if(p.getBinding()->isGeneric){
                auto g = getGenerics(p.getBinding());
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

    /**
     *  Converts a vector of structured bindings to a vector
     *  of nominal bindings.  Used when binding the converting
     *  the generic args of a generic variant to the nominal args
     *  needed to actually bind its contained types.
     */
    vector<TypeBinding> mapStructuredBindingsToNamedBindings(AnDataType *unboundType,
            vector<TypeBinding> const& bindings){

        auto ret = vecOf<TypeBinding>(bindings.size());

        for(const auto& binding : bindings){
            if(binding.isNominalBinding()){
                cerr << lazy_str("WARNING: ", AN_WARN_COLOR) << "Nominal binding `"
                    << binding << "` used in datatype mapping, ignoring.\n";
            }else{
                string typeVarName = unboundType->generics[binding.getIndex()].typeVarName;
                ret.emplace_back(typeVarName, binding.getBinding());
            }
        }
        return ret;
    }

    /*
     * Helper function for AnDataType::getVariant functions.
     * Overwrites a given AnDataType to be a bound variant of
     * the given generic type specified by unboundType.
     */
    AnDataType* bindVariant(AnDataType *unboundType,
            vector<TypeBinding> const& bindings, AnDataType *variant){

        auto boundExts = vecOf<AnType*>(unboundType->extTys.size());

        unboundType->variants.push_back(variant);

        if(unboundType->generics.empty()){
            cerr << "WARNING: empty generics for parent type " << anTypeToStr(unboundType) << endl;
            variant->boundGenerics = bindings;

            vector<TypeBinding> boundBindings;
            for(auto &p : unboundType->boundGenerics){
                boundBindings.emplace_back(p.getTypeVarName(), bindGenericToType(p.getBinding(), bindings));
            }
        }

        variant->boundGenerics = bindings;
        variant->numMatchedTys = variant->boundGenerics.size() + 1;

        addGenerics(variant->generics, variant->boundGenerics);

        auto internalBindings = mapStructuredBindingsToNamedBindings(unboundType, bindings);
        for(auto *e : unboundType->extTys){
            auto *be = bindGenericToType(e, internalBindings);
            boundExts.push_back(be);
        }

        if(unboundType->isUnionTag()){
            auto *unionType = unboundType->parentUnionType;
            unionType = try_cast<AnDataType>(bindGenericToType(unionType, bindings));
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
        return variant;
    }


    /*
     * Returns a bound variant of an unbound type whose bound
     * types match the given map of boundTys.  Returns nullptr
     * if such a type is not found.
     */
    AnDataType* findMatchingVariant(AnDataType *unboundType, vector<TypeBinding> const& boundTys){
        auto filteredBindings = filterMatchingBindings(unboundType, boundTys);

        for(auto &v : unboundType->variants){
            if(v->boundGenerics == filteredBindings){
                return v;
            }
        }
        return nullptr;
    }

    /* If generic types are bound to any other type (that is possibly already generic),
     * then the generic type list of a given type will become more of a tree, eg.
     *
     * List 't => List (Ptr 'u) => List (Ptr ('a, 'b)) => List (Ptr (i32, i32))
     *         => List i32
     *         => List ('a, 'a) => List (Str, Str)
     *                          => List (usz, usz)
     *
     * The flatten function takes a generic variant and a list of bindings and binds
     * the type relative to the parent type so that the full tree above never forms.
     * The tree is flattened to a lsit as soon as a variant is bound relative to another,
     * take, eg:
     *
     * List 't => List (Ptr 'u)
     *
     * After receiving the binding 'u => ('a, 'b) flatten performs this binding relative
     * to List 't rather than List (Ptr 'u) so the result after this step is
     *
     * List 't => List (Ptr 'u)
     *         => List (Ptr ('a, 'b))
     */
    vector<TypeBinding> flatten(const AnDataType *dt,
            vector<TypeBinding> const& bindings){

        vector<TypeBinding> ret;

        if(dt->isVariant()){
            ret = dt->boundGenerics;

            for(auto &p : ret){
                p.setBinding(bindGenericToType(p.getBinding(), bindings));
            }
        }

        //bind any structural bindings
        for(auto &g : dt->generics){
            if(!g.isNominalBinding()){
                auto binding = findBindingFor({"", dt, g.pos}, bindings);
                if(binding)
                    ret.push_back(*binding);
            }
        }

        return ret;
    }

    /*
     * Searches for the bound variant of the generic type
     * unboundType and creates it if it has not been
     * previously bound.
     */
    AnDataType* AnDataType::getVariant(AnDataType *unboundType,
            vector<TypeBinding> const& boundTys){

        //type is fully bound and no longer generic, early return
        if(!unboundType->isGeneric && !unboundType->isVariant())
            return unboundType;

        auto filteredBindings = filterMatchingBindings(unboundType, boundTys);
        filteredBindings = flatten(unboundType, filteredBindings);

        if(filteredBindings.empty())
            return unboundType;

        if(unboundType->isVariant())
            unboundType = unboundType->unboundType;

        AnDataType *variant = findMatchingVariant(unboundType, filteredBindings);

        //variant is already bound
        if(variant)
            return variant;

        variant = new AnDataType(unboundType->name, {}, false);

        variant = bindVariant(unboundType, filteredBindings, variant);
        addKVPair(typeArena.genericVariants, make_pair(variant->name, variant->boundGenerics), variant);
        return variant;
    }

    /*
     * Searches for the bound variant of the generic type
     * specified by name and creates it if it has not been
     * previously bound.  Will fail if the given name does
     * not correspond to any defined type.
     */
    AnDataType* AnDataType::getVariant(string const& name,
            vector<TypeBinding> const& boundTys){

        auto *unboundType = AnDataType::get(name);
        if(unboundType->isStub()){
            cerr << "Warning: Cannot bind undeclared type " << name << endl;
            return unboundType;
        }

        return AnDataType::getVariant(unboundType, boundTys);
    }

    AnDataType* AnDataType::create(string const& name, vector<AnType*> const& elems,
            bool isUnion, vector<GenericTypeParam> const& generics){
        auto key = name;

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

        for(size_t i = 0; i < dt->generics.size(); i++){
            auto &g = dt->generics[i];
            if(!g.dt){
                g.dt = dt;
                g.pos = i;
            }
        }

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


    AnType* toAnType(const TypeNode *tn){
        if(!tn) return AnType::getVoid();
        AnType *ret;

        switch(tn->typeTag){
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
                ret = AnType::getPrimitive(tn->typeTag);
                break;

            case TT_Function:
            case TT_MetaFunction:
            case TT_FunctionList: {
                TypeNode *ext = tn->extTy.get();
                AnType *retty = 0;
                vector<AnType*> tys;
                while(ext){
                    if(retty){
                        tys.push_back(toAnType(ext));
                    }else{
                        retty = toAnType(ext);
                    }
                    ext = (TypeNode*)ext->next.get();
                }
                ret = AnFunctionType::get(retty, tys, tn->typeTag == TT_MetaFunction);
                break;
            }
            case TT_Tuple: {
                TypeNode *ext = tn->extTy.get();
                vector<AnType*> tys;
                while(ext){
                    tys.push_back(toAnType(ext));
                    ext = (TypeNode*)ext->next.get();
                }
                ret = AnAggregateType::get(TT_Tuple, tys);
                break;
            }

            case TT_Array: {
                TypeNode *elemTy = tn->extTy.get();
                IntLitNode *len = (IntLitNode*)elemTy->next.get();
                ret = AnArrayType::get(toAnType(elemTy), len ? stoi(len->val) : 0);
                break;
            }
            case TT_Ptr:
                ret = AnPtrType::get(toAnType(tn->extTy.get()));
                break;
            case TT_Data:
            case TT_TaggedUnion: {
                if(!tn->params.empty()){
                    auto *basety = AnDataType::get(tn->typeName);

                    vector<TypeBinding> bindings;
                    for(size_t i = 0; i < tn->params.size(); i++){
                        auto *b = toAnType(tn->params[i].get());
                        //empty string because we cannot know the original typevar used in the declaration
                        //and it is unneeded except for when printing a parent datatype, which this is not.
                        bindings.emplace_back("", basety, i, b);
                    }

                    ret = try_cast<AnDataType>(bindGenericToType(basety, bindings));
                }else{
                    ret = AnDataType::get(tn->typeName);
                }
                break;
            }
            case TT_TypeVar:
                ret = AnTypeVarType::get(tn->typeName);
                break;
            default:
                cerr << "Unknown TypeTag " << typeTagToStr(tn->typeTag) << endl;
                return nullptr;
        }

        for(auto &m : tn->modifiers){
            if(m->isCompilerDirective()){
                ret = CompilerDirectiveModifier::get(ret, m->directive.get());
            }else{
                ret = (AnType*)ret->addModifier((TokenType)m->mod);
            }
        }
        return ret;
    }

    const AnType* BasicModifier::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        if(this->mod == m or (this->mod == Tok_Const && m == Tok_Mut))
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
