#include <parser.h>


char Compiler::getBitWidthOfTokTy(int tokTy){
    switch(tokTy){
        case Tok_I8: case Tok_U8: return 8;
        case Tok_I16: case Tok_U16: case Tok_F16: return 16;
        case Tok_I32: case Tok_U32: case Tok_F32: return 32;
        case Tok_I64: case Tok_U64: case Tok_F64: return 64;
        case Tok_Isz: case Tok_Usz: return 64; //TODO: detect 32-bit platform
        case Tok_Bool: return 1;
        default: return 0;
    }
}


/*
 *  Returns the type of a node in an expression.  Node must be
 *  valid in an expression context, ie no statement-only nodes.
 */
Type* VarNode::getType(Compiler *c){
    if(TypedValue *val = c->lookup(name)){
        return val->val->getType();
    }
    return (Type*)c->compErr("Use of undeclared variable " + name + " in expression", row, col);
}

Type* StrLitNode::getType(Compiler *c){
    return Type::getInt8PtrTy(getGlobalContext()); 
}

Type* IntLitNode::getType(Compiler *c){
    return Compiler::tokTypeToLlvmType(type, ""); 
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

Type* BinOpNode::getType(Compiler *c){
    return lval->getType(c);
}


/*
 *  Assures two IntegerType'd Values have the same bitwidth.
 *  If not, one is casted to the larger bitwidth and mutated.
 *  Assumes the Type of both values to be IntegerType.
 *  lSigned and rSigned are set to true if the left and right
 *  Values are both signed, this determines if they are zero
 *  extended or sign extended.
 */
void Compiler::checkIntSize(TypedValue **lhs, TypedValue **rhs){
    int lbw = getBitWidthOfTokTy((*lhs)->type);
    int rbw = getBitWidthOfTokTy((*rhs)->type);

    if(lbw != rbw){
        //Cast the value with the smaller bitwidth to the type with the larger bitwidth
        if(lbw < rbw){
            (*lhs)->val = builder.CreateIntCast((*lhs)->val, (*rhs)->val->getType(), !isUnsignedTokTy((*lhs)->type));
            (*lhs)->type = (*rhs)->type;
        }else{//lbw > rbw
            (*rhs)->val = builder.CreateIntCast((*rhs)->val, (*lhs)->val->getType(), !isUnsignedTokTy((*rhs)->type));
            (*rhs)->type = (*lhs)->type;
        }
    }
}

Type* Node::getType(Compiler *c){
    return (Type*)c->compErr("Void type used in expression", row, col);
}

/*
 *  Translates an individual type in token form to an llvm::Type
 */
Type* Compiler::tokTypeToLlvmType(int tokTy, string typeName = ""){
    switch(tokTy){
        case Tok_UserType: //TODO: implement
            return Type::getVoidTy(getGlobalContext());
        case Tok_I8:  case Tok_U8:  return Type::getInt8Ty(getGlobalContext());
        case Tok_I16: case Tok_U16: return Type::getInt16Ty(getGlobalContext());
        case Tok_I32: case Tok_U32: return Type::getInt32Ty(getGlobalContext());
        case Tok_I64: case Tok_U64: return Type::getInt64Ty(getGlobalContext());
        case Tok_Isz:    return Type::getVoidTy(getGlobalContext()); //TODO: implement
        case Tok_Usz:    return Type::getVoidTy(getGlobalContext()); //TODO: implement
        case Tok_F16:    return Type::getHalfTy(getGlobalContext());
        case Tok_F32:    return Type::getFloatTy(getGlobalContext());
        case Tok_F64:    return Type::getDoubleTy(getGlobalContext());
        case Tok_C8:     return Type::getInt8Ty(getGlobalContext()); //TODO: implement
        case Tok_C32:    return Type::getInt32Ty(getGlobalContext()); //TODO: implement
        case Tok_Bool:   return Type::getInt1Ty(getGlobalContext());
        case Tok_StrLit: return Type::getInt8PtrTy(getGlobalContext());
        case Tok_Void:   return Type::getVoidTy(getGlobalContext());
    }
    return nullptr;
}

/*
 *  Translates a llvm::Type to a TokenType
 *  Not intended for in-depth analysis as it loses
 *  specificity, specifically it loses data about the type,
 *  and name of UserData.  As such, this should mainly be
 *  used for comparing primitive datatypes, or just to detect
 *  if something is a UserType.
 */
int Compiler::llvmTypeToTokType(Type *t){
    if(t->isIntegerTy(8)) return Tok_I8;
    if(t->isIntegerTy(16)) return Tok_I16;
    if(t->isIntegerTy(32)) return Tok_I32;
    if(t->isIntegerTy(64)) return Tok_I64;
    if(t->isHalfTy()) return Tok_F16;
    if(t->isFloatTy()) return Tok_F32;
    if(t->isDoubleTy()) return Tok_F64;
    
    if(t->isArrayTy()) return '[';
    if(t->isStructTy()) return Tok_Data;
    if(t->isPointerTy()) return '*';
    if(t->isFunctionTy()) return '(';

    return Tok_Void;
}

