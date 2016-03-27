#include <parser.h>


char getBitWidthOfTypeTag(const TypeTag ty){
    switch(ty){
        case TT_I8:  case TT_U8:  return 8;
        case TT_I16: case TT_U16: case TT_F16: return 16;
        case TT_I32: case TT_U32: case TT_F32: return 32;
        case TT_I64: case TT_U64: case TT_F64: return 64;
        case TT_Isz: case TT_Usz: return 64; //TODO: detect 32-bit platform
        case TT_Bool: return 1;
        default: return 0;
    }
}


/*
 *  Assures two IntegerType'd Values have the same bitwidth.
 *  If not, one is extended to the larger bitwidth and mutated appropriately.
 *  If the extended integer value is unsigned, it is zero extended, otherwise
 *  it is sign extended.
 *  Assumes the llvm::Type of both values to be an instance of IntegerType.
 */
void Compiler::checkIntSize(TypedValue **lhs, TypedValue **rhs){
    int lbw = getBitWidthOfTypeTag((*lhs)->type);
    int rbw = getBitWidthOfTypeTag((*rhs)->type);

    if(lbw != rbw){
        //Cast the value with the smaller bitwidth to the type with the larger bitwidth
        if(lbw < rbw){
            (*lhs)->val = builder.CreateIntCast((*lhs)->val, (*rhs)->val->getType(), !isUnsignedTypeTag((*lhs)->type));
            (*lhs)->type = (*rhs)->type;
        }else{//lbw > rbw
            (*rhs)->val = builder.CreateIntCast((*rhs)->val, (*lhs)->val->getType(), !isUnsignedTypeTag((*rhs)->type));
            (*rhs)->type = (*lhs)->type;
        }
    }
}


/*
 *  Translates an individual TypeTag to an llvm::Type.
 *  Only intended for primitive types, as there is not enough
 *  information stored in a TypeTag to convert to array, tuple,
 *  or function types.
 */
Type* typeTagToLlvmType(TypeTag ty, string typeName = ""){
    switch(ty){
        case TT_Data:
            cerr << "tokTypeToLlvmType cannot be used with UserTypes.\n";
            return nullptr;
        case TT_I8:  case TT_U8:  return Type::getInt8Ty(getGlobalContext());
        case TT_I16: case TT_U16: return Type::getInt16Ty(getGlobalContext());
        case TT_I32: case TT_U32: return Type::getInt32Ty(getGlobalContext());
        case TT_I64: case TT_U64: return Type::getInt64Ty(getGlobalContext());
        case TT_Isz:    return Type::getVoidTy(getGlobalContext()); //TODO: implement
        case TT_Usz:    return Type::getVoidTy(getGlobalContext()); //TODO: implement
        case TT_F16:    return Type::getHalfTy(getGlobalContext());
        case TT_F32:    return Type::getFloatTy(getGlobalContext());
        case TT_F64:    return Type::getDoubleTy(getGlobalContext());
        case TT_C8:     return Type::getInt8Ty(getGlobalContext());
        case TT_C32:    return Type::getInt32Ty(getGlobalContext());
        case TT_Bool:   return Type::getInt1Ty(getGlobalContext());
        case TT_StrLit: return Type::getInt8PtrTy(getGlobalContext());
        case TT_Void:   return Type::getVoidTy(getGlobalContext());
        default:
            cerr << "typeTagToLlvmType: Unknown TypeTag " << ty << ", returning nullptr.\n";
            return nullptr;
    }
}

/*
 *  Translates a llvm::Type to a TypeTag. Not intended for in-depth analysis
 *  as it loses data about the type and name of UserTypes, and cannot distinguish 
 *  between signed and unsigned integer types.  As such, this should mainly be 
 *  used for comparing primitive datatypes, or just to detect if something is a
 *  primitive.
 */
TypeTag llvmTypeToTypeTag(Type *t){
    if(t->isIntegerTy(8)) return TT_I8;
    if(t->isIntegerTy(16)) return TT_I16;
    if(t->isIntegerTy(32)) return TT_I32;
    if(t->isIntegerTy(64)) return TT_I64;
    if(t->isHalfTy()) return TT_F16;
    if(t->isFloatTy()) return TT_F32;
    if(t->isDoubleTy()) return TT_F64;
    
    if(t->isArrayTy()) return TT_Array;
    if(t->isStructTy()) return TT_Tuple; /* Could also be a TT_Data! */
    if(t->isPointerTy()) return TT_Ptr;
    if(t->isFunctionTy()) return TT_Func;

    return TT_Void;
}

