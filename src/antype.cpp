#include "antype.h"
#include "types.h"

using namespace std;

namespace ante {

    AnTypeContainer typeArena;

    void AnType::dump(){
        cout << anTypeToStr(this);
        if(auto *dt = llvm::dyn_cast<AnDataType>(this)){
            if(!dt->generics.empty()){
                cout << "<";
                for(auto &t : dt->generics){
                    if(&t != &dt->generics.back())
                        cout << anTypeToStr(t) << ", ";
                    else
                        cout << anTypeToStr(t) << ">";
                }
            }
            cout << " = " << anTypeToStr(AnAggregateType::get(TT_Tuple, dt->extTys));
        }
        cout << endl;
    }

    bool isGeneric(const std::vector<AnType*> &vec){
        for(auto *t : vec)
            if(t->isGeneric)
                return true;
        return false;
    }


    bool AnType::hasModifier(TokenType m){
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
            try{
                return typeArena.otherTypes.at(key).get();
            }catch(out_of_range r){
                auto *ty = new AnType(tag, false, m);
                typeArena.otherTypes.emplace(key, ty);
                return ty;
            }
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
        try{
            return typeArena.modifiers.at(key).get();
        }catch(out_of_range r){
            auto mod = new AnModifier(modifiers);
            typeArena.modifiers.emplace(key, mod);
            return mod;
        }
    }


    AnPtrType* AnType::getPtr(AnType* ext){ return AnPtrType::get(ext); }
    AnPtrType* AnPtrType::get(AnType* ext, AnModifier *m){
        if(!m){
            try{
                auto *ptr = typeArena.ptrTypes.at(ext).get();
                //cout << "  Got AnPtrType " << anTypeToStr(ptr) << ", isGeneric = " << ptr->isGeneric << ", ext->isGeneric = " << ptr->extTy->isGeneric << endl;
                return ptr;
            }catch(out_of_range r){
                auto ptr = new AnPtrType(ext, nullptr);
                //cout << "  Created AnPtrType " << anTypeToStr(ptr) << ", isGeneric = " << ptr->isGeneric << ", ext->isGeneric = " << ptr->extTy->isGeneric << endl;
                typeArena.ptrTypes.emplace(ext, ptr);
                return ptr;
            }
        }else{
            string key = modifiersToStr(m) + anTypeToStr(ext) + "*";
            try{
                return (AnPtrType*)typeArena.otherTypes.at(key).get();
            }catch(out_of_range r){
                auto ptr = new AnPtrType(ext, m);
                typeArena.otherTypes.emplace(key, ptr);
                return ptr;
            }
        }
    }

    AnArrayType* AnType::getArray(AnType* t, size_t len){ return AnArrayType::get(t,len); }
    AnArrayType* AnArrayType::get(AnType* t, size_t len, AnModifier *m){
        auto key = modifiersToStr(m) + to_string(len) + anTypeToStr(t);
        try{
            return typeArena.arrayTypes.at(key).get();
        }catch(out_of_range r){
            auto arr = new AnArrayType(t, len, m);
            typeArena.arrayTypes.emplace(key, arr);
            return arr;
        }
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
        try{
            return typeArena.aggregateTypes.at(key).get();
        }catch(out_of_range r){
            auto agg = new AnAggregateType(t, exts, m);
            typeArena.aggregateTypes.emplace(key, agg);
            return agg;
        }
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
        try{
            return typeArena.functionTypes.at(key).get();
        }catch(out_of_range r){
            auto f = new AnFunctionType(retTy, elems, isMetaFunction, m);
            typeArena.functionTypes.emplace(key, f);
            return f;
        }
    }


    AnTypeVarType* AnType::getTypeVar(std::string name){
        return AnTypeVarType::get(name);
    }

    AnTypeVarType* AnTypeVarType::get(std::string name, AnModifier *m){
        string key = modifiersToStr(m) + name;
        try{
            return typeArena.typeVarTypes.at(key).get();
        }catch(out_of_range r){
            auto tvar = new AnTypeVarType(name, m);
            typeArena.typeVarTypes.emplace(key, tvar);
            return tvar;
        }
    }

    AnDataType* AnType::getDataType(string name){
        return AnDataType::get(name);
    }


    AnDataType* AnDataType::get(string name, AnModifier *m){
        string key = modifiersToStr(m) + name;
        try{
            return typeArena.declaredTypes.at(key).get();
        }catch(out_of_range r){
            //create declaration w/out definition
            if(m){
                auto dt = AnDataType::get(name, nullptr);
                return dt->setModifier(m);
            }else{
                auto decl = new AnDataType(name, {}, false, m);
                typeArena.declaredTypes.emplace(key, decl);
                return decl;
            }
        }
    }

