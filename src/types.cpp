#include <types.h>
#include <trait.h>
#include <nameresolution.h>
#include <util.h>
using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

char getBitWidthOfTypeTag(const TypeTag ty){
    switch(ty){
        case TT_I8:  case TT_U8: case TT_C8:  return 8;
        case TT_I16: case TT_U16: case TT_F16: return 16;
        case TT_I32: case TT_U32: case TT_F32: return 32;
        case TT_I64: case TT_U64: case TT_F64: return 64;
        case TT_Isz: case TT_Usz: return AN_USZ_SIZE;
        case TT_Bool: return 8;

        case TT_Ptr:
        case TT_Function:
        case TT_MetaFunction:
        case TT_FunctionList: return AN_USZ_SIZE;

        default: return 0;
    }
}


// TODO: Remove hardcoded check for Type type
bool isCompileTimeOnlyParamType(AnType *ty){
    return ty->typeTag == TT_Type || ty->typeTag == TT_Unit || ty->hasModifier(Tok_Ante)
        || (ty->typeTag == TT_Data && try_cast<AnDataType>(ty)->name == "Type");
}


/*
 *  Returns the TypeNode* value of a TypedValue of type TT_Type
 */
AnType* extractTypeValue(const TypedValue &tv){
    auto zext = dyn_cast<ConstantInt>(tv.val)->getZExtValue();
    return (AnType*) zext;
}

AnType* findBinding(Substitutions const& subs, const AnType *key){
    for(auto it = subs.rbegin(); it != subs.rend(); ++it){
        if(it->first == key){
            return it->second;
        }
    }
    return nullptr;
}

Result<size_t, string> AnType::getSizeInBits(Compiler *c, string *incompleteType, bool force) const{
    size_t total = 0;

    if(isPrimitiveTypeTag(this->typeTag))
        return getBitWidthOfTypeTag(this->typeTag);

    if(auto *dataTy = try_cast<AnProductType>(this)){
        if(incompleteType && dataTy->name == *incompleteType){
            cerr << "Incomplete type " << anTypeToColoredStr(this) << endl;
            throw new IncompleteTypeError();
        }

        for(auto *ext : dataTy->fields){
            auto val = ext->getSizeInBits(c, incompleteType, force);
            if(!val) return val;
            total += val.getVal();
        }

    }else if(auto *sumTy = try_cast<AnSumType>(this)){
        if(incompleteType && sumTy->name == *incompleteType){
            cerr << "Incomplete type " << anTypeToColoredStr(this) << endl;
            throw new IncompleteTypeError();
        }

        for(auto *ext : sumTy->tags){
            auto val = ext->getSizeInBits(c, incompleteType, force);
            if(!val) return val;
            if(val.getVal() > total)
                total= val.getVal();
        }

    // function & metafunction are aggregate types but have different sizes than
    // a tuple so this case must be checked for before AnAggregateType is
    }else if(typeTag == TT_Ptr or typeTag == TT_Function or typeTag == TT_MetaFunction){
        return AN_USZ_SIZE;

    }else if(auto *tup = try_cast<AnAggregateType>(this)){
        for(auto *ext : tup->extTys){
            auto val = ext->getSizeInBits(c, incompleteType, force);
            if(!val) return val;
            total += val.getVal();
        }

    }else if(auto *arr = try_cast<AnArrayType>(this)){
        auto val = arr->extTy->getSizeInBits(c, incompleteType, force);
        if(!val) return val;
        return arr->len * val.getVal();

    }else if(auto *tvt = try_cast<AnTypeVarType>(this)){
        AnType *lookup = findBinding(c->compCtxt->monomorphisationMappings, tvt);
        return lookup->getSizeInBits(c);
    }

    return total;
}


size_t hashCombine(size_t l, size_t r){
    return l ^ (r + AN_HASH_PRIME + (l << 6) + (l >> 2));
}


string toLlvmTypeName(const AnDataType *dt){
    auto &typeArgs = dt->typeArgs;
    auto &baseName = dt->name;

    if(typeArgs.empty())
        return baseName;

    string name = baseName + "<";
    for(auto &b : typeArgs){
        if(AnDataType *ext = try_cast<AnDataType>(b))
            name += toLlvmTypeName(ext);
        else
            name += anTypeToStr(b);

        if(&b != &typeArgs.back())
            name += ",";
    }
    return name + ">";
}

