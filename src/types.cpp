#include <parser.h>


char Compiler::getBitWidthOfTypeTag(TypeTag ty){
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
 *  Returns the type of a node in an expression.  Node must be
 *  valid in an expression context, ie no statement-only nodes.
 */
Type* VarNode::getType(Compiler *c){
    if(Variable *var = c->lookup(name)){
        return var->getVal()->getType();
    }
    return (Type*)c->compErr("Use of undeclared variable " + name + " in expression", row, col);
}

Type* RefVarNode::getType(Compiler *c){
    if(Variable *var = c->lookup(name)){
        return var->getVal()->getType();
    }
    return (Type*)c->compErr("Use of undeclared variable " + name + " in expression", row, col);
}

Type* StrLitNode::getType(Compiler *c){
    return Type::getInt8PtrTy(getGlobalContext()); 
}

Type* IntLitNode::getType(Compiler *c){
    return typeTagToLlvmType(type, ""); 
}

//TODO: give floats a type field like integers
Type* FltLitNode::getType(Compiler *c){
    return Type::getDoubleTy(getGlobalContext());
}

Type* BoolLitNode::getType(Compiler *c){
    return Type::getInt1Ty(getGlobalContext());
}

Type* FuncCallNode::getType(Compiler *c){
    if(auto* fn = c->module->getFunction(name)){
        return fn->getReturnType();
    }
    return (Type*)c->compErr("Undeclared function " + name + " called", row, col);
}

Type* ArrayNode::getType(Compiler *c){
    if(exprs.size() > 0){
        Type *elemTy = exprs[0]->getType(c);

        //check each element's type against the first
        for(size_t i = 1; i < exprs.size(); i++){
            if(!llvmTypeEq(elemTy, exprs[i]->getType(c))){
                return (Type*)c->compErr("Array index " + to_string(i) + " does not match the other array element's types", row, col);
            }
        }
        return ArrayType::get(elemTy, exprs.size());
    }
    return ArrayType::get(Type::getVoidTy(getGlobalContext()), 0);
}

Type* TupleNode::getType(Compiler *c){
    if(exprs.size() > 0){
        vector<Type*> elemTys;

        for(Node *n : exprs){
            elemTys.push_back(n->getType(c));
        }
        return StructType::get(getGlobalContext(), elemTys, "Tuple");
    }
    //return empty struct
    return StructType::get(getGlobalContext());
}

Type* BinOpNode::getType(Compiler *c){
    return lval->getType(c);
}

Type* UnOpNode::getType(Compiler *c){
    Type* rty = rval->getType(c);
    int tokTy;
    switch(op){
        case '*':
            tokTy = llvmTypeToTypeTag(rty);
            if(tokTy != '*')
                return (Type*)c->compErr("Cannot dereference non-pointer type " + Lexer::getTokStr(tokTy), this->row, this->col);
            else
                return rty->getPointerElementType();
        case '&': 
            return PointerType::get(rty, 0);
    }
    return rty;
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

Type* Node::getType(Compiler *c){
    return (Type*)c->compErr("Unable to discern type of generic Node.", row, col);
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
            return StructType::get(getGlobalContext(), tys, "Tuple");
        case TT_Array: //TODO array type
        case TT_Data:
        case TT_Func: //TODO function pointer type
            cout << "typeNodeToLlvmType: Array types, and function pointer types are currently unimplemented.  A void type will be returned instead.\n";
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
        return l == r;
    }else{ //primitive type
        return ltt == rtt;
    }
}
