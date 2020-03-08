#include "antype.h"
#include "trait.h"
#include "types.h"
#include "uniontag.h"
#include "unification.h"
#include "util.h"

using namespace std;
using namespace ante::parser;

namespace ante {
    vector<AnType> AnType::typeContainer;

    void AnType::dump() const{
        cout << anTypeToStr(this) << endl;
    }

    bool AnType::isRowVar() const {
        auto tvt = try_cast<const AnTypeVarType>(this);
        return tvt && tvt->isRowVariable;
    }

    bool isGeneric(vector<AnType*> const& vec){
        for(auto *t : vec)
            if(t->isGeneric)
                return true;
        return false;
    }

    bool isGeneric(AnType *retTy, std::vector<AnType*> const& params, std::vector<TraitImpl*> const& traits){
        if(retTy->isGeneric || isGeneric(params))
            return true;

        for(auto *t : traits)
            if(isGeneric(t->typeArgs))
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

    template<typename Key, typename Val>
    Val* search(unordered_map<Key, unique_ptr<Val>> &map, Key const& key){
        auto it = map.find(key);
        if(it != map.end())
            return it->second.get();
        return nullptr;
    }

    template<typename Key, typename Val>
    void addKVPair(unordered_map<Key, unique_ptr<Val>> &map, Key const& key, Val* val){
        if(map[key]){
            cerr << lazy_str("WARNING", AN_WARN_COLOR) << ": Hash collision between "
                << anTypeToColoredStr(map[key].get()) << " and " << anTypeToColoredStr(val) << endl;
        }
        map[key] = unique_ptr<Val>(val);
    }

    AnType* AnType::getPrimitive(TypeTag tag){
        switch(tag){
            case TT_I8:           return &typeContainer[tag];
            case TT_I16:          return &typeContainer[tag];
            case TT_I32:          return &typeContainer[tag];
            case TT_I64:          return &typeContainer[tag];
            case TT_Isz:          return &typeContainer[tag];
            case TT_U8:           return &typeContainer[tag];
            case TT_U16:          return &typeContainer[tag];
            case TT_U32:          return &typeContainer[tag];
            case TT_U64:          return &typeContainer[tag];
            case TT_Usz:          return &typeContainer[tag];
            case TT_F16:          return &typeContainer[tag];
            case TT_F32:          return &typeContainer[tag];
            case TT_F64:          return &typeContainer[tag];
            case TT_C8:           return &typeContainer[tag];
            case TT_Bool:         return &typeContainer[tag];
            case TT_Unit:         return &typeContainer[tag];
            default:
                cerr << "error: AnType::getPrimitive: TypeTag " << typeTagToStr(tag) << " is not primitive!\n";
                throw CtError();
        }
    }


    AnType* AnType::getI8(){
        return &typeContainer[TT_I8];
    }

    AnType* AnType::getI16(){
        return &typeContainer[TT_I16];
    }

    AnType* AnType::getI32(){
        return &typeContainer[TT_I32];
    }

    AnType* AnType::getI64(){
        return &typeContainer[TT_I64];
    }

    AnType* AnType::getIsz(){
        return &typeContainer[TT_Isz];
    }

    AnType* AnType::getU8(){
        return &typeContainer[TT_U8];
    }

    AnType* AnType::getU16(){
        return &typeContainer[TT_U16];
    }

    AnType* AnType::getU32(){
        return &typeContainer[TT_U32];
    }

    AnType* AnType::getU64(){
        return &typeContainer[TT_U64];
    }

    AnType* AnType::getUsz(){
        return &typeContainer[TT_Usz];
    }

    AnType* AnType::getF16(){
        return &typeContainer[TT_F16];
    }

    AnType* AnType::getF32(){
        return &typeContainer[TT_F32];
    }

    AnType* AnType::getF64(){
        return &typeContainer[TT_F64];
    }

    AnType* AnType::getBool(){
        return &typeContainer[TT_Bool];
    }

    AnType* AnType::getUnit(){
        return &typeContainer[TT_Unit];
    }

    BasicModifier* BasicModifier::get(const AnType *modifiedType, TokenType mod){
        return new BasicModifier(modifiedType, mod);
    }

    CompilerDirectiveModifier* CompilerDirectiveModifier::get(const AnType *modifiedType, Node *directive){
        return new CompilerDirectiveModifier(modifiedType, directive);
    }

    AnPtrType* AnPtrType::get(AnType* ext){
        return new AnPtrType(ext);
    }

    AnArrayType* AnArrayType::get(AnType* t, size_t len){
        return new AnArrayType(t, len);
    }

    AnTupleType* AnTupleType::get(vector<AnType*> const& fields){
        return new AnTupleType(fields, {});
    }

    AnTupleType* AnTupleType::getAnonRecord(vector<AnType*> const& fields,
            vector<string> const& fieldNames){

        return new AnTupleType(fields, fieldNames);
    }

    AnFunctionType* AnFunctionType::get(AnType* retty,
            NamedValNode* params, Module *module){

        vector<AnType*> paramTys;

        while(params && params->typeExpr.get()){
            TypeNode *pty = (TypeNode*)params->typeExpr.get();
            auto *aty = toAnType(pty, module);
            paramTys.push_back(aty);
            params = (NamedValNode*)params->next.get();
        }
        return AnFunctionType::get(retty, paramTys, {});
    }

    AnFunctionType* AnFunctionType::get(AnType *retTy, vector<AnType*> const& elems,
            vector<TraitImpl*> const& tcConstrains){

        auto const& params = elems.empty() ? vector<AnType*>{AnType::getUnit()} : elems;
        return new AnFunctionType(retTy, params, tcConstrains);
    }


    AnTypeVarType* AnTypeVarType::get(string const& name){
        return new AnTypeVarType(name);
    }

    AnDataType* AnDataType::get(std::string const& name, TypeArgs const& args, TypeDecl *decl){
        return new AnDataType(name, args, decl);
    }


    template<typename T>
    void addVariant(T* type, T* variant){
        type->genericVariants.push_back(variant);
        variant->unboundType = type;
    }


    template<typename T>
    typename vector<T*>::iterator findVariant(T* type, TypeArgs args){
        auto end = type->genericVariants.end();
        for(auto it = type->genericVariants.begin(); it != end; ++it){
            if((*it)->typeArgs == args){
                return it;
            }
        }
        return end;
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
    void AnType::initTypeSystem(){
        assert(typeContainer.empty());
        typeContainer = vector<AnType>(numPrimitiveTypeTags, AnType(TT_I8, false));
        typeContainer[TT_I8] = AnType(TT_I8, false);
        typeContainer[TT_I16] = AnType(TT_I16, false);
        typeContainer[TT_I32] = AnType(TT_I32, false);
        typeContainer[TT_I64] = AnType(TT_I64, false);
        typeContainer[TT_Isz] = AnType(TT_Isz, false);
        typeContainer[TT_U8] = AnType(TT_U8, false);
        typeContainer[TT_U16] = AnType(TT_U16, false);
        typeContainer[TT_U32] = AnType(TT_U32, false);
        typeContainer[TT_U64] = AnType(TT_U64, false);
        typeContainer[TT_Usz] = AnType(TT_Usz, false);
        typeContainer[TT_F16] = AnType(TT_F16, false);
        typeContainer[TT_F32] = AnType(TT_F32, false);
        typeContainer[TT_F64] = AnType(TT_F64, false);
        typeContainer[TT_Bool] = AnType(TT_Bool, false);
        typeContainer[TT_C8] = AnType(TT_C8, false);
        typeContainer[TT_Unit] = AnType(TT_Unit, false);
    }


    AnType* AnType::getFunctionReturnType() const{
        return try_cast<AnFunctionType>(this)->retTy;
    }


    AnType* toAnType(const TypeNode *tn, Module *module){
        if(!tn) return AnType::getUnit();
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
            case TT_Bool:
            case TT_Unit:
                ret = AnType::getPrimitive(tn->typeTag);
                break;

            case TT_Function: {
                TypeNode *ext = tn->extTy.get();
                AnType *retty = 0;
                vector<AnType*> tys;
                while(ext){
                    if(retty){
                        tys.push_back(toAnType(ext, module));
                    }else{
                        retty = toAnType(ext, module);
                    }
                    ext = (TypeNode*)ext->next.get();
                }
                ret = AnFunctionType::get(retty, tys, {});
                break;
            }
            case TT_Tuple: {
                TypeNode *ext = tn->extTy.get();
                vector<AnType*> tys;
                while(ext){
                    tys.push_back(toAnType(ext, module));
                    ext = (TypeNode*)ext->next.get();
                }
                ret = AnTupleType::get(tys);
                break;
            }

            case TT_Array: {
                TypeNode *elemTy = tn->extTy.get();
                IntLitNode *len = (IntLitNode*)elemTy->next.get();
                ret = AnArrayType::get(toAnType(elemTy, module), len ? stoi(len->val) : 0);
                break;
            }
            case TT_Ptr:
                ret = AnPtrType::get(toAnType(tn->extTy.get(), module));
                break;
            case TT_Data: {
                TypeDecl *decl = module->lookupTypeDecl(tn->typeName);
                if(!decl){
                    error("Use of undeclared type " + lazy_str(tn->typeName, AN_TYPE_COLOR), tn->loc);
                }

                if(decl->isAlias){
                    return decl->aliasedType;
                }

                auto basety = cast<AnDataType>(decl->type);

                // TODO: This will fail if type is a type alias to another data type with differing type args
                ret = basety;
                size_t tnpSize = tn->params.size();
                size_t btaSize = basety->typeArgs.size();
                if(tnpSize > btaSize){
                    error(anTypeToColoredStr(basety) + " takes " + to_string(btaSize)
                        + " argument" + plural(btaSize) + ", but " + to_string(tnpSize) + ' '
                        + pluralIsAre(tnpSize) + " given here", tn->loc);
                }

                size_t i = 0;
                if(!tn->params.empty()){
                    Substitutions subs;
                    for(i = 0; i < tn->params.size() && i < basety->typeArgs.size(); i++){
                        auto *b = static_cast<AnTypeVarType*>(toAnType(tn->params[i].get(), module));
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
            case TT_TypeVar: {
                auto tv = AnTypeVarType::get(tn->typeName);
                tv->isRowVariable = tn->isRowVar;
                ret = tv;
                break;
            }
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

    const AnType* AnTupleType::addModifier(TokenType m) const{
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

    bool AnType::operator!=(AnType const& other) const noexcept {
        return !(*this == other);
    }

    bool allEq(vector<string> const& l, vector<string> const& r){
        if(l.size() != r.size())
            return false;

        for(size_t i = 0; i < l.size(); ++i){
            if(l[i] != r[i]){
                return false;
            }
        }
        return true;
    }

    bool allEq(vector<AnType*> const& l, vector<AnType*> const& r){
        if(l.size() != r.size())
            return false;

        for(size_t i = 0; i < l.size(); ++i){
            if(*l[i] != *r[i]){
                return false;
            }
        }
        return true;
    }

    bool AnType::operator==(AnType const& other) const noexcept {
        if(this == &other) return true;
        if(typeTag != other.typeTag) return false;

        if(this->isModifierType()){
            if(other.isModifierType()){
                auto lmod = dynamic_cast<const BasicModifier*>(this);
                auto rmod = dynamic_cast<const BasicModifier*>(&other);
                return lmod && rmod
                    && lmod->mod == rmod->mod
                    && *lmod->extTy == *rmod->extTy;
            }else{
                return false;
            }
        }

        if(typeTag == TT_Tuple){
            auto l = static_cast<const AnTupleType*>(this);
            auto r = static_cast<const AnTupleType*>(&other);
            return allEq(l->fields, r->fields) && allEq(l->fieldNames, r->fieldNames);

        }else if(typeTag == TT_Data){
            auto l = static_cast<const AnDataType*>(this);
            auto r = static_cast<const AnDataType*>(&other);
            return l->name == r->name && allEq(l->typeArgs, r->typeArgs);
        }else if(typeTag == TT_Array){
            auto l = static_cast<const AnArrayType*>(this);
            auto r = static_cast<const AnArrayType*>(&other);
            return *l->extTy == *r->extTy;
        }else if(typeTag == TT_Ptr){
            auto l = static_cast<const AnPtrType*>(this);
            auto r = static_cast<const AnPtrType*>(&other);
            return *l->elemTy == *r->elemTy;
        }else if(typeTag == TT_Function){
            auto l = static_cast<const AnFunctionType*>(this);
            auto r = static_cast<const AnFunctionType*>(&other);
            return *l->retTy == *r->retTy && allEq(l->paramTys, r->paramTys);
        }
        /* Should only be reached with primitive types with modifiers, eg mut u64 == u64 */
        return true;
    }

    bool allApproxEq(vector<AnType*> const& l, vector<AnType*> const& r){
        for(size_t i = 0; i < l.size(); ++i){
            if(!l[i]->approxEq(r[i])){
                return false;
            }
        }
        return l.size() == r.size();
    }

    bool AnType::approxEq(const AnType *other) const noexcept {
        if(this == other || typeTag == TT_TypeVar)
            return true;

        if(typeTag != other->typeTag) return false;

        if(this->isModifierType()){
            auto lmod = dynamic_cast<const BasicModifier*>(this);
            return lmod->extTy->approxEq(other);
        }
        if(other->isModifierType()){
            auto rmod = dynamic_cast<const BasicModifier*>(other);
            return approxEq(rmod->extTy);
        }

        if(typeTag == TT_Tuple){
            auto l = static_cast<const AnTupleType*>(this);
            auto r = static_cast<const AnTupleType*>(other);
            return allApproxEq(l->fields, r->fields) && allEq(l->fieldNames, r->fieldNames);

        }else if(typeTag == TT_Data){
            auto l = static_cast<const AnDataType*>(this);
            auto r = static_cast<const AnDataType*>(other);
            return l->name == r->name && allApproxEq(l->typeArgs, r->typeArgs);
        }else if(typeTag == TT_Array){
            auto l = static_cast<const AnArrayType*>(this);
            auto r = static_cast<const AnArrayType*>(other);
            return l->extTy->approxEq(r->extTy);
        }else if(typeTag == TT_Ptr){
            auto l = static_cast<const AnPtrType*>(this);
            auto r = static_cast<const AnPtrType*>(other);
            return l->elemTy->approxEq(r->elemTy);
        }else if(typeTag == TT_Function){
            auto l = static_cast<const AnFunctionType*>(this);
            auto r = static_cast<const AnFunctionType*>(other);
            return l->retTy->approxEq(r->retTy) && allApproxEq(l->paramTys, r->paramTys);
        }
        ASSERT_UNREACHABLE("Unknown TypeTag in AnType::approxEq");
    }

    bool AnType::isPrimitiveTy() const noexcept {
        return isPrimitiveTypeTag(this->typeTag);
    }

    bool AnType::isSignedTy() const noexcept {
        return isSignedTypeTag(this->typeTag);
    }

    bool AnType::isUnsignedTy() const noexcept {
        return isUnsignedTypeTag(this->typeTag);
    }

    bool AnType::isFloatTy() const noexcept {
        return isFloatTypeTag(this->typeTag);
    }

    bool AnType::isIntegerTy() const noexcept {
        return isIntegerTypeTag(this->typeTag);
    }

    bool AnType::isNumericTy() const noexcept {
        return isNumericTypeTag(this->typeTag);
    }

    llvm::Type* AnDataType::toLlvmType(Compiler *c) const {
        return decl->toLlvmType(c, this);
    }

    std::vector<AnType*> AnDataType::getBoundFieldTypes() const {
        return decl->getBoundFieldTypes(this);
    }

    Result<size_t, std::string> AnDataType::getSizeInBits(Compiler *c,
            std::string const& incompleteType) const {

        return decl->getSizeInBits(c, incompleteType, this);
    }

    llvm::Value* AnDataType::getTagValue(Compiler *c, std::string const& variantName,
            std::vector<TypedValue> const& args) const {

        return decl->getTagValue(c, this, variantName, args);
    }
}
