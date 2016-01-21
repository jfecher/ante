#include "compiler.h"
#include "parser.h"

using namespace llvm;


/*
 *  Inform the user of an error and return nullptr.
 *  (perhaps this should throw an exception?)
 */
template<typename T>
Value* Compiler::compErr(T msg)
{
    cout << msg << endl;
    errFlag = true;
    return nullptr;
}

template<typename T, typename... Args>
Value* Compiler::compErr(T msg, Args... args)
{
    cout << msg;
    return compErr(args...);
}

/*
 *  Translates an individual type in token form to an llvm::Type
 */
Type* translateType(int tokTy, string typeName = "")
{
    switch(tokTy){
        case Tok_UserType: //TODO: implement
            return Type::getVoidTy(getGlobalContext());
        case Tok_I8:  case Tok_U8:  return Type::getInt8Ty(getGlobalContext());
        case Tok_I16: case Tok_U16: return Type::getInt16Ty(getGlobalContext());
        case Tok_I32: case Tok_U32: return Type::getInt32Ty(getGlobalContext());
        case Tok_I64: case Tok_U64: return Type::getInt64Ty(getGlobalContext());
        case Tok_Isz: return Type::getVoidTy(getGlobalContext()); //TODO: implement
        case Tok_Usz: return Type::getVoidTy(getGlobalContext()); //TODO: implement
        case Tok_F32: return Type::getFloatTy(getGlobalContext());
        case Tok_F64: return Type::getDoubleTy(getGlobalContext());
        case Tok_C8:  return Type::getInt8Ty(getGlobalContext()); //TODO: implement
        case Tok_C32: return Type::getInt32Ty(getGlobalContext()); //TODO: implement
        case Tok_Bool:return Type::getInt1Ty(getGlobalContext());
        case Tok_Void:return Type::getVoidTy(getGlobalContext());
    }
    return nullptr;
}

/*
 *  Returns amount of values in a tuple, from 0 to max uint.
 *  Does not assume argument is a tuple.
 */
size_t getTupleSize(Node *tup)
{
    return 1;
}

void compileStmtList(Node *nList, Compiler *c, Module *m)
{
    while(nList){
        nList->compile(c, m);
        nList = nList->next.get();
    }
}

Value* IntLitNode::compile(Compiler *c, Module *m)
{   //TODO: unsigned int with APUInt
    return ConstantInt::get(Type::getInt32Ty(getGlobalContext()), val, 10);
}

Value* FltLitNode::compile(Compiler *c, Module *m)
{
    return ConstantFP::get(getGlobalContext(), APFloat(APFloat::IEEEdouble, val.c_str()));
}

Value* BoolLitNode::compile(Compiler *c, Module *m)
{
    return ConstantInt::get(getGlobalContext(), APInt(1, (bool)val, true));
}

Value* TypeNode::compile(Compiler *c, Module *m)
{ return nullptr; }

Value* StrLitNode::compile(Compiler *c, Module *m)
{
    return c->builder.CreateGlobalStringPtr(val);
}

/*
 *  Compiles an operation along with its lhs and rhs
 *
 *  TODO: type checking
 *  TODO: CreateExactUDiv for when it is known there is no remainder
 *  TODO: CreateFcmpOEQ vs CreateFCmpUEQ
 */
Value* BinOpNode::compile(Compiler *c, Module *m)
{
    Value *lhs = lval->compile(c, m);
    Value *rhs = rval->compile(c, m);

    switch(op){
        case '+': return c->builder.CreateAdd(lhs, rhs, "iAddTmp");
        case '-': return c->builder.CreateSub(lhs, rhs, "iSubTmp");
        case '*': return c->builder.CreateMul(lhs, rhs, "iMulTmp");
        case '/': return c->builder.CreateSDiv(lhs, rhs, "iDivTmp");
        case '%': return c->builder.CreateSRem(lhs, rhs, "iModTmp");
        case '<': return c->builder.CreateICmpULT(lhs, rhs, "iLtTmp");
        case '>': return c->builder.CreateICmpUGT(lhs, rhs, "iGtTmp");
        case '^': return c->builder.CreateXor(lhs, rhs, "xorTmp");
        case '.': break;
        case Tok_Eq: return c->builder.CreateICmpEQ(lhs, rhs, "iCmpEqTmp");
        case Tok_NotEq: return c->builder.CreateICmpNE(lhs, rhs, "iCmpNeTmp");
        case Tok_LesrEq: return c->builder.CreateICmpULE(lhs, rhs, "iLeTmp");
        case Tok_GrtrEq: return c->builder.CreateICmpUGE(lhs, rhs, "iGeTmp");
        case Tok_Or: break;
        case Tok_And: break;
    }

    return c->compErr("Unknown operator ", lexer::getTokStr(op));
}

