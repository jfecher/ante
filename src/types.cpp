#include <parser.h>

/*
 *  Returns the type of a node in an expression.  Node must be
 *  valid in an expression context, ie no statement-only nodes.
 */
Type* VarNode::getType(Compiler *c){
    if(Value *val = c->lookup(name)){
        cout << "Var type = " << val->getType()->isIntegerTy() << endl;
        return val->getType();
    }
    return (Type*)c->compErr("Use of undeclared variable " + name + " in expression", row, col);
}

Type* StrLitNode::getType(Compiler *c){
    return Type::getInt8PtrTy(getGlobalContext()); 
}

Type* IntLitNode::getType(Compiler *c){
    cout << "Type: " << Lexer::getTokStr(type) << endl;
    return Compiler::translateType(type, ""); 
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

Type* Node::getType(Compiler *c){
    return (Type*)c->compErr("Void type used in expression", row, col);
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
