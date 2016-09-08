#include <compiler.h>


char getBitWidthOfTypeTag(const TypeTag ty){
    switch(ty){
        case TT_I8:  case TT_U8: case TT_C8:  return 8;
        case TT_I16: case TT_U16: case TT_F16: return 16;
        case TT_I32: case TT_U32: case TT_F32: case TT_C32: return 32;
        case TT_I64: case TT_U64: case TT_F64: return 64;
        case TT_Isz: case TT_Usz: return 64; //TODO: detect 32-bit platform
        case TT_Bool: return 1;
   
        case TT_Ptr: case TT_StrLit: case TT_Array: return 64;
        case TT_Function: case TT_Method: return 64;
        default: return 0;
    }
}


unsigned int TypeNode::getSizeInBits(Compiler *c){
    int total = 0;
    TypeNode *ext = this->extTy.get();

    if(isPrimitiveTypeTag(this->type))
        return getBitWidthOfTypeTag(this->type);
   
    if(type == TT_Data){
        auto *dataTy = c->lookupType(typeName);
        return dataTy->tyn->getSizeInBits(c);
    }

    if(type == TT_Tuple || type == TT_TaggedUnion){
        while(ext){
            total += ext->getSizeInBits(c);
            ext = (TypeNode*)ext->next.get();
        }
    }else if(type == TT_Array || type == TT_Ptr || type == TT_Function || type == TT_Method){
        return 64;
    }
    
    return total;
}


/*
 *  Assures two IntegerType'd Values have the same bitwidth.
 *  If not, one is extended to the larger bitwidth and mutated appropriately.
 *  If the extended integer value is unsigned, it is zero extended, otherwise
 *  it is sign extended.
 *  Assumes the llvm::Type of both values to be an instance of IntegerType.
 */
void Compiler::implicitlyCastIntToInt(TypedValue **lhs, TypedValue **rhs){
    int lbw = getBitWidthOfTypeTag((*lhs)->type->type);
    int rbw = getBitWidthOfTypeTag((*rhs)->type->type);

    if(lbw != rbw){
        //Cast the value with the smaller bitwidth to the type with the larger bitwidth
        if(lbw < rbw){
            (*lhs)->val = builder.CreateIntCast((*lhs)->val, (*rhs)->getType(), !isUnsignedTypeTag((*lhs)->type->type));
            (*lhs)->type.reset(deepCopyTypeNode((*rhs)->type.get()));
        }else{//lbw > rbw
            (*rhs)->val = builder.CreateIntCast((*rhs)->val, (*lhs)->getType(), !isUnsignedTypeTag((*rhs)->type->type));
            (*rhs)->type.reset(deepCopyTypeNode((*lhs)->type.get()));
        }
    }
}

inline bool isIntTypeTag(const TypeTag ty){
    return ty==TT_I8||ty==TT_I16||ty==TT_I32||ty==TT_I64||
           ty==TT_U8||ty==TT_U16||ty==TT_U32||ty==TT_U64||
           ty==TT_Isz||ty==TT_Usz;
}

inline bool isFPTypeTag(const TypeTag tt){
    return tt==TT_F16||tt==TT_F32||tt==TT_F64;
}

/*
 *  Performs an implicit cast from a float to int.  Called in any operation
 *  involving an integer, a float, and a binop.  No matter the ints size,
 *  it is always casted to the (possibly smaller) float value.
 */
void Compiler::implicitlyCastIntToFlt(TypedValue **lhs, Type *ty){
    if(isUnsignedTypeTag((*lhs)->type->type)){
        (*lhs)->val = builder.CreateUIToFP((*lhs)->val, ty);
    }else{
        (*lhs)->val = builder.CreateSIToFP((*lhs)->val, ty);
    }
    (*lhs)->type.reset(mkAnonTypeNode(llvmTypeToTypeTag(ty)));
}


/*
 *  Performs an implicit cast from a float to float.
 */
