#include "antype.h"
#include "types.h"
#include "uniontag.h"
#include "unification.h"

using namespace std;
using namespace ante::parser;

namespace ante {

    AnTypeContainer typeArena;

    void AnType::dump() const{
        if(auto *dt = try_cast<AnDataType>(this)){
            cout << dt->name;
            for(auto &arg : dt->typeArgs){
                cout << ' ' << anTypeToStr(arg);
            }
            cout << " = ";
            if(auto *pt = try_cast<AnProductType>(this)){
                if(pt->fields.empty()){
                    cout << "()";
                }else{
                    for(auto &ext : pt->fields){
                        cout << anTypeToStr(ext);
                        if(&ext != &pt->fields.back())
                            cout << ", ";
                    }
                }
            }else if(auto *st = try_cast<AnSumType>(this)){
                for(auto &ext : st->tags){
                    cout << anTypeToStr(ext);
                    if(&ext != &st->tags.back())
                        cout << " | ";
                }
            }else{
                cout << "(unknown)";
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
            cerr << lazy_str("WARNING", AN_WARN_COLOR) << ": Hash collision between "
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

    AnProductType* AnProductType::create(string const& name, std::vector<AnType*> const& elems,
            TypeArgs const& typeArgs){

        auto existing_ty = AnProductType::get(name);
        if(existing_ty) return existing_ty;

        AnDataType* decl = new AnProductType(name, elems);
        decl->typeArgs = typeArgs;
        decl->isGeneric = !typeArgs.empty();

        addKVPair(typeArena.dataTypes, name, decl);
        return static_cast<AnProductType*>(decl);
    }

    AnProductType* AnProductType::get(string const& name){
        auto t = search(typeArena.dataTypes, name);
        if(!t) return nullptr;
        return t->typeTag == TT_Data ? static_cast<AnProductType*>(t) : nullptr;
    }

    AnSumType* AnSumType::create(string const& name, std::vector<AnProductType*> const& unionMembers,
            TypeArgs const& typeArgs){

        auto existing_ty = AnSumType::get(name);
        if(existing_ty) return existing_ty;

        AnDataType* decl = new AnSumType(name, unionMembers);
        decl->typeArgs = typeArgs;
        decl->isGeneric = !typeArgs.empty();

        addKVPair(typeArena.dataTypes, name, decl);
        return static_cast<AnSumType*>(decl);
    }

    AnSumType* AnSumType::get(string const& name){
        auto t = search(typeArena.dataTypes, name);
        if(!t) return nullptr;
        return t->typeTag == TT_TaggedUnion ? static_cast<AnSumType*>(t) : nullptr;
    }

    AnDataType* AnDataType::get(string const& name){
        return search(typeArena.dataTypes, name);
    }

    /**
     * Search for a data type generic variant by name.
     * Returns it if found, or creates it otherwise.
     */
    AnProductType* AnProductType::getOrCreateVariant(AnProductType *parent,
            std::vector<AnType*> const& elems, TypeArgs const& typeArgs){

        pair<string, vector<AnType*>> key{parent->name, elems};
        auto t = search(typeArena.productTypeVariants, key);
        if(t) return t;

        auto ret = new AnProductType(parent->name, elems);
        ret->typeArgs = typeArgs;
        ret->isGeneric = ante::isGeneric(typeArgs);
        ret->fields = parent->fields;
        ret->parentUnionType = parent->parentUnionType; //Will never bind the parent union type!
        typeArena.productTypeVariants.try_emplace(key, ret);
        return ret;
    }

    /**
     * Search for a data type generic variant by name.
     * Returns it if found, or creates it otherwise.
     */
    AnSumType* AnSumType::getOrCreateVariant(AnSumType *parent,
            std::vector<AnProductType*> const& elems, TypeArgs const& typeArgs){

        pair<string, vector<AnProductType*>> key{parent->name, elems};
        auto t = search(typeArena.sumTypeVariants, key);
        if(t) return t;

        auto ret = new AnSumType(parent->name, elems);
        ret->typeArgs = typeArgs;
        typeArena.sumTypeVariants.try_emplace(key, ret);
        return ret;
    }


    /** Search for a data type by name.
        * Returns null if no type with a matching name is found. */
    AnTraitType* AnTraitType::get(string const& name){
        auto t = search(typeArena.dataTypes, name);
        return try_cast<AnTraitType>(t);
    }


    AnTraitType* AnTraitType::getOrCreateVariant(AnTraitType *parent, TypeArgs const& generics){
        //TODO: create traitvariants field to store these permanently in
        auto ret = new AnTraitType({parent}, parent->traits, generics);
        ret->composedTraitTypes[0] = ret;
        return ret;
    }


    AnTraitType* AnTraitType::create(Trait *trait, TypeArgs const& tArgs){
        auto ret = new AnTraitType(trait, tArgs);
        typeArena.dataTypes.try_emplace(trait->name, ret);
        return ret;
    }


    template<typename T>
    vector<T> union_of(vector<T> const& l, vector<T> const& r){
        vector<T> unionVec{l.size() + r.size()};

        /* TODO: find a simpler set union method.
         * we are more concerned with constant factors
         * than asymptotic behaviour */
        auto it = std::set_union(l.begin(), l.end(),
                r.begin(), r.end(), unionVec.begin());
        unionVec.resize(it - unionVec.begin());
        return unionVec;
    }


    /** Creates a new TraitType that is a union of the 2 given. */
    AnTraitType* AnTraitType::merge(AnTraitType *t){
        auto unionVec = union_of(traits, t->traits);

        auto existing_ty = search(typeArena.multiTraitTypes, unionVec);
        if(existing_ty) return existing_ty;

        //merge all typeArgs, save for the shared 'self' type arg
        TypeArgs typeArgs = this->typeArgs;

        // If either typeArg is bound and different from the other they can't be merged.
        // FIXME: This will fail when checking eg.  Eq (Show 'a) = Eq (Other 'b)
        //        Resulting in a merge of Eq (Other 'b) instead of Eq (Show+Other 'b)
        if(typeArgs.back() != t->typeArgs.back() &&
                (!typeArgs.back()->isGeneric || !t->typeArgs.back()->isGeneric)){

            cerr << "ERROR: Union of two incompatible trait types: " << anTypeToColoredStr(this)
                 << " and " << anTypeToColoredStr(t) << endl
                 << "NOTE: Cause is generic parameter " << anTypeToColoredStr(typeArgs.back())
                 << " != " << anTypeToColoredStr(t->typeArgs.back()) << endl;
        }

        typeArgs.pop_back();
        typeArgs.insert(typeArgs.end(), t->typeArgs.begin(), t->typeArgs.end());

        auto newTraitTypes = union_of(composedTraitTypes, t->composedTraitTypes);

        auto ret = new AnTraitType(newTraitTypes, unionVec, typeArgs);
        typeArena.multiTraitTypes.try_emplace(unionVec, ret);
        return ret;
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


    /**
     * Return the names of all traits concatenated with '+'
     */
    string AnTraitType::combineNames(std::vector<AnTraitType*> const& traits){
        string name = "";
        for(const auto &trait : traits){
            name += trait->name;
            if(&trait != &traits.back())
                name += "+";
        }
        return name;
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
            case TT_Trait:
            case TT_TaggedUnion: {
                AnDataType *basety = AnDataType::get(tn->typeName);
                if(!basety){
                    ante::error("Use of undeclared type " + lazy_str(tn->typeName, AN_TYPE_COLOR), tn->loc);
                    return nullptr;
                }

                ret = basety;

                size_t i = 0;
                if(!tn->params.empty()){
                    Substitutions subs;
                    for(; i < tn->params.size() && i < basety->typeArgs.size(); i++){
                        auto *b = static_cast<AnTypeVarType*>(toAnType(tn->params[i].get()));
                        auto *basetyTypeArg = try_cast<AnTypeVarType>(basety->typeArgs[i]);
                        subs.emplace_back(basetyTypeArg, b);
                    }
                    ret = applySubstitutions(subs, ret);
                }

                // Fill in unspecified typevars;  eg change List to List 't
                for(; i < basety->typeArgs.size(); i++){
                    Substitutions subs;
                    auto *b = nextTypeVar();
                    auto *basetyTypeArg = try_cast<AnTypeVarType>(basety->typeArgs[i]);
                    subs.emplace_back(basetyTypeArg, b);
                    ret = applySubstitutions(subs, ret);
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

    const AnType* AnProductType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }

    const AnType* AnSumType::addModifier(TokenType m) const{
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }

    const AnType* AnTraitType::addModifier(TokenType m) const {
        if(m == Tok_Let) return this;
        return BasicModifier::get(this, m);
    }


    /**
    * Returns the UnionTag of a tag within the union type.
    *
    * If the given tag is not found, this function issues an
    * error message and throws a CtError exception.
    *
    * @return the value of the tag found, or 0 on failure
    */
    size_t AnSumType::getTagVal(std::string const& name){
        for(size_t i = 0; i < tags.size(); i++){
            if(tags[i]->name == name)
                return i;
        }
        return 0;
    }
}