/*
 *  Converts a TypeNode to an llvm::Type.  While much less information is lost than
 *  llvmTypeToTokType, information on signedness of integers is still lost, causing the
 *  unfortunate necessity for the use of a TypedValue for the storage of this information.
 */
Type* typeNodeToLlvmType(TypeNode *tyNode){
    vector<Type*> tys;
    TypeNode *tyn = tyNode->extTy.get();
    switch(tyNode->type){
        case TT_Ptr:
            return PointerType::get(typeNodeToLlvmType(tyNode->extTy.get()), 0);
        case TT_Tuple:
            while(tyn){
                tys.push_back(typeNodeToLlvmType(tyn));
                tyn = (TypeNode*)tyn->next.get();
            }
            return StructType::get(getGlobalContext(), tys);
        case TT_Array: //TODO array type
            return ArrayType::get(typeNodeToLlvmType(tyn), 0/*num elements*/);
        case TT_Data:
        case TT_Func: //TODO function pointer type
            cout << "typeNodeToLlvmType: UserTypes and Function pointer types are currently unimplemented.  A void type will be returned instead.\n";
            return Type::getVoidTy(getGlobalContext());
        default:
            return typeTagToLlvmType(tyNode->type);
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
        return llvmTypeEq(l->getPointerElementType(), r->getPointerElementType());
    }else if(ltt == TT_Array){
        return llvmTypeEq(l->getArrayElementType(), r->getArrayElementType());
    }else if(ltt == TT_Func){
        int lParamCount = l->getFunctionNumParams();
        int rParamCount = r->getFunctionNumParams();
        
        if(lParamCount != rParamCount)
            return false;

        for(int i = 0; i < lParamCount; i++){
            if(!llvmTypeEq(l->getFunctionParamType(i), r->getFunctionParamType(i)))
                return false;
        } 
        return true;
    }else if(ltt == TT_Tuple || ltt == TT_Data){
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
        return ltt == rtt;
    }
}

/*
 *  Returns true if the given typetag is a primitive type, and thus
 *  accurately represents the entire type without information loss.
 *  NOTE: this function relies on the fact all primitive types are
 *        declared before non-primitive types in the TypeTag definition.
 */
bool isPrimitiveTypeTag(TypeTag ty){
    return ty >= TT_I8 && ty <= TT_StrLit;
}

/*
 *  Converts a TypeTag to its string equivalent for
 *  helpful error messages.  For most cases, llvmTypeToStr
 *  should be used instead to provide the full type.
 */
string typeTagToStr(TypeTag ty){
    switch(ty){
        case TT_I8:    return "i8";
        case TT_I16:   return "i16";
        case TT_I32:   return "i32";
        case TT_I64:   return "i64";
        case TT_U8:    return "u8";
        case TT_U16:   return "u16";
        case TT_U32:   return "u32";
        case TT_U64:   return "u64";
        case TT_F16:   return "f16";
        case TT_F32:   return "f32";
        case TT_F64:   return "f64";
        case TT_Isz:   return "isz";
        case TT_Usz:   return "usz";
        case TT_C8:    return "c8";
        case TT_C32:   return "c32";
        case TT_Bool:  return "bool";
        case TT_Void:  return "void";

        /* 
         * Because of the loss of specificity for these last four types, 
         * these strings are most likely insufficient.  The llvm::Type
         * should instead be printed for these types
         */
        case TT_Tuple: return "Tuple";
        case TT_Array: return "Array";
        case TT_Ptr:   return "Ptr";
        case TT_Data:  return "Data";
        default:       return "Unknown TypeTag " + to_string(ty);
    }
}

/*
 *  Returns a string representing the full type of ty.  Since it is converting
 *  from a llvm::Type, this will never return an unsigned integer type.
 */
string llvmTypeToStr(Type *ty){
    TypeTag tt = llvmTypeToTypeTag(ty);
    if(isPrimitiveTypeTag(tt)){
        return typeTagToStr(tt);
    }else if(tt == TT_Tuple){
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
        return llvmTypeToStr(ty->getArrayElementType()) + "[]";
    }else if(tt == TT_Ptr){
        return llvmTypeToStr(ty->getPointerElementType()) + "*";
    }else if(tt == TT_Func){
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
        return "typevar";
    }else if(tt == TT_Void){
        return "void";
    }
    return "(Unknown type)";
}