    AnDataType* AnDataType::getOrCreate(std::string name, std::vector<AnType*> &elems, bool isUnion, AnModifier *m){
        string key = modifiersToStr(m) + name;
        try{
            return typeArena.declaredTypes.at(key).get();
        }catch(out_of_range r){
            //create declaration w/out definition
            return AnDataType::create(name, elems, isUnion, {}, m);
        }
    }

    AnDataType* AnDataType::getOrCreate(const AnDataType *dt, AnModifier *m){
        string key = modifiersToStr(m) + dt->name;
        try{
            return typeArena.declaredTypes.at(key).get();
        }catch(out_of_range r){
            //create declaration w/out definition
            auto *ret = AnDataType::create(dt->name, {}, dt->typeTag == TT_TaggedUnion, dt->generics, m);
            
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

    void addGenerics(vector<AnTypeVarType*> &dest, vector<AnType*> &src){

    }
    
    void print(const vector<pair<string, AnType*>> boundTys){
        cout << "[";
        for(auto &p : boundTys){
            cout << p.first << " -> " << anTypeToStr(p.second);
            if(AnDataType *dt = llvm::dyn_cast<AnDataType>(p.second)){
                if(!dt->boundGenerics.empty())
                    print(dt->boundGenerics);
            }

            if(&p != &boundTys.back()){
                cout << ", ";
            }
        }
        cout << "]\n";
    }

    /*
     * Helper function for AnDataType::getVariant functions.
     * Overwrites a given AnDataType to be a bound variant of
     * the given generic type specified by unboundType.
     */
    AnDataType* bindVariant(Compiler *c, AnDataType *unboundType, const std::vector<std::pair<std::string, AnType*>> &bindings, AnModifier *m, AnDataType *variant){
        //This variant has never been bound before so fill in the stub
        vector<AnType*> boundExts;
        boundExts.reserve(unboundType->extTys.size());

        unboundType->variants.push_back(variant);

        if(unboundType->generics.empty()){
            cout << "WARNING: empty generics for parent type\n";
        }

        //variant->boundGenerics = filterMatchingBindings(unboundType, bindings);
        variant->boundGenerics = bindings;

        cout << "Just pushed variant " << unboundType->name;
        print(variant->boundGenerics);
        cout << "   Its old generics were ";
        print(bindings);

        for(auto *e : unboundType->extTys){
            auto *be = bindGenericToType(c, e, bindings);
            cout << "Binding " << anTypeToStr(variant) << " ext " << anTypeToStr(e) << " to " << anTypeToStr(be) << endl;
            boundExts.push_back(be);
        }

        if(unboundType->isUnionTag()){
            auto *unionType = unboundType->parentUnionType;
            unionType = (AnDataType*)bindGenericToType(c, unionType, bindings);
            variant->parentUnionType = unionType;
        }

        if(boundExts.empty()){
            variant->isGeneric = true;

        }else{
            bool extsGeneric = isGeneric(boundExts);
            variant->isGeneric = extsGeneric;
        }

        cout << "  Just bound " << variant->name << " and I must say, it is " <<
            (variant->isGeneric ? "" : "not ") << "generic\n";

        cout << "isGeneric(\n\t" << flush;
        for(auto &t : boundExts){
            if(&t == &boundExts.back())
                cout << anTypeToStr(t);
            else
                cout << anTypeToStr(t) << ", ";
        }

        cout << " = " << isGeneric(boundExts) << endl << ")\n";

        variant->typeTag = unboundType->typeTag;
        variant->fields = unboundType->fields;
        variant->unboundType = unboundType;
        variant->extTys = boundExts;
        variant->tags = unboundType->tags;
        variant->traitImpls = unboundType->traitImpls;
        updateLlvmTypeBinding(c, variant);
        return variant;
    }


    /*
     * Returns a bound variant of an unbound type whose bound
     * types match the given map of boundTys.  Returns nullptr
     * if such a type is not found.
     */
    AnDataType* findMatchingVariant(AnDataType *unboundType, const vector<pair<string, AnType*>> &boundTys){
        for(auto *v : unboundType->variants){
            if(v->boundGenerics == boundTys){
                cout << "Found ";
                print(boundTys);
                return v;
            }else{
                cout << "Well ";
                print(v->boundGenerics);
                cout << " is not ";
                print(boundTys);
            }
        }
        cout << "Did not find variant ";
        print(boundTys);
        return nullptr;
    }

    /*
     * Searches for the bound variant of the generic type
     * unboundType and creates it if it has not been
     * previously bound.
     */
    AnDataType* AnDataType::getVariant(Compiler *c, AnDataType *unboundType, const vector<pair<string, AnType*>> &boundTys, AnModifier *m){
        AnDataType *variant = findMatchingVariant(unboundType, boundTys);

        //variant is already bound
        if(variant)
            return variant;
        
        variant = new AnDataType(unboundType->name, {}, false, unboundType->mods);

        cout << "  Variant " << anTypeToStr(variant) << " is a stub, isGeneric = " << variant->isGeneric
            << ", binding.  (Parentty = " << anTypeToStr(unboundType) << ", isGeneric = " << unboundType->isGeneric << ")\n";

        return bindVariant(c, unboundType, boundTys, m, variant);
    }

    /*
     * Searches for the bound variant of the generic type
     * specified by name and creates it if it has not been
     * previously bound.  Will fail if the given name does
     * not correspond to any defined type.
     */
    AnDataType* AnDataType::getVariant(Compiler *c, const string &name, const vector<pair<string, AnType*>> &boundTys, AnModifier *m){
        auto *unboundType = AnDataType::get(name);
        if(unboundType->isStub()){
            cerr << "Warning: Cannot bind undeclared type " << name << endl;
            return unboundType;
        }

        AnDataType *variant = findMatchingVariant(unboundType, boundTys);

        //variant is already bound
        if(variant)
            return variant;
        
        variant = new AnDataType(unboundType->name, {}, false, unboundType->mods);
        return bindVariant(c, unboundType, boundTys, m, variant);
    }

    AnDataType* AnDataType::create(string name, vector<AnType*> elems, bool isUnion, const vector<AnTypeVarType*> &generics, AnModifier *m){
        string key = modifiersToStr(m) + getBoundName(name, generics);
        AnDataType *dt = 0;
        try{
            dt = typeArena.declaredTypes.at(key).get();
            if(!dt->isStub()){
                dt->extTys = elems;
                dt->isGeneric = !generics.empty();
                dt->generics = generics;
                return dt;
            }
            //cout << "AnDataType::create found stub for " << name << ", overwriting.\n";
        }catch(out_of_range r){}

        if(!dt){
            dt = new AnDataType(name, {}, isUnion, m);
            typeArena.declaredTypes.emplace(key, dt);
        }

        dt->isGeneric = !generics.empty();
        dt->generics = generics;

        vector<AnType*> elemsWithMods;
        elems.reserve(elems.size());
        for(auto *ty : elems){
            auto *mod_type = ty->setModifier(m);
            elemsWithMods.emplace_back(mod_type);
        }

        dt->extTys = elemsWithMods;
        return dt;
    }

    AnTypeContainer::AnTypeContainer(){
        typeArena.primitiveTypes[TT_I8].reset(new AnType(TT_I8, false, nullptr));
        typeArena.primitiveTypes[TT_I16].reset(new AnType(TT_I16, false, nullptr));
        typeArena.primitiveTypes[TT_I32].reset(new AnType(TT_I32, false, nullptr));
        typeArena.primitiveTypes[TT_I64].reset(new AnType(TT_I64, false, nullptr));
        typeArena.primitiveTypes[TT_Isz].reset(new AnType(TT_Isz, false, nullptr));
        typeArena.primitiveTypes[TT_U8].reset(new AnType(TT_U8, false, nullptr));
        typeArena.primitiveTypes[TT_U16].reset(new AnType(TT_U16, false, nullptr));
        typeArena.primitiveTypes[TT_U32].reset(new AnType(TT_U32, false, nullptr));
        typeArena.primitiveTypes[TT_U64].reset(new AnType(TT_U64, false, nullptr));
        typeArena.primitiveTypes[TT_Usz].reset(new AnType(TT_Usz, false, nullptr));
        typeArena.primitiveTypes[TT_F16].reset(new AnType(TT_F16, false, nullptr));
        typeArena.primitiveTypes[TT_F32].reset(new AnType(TT_F32, false, nullptr));
        typeArena.primitiveTypes[TT_F64].reset(new AnType(TT_F64, false, nullptr));
        typeArena.primitiveTypes[TT_Bool].reset(new AnType(TT_Bool, false, nullptr));
        typeArena.primitiveTypes[TT_Void].reset(new AnType(TT_Void, false, nullptr));
        typeArena.primitiveTypes[TT_C8].reset(new AnType(TT_C8, false, nullptr));
        typeArena.primitiveTypes[TT_C32].reset(new AnType(TT_C32, false, nullptr));
        typeArena.primitiveTypes[TT_Type].reset(new AnType(TT_Type, false, nullptr));
        typeArena.primitiveTypes[TT_FunctionList].reset(new AnType(TT_FunctionList, false, nullptr));
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