AnDataType* getUnboundType(AnDataType *dt){
    return dt->unboundType ? dt->unboundType : dt;
}

void AnDataType::setLlvmType(llvm::Type *type, ante::Substitutions const& monomorphisationBindings){
    auto substitutedTA = ante::applyToAll(typeArgs, [&](AnType *typeArg){
        return ante::applySubstitutions(monomorphisationBindings, typeArg);
    });
    auto unbound = getUnboundType(this);
    for(auto &pair : unbound->llvmTypes){
        if(allEq(pair.first, substitutedTA)){
            this->llvmType = type;
            pair.second = type;
            return;
        }
    }

    unbound->llvmTypes.emplace_back(substitutedTA, type);
}

Type* AnDataType::findLlvmType(ante::Substitutions const& monomorphisationBindings){
    auto substitutedTA = ante::applyToAll(typeArgs, [&](AnType *typeArg){
        return ante::applySubstitutions(monomorphisationBindings, typeArg);
    });
    for(auto &pair : getUnboundType(this)->llvmTypes){
        if(allEq(pair.first, substitutedTA))
            return pair.second;
    }
    return nullptr;
}

Type* updateLlvmTypeBinding(Compiler *c, AnDataType *dt, bool force){
    //create an empty type first so we dont end up with infinite recursion
    bool isPacked = dt->typeTag == TT_TaggedUnion;
    StructType* structTy = static_cast<StructType*>(dt->llvmType);
    
    if(!structTy){
        auto existing = dt->findLlvmType(c->compCtxt->monomorphisationMappings);
        if(existing){
            return existing;
        }
        structTy = StructType::create(*c->ctxt, toLlvmTypeName(dt));
    }

    dt->setLlvmType(structTy, c->compCtxt->monomorphisationMappings);

    AnType *ext = dt;
    if(auto *st = try_cast<AnSumType>(dt))
        ext = getLargestExt(c, st, force);

    vector<Type*> tys;
    if(auto *aggty = try_cast<AnProductType>(ext)){
        for(auto *e : aggty->fields){
            auto *llvmTy = c->anTypeToLlvmType(e, force);
            if(!llvmTy->isVoidTy())
                tys.push_back(llvmTy);
        }
    }else{
        tys.push_back(c->anTypeToLlvmType(ext, force));
    }

    structTy->setBody(tys, isPacked);
    return structTy;
}

bool isIntTypeTag(const TypeTag ty){
    return ty==TT_I8 or ty==TT_I16 or ty==TT_I32 or ty==TT_I64 or
           ty==TT_U8 or ty==TT_U16 or ty==TT_U32 or ty==TT_U64 or
           ty==TT_Isz or ty==TT_Usz or ty==TT_C8;
}

bool isFPTypeTag(const TypeTag tt){
    return tt==TT_F16 or tt==TT_F32 or tt==TT_F64;
}

bool isNumericTypeTag(const TypeTag ty){
    return isIntTypeTag(ty) or isFPTypeTag(ty);
}

bool containsTypeVar(const TypeNode *tn){
    auto tt = tn->typeTag;
    if(tt == TT_Array or tt == TT_Ptr){
        return tn->extTy->typeTag == tt;
    }else if(tt == TT_Tuple or tt == TT_Data or tt == TT_TaggedUnion or
             tt == TT_Function or tt == TT_MetaFunction){
        TypeNode *ext = tn->extTy.get();
        while(ext){
            if(containsTypeVar(ext))
                return true;
        }
    }
    return tt == TT_TypeVar;
}


/*
 *  Translates an individual TypeTag to an llvm::Type.
 *  Only intended for primitive types, as there is not enough
 *  information stored in a TypeTag to convert to array, tuple,
 *  or function types.
 */
