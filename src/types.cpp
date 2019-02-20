#include <types.h>
#include <trait.h>
#include <nameresolution.h>
using namespace std;
using namespace llvm;
using namespace ante::parser;

namespace ante {

char getBitWidthOfTypeTag(const TypeTag ty){
    switch(ty){
        case TT_I8:  case TT_U8: case TT_C8:  return 8;
        case TT_I16: case TT_U16: case TT_F16: return 16;
        case TT_I32: case TT_U32: case TT_F32: case TT_C32: return 32;
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


bool isCompileTimeOnlyParamType(AnType *ty){
    return ty->typeTag == TT_Type || ty->typeTag == TT_Void || ty->hasModifier(Tok_Ante);
}


/*
 *  Returns the TypeNode* value of a TypedValue of type TT_Type
 */
AnType* extractTypeValue(const TypedValue &tv){
    auto zext = dyn_cast<ConstantInt>(tv.val)->getZExtValue();
    return (AnType*) zext;
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
        if(force) return AN_USZ_SIZE;
        else return "Lookup for typevar " + tvt->name + " not found";
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

Type* updateLlvmTypeBinding(Compiler *c, AnDataType *dt, bool force){
    if(dt->isGeneric && !force){
        error("Type " + anTypeToColoredStr(dt) + " is generic and cannot be translated", unknownLoc());
    }

    if(auto *tt = try_cast<AnTraitType>(dt)){
        auto llvmty = c->anTypeToLlvmType(tt->typeArgs.back());
        dt->llvmType = llvmty;
        return llvmty;
    }

    //create an empty type first so we dont end up with infinite recursion
    bool isPacked = dt->typeTag == TT_TaggedUnion;
    auto* structTy = dt->llvmType ? (StructType*)dt->llvmType
        : StructType::create(*c->ctxt, toLlvmTypeName(dt));

    dt->llvmType = structTy;

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

/*
 *  Checks for, and implicitly widens an integer or float type.
 *  The original value of num is returned if no widening can be performed.
 */
TypedValue Compiler::implicitlyWidenNum(TypedValue &num, TypeTag castTy){
    bool lIsInt = isIntTypeTag(num.type->typeTag);
    bool lIsFlt = isFPTypeTag(num.type->typeTag);

    if(lIsInt or lIsFlt){
        bool rIsInt = isIntTypeTag(castTy);
        bool rIsFlt = isFPTypeTag(castTy);
        if(!rIsInt && !rIsFlt){
            cerr << "castTy argument of implicitlyWidenNum must be a numeric primitive type\n";
            exit(1);
        }

        int lbw = getBitWidthOfTypeTag(num.type->typeTag);
        int rbw = getBitWidthOfTypeTag(castTy);
        Type *ty = typeTagToLlvmType(castTy, *ctxt);

        //integer widening
        if(lIsInt && rIsInt){
            if(lbw <= rbw){
                return TypedValue(
                    builder.CreateIntCast(num.val, ty, !isUnsignedTypeTag(num.type->typeTag)),
                    AnType::getPrimitive(castTy)
                );
            }

        //int -> flt, (flt -> int is never implicit)
        }else if(lIsInt && rIsFlt){
            return TypedValue(
                isUnsignedTypeTag(num.type->typeTag)
                    ? builder.CreateUIToFP(num.val, ty)
                    : builder.CreateSIToFP(num.val, ty),

                AnType::getPrimitive(castTy)
            );

        //float widening
        }else if(lIsFlt && rIsFlt){
            if(lbw < rbw){
                return TypedValue(
                    builder.CreateFPExt(num.val, ty),
                    AnType::getPrimitive(castTy)
                );
            }
        }
    }

    return num;
}


/*
 *  Assures two IntegerType'd Values have the same bitwidth.
 *  If not, one is extended to the larger bitwidth and mutated appropriately.
 *  If the extended integer value is unsigned, it is zero extended, otherwise
 *  it is sign extended.
 *  Assumes the llvm::Type of both values to be an instance of IntegerType.
 */
void Compiler::implicitlyCastIntToInt(TypedValue *lhs, TypedValue *rhs){
    int lbw = getBitWidthOfTypeTag(lhs->type->typeTag);
    int rbw = getBitWidthOfTypeTag(rhs->type->typeTag);

    if(lbw != rbw){
        //Cast the value with the smaller bitwidth to the type with the larger bitwidth
        if(lbw < rbw){
            auto ret = TypedValue(
                builder.CreateIntCast(lhs->val, rhs->getType(),
                    !isUnsignedTypeTag(lhs->type->typeTag)),
                rhs->type
            );

            *lhs = ret;

        }else{//lbw > rbw
            auto ret = TypedValue(
                builder.CreateIntCast(rhs->val, lhs->getType(),
                    !isUnsignedTypeTag(rhs->type->typeTag)),
                lhs->type
            );

            *rhs = ret;
        }
    }
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

/*
 *  Performs an implicit cast from a float to int.  Called in any operation
 *  involving an integer, a float, and a binop.  No matter the ints size,
 *  it is always casted to the (possibly smaller) float value.
 */
void Compiler::implicitlyCastIntToFlt(TypedValue *lhs, Type *ty){
    auto ret = TypedValue(
        isUnsignedTypeTag(lhs->type->typeTag)
            ? builder.CreateUIToFP(lhs->val, ty)
            : builder.CreateSIToFP(lhs->val, ty),

        AnType::getPrimitive(llvmTypeToTypeTag(ty))
    );
    *lhs = ret;
}


/*
 *  Performs an implicit cast from a float to float.
 */
void Compiler::implicitlyCastFltToFlt(TypedValue *lhs, TypedValue *rhs){
    int lbw = getBitWidthOfTypeTag(lhs->type->typeTag);
    int rbw = getBitWidthOfTypeTag(rhs->type->typeTag);

    if(lbw != rbw){
        if(lbw < rbw){
            auto ret = TypedValue(
                builder.CreateFPExt(lhs->val, rhs->getType()),
                rhs->type
            );
            *lhs = ret;
        }else{//lbw > rbw
            auto ret = TypedValue(
                builder.CreateFPExt(rhs->val, lhs->getType()),
                lhs->type
            );
            *rhs = ret;
        }
    }
}


/*
 *  Detects, and creates an implicit type conversion when necessary.
 */
void Compiler::handleImplicitConversion(TypedValue *lhs, TypedValue *rhs){
    bool lIsInt = isIntTypeTag(lhs->type->typeTag);
    bool lIsFlt = isFPTypeTag(lhs->type->typeTag);
    if(!lIsInt && !lIsFlt) return;

    bool rIsInt = isIntTypeTag(rhs->type->typeTag);
    bool rIsFlt = isFPTypeTag(rhs->type->typeTag);
    if(!rIsInt && !rIsFlt) return;

    //both values are numeric, so forward them to the relevant casting method
    if(lIsInt && rIsInt){
        implicitlyCastIntToInt(lhs, rhs);  //implicit int -> int (widening)
    }else if(lIsInt && rIsFlt){
        implicitlyCastIntToFlt(lhs, rhs->getType()); //implicit int -> flt
    }else if(lIsFlt && rIsInt){
        implicitlyCastIntToFlt(rhs, lhs->getType()); //implicit int -> flt
    }else if(lIsFlt && rIsFlt){
        implicitlyCastFltToFlt(lhs, rhs); //implicit int -> flt
    }
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
        case TT_C32:    return Type::getInt32Ty(ctxt);
        case TT_Bool:   return Type::getInt1Ty(ctxt);
        case TT_Void:   return Type::getVoidTy(ctxt);
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

    return TT_Void;
}

/*
 *  Converts a TypeNode to an llvm::Type.  While much less information is lost than
 *  llvmTypeToTokType, information on signedness of integers is still lost, causing the
 *  unfortunate necessity for the use of a TypedValue for the storage of this information.
 */
Type* Compiler::anTypeToLlvmType(const AnType *ty, bool force){
    vector<Type*> tys;

    switch(ty->typeTag){
        case TT_Ptr: {
            auto *ptr = try_cast<AnPtrType>(ty);
            return ptr->extTy->typeTag != TT_Void ?
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
            if(dt->llvmType)
                return dt->llvmType;
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
                tys.push_back(anTypeToLlvmType(f->extTys[i], force));
            }

            return FunctionType::get(anTypeToLlvmType(f->retTy, force), tys, false)->getPointerTo();
        }
        case TT_TypeVar: {
            auto *tvt = try_cast<AnTypeVarType>(ty);
            //error("Use of undeclared type variable " + ty->typeName, ty->loc);
            //error("tn2llvmt: TypeVarError; lookup for "+ty->typeName+" not found", ty->loc);
            //throw new TypeVarError();
            if(!force)
                cerr << "Warning: cannot translate undeclared typevar " << tvt->name << endl;

            return Type::getInt64PtrTy(*ctxt);
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


bool dataTypeImplementsTrait(AnDataType *dt, string trait){
    for(auto traitImpl : dt->traitImpls){
        if(traitImpl->name == trait)
            return true;
    }
    return false;
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
        case TT_C32:   return "c32";
        case TT_Bool:  return "bool";
        case TT_Void:  return "void";
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
            name += "<";
            name += typeNodeToStr(t->params[0].get());
            for(unsigned i = 1; i < t->params.size(); i++){
                name += ", ";
                name += typeNodeToStr(t->params[i].get());
            }
            name += ">";
        }
        return name;
    }else if(t->typeTag == TT_Array){
        auto *len = (IntLitNode*)t->extTy->next.get();
        return '[' + len->val + " " + typeNodeToStr(t->extTy.get()) + ']';
    }else if(t->typeTag == TT_Ptr){
        return typeNodeToStr(t->extTy.get()) + "*";
    }else if(t->typeTag == TT_Function or t->typeTag == TT_MetaFunction){
        string ret = "(";
        string retTy = typeNodeToStr(t->extTy.get());
        TypeNode *cur = (TypeNode*)t->extTy->next.get();
        while(cur){
            ret += typeNodeToStr(cur);
            cur = (TypeNode*)cur->next.get();
            if(cur) ret += ",";
        }
        return ret + ")->" + retTy;
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

    auto adt = try_cast<AnDataType>(type);
    if (!adt) return false;
    return !adt->typeArgs.empty() || try_cast<AnTraitType>(type);
}

/**
 * true if the type should be wrapped in parenthesis
 * when being outputted as a string as a tuple member
 *
 * eg.  in  (Vec i32, i32, (i32, Str))
 * type = any element of the three element tuple above
 * and the return value should be true only for the contained
 * tuple (in this case)
 */
bool shouldWrapInParenthesisWhenInTuple(AnType *type){
    //Quick and dirty checks just to see if we need parenthesis wrapping the type or not
    if(ante::isPrimitiveTypeTag(type->typeTag) || type->typeTag == TT_TypeVar || type->typeTag == TT_Array)
        return false;

    return type->typeTag == TT_Tuple;
}


string commaSeparated(std::vector<AnTraitType*> const& types){
    string ret = "";
    for(const auto &ty : types){
        ret += anTypeToStr(ty);
        if(&ty != &types.back())
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

        if(auto *tt = try_cast<AnTraitType>(t)){
            if(shouldWrapInParenthesis(tt->selfType))
                n += " (" + anTypeToStr(tt->selfType) + ')';
            else
                n += ' ' + anTypeToStr(tt->selfType);
        }

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
        string ret = "(";
        string retTy = anTypeToStr(f->retTy);
        string tcConstraints = f->typeClassConstraints.empty() ? ""
            : " : " + commaSeparated(f->typeClassConstraints);

        if(f->extTys.size() == 1){
            if(f->extTys[0]->typeTag == TT_Function || f->extTys[0]->typeTag == TT_Tuple)
                return '(' + anTypeToStr(f->extTys[0]) + ") -> " + retTy + tcConstraints;
            else
                return anTypeToStr(f->extTys[0]) + " -> " + retTy + tcConstraints;
        }

        auto paramTypes = AnAggregateType::get(TT_Tuple, f->extTys);
        return anTypeToStr(paramTypes) + " -> " + retTy + tcConstraints;
    }else if(auto *tup = try_cast<AnAggregateType>(t)){
        string ret = "(";

        for(const auto &ext : tup->extTys){
            if(shouldWrapInParenthesisWhenInTuple(ext))
                ret += '(' + anTypeToStr(ext) + ')';
            else
                ret += anTypeToStr(ext);

            if(&ext != &tup->extTys.back())
                ret += ", ";
        }
        return ret + ")";
    }else if(auto *arr = try_cast<AnArrayType>(t)){
        return '[' + to_string(arr->len) + " " + anTypeToStr(arr->extTy) + ']';
    }else if(auto *ptr = try_cast<AnPtrType>(t)){
        return anTypeToStr(ptr->extTy) + "*";
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
    }else if(tt == TT_Void){
        return "void";
    }
    return "(Unknown type)";
}

} //end of namespace ante