Value* RetNode::compile(Compiler *c, Module *m)
{
    return c->builder.CreateRet(expr->compile(c, m));
}

Value* IfNode::compile(Compiler *c, Module *m)
{
    //cond should always evaluate to a bool explicitely
    Value* cond = condition->compile(c, m);
    if(!cond) return nullptr;

    Function *f = c->builder.GetInsertBlock()->getParent();
    
    //Create thenbb and forward declare the others but dont inser them
    //into function f just yet.
    BasicBlock *thenbb = BasicBlock::Create(getGlobalContext(), "then", f);
    //BasicBlock *elsebb = BasicBlock::Create(getGlobalContext(), "else");
    BasicBlock *mergbb = BasicBlock::Create(getGlobalContext(), "endif");

    c->builder.CreateCondBr(cond, thenbb, mergbb);

    //Compile the if statement's then body
    c->builder.SetInsertPoint(thenbb);
    
    //Compile the then block
    compileStmtList(child.get(), c, m);

    c->builder.CreateBr(mergbb);

    //create unconditional merge
    //c->builder.CreateBr(mergbb);

    //then block must be updated in case it is changed by nested blocks.
    thenbb = c->builder.GetInsertBlock();

    //add the floating else to the function
    //f->getBasicBlockList().push_back(elsebb);
    
    f->getBasicBlockList().push_back(mergbb);
    c->builder.SetInsertPoint(mergbb);
    return f;
}

Value* NamedValNode::compile(Compiler *c, Module *m)
{ return nullptr; }

/*
 *  Loads a variable from the stack
 */
Value* VarNode::compile(Compiler *c, Module *m)
{
    Value *val = c->lookup(name);
    if(!val){
        return c->compErr("Variable ", name, " has not been declared.");
    }
    return val;
}

Value* FuncCallNode::compile(Compiler *c, Module *m)
{
    Function *f = m->getFunction(name);
    if(!f){
        return c->compErr("Called function ", name, " has not been declared.");
    }

    size_t paramSize = getTupleSize(params.get());
    if(f->arg_size() != paramSize){
        if(paramSize == 1)
            return c->compErr("Called function ", name, " was given 1 paramter but was declared to take ", f->arg_size());
        else
            return c->compErr("Called function ", name, " was given ", paramSize, " paramters but was declared to take ", f->arg_size());
    }

    std::vector<Value*> args;
    for(unsigned i = 0; i < paramSize; i++){
        args.push_back(params->compile(c, m));
        if(!args.back())
            c->compErr("Argument ", i, " of called function ", name, " evaluated to null.");
    }

    return c->builder.CreateCall(f, args, "callTmp");
}

Value* VarDeclNode::compile(Compiler *c, Module *m)
{ return nullptr; }

Value* VarAssignNode::compile(Compiler *c, Module *m)
{ return nullptr; }