Type* typeTagToLlvmType(TypeTag ty, LLVMContext &ctxt){
    switch(ty){
        case TT_I8:  case TT_U8:  return Type::getInt8Ty(ctxt);
        case TT_I16: case TT_U16: return Type::getInt16Ty(ctxt);
        case TT_I32: case TT_U32: return Type::getInt32Ty(ctxt);
        case TT_I64: case TT_U64: return Type::getInt64Ty(ctxt);
        case TT_Isz:    return Type::getIntNTy(ctxt, AN_USZ_SIZE); //TODO: implement
        case TT_Usz:    return Type::getIntNTy(ctxt, AN_USZ_SIZE); //TODO: implement
        case TT_F16:    return Type::getHalfTy(ctxt);
        case TT_F32:    return Type::getFloatTy(ctxt);
        case TT_F64:    return Type::getDoubleTy(ctxt);
        case TT_C8:     return Type::getInt8Ty(ctxt);
        case TT_Bool:   return Type::getInt1Ty(ctxt);
        case TT_Unit:   return Type::getVoidTy(ctxt);
        case TT_TypeVar:
            throw new TypeVarError();
        default:
            cerr << "typeTagToLlvmType: Unknown/Unsupported TypeTag " << ty << ", exiting.\n";
            exit(1);
    }
}

AnType* getLargestExt(Compiler *c, AnSumType *unionType, bool force){
    AnType *largest = 0;
    size_t largest_size = 0;

    for(auto *e : unionType->tags){
        auto size = e->getSizeInBits(c, nullptr, force);
        if(!size){
            cerr << size.getErr() << endl;
            size = 0;
        }

        if(size.getVal() > largest_size){
            largest = e;
            largest_size = size.getVal();
        }
    }
    return largest;
}



/*
 *  Translates a llvm::Type to a TypeTag. Not intended for in-depth analysis
 *  as it loses data about the type and name of UserTypes, and cannot distinguish
 *  between signed and unsigned integer types.  As such, this should mainly be
 *  used for comparing primitive datatypes, or just to detect if something is a
 *  primitive.
 */
TypeTag llvmTypeToTypeTag(Type *t){
    if(t->isIntegerTy(1)) return TT_Bool;

    if(t->isIntegerTy(8)) return TT_I8;
    if(t->isIntegerTy(16)) return TT_I16;
    if(t->isIntegerTy(32)) return TT_I32;
    if(t->isIntegerTy(64)) return TT_I64;
    if(t->isHalfTy()) return TT_F16;
    if(t->isFloatTy()) return TT_F32;
    if(t->isDoubleTy()) return TT_F64;

    if(t->isArrayTy()) return TT_Array;
    if(t->isStructTy() && !t->isEmptyTy()) return TT_Tuple; /* Could also be a TT_Data! */
    if(t->isPointerTy()) return TT_Ptr;
    if(t->isFunctionTy()) return TT_Function;

    return TT_Unit;
}


/*
 *  Converts a TypeNode to an llvm::Type.  While much less information is lost than
 *  llvmTypeToTokType, information on signedness of integers is still lost, causing the
 *  unfortunate necessity for the use of a TypedValue for the storage of this information.
 */
Type* Compiler::anTypeToLlvmType(const AnType *ty, bool force){
    vector<Type*> tys;

    if(ty->hasModifier(Tok_Mut)){
        auto bm = dynamic_cast<const BasicModifier*>(ty);
        return anTypeToLlvmType(bm->extTy, force)->getPointerTo();
    }

    switch(ty->typeTag){
        case TT_Ptr: {
            auto *ptr = try_cast<AnPtrType>(ty);
            return ptr->extTy->typeTag != TT_Unit ?
                anTypeToLlvmType(ptr->extTy, force)->getPointerTo()
                : Type::getInt8Ty(*ctxt)->getPointerTo();
        }
        case TT_Type:
            return Type::getInt8Ty(*ctxt)->getPointerTo();
        case TT_Array:{
            auto *arr = try_cast<AnArrayType>(ty);
            return ArrayType::get(anTypeToLlvmType(arr->extTy, force), arr->len);
        }
        case TT_Tuple:
            for(auto *e : try_cast<AnAggregateType>(ty)->extTys){
                auto *ty = anTypeToLlvmType(e, force);
                if(!ty->isVoidTy())
                    tys.push_back(ty);
            }
            return StructType::get(*ctxt, tys);
        case TT_Data: case TT_TaggedUnion: case TT_Trait: {
            auto *dt = (AnDataType*)try_cast<AnDataType>(ty);
            auto existing = dt->llvmType;
            if(existing)
                return existing;
            else
                return updateLlvmTypeBinding(this, dt, force);
        }
        case TT_Function: case TT_MetaFunction: {
            auto *f = try_cast<AnFunctionType>(ty);
            for(size_t i = 0; i < f->extTys.size(); i++){
                if(auto *tvt = try_cast<AnTypeVarType>(f->extTys[i])){
                    if(tvt->isVarArgs()){
                        return FunctionType::get(anTypeToLlvmType(f->retTy, force), tys, true)->getPointerTo();
                    }
                }
                // All Ante functions take at least 1 arg: (), which are ignored in llvm ir
                // and translated to 0 arg functions instead
                if(f->extTys[i]->typeTag != TT_Unit)
                    tys.push_back(anTypeToLlvmType(f->extTys[i], force));
            }

            return FunctionType::get(anTypeToLlvmType(f->retTy, force), tys, false)->getPointerTo();
        }
        case TT_TypeVar: {
            auto binding = findBinding(compCtxt->monomorphisationMappings, ty); 
            if(binding){
                return anTypeToLlvmType(binding, force);
            }
            std::cerr << "Typevar: " << (AnType*)ty << '\n' << "Bindings: " << compCtxt->monomorphisationMappings << '\n';
            ASSERT_UNREACHABLE("Unbound typevar found during monomorphisation");
        }
        default:
            return typeTagToLlvmType(ty->typeTag, *ctxt);
    }
}


