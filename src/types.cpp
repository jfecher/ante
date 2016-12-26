#include <compiler.h>


char getBitWidthOfTypeTag(const TypeTag ty){
    switch(ty){
        case TT_I8:  case TT_U8: case TT_C8:  return 8;
        case TT_I16: case TT_U16: case TT_F16: return 16;
        case TT_I32: case TT_U32: case TT_F32: case TT_C32: return 32;
        case TT_I64: case TT_U64: case TT_F64: return 64;
        case TT_Isz: case TT_Usz: return 64; //TODO: detect 32-bit platform
        case TT_Bool: return 1;
   
        case TT_Ptr: return 64;
        case TT_Function: case TT_Method: case TT_MetaFunction: return 64;
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
        if(!dataTy){
            c->compErr("Type "+typeName+" has not been declared", loc);
            return 0;
        }
        return dataTy->tyn->getSizeInBits(c);
    }

    if(type == TT_Tuple || type == TT_TaggedUnion){
        while(ext){
            total += ext->getSizeInBits(c);
            ext = (TypeNode*)ext->next.get();
        }
    }else if(type == TT_Array){
        auto *len = (IntLitNode*)ext->next.get();
        return stoi(len->val) * ext->getSizeInBits(c);
    }else if(type == TT_Ptr || type == TT_Function || type == TT_MetaFunction || type == TT_Method){
        return 64;
    }
    
    return total;
}


/*
 *  Checks for, and implicitly widens an integer or float type.
 *  The original value of num is returned if no widening can be performed.
 */