Value* FuncDeclNode::compile(Compiler *c, Module *m)
{
    //Get and translate the function's return type to an llvm::Type*
    TypeNode *retNode = (TypeNode*)type.get();
    Type *retType = translateType(retNode->type, retNode->typeName);

    //Get and translate the function's parameter's type(s) to an llvm::Type*
    NamedValNode *param = params.get();
    TypeNode *paramTyNode = (TypeNode*)param->typeExpr.get();
    Type *paramsType = translateType(paramTyNode->type, paramTyNode->typeName);

    //Get the corresponding function type for the above return type, parameter types,
    //with no varargs
    FunctionType *ft = FunctionType::get(retType, paramsType, false);
    
    //Actually create the function in module m
    Function *f = Function::Create(ft, Function::ExternalLinkage, name, m);

    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", f);
    c->builder.SetInsertPoint(bb);

    //tell the compiler to create a new scope on the stack.
    c->enterNewScope();

    //iterate through each parameter and add its value to the new scope.
    for(auto &arg : f->args()){
        c->stoVar(param->name, &arg);
        param = (NamedValNode*)param->next.get();
        if(!param) break;
    }

    compileStmtList(child.get(), c, m);

    //llvm requires explicit returns, so generate a void return even if
    //the user created a void function.
    if(retNode->type == Tok_Void){
        c->builder.CreateRetVoid();
    }
    c->exitScope();
    
    c->builder.SetInsertPoint(&c->module->getFunction("main")->back());

    verifyFunction(*f);
    return f;
}


Value* DataDeclNode::compile(Compiler *c, Module *m)
{ return nullptr; }


/*
void IntLitNode::exec(){}

void FltLitNode::exec(){}

void BoolLitNode::exec(){}

void TypeNode::exec(){}

void StrLitNode::exec(){}

void BinOpNode::exec(){}

void RetNode::exec(){}

void IfNode::exec(){}

void VarNode::exec(){}

void NamedValNode::exec(){}

void FuncCallNode::exec(){}

void VarDeclNode::exec(){}

void VarAssignNode::exec(){}

void FuncDeclNode::exec(){}

void DataDeclNode::exec(){}
*/

/*
 *  Compiles function definitions found
 *  in every program.
 */
void Compiler::compilePrelude()
{
    FunctionType *i8pRetVoidVarargsTy = FunctionType::get(Type::getVoidTy(getGlobalContext()), Type::getInt8PtrTy(getGlobalContext()), true);
    Function::Create(i8pRetVoidVarargsTy, Function::ExternalLinkage, "printf", module.get());

    FunctionType *i32RetI8pTy = FunctionType::get(Type::getInt8PtrTy(getGlobalContext()), Type::getInt32Ty(getGlobalContext()), false);
    Function::Create(i32RetI8pTy, Function::ExternalLinkage, "itoa", module.get());

    FunctionType *i8pRetI32Ty = FunctionType::get(Type::getInt32Ty(getGlobalContext()), Type::getInt8PtrTy(getGlobalContext()), false);
    Function::Create(i8pRetI32Ty, Function::ExternalLinkage, "puts", module.get());

    FunctionType *i32RetI32Ty = FunctionType::get(Type::getInt32Ty(getGlobalContext()), Type::getInt32Ty(getGlobalContext()), false);
    Function::Create(i32RetI32Ty, Function::ExternalLinkage, "putchar", module.get());
}

void Compiler::compile()
{
    compilePrelude();

    //get or create the function type for the main method: void()
    FunctionType *ft = FunctionType::get(Type::getInt8Ty(getGlobalContext()), false);
    
    //Actually create the function in module m
    Function *f = Function::Create(ft, Function::ExternalLinkage, "main", module.get());
   
    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", f);
    builder.SetInsertPoint(bb);

    compileStmtList(ast.get(), this, module.get());
    builder.CreateRet(ConstantInt::get(getGlobalContext(), APInt(8, 0, true)));

    verifyFunction(*f);
    module->dump();

    if(errFlag){
        puts("Compilation aborted.");
        return;
    }
}

inline void Compiler::enterNewScope()
{
    varTable.push(map<string, Value*>());
}

inline void Compiler::exitScope()
{
    varTable.pop();
}

inline Value* Compiler::lookup(string var)
{
    return varTable.top()[var];
}

inline void Compiler::stoVar(string var, Value *val)
{
    varTable.top()[var] = val;
}

/*
 *  Allocates a value on the stack at the entry to a block
 */
/*static AllocaInst* createBlockAlloca(Function *f, string var, Type *varType)
{
    IRBuilder<> builder{&f->getEntryBlock(), f->getEntryBlock().begin()};
    return builder.CreateAlloca(varType, 0, var);
}*/