/*
 *  Returns true if two given types are approximately equal.  This will return
 *  true if they are the same primitive datatype, or are both pointers pointing
 *  to the same elementtype, or are both arrays of the same element type, even
 *  if the arrays differ in size.  If two types are needed to be exactly equal,
 *  pointer comparison can be used instead since llvm::Types are uniqued.
 */
bool llvmTypeEq(Type *l, Type *r){
    TypeTag ltt = llvmTypeToTypeTag(l);
    TypeTag rtt = llvmTypeToTypeTag(r);

    if(ltt != rtt) return false;

    if(ltt == TT_Ptr){
        Type *lty = l->getPointerElementType();
        Type *rty = r->getPointerElementType();

        if(lty->isVoidTy() or rty->isVoidTy()) return true;

        return llvmTypeEq(lty, rty);
    }else if(ltt == TT_Array){
        return l->getArrayElementType() == r->getArrayElementType() and
               l->getArrayNumElements() == r->getArrayNumElements();
    }else if(ltt == TT_Function or ltt == TT_MetaFunction){
        int lParamCount = l->getFunctionNumParams();
        int rParamCount = r->getFunctionNumParams();

        if(lParamCount != rParamCount)
            return false;

        for(int i = 0; i < lParamCount; i++){
            if(!llvmTypeEq(l->getFunctionParamType(i), r->getFunctionParamType(i)))
                return false;
        }
        return true;
    }else if(ltt == TT_Tuple or ltt == TT_Data){
        int lElemCount = l->getStructNumElements();
        int rElemCount = r->getStructNumElements();

        if(lElemCount != rElemCount)
            return false;

        for(int i = 0; i < lElemCount; i++){
            if(!llvmTypeEq(l->getStructElementType(i), r->getStructElementType(i)))
                return false;
        }

        return true;
    }else{ //primitive type
        return true; /* true since ltt != rtt check above is false */
    }
}


/*
 *  Returns true if the given typetag is a primitive type, and thus
 *  accurately represents the entire type without information loss.
 *  NOTE: this function relies on the fact all primitive types are
 *        declared before non-primitive types in the TypeTag definition.
 */
bool isPrimitiveTypeTag(TypeTag ty){
    return ty >= TT_I8 && ty <= TT_Bool;
}


/*
 *  Converts a TypeTag to its string equivalent for
 *  helpful error messages.  For most cases, llvmTypeToStr
 *  should be used instead to provide the full type.
 */