void Compiler::implicitlyCastFltToFlt(TypedValue **lhs, TypedValue **rhs){
    int lbw = getBitWidthOfTypeTag((*lhs)->type->type);
    int rbw = getBitWidthOfTypeTag((*rhs)->type->type);

    if(lbw != rbw){
        if(lbw < rbw){
            (*lhs)->val = builder.CreateFPExt((*lhs)->val, (*rhs)->getType());
            (*lhs)->type.reset((*rhs)->type.get());
        }else{//lbw > rbw
            (*rhs)->val = builder.CreateFPExt((*rhs)->val, (*lhs)->getType());
            (*rhs)->type.reset((*lhs)->type.get());
        }
    }
}


/*
 *  Detects, and creates an implicit type conversion when necessary.
 */
void Compiler::handleImplicitConversion(TypedValue **lhs, TypedValue **rhs){
    bool lIsInt = isIntTypeTag((*lhs)->type->type);
    bool lIsFlt = isFPTypeTag((*lhs)->type->type);
    if(!lIsInt && !lIsFlt) return;

    bool rIsInt = isIntTypeTag((*rhs)->type->type);
    bool rIsFlt = isFPTypeTag((*rhs)->type->type);
    if(!rIsInt && !rIsFlt) return;

    //both values are numeric, so forward them to the relevant casting method
    if(lIsInt && rIsInt){
        implicitlyCastIntToInt(lhs, rhs);  //implicit int -> int (widening)
    }else if(lIsInt && rIsFlt){
        implicitlyCastIntToFlt(lhs, (*rhs)->getType()); //implicit int -> flt
    }else if(lIsFlt && rIsInt){
        implicitlyCastIntToFlt(rhs, (*lhs)->getType()); //implicit int -> flt
    }else if(lIsFlt && rIsFlt){
        implicitlyCastFltToFlt(lhs, rhs); //implicit int -> flt
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
        case TT_Void:   return Type::getVoidTy(getGlobalContext());
        default:
            cerr << "typeTagToLlvmType: Unknown/Unsupported TypeTag " << ty << ", returning nullptr.\n";
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
    if(t->isIntegerTy(1)) return TT_Bool;

    if(t->isIntegerTy(8)) return TT_I8;
    if(t->isIntegerTy(16)) return TT_I16;
    if(t->isIntegerTy(32)) return TT_I32;
    if(t->isIntegerTy(64)) return TT_I64;
    if(t->isHalfTy()) return TT_F16;
    if(t->isFloatTy()) return TT_F32;
    if(t->isDoubleTy()) return TT_F64;
    
    //if(t->isVectorTy()) return TT_Array;
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
Type* Compiler::typeNodeToLlvmType(TypeNode *tyNode){
    vector<Type*> tys;
    TypeNode *tyn = tyNode->extTy.get();
    DataType *userType;

    switch(tyNode->type){
        case TT_Ptr:
            return tyn->type != TT_Void ?
                PointerType::get(typeNodeToLlvmType(tyn), 0)
                : Type::getInt8Ty(getGlobalContext())->getPointerTo();
        case TT_Array:
            return PointerType::get(typeNodeToLlvmType(tyn), 0);
        case TT_Tuple:
            while(tyn){
                tys.push_back(typeNodeToLlvmType(tyn));
                tyn = (TypeNode*)tyn->next.get();
            }
            return PointerType::get(StructType::get(getGlobalContext(), tys), 0);
        case TT_Data:
            userType = lookupType(tyNode->typeName);
            if(!userType)
                return (Type*)compErr("Use of undeclared type " + tyNode->typeName, tyNode->loc);

            //((StructType*)userType->tyn)->setName(tyNode->typeName);
            return typeNodeToLlvmType(userType->tyn.get());
        case TT_Function: //TODO function pointer type
            cout << "typeNodeToLlvmType: Function pointer types are currently unimplemented.  A void type will be returned instead.\n";
            return Type::getVoidTy(getGlobalContext());
        case TT_TaggedUnion:
            userType = lookupType(tyNode->typeName);
            if(!userType)
                return (Type*)compErr("Use of undeclared type " + tyNode->typeName, tyNode->loc);

            tyn = userType->tyn->extTy.get();
            while(tyn){
                tys.push_back(typeNodeToLlvmType(tyn));
                tyn = (TypeNode*)tyn->next.get();
            }
            return StructType::get(getGlobalContext(), tys);
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
        Type *lty = l->getPointerElementType();
        Type *rty = r->getPointerElementType();
        Type *vty = Type::getVoidTy(getGlobalContext());

        if(lty == vty || rty == vty) return true;

        return llvmTypeEq(lty, rty);
    }else if(ltt == TT_Array){
        return llvmTypeEq(l->getPointerElementType(), r->getPointerElementType());
    }else if(ltt == TT_Function){
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
        return true; /* true since ltt != rtt check above is false */
    }
}

/*
 *  Helper function to check if each type's list of extension
 *  types are all approximately equal.  Used when checking the
 *  equality of TypeNodes of type Tuple, Data, Function, or any
 *  type with multiple extTys.
 */
bool extTysEq(const TypeNode *l, const TypeNode *r){
    TypeNode *lExt = l->extTy.get();
    TypeNode *rExt = r->extTy.get();

    while(lExt && rExt){
        if(*lExt != *rExt) return false;
        lExt = (TypeNode*)lExt->next.get();
        rExt = (TypeNode*)rExt->next.get();
        if((lExt && !rExt) || (rExt && !lExt)) return false;
    }
    return true;
 }

/*
 *  Return true if both typenodes are approximately equal
 */
bool TypeNode::operator==(TypeNode &r) const {
    if(this->type == TT_TaggedUnion and r.type == TT_Data) return typeName == r.typeName;
    if(this->type == TT_Data and r.type == TT_TaggedUnion) return typeName == r.typeName;
    
    if(this->type != r.type) return false;

    if(r.type == TT_Ptr || r.type == TT_Array){
        if(extTy->type == TT_Void || r.extTy->type == TT_Void)
            return true;

        return *this->extTy.get() == *r.extTy.get();
    }else if(r.type == TT_Data || r.type == TT_TaggedUnion){
        return typeName == r.typeName;
    }else if(r.type == TT_Function || r.type == TT_Method || r.type == TT_Tuple){
        return extTysEq(this, &r);
    }
    //primitive type
    return true;
}

bool TypeNode::operator!=(TypeNode &r) const {
    return !(*this == r);
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

        /* 
         * Because of the loss of specificity for these last four types, 
         * these strings are most likely insufficient.  The llvm::Type
         * should instead be printed for these types
         */
        case TT_Tuple:       return "Tuple";
        case TT_Array:       return "Array";
        case TT_Ptr:         return "Ptr"  ;
        case TT_Data:        return "Data" ;
        case TT_Function:    return "Function";
        case TT_Method:      return "Method";
        case TT_TaggedUnion: return "|";
        default:             return "(Unknown TypeTag " + to_string(ty) + ")";
    }
}

/*
 *  Converts a typeNode directly to a string with no information loss.
 *  Used in ExtNode::compile
 */
string typeNodeToStr(TypeNode *t){
    if(t->type == TT_Tuple){
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
    }else if(t->type == TT_Data || t->type == TT_TaggedUnion){
        return t->typeName;
    }else if(t->type == TT_Array){
        return '[' + typeNodeToStr(t->extTy.get()) + ']';
    }else if(t->type == TT_Ptr){
        return typeNodeToStr(t->extTy.get()) + "*";
    }else if(t->type == TT_Function || t->type == TT_Method){
        string ret = "(";
        string retTy = typeNodeToStr(t->extTy.get());
        TypeNode *cur = (TypeNode*)t->extTy->next.get();
        while(cur){
            ret += typeNodeToStr(cur);
            cur = (TypeNode*)cur->next.get();
            if(cur) ret += ",";
        }
        return ret + ")=>" + retTy;
    }else{
        return typeTagToStr(t->type);
    }
}

/*
 *  Returns a string representing the full type of ty.  Since it is converting
 *  from a llvm::Type, this will never return an unsigned integer type.
 *
 *  Gives output in a different terminal color intended for printing, use typeNodeToStr
 *  to get a type without print color.
 */
string llvmTypeToStr(Type *ty){
    TypeTag tt = llvmTypeToTypeTag(ty);
    if(isPrimitiveTypeTag(tt)){
        return typeTagToStr(tt);
    }else if(tt == TT_Tuple){
        //if(!ty->getStructName().empty())
        //    return string(ty->getStructName());

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
        return "[" + llvmTypeToStr(ty->getPointerElementType()) + "]";
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