TypedValue* Compiler::implicitlyWidenNum(TypedValue *num, TypeTag castTy){
    bool lIsInt = isIntTypeTag(num->type->type);
    bool lIsFlt = isFPTypeTag(num->type->type);

    if(lIsInt || lIsFlt){
        bool rIsInt = isIntTypeTag(castTy);
        bool rIsFlt = isFPTypeTag(castTy);
        if(!rIsInt && !rIsFlt){
            cerr << "castTy argument of implicitlyWidenNum must be a numeric primitive type\n";
            exit(1);
        }

        int lbw = getBitWidthOfTypeTag(num->type->type);
        int rbw = getBitWidthOfTypeTag(castTy);
        Type *ty = typeTagToLlvmType(castTy, ctxt);

        //integer widening
        if(lIsInt && rIsInt){
            if(lbw <= rbw){
                return new TypedValue(
                    builder.CreateIntCast(num->val, ty, !isUnsignedTypeTag(num->type->type)),
                    mkAnonTypeNode(castTy)
                );
            }

        //int -> flt, flt -> int is never implicit
        }else if(lIsInt && rIsFlt){
            return new TypedValue(
                isUnsignedTypeTag(num->type->type)
                    ? builder.CreateUIToFP(num->val, ty)
                    : builder.CreateSIToFP(num->val, ty),

                mkAnonTypeNode(castTy)
            );

        //float widening
        }else if(lIsFlt && rIsFlt){
            if(lbw < rbw){
                return new TypedValue(
                    builder.CreateFPExt(num->val, ty),
                    mkAnonTypeNode(castTy)
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
void Compiler::implicitlyCastIntToInt(TypedValue **lhs, TypedValue **rhs){
    int lbw = getBitWidthOfTypeTag((*lhs)->type->type);
    int rbw = getBitWidthOfTypeTag((*rhs)->type->type);

    if(lbw != rbw){
        //Cast the value with the smaller bitwidth to the type with the larger bitwidth
        if(lbw < rbw){
            auto *ret = new TypedValue(
                builder.CreateIntCast((*lhs)->val, (*rhs)->getType(), !isUnsignedTypeTag((*lhs)->type->type)),
                deepCopyTypeNode((*rhs)->type.get())
            );
            
            *lhs = ret;

        }else{//lbw > rbw
            auto *ret = new TypedValue(
                builder.CreateIntCast((*rhs)->val, (*lhs)->getType(), !isUnsignedTypeTag((*rhs)->type->type)),
                deepCopyTypeNode((*lhs)->type.get())
            );

            *rhs = ret;
        }
    }
}

bool isIntTypeTag(const TypeTag ty){
    return ty==TT_I8||ty==TT_I16||ty==TT_I32||ty==TT_I64||
           ty==TT_U8||ty==TT_U16||ty==TT_U32||ty==TT_U64||
           ty==TT_Isz||ty==TT_Usz||ty==TT_C8;
}

bool isFPTypeTag(const TypeTag tt){
    return tt==TT_F16||tt==TT_F32||tt==TT_F64;
}

bool isNumericTypeTag(const TypeTag ty){
    return isIntTypeTag(ty) || isFPTypeTag(ty);
}

/*
 *  Performs an implicit cast from a float to int.  Called in any operation
 *  involving an integer, a float, and a binop.  No matter the ints size,
 *  it is always casted to the (possibly smaller) float value.
 */
void Compiler::implicitlyCastIntToFlt(TypedValue **lhs, Type *ty){
    auto *ret = new TypedValue(
        isUnsignedTypeTag((*lhs)->type->type)
            ? builder.CreateUIToFP((*lhs)->val, ty)
            : builder.CreateSIToFP((*lhs)->val, ty),

        mkAnonTypeNode(llvmTypeToTypeTag(ty))
    );
    *lhs = ret;
}


/*
 *  Performs an implicit cast from a float to float.
 */
void Compiler::implicitlyCastFltToFlt(TypedValue **lhs, TypedValue **rhs){
    int lbw = getBitWidthOfTypeTag((*lhs)->type->type);
    int rbw = getBitWidthOfTypeTag((*rhs)->type->type);

    if(lbw != rbw){
        if(lbw < rbw){
            auto *ret = new TypedValue(
                builder.CreateFPExt((*lhs)->val, (*rhs)->getType()),
                deepCopyTypeNode((*rhs)->type.get())
            );
            *lhs = ret;
        }else{//lbw > rbw
            auto *ret = new TypedValue(
                builder.CreateFPExt((*rhs)->val, (*lhs)->getType()),
                deepCopyTypeNode((*lhs)->type.get())
            );
            *rhs = ret;
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
Type* typeTagToLlvmType(TypeTag ty, LLVMContext &ctxt, string typeName){
    switch(ty){
        case TT_I8:  case TT_U8:  return Type::getInt8Ty(ctxt);
        case TT_I16: case TT_U16: return Type::getInt16Ty(ctxt);
        case TT_I32: case TT_U32: return Type::getInt32Ty(ctxt);
        case TT_I64: case TT_U64: return Type::getInt64Ty(ctxt);
        case TT_Isz:    return Type::getVoidTy(ctxt); //TODO: implement
        case TT_Usz:    return Type::getVoidTy(ctxt); //TODO: implement
        case TT_F16:    return Type::getHalfTy(ctxt);
        case TT_F32:    return Type::getFloatTy(ctxt);
        case TT_F64:    return Type::getDoubleTy(ctxt);
        case TT_C8:     return Type::getInt8Ty(ctxt);
        case TT_C32:    return Type::getInt32Ty(ctxt);
        case TT_Bool:   return Type::getInt1Ty(ctxt);
        case TT_Void:   return Type::getVoidTy(ctxt);
        case TT_TypeVar:
            cerr << "WARNING: typevars should not be converted to llvm types!\n";
            return nullptr;
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
Type* Compiler::typeNodeToLlvmType(TypeNode *tyNode){
    vector<Type*> tys;
    TypeNode *tyn = tyNode->extTy.get();
    DataType *userType;

    switch(tyNode->type){
        case TT_Ptr:
            return tyn->type != TT_Void ?
                PointerType::get(typeNodeToLlvmType(tyn), 0)
                : Type::getInt8Ty(ctxt)->getPointerTo();
        case TT_Array:{
            auto *intlit = (IntLitNode*)tyn->next.get();
            return ArrayType::get(typeNodeToLlvmType(tyn), stoi(intlit->val));
        }
        case TT_Tuple:
            while(tyn){
                tys.push_back(typeNodeToLlvmType(tyn));
                tyn = (TypeNode*)tyn->next.get();
            }
            return StructType::get(ctxt, tys);
        case TT_Data:
            userType = lookupType(tyNode->typeName);
            if(!userType)
                return (Type*)compErr("Use of undeclared type " + tyNode->typeName, tyNode->loc);

            //((StructType*)userType->tyn)->setName(tyNode->typeName);
            return typeNodeToLlvmType(userType->tyn.get());
        case TT_Function: case TT_MetaFunction: {
            //ret ty is tyn from above
            //
            //get param tys
            TypeNode *cur = (TypeNode*)tyn->next.get();
            while(cur){
                tys.push_back(typeNodeToLlvmType(cur));
                cur = (TypeNode*)cur->next.get();
            }

            return FunctionType::get(typeNodeToLlvmType(tyn), tys, false)->getPointerTo();
        }
        case TT_TaggedUnion:
            userType = lookupType(tyNode->typeName);
            if(!userType)
                return (Type*)compErr("Use of undeclared type " + tyNode->typeName, tyNode->loc);

            tyn = userType->tyn->extTy.get();
            while(tyn){
                tys.push_back(typeNodeToLlvmType(tyn));
                tyn = (TypeNode*)tyn->next.get();
            }
            return StructType::get(ctxt, tys);
        default:
            return typeTagToLlvmType(tyNode->type, ctxt);
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

        if(lty->isVoidTy() || rty->isVoidTy()) return true;

        return llvmTypeEq(lty, rty);
    }else if(ltt == TT_Array){
        return l->getArrayElementType() == r->getArrayElementType() &&
               l->getArrayNumElements() == r->getArrayNumElements();
    }else if(ltt == TT_Function || ltt == TT_MetaFunction){
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

    if(this->type == TT_TypeVar or r.type == TT_TypeVar)
        return true;

    if(this->type != r.type)
        return false;

    if(r.type == TT_Ptr){
        if(extTy->type == TT_Void || r.extTy->type == TT_Void)
            return true;

        return *this->extTy.get() == *r.extTy.get();
    }else if(r.type == TT_Array){
        //size of an array is part of its type and stored in 2nd extTy
        auto lsz = std::stoi( ((IntLitNode*)extTy->next.get())->val );
        auto rsz = std::stoi( ((IntLitNode*)r.extTy->next.get())->val );

        return lsz == rsz and *extTy == *r.extTy;
    }else if(r.type == TT_Data or r.type == TT_TaggedUnion){
        return typeName == r.typeName;
    }else if(r.type == TT_Function or r.type == TT_MetaFunction or r.type == TT_Method or r.type == TT_Tuple){
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

        /* 
         * Because of the loss of specificity for these last four types, 
         * these strings are most likely insufficient.  The llvm::Type
         * should instead be printed for these types
         */
        case TT_Tuple:        return "Tuple";
        case TT_Array:        return "Array";
        case TT_Ptr:          return "Ptr"  ;
        case TT_Data:         return "Data" ;
        case TT_TypeVar:      return "'t";
        case TT_Function:     return "Function";
        case TT_MetaFunction: return "Meta Function";
        case TT_Method:       return "Method";
        case TT_TaggedUnion:  return "|";
        default:              return "(Unknown TypeTag " + to_string(ty) + ")";
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
        return string(t->typeName.c_str()); //make a copy of the typename
    }else if(t->type == TT_Array){
        auto *len = (IntLitNode*)t->extTy->next.get();
        return '[' + len->val + " " + typeNodeToStr(t->extTy.get()) + ']';
    }else if(t->type == TT_Ptr){
        return typeNodeToStr(t->extTy.get()) + "*";
    }else if(t->type == TT_Function || t->type == TT_MetaFunction || t->type == TT_Method){
        string ret = "(";
        string retTy = typeNodeToStr(t->extTy.get());
        TypeNode *cur = (TypeNode*)t->extTy->next.get();
        while(cur){
            ret += typeNodeToStr(cur);
            cur = (TypeNode*)cur->next.get();
            if(cur) ret += ",";
        }
        return ret + ")->" + retTy;
    }else if(t->type == TT_TypeVar){
        return "'" + t->typeName;
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