string typeTagToStr(TypeTag ty){

    switch(ty){
        case TT_I8:    return "i8" ;
        case TT_I16:   return "i16";
        case TT_I32:   return "i32";
        case TT_I64:   return "i64";
        case TT_U8:    return "u8" ;
        case TT_U16:   return "u16";
        case TT_U32:   return "u32";
        case TT_U64:   return "u64";
        case TT_F16:   return "f16";
        case TT_F32:   return "f32";
        case TT_F64:   return "f64";
        case TT_Isz:   return "isz";
        case TT_Usz:   return "usz";
        case TT_C8:    return "c8" ;
        case TT_Bool:  return "bool";
        case TT_Unit:  return "unit";
        case TT_Type:  return "Type";

        /*
         * Because of the loss of specificity for these last types,
         * these strings are most likely insufficient.  The entire
         * AnType should be preferred to be used instead.
         */
        case TT_Tuple:        return "Tuple";
        case TT_Array:        return "Array";
        case TT_Ptr:          return "Ptr"  ;
        case TT_Data:         return "Data" ;
        case TT_TypeVar:      return "'t";
        case TT_Function:     return "Function";
        case TT_MetaFunction: return "Meta Function";
        case TT_FunctionList: return "Function List";
        case TT_TaggedUnion:  return "|";
        default:              return "(Unknown TypeTag " + to_string(ty) + ")";
    }
}

bool shouldWrapInParenthesis(TypeNode *type){
    return !type->params.empty() || type->typeTag == TT_Array;
}

/*
 *  Converts a typeNode directly to a string with no information loss.
 *  Used in ExtNode::compile
 */
string typeNodeToStr(const TypeNode *t){
    if(!t) return "null";

    if(t->typeTag == TT_Tuple){
        string ret = "(";
        TypeNode *elem = t->extTy.get();
        while(elem){
            if(elem->next.get())
                ret += typeNodeToStr(elem) + ", ";
            else
                ret += typeNodeToStr(elem) + ")";
            elem = (TypeNode*)elem->next.get();
        }
        return ret;
    }else if(t->typeTag == TT_Data or t->typeTag == TT_TaggedUnion or t->typeTag == TT_TypeVar){
        string name = t->typeName;
        if(!t->params.empty()){
            for(auto &param : t->params){
                auto pstr = typeNodeToStr(param.get());
                if(shouldWrapInParenthesis(param.get())) name += " (" + pstr + ")";
                else name += ' ' + pstr;
            }
        }
        return name;
    }else if(t->typeTag == TT_Array){
        auto *len = (IntLitNode*)t->extTy->next.get();
        return '[' + len->val + " " + typeNodeToStr(t->extTy.get()) + ']';
    }else if(t->typeTag == TT_Ptr){
        return "ref " + typeNodeToStr(t->extTy.get());
    }else if(t->typeTag == TT_Function or t->typeTag == TT_MetaFunction){
        string ret = "";
        string retTy = typeNodeToStr(t->extTy.get());
        TypeNode *cur = (TypeNode*)t->extTy->next.get();
        while(cur){
            auto pstr = typeNodeToStr(cur);
            if(shouldWrapInParenthesis(cur)) ret += "(" + pstr + ") ";
            else ret += pstr + ' ';
            cur = (TypeNode*)cur->next.get();
        }
        return ret + "-> " + retTy;
    }else{
        return typeTagToStr(t->typeTag);
    }
}


/**
 * true if the type should be wrapped in parenthesis
 * when being outputted as a string as a datatype typeArg
 *
 * eg.  in  Vec (Vec i32)
 * type = (Vec i32) and the return value should be true.
 */
bool shouldWrapInParenthesis(AnType *type){
    //Quick and dirty checks just to see if we need parenthesis wrapping the type or not
    if(ante::isPrimitiveTypeTag(type->typeTag) || type->typeTag == TT_TypeVar || type->typeTag == TT_Array)
        return false;

    if(type->typeTag == TT_Ptr)
        return true;

    auto adt = try_cast<AnDataType>(type);
    if (!adt) return false;
    return !adt->typeArgs.empty();
}


string traitToStr(const TraitImpl *trait){
    string ret = trait->getName();

    for(auto &type : trait->typeArgs){
        if(shouldWrapInParenthesis(type))
            ret += " (" + anTypeToStr(type) + ')';
        else
            ret += ' ' + anTypeToStr(type);
    }
    return ret;
}


lazy_str traitToColoredStr(const TraitImpl *trait){
    return lazy_str(traitToStr(trait), AN_TYPE_COLOR);
}


