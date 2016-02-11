#include <parser.h>

/*
 *  Returns the type of a node in an expression.  Node must be
 *  valid in an expression context, ie no statement-only nodes.
 */
Type* Compiler::getNodeType(VarNode *v){
    if(Value *val = lookup(v->name)){
        return val->getType();
    }
    return (Type*)compErr("Use of undeclared variable " + v->name + " in expression", v->row, v->col);
}

Type* Compiler::getNodeType(StrLitNode *v){
    return Type::getInt8PtrTy(getGlobalContext()); 
}

Type* Compiler::getNodeType(IntLitNode *v){
    return translateType(v->type, ""); 
}

Type* Compiler::getNodeType(FuncCallNode *v){
    if(auto* fn = module->getFunction(v->name)){
        return fn->getReturnType();
    }
    return (Type*)compErr("Undeclared function " + v->name + " called", v->row, v->col);
}

Type* Compiler::getNodeType(BinOpNode *v){
    return getNodeType(v->lval.get());
}

Type* Compiler::getNodeType(Node *n){
    return (Type*)compErr("Cannot get type of Node", n->row, n->col);
}

/*
 *  Translates an individual type in token form to an llvm::Type
 */
Type* Compiler::translateType(int tokTy, string typeName = ""){
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