string commaSeparated(std::vector<TraitImpl*> const& traits){
    string ret = "";
    for(const auto &tr : traits){
        ret += traitToStr(tr);
        if(&tr != &traits.back())
            ret += ", ";
    }
    return ret;
}


string anTypeToStr(const AnType *t){
    if(!t) return "(null)";

    /** Must check for modifiers first as they can be lost after dyn_cast */
    if(t->isModifierType()){
        if(auto *mod = dynamic_cast<const BasicModifier*>(t)){
            return Lexer::getTokStr(mod->mod) + ' ' + anTypeToStr(mod->extTy);

        }else if(auto *cdmod = dynamic_cast<const CompilerDirectiveModifier*>(t)){
            //TODO: modify printingvisitor to print to streams
            // PrintingVisitor::print(cdmod->directive.get());
            return anTypeToStr(cdmod->extTy);
        }else{
            return "(unknown modifier type)";
        }
    }else if(auto *dt = try_cast<AnDataType>(t)){
        string n = dt->name;

        for(auto &a : dt->typeArgs){
            if(shouldWrapInParenthesis(a))
                n += " (" + anTypeToStr(a) + ')';
            else
                n += ' ' + anTypeToStr(a);
        }
        return n;
    }else if(auto *tvt = try_cast<AnTypeVarType>(t)){
        return tvt->name;
    }else if(auto *f = try_cast<AnFunctionType>(t)){
        string ret = "";
        for(auto &param : f->extTys){
            auto pstr = anTypeToStr(param);
            ret += (shouldWrapInParenthesis(param) ? '(' + pstr + ')' : pstr) + ' ';
        }

        string tcConstraints = f->typeClassConstraints.empty() ? ""
            : " given " + commaSeparated(f->typeClassConstraints);

        return ret + "-> " + anTypeToStr(f->retTy) + tcConstraints;
    }else if(auto *tup = try_cast<AnAggregateType>(t)){
        string ret = "(";

        for(const auto &ext : tup->extTys){
            ret += anTypeToStr(ext);

            if(&ext != &tup->extTys.back()){
                ret += ", ";
            }else if(tup->extTys.size() == 1){
                ret += ',';
            }
        }
        return ret + ")";
    }else if(auto *arr = try_cast<AnArrayType>(t)){
        return '[' + to_string(arr->len) + " " + anTypeToStr(arr->extTy) + ']';
    }else if(auto *ptr = try_cast<AnPtrType>(t)){
        return "ref " + anTypeToStr(ptr->extTy);
    }else{
        return typeTagToStr(t->typeTag);
    }
}


/*
 *  Returns a string representing the full type of ty.  Since it is converting
 *  from a llvm::Type, this will never return an unsigned integer type.
 */
string llvmTypeToStr(Type *ty){
    if(!ty) return "(null)";

    TypeTag tt = llvmTypeToTypeTag(ty);
    if(isPrimitiveTypeTag(tt)){
        return typeTagToStr(tt);
    }else if(tt == TT_Tuple){
        if(!ty->getStructName().empty())
            return string(ty->getStructName());

        string ret = "(";
        const unsigned size = ty->getStructNumElements();

        for(unsigned i = 0; i < size; i++){
            if(i == size-1){
                ret += llvmTypeToStr(ty->getStructElementType(i)) + ")";
            }else{
                ret += llvmTypeToStr(ty->getStructElementType(i)) + ", ";
            }
        }
        return ret;
    }else if(tt == TT_Array){
        return "[" + to_string(ty->getArrayNumElements()) + " " + llvmTypeToStr(ty->getArrayElementType()) + "]";
    }else if(tt == TT_Ptr){
        return llvmTypeToStr(ty->getPointerElementType()) + "*";
    }else if(tt == TT_Function){
        string ret = "func("; //TODO: get function return type
        const unsigned paramCount = ty->getFunctionNumParams();

        for(unsigned i = 0; i < paramCount; i++){
            if(i == paramCount-1)
                ret += llvmTypeToStr(ty->getFunctionParamType(i)) + ")";
            else
                ret += llvmTypeToStr(ty->getFunctionParamType(i)) + ", ";
        }
        return ret;
    }else if(tt == TT_TypeVar){
        return "(typevar)";
    }else if(tt == TT_Unit){
        return "unit";
    }
    return "(Unknown type)";
}

} //end of namespace ante
