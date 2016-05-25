#include "parser.h"
#include "compiler.h"
#include <llvm/IR/Verifier.h>          //for verifying basic structure of functions
#include <llvm/Bitcode/ReaderWriter.h> //for r/w when outputting bitcode
#include <llvm/Support/FileSystem.h>   //for r/w when outputting bitcode
#include <llvm/Support/raw_ostream.h>  //for ostream when outputting bitcode
#include "llvm/Transforms/Scalar.h"    //for most passes
#include "llvm/Support/TargetRegistry.h"
#include "llvm/Target/TargetMachine.h"
#include "llvm/Linker/Linker.h"

using namespace llvm;


/* 
 * Skips input in a given istream until it encounters the given coordinates,
 * with each newline signalling the end of a row.
 *
 * precondition: coordinates must be valid
 */
void skipToCoords(istream& ifs, unsigned int row, unsigned int col){
    unsigned int line = 1;
    if(line != row){
        while(true){
            char c = ifs.get();
            if(c == '\n'){
                line++;
                if(line >= row){
                    c = 0;
                    break;
                }
            }else if(c == EOF){
                break;
            }
        }
    }
}

/*
 *  Prints a given line (row) of a file, along with an arrow pointing to
 *  the specified column.
 */
void printErrLine(const char* fileName, unsigned int row, unsigned int col){
    ifstream f{fileName};

    //Premature newline error, show previous line as error instead
    if(col == 0) row--;

    //skip to line in question
    skipToCoords(f, row, col);

    //print line
    string s;
    getline(f, s);
    if(col == 0) col = s.length() + 1;
    cout << s;

    //draw arrow
    putchar('\n');
    cout << "\033[;31m"; //red
    for(unsigned int i = 1; i <= col; i++){
        if(i < col) putchar(' ');
        else putchar('^');
    }
    cout << "\033[;m"; //reset color
}


void ante::error(const char* msg, const char* fileName, unsigned int row, unsigned int col){
    cout << "\033[;3m" << fileName << "\033[;m: ";
    cout << "\033[;1m" << row << "," << col << "\033[;0m";
    cout << ": " <<  msg << endl;
    printErrLine(fileName, row, col);
    cout << endl << endl;
}


/*
 *  Inform the user of an error and return nullptr.
 *  (perhaps this should throw an exception?)
 */
TypedValue* Compiler::compErr(string msg, unsigned int row, unsigned int col){
    error(msg.c_str(), fileName.c_str(), row, col);
    errFlag = true;
    return nullptr;
}


/*
 *  Returns amount of values in a tuple, from 0 to max uint.
 *  Assumes argument is a tuple.
 *  
 *  Doubles as a getNodesInBlock function, but does not
 *  count child nodes.
 */
size_t Compiler::getTupleSize(Node *tup){
    size_t size = 0;
    while(tup){
        tup = tup->next.get();
        size++;
    }
    return size;
}


/*
 *  Compiles a statement list and returns its last statement.
 */
TypedValue* compileStmtList(Node *nList, Compiler *c){
    TypedValue *ret = nullptr;
    while(nList){
        ret = nList->compile(c);
        nList = nList->next.get();
    }
    return ret;
}


bool isUnsignedTypeTag(const TypeTag tt){
    return tt==TT_U8||tt==TT_U16||tt==TT_U32||tt==TT_U64||tt==TT_Usz;
}


TypedValue* IntLitNode::compile(Compiler *c){
    return new TypedValue(ConstantInt::get(getGlobalContext(),
                            APInt(getBitWidthOfTypeTag(type), 
                            atol(val.c_str()), isUnsignedTypeTag(type))), type);
}


const fltSemantics& typeTagToFltSemantics(TypeTag tokTy){
    switch(tokTy){
        case TT_F16: return APFloat::IEEEhalf;
        case TT_F32: return APFloat::IEEEsingle;
        case TT_F64: return APFloat::IEEEdouble;
        default:     return APFloat::IEEEdouble;
    }
}

/*
 *  TODO: type field for float literals
 */
TypedValue* FltLitNode::compile(Compiler *c){
    return new TypedValue(ConstantFP::get(getGlobalContext(), APFloat(typeTagToFltSemantics(type), val.c_str())), type);
}


TypedValue* BoolLitNode::compile(Compiler *c){
    return new TypedValue(ConstantInt::get(getGlobalContext(), APInt(1, (bool)val, true)), TT_Bool);
}


TypedValue* ModNode::compile(Compiler *c){
    return nullptr;
}


TypedValue* TypeNode::compile(Compiler *c){
    return nullptr;
}


TypedValue* StrLitNode::compile(Compiler *c){
    return new TypedValue(c->builder.CreateGlobalStringPtr(val), TT_StrLit);
}


TypedValue* ArrayNode::compile(Compiler *c){
    vector<Constant*> arr;
    for(Node *n : exprs){
       auto *tval = n->compile(c);
       arr.push_back((Constant*)tval->val);
    }
   
    auto* arrTy = ArrayType::get(arr[0]->getType(), arr.size());
    return new TypedValue(ConstantArray::get(arrTy, arr), TT_Array);
}


TypedValue* TupleNode::compile(Compiler *c){
    vector<Constant*> elems;
    elems.reserve(exprs.size());

    vector<Type*> elemTys;
    elemTys.reserve(exprs.size());

    map<unsigned, Value*> pathogenVals;

    //Compile every value in the tuple, and if it is not constant,
    //add it to pathogenVals
    for(unsigned i = 0; i < exprs.size(); i++){
        auto *tval = exprs[i]->compile(c);
        if(dynamic_cast<Constant*>(tval->val)){
            elems.push_back((Constant*)tval->val);
        }else{
            pathogenVals[i] = tval->val;
            elems.push_back(UndefValue::get(tval->getType()));
        }
        elemTys.push_back(tval->getType());
    }

    //Create the constant tuple with undef values in place for the non-constant values
    Value* tuple = ConstantStruct::get(StructType::get(getGlobalContext(), elemTys), elems);
    
    //Insert each pathogen value into the tuple individually
    for(auto it = pathogenVals.cbegin(); it != pathogenVals.cend(); it++){
        tuple = c->builder.CreateInsertValue(tuple, it->second, it->first);
    }

    return new TypedValue(tuple, TT_Tuple);
}


vector<Value*> TupleNode::unpack(Compiler *c){
    vector<Value*> ret;
    for(Node *n : exprs){
        auto *tval = n->compile(c);
        if(tval)
            ret.push_back(tval->val);
        else
            ret.push_back(nullptr); //compile error
    }
    return ret;
}


/*
 *  When a retnode is compiled within a block, care must be taken to not
 *  forcibly insert the branch instruction afterwards as it leads to dead code.
 */
TypedValue* RetNode::compile(Compiler *c){
    TypedValue *ret = expr->compile(c);
    
    Function *f = c->builder.GetInsertBlock()->getParent();

    if(!llvmTypeEq(ret->getType(), f->getReturnType())){
        return c->compErr("return expression of type " + llvmTypeToStr(ret->getType()) +
               " does not match function return type " + llvmTypeToStr(f->getReturnType()), 
               this->row, this->col);
    }

    return new TypedValue(c->builder.CreateRet(ret->val), ret->type);
}


TypedValue* ImportNode::compile(Compiler *c){
    if(!dynamic_cast<StrLitNode*>(expr.get())) return 0;

    c->importFile(((StrLitNode*)expr.get())->val.c_str());

    return 0;
}


void compileIfNodeHelper(IfNode *ifN, BasicBlock *mergebb, Function *f, Compiler *c){
    BasicBlock *thenbb = BasicBlock::Create(getGlobalContext(), "then", f);

    if(ifN->elseN.get()){
        TypedValue *cond = ifN->condition->compile(c);

        BasicBlock *elsebb = BasicBlock::Create(getGlobalContext(), "else");
        c->builder.CreateCondBr(cond->val, thenbb, elsebb);

        //Compile the if statement's then body
        c->builder.SetInsertPoint(thenbb);

        //Compile the then block
        TypedValue *v = compileStmtList(ifN->child.get(), c);
        
        //If the user did not return from the function themselves, then
        //merge to the endif.
        if(!dynamic_cast<ReturnInst*>(v->val)){
            c->builder.CreateBr(mergebb);
        }

        f->getBasicBlockList().push_back(elsebb);
        c->builder.SetInsertPoint(elsebb);

        //if elseN is else, and not elif, insert merge instruction.
        if(ifN->elseN->condition.get()){
            compileIfNodeHelper(ifN->elseN.get(), mergebb, f, c);
        }else{
            //compile the else node's body directly
            TypedValue *elseBody = ifN->elseN->child->compile(c);
            if(!dynamic_cast<ReturnInst*>(elseBody->val)){
                c->builder.CreateBr(mergebb);
            }
        }
    }else{ //this must be an if or elif node with no proceeding elif or else nodes.
        TypedValue *cond = ifN->condition->compile(c);
        c->builder.CreateCondBr(cond->val, thenbb, mergebb);
        c->builder.SetInsertPoint(thenbb);
        TypedValue *v = compileStmtList(ifN->child.get(), c);
        if(!dynamic_cast<ReturnInst*>(v->val)){
            c->builder.CreateBr(mergebb);
        }
    }
}


TypedValue* IfNode::compile(Compiler *c){
    //Create thenbb and forward declare the others but dont inser them
    //into function f just yet.
    BasicBlock *mergbb = BasicBlock::Create(getGlobalContext(), "endif");
    Function *f = c->builder.GetInsertBlock()->getParent();

    compileIfNodeHelper(this, mergbb, f, c);

    f->getBasicBlockList().push_back(mergbb);
    c->builder.SetInsertPoint(mergbb);
    return new TypedValue(f, TT_Void);
}


TypedValue* WhileNode::compile(Compiler *c){
    Function *f = c->builder.GetInsertBlock()->getParent();
    BasicBlock *begin = BasicBlock::Create(getGlobalContext(), "while", f);
    BasicBlock *end   = BasicBlock::Create(getGlobalContext(), "end_while", f);

    auto *cond = condition->compile(c);
    c->builder.CreateCondBr(cond->val, begin, end);

    c->enterNewScope();
    //f->getBasicBlockList().push_back(begin);
    c->builder.SetInsertPoint(begin);
    compileStmtList(child.get(), c); //compile the while loop's body

    auto *reCond = condition->compile(c);
    c->builder.CreateCondBr(reCond->val, begin, end);

    //exit scope before the end block is reached to make sure to free
    //allocated pointers after each iteration to avoid memory leaks.
    c->exitScope();

    //f->getBasicBlockList().push_back(end);

    c->builder.SetInsertPoint(end);

    return 0;
}


//Since parameters are managed in Compiler::compfn, this need not do anything
TypedValue* NamedValNode::compile(Compiler *c)
{ return nullptr; }


/*
 *  Loads a variable from the stack
 */
TypedValue* VarNode::compile(Compiler *c){
    auto *var = c->lookup(name);
    if(!var)
        return c->compErr("Variable " + name + " has not been declared.", this->row, this->col);

    return dynamic_cast<AllocaInst*>(var->getVal())? new TypedValue(c->builder.CreateLoad(var->getVal(), name), var->getType()) : var->tval;
}


TypedValue* RefVarNode::compile(Compiler *c){
    Variable *var = c->lookup(name);
    
    if(!var)
        return c->compErr("Variable " + name + " has not been declared.", this->row, this->col);

    if(!dynamic_cast<AllocaInst*>(var->getVal()))
        return c->compErr("Cannot assign to immutable variable " + name, this->row, this->col);

    return new TypedValue(var->getVal(), TT_Ptr);
}


TypedValue* FuncCallNode::compile(Compiler *c){
    Function *f = c->getFunction(name);
    if(!f)
        return c->compErr("Called function " + name + " has not been declared.", this->row, this->col);

    /* Check given argument count matches declared argument count. */
    if(f->arg_size() != params->exprs.size() && !f->isVarArg()){
        if(params->exprs.size() == 1)
            return c->compErr("Called function " + name + " was given 1 argument but was declared to take " + to_string(f->arg_size()), this->row, this->col);
        else
            return c->compErr("Called function " + name + " was given " + to_string(params->exprs.size()) + " arguments but was declared to take " + to_string(f->arg_size()), this->row, this->col);
    }

    /* unpack the tuple of arguments into a vector containing each value */
    auto args = params->unpack(c);
    int i = 0;
    for(auto &param : f->args()){//type check each parameter
        if(!args[i]) return 0; //compile error

        if(!llvmTypeEq(args[i++]->getType(), param.getType())){
            return c->compErr("Argument " + to_string(i) + " of function " + name + " is a(n) " + llvmTypeToStr(args[i-1]->getType())
                    + " but was declared to be a(n) " + llvmTypeToStr(param.getType()), this->row, this->col);
        }
    }

    return new TypedValue(c->builder.CreateCall(f, args), llvmTypeToTypeTag(f->getReturnType()));
}


TypedValue* LetBindingNode::compile(Compiler *c){
    /*if(c->lookup(name)){ //check for redeclaration
        return c->compErr("Variable " + name + " was redeclared.", row, col);
    }*/
    
    TypedValue *val = expr->compile(c);
    if(!val) return nullptr;

    TypeNode *tyNode;
    if((tyNode = (TypeNode*)typeExpr.get())){
        if(!llvmTypeEq(val->val->getType(), c->typeNodeToLlvmType(tyNode))){
            return c->compErr("Incompatible types in explicit binding.", row, col);
        }
    }

    bool nofree = val->type != TT_Ptr || dynamic_cast<Constant*>(val->val);
    c->stoVar(name, new Variable(name, val, c->getScope(), nofree));
    return val;
}

TypedValue* compVarDeclWithInferredType(VarDeclNode *node, Compiler *c){
    if(c->lookup(node->name)){ //check for redeclaration
        return c->compErr("Variable " + node->name + " was redeclared.", node->row, node->col);
    }
    
    TypedValue *val = node->expr->compile(c);
    if(!val) return nullptr;

    TypedValue *alloca = new TypedValue(c->builder.CreateAlloca(val->getType(), 0, node->name.c_str()), val->type);
    val = new TypedValue(c->builder.CreateStore(val->val, alloca->val), val->type);
    
    bool nofree = val->type != TT_Ptr || dynamic_cast<Constant*>(val->val);
    c->stoVar(node->name, new Variable(node->name, alloca, c->getScope(), nofree));
    return val;
}

TypedValue* VarDeclNode::compile(Compiler *c){
    if(c->lookup(name)){ //check for redeclaration
        return c->compErr("Variable " + name + " was redeclared.", row, col);
    }

    TypeNode *tyNode = (TypeNode*)typeExpr.get();
    if(!tyNode) return compVarDeclWithInferredType(this, c);

    Type *ty = c->typeNodeToLlvmType(tyNode);
    TypedValue *alloca = new TypedValue(c->builder.CreateAlloca(ty, 0, name.c_str()), tyNode->type);

    Variable *var = new Variable(name, alloca, c->getScope());
    c->stoVar(name, var);
    if(expr.get()){
        TypedValue *val = expr->compile(c);
        if(!val) return 0;
        var->noFree = var->getType() != TT_Ptr || dynamic_cast<Constant*>(val->val);
        
        //Make sure the assigned value matches the variable's type
        if(!llvmTypeEq(alloca->getType()->getPointerElementType(), val->getType())){
            return c->compErr("Cannot assign expression of type " + llvmTypeToStr(val->getType())
                        + " to a variable of type " + llvmTypeToStr(alloca->getType()->getPointerElementType()),
                        expr->row, expr->col);
        }

        return new TypedValue(c->builder.CreateStore(val->val, alloca->val), tyNode->type);
    }else{
        return alloca;
    }
}


TypedValue* VarAssignNode::compile(Compiler *c){
    //If this is an insert value (where the lval resembles var[index] = ...)
    //then this must be instead compiled with compInsert, otherwise the [ operator
    //would retrieve the value at the index instead of the reference for storage.
    if(dynamic_cast<BinOpNode*>(ref_expr))
        return c->compInsert((BinOpNode*)ref_expr, expr.get());

    //otherwise, this is just a normal assign to a variable
    TypedValue *v = ref_expr->compile(c);
    
    //compile the expression to store
    TypedValue *assignExpr = expr->compile(c);
    
    //Check for errors before continuing
    if(!v || !assignExpr) return 0;

    //lvalue must compile to a pointer for storage, usually an alloca value
    if(llvmTypeToTypeTag(v->getType()) != TT_Ptr){
        return c->compErr("Attempted assign without a memory address, with type "
                + llvmTypeToStr(v->getType()), ref_expr->row, ref_expr->col);
    }

    //and finally, make sure the assigned value matches the variable's type
    if(!llvmTypeEq(v->getType()->getPointerElementType(), assignExpr->getType())){
        return c->compErr("Cannot assign expression of type " + llvmTypeToStr(assignExpr->getType())
                    + " to a variable of type " + llvmTypeToStr(v->getType()->getPointerElementType()),
                    expr->row, expr->col);
    }
    
    //now actually create the store
    return new TypedValue(c->builder.CreateStore(expr->compile(c)->val, v->val), TT_Void);
}


/*
 * Translates a NamedValNode list to a vector
 * of the types it contains.  If the list contains
 * a varargs type (represented by the absence of a type)
 * then a nullptr is inserted for that parameter.
 */
vector<Type*> getParamTypes(Compiler *c, NamedValNode *nvn, size_t paramCount){
    vector<Type*> paramTys;
    paramTys.reserve(paramCount);

    for(size_t i = 0; i < paramCount && nvn; i++){

        TypeNode *paramTyNode = (TypeNode*)nvn->typeExpr.get();
        if(paramTyNode)
            paramTys.push_back(c->typeNodeToLlvmType(paramTyNode));
        else
            paramTys.push_back(nullptr); //terminating null = varargs function
        nvn = (NamedValNode*)nvn->next.get();
    }
    return paramTys;
}


TypedValue* Compiler::compLetBindingFn(FuncDeclNode *fdn, size_t nParams, vector<Type*> &paramTys, Type *retTy = 0){
    FunctionType *ft;
    
    if(retTy)
        ft = FunctionType::get(retTy, paramTys, fdn->varargs);
    else
        ft = FunctionType::get(Type::getVoidTy(getGlobalContext()), paramTys, fdn->varargs);

    Function *f = Function::Create(ft, Function::ExternalLinkage, fdn->name, module.get());
    
    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", f);
    builder.SetInsertPoint(bb);

    //tell the compiler to create a new scope on the stack.
    enterNewScope();

    //iterate through each parameter and add its value to the new scope.
    NamedValNode *cParam = fdn->params.get();
    for(auto &arg : f->args()){
        TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();
        stoVar(cParam->name, new Variable(cParam->name, new TypedValue(&arg, paramTyNode->type), scope));
        if(!(cParam = (NamedValNode*)cParam->next.get())) break;
    }

    //actually compile the function, and hold onto the last value
    TypedValue *v = fdn->child->compile(this);
    //End of the function, discard the function's scope.
    exitScope();
    return v;
}


Function* Compiler::compFn(FuncDeclNode *fdn){
    //Get and translate the function's return type to an llvm::Type*
    TypeNode *retNode = (TypeNode*)fdn->type.get();

    //Count the number of parameters
    NamedValNode *paramsBegin = fdn->params.get();
    size_t nParams = getTupleSize(paramsBegin);

    vector<Type*> paramTys = getParamTypes(this, paramsBegin, nParams);

    if(paramTys.size() > 0 && !paramTys.back()){ //varargs fn
        fdn->varargs = true;
        paramTys.pop_back();
    }

    Type *retType = typeNodeToLlvmType(retNode);
    //Get the corresponding function type for the above return type, parameter types,
    //with no varargs
    FunctionType *ft = FunctionType::get(retType, paramTys, fdn->varargs);
    Function *f = Function::Create(ft, Function::ExternalLinkage, fdn->name, module.get());

    //The above handles everything for a function declaration
    //If the function is a definition, then the body will be compiled here.
    if(fdn->child){
        //Create the entry point for the function
        BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", f);
        builder.SetInsertPoint(bb);

        //tell the compiler to create a new scope on the stack.
        enterNewScope();

        //iterate through each parameter and add its value to the new scope.
        NamedValNode *cParam = paramsBegin;
        for(auto &arg : f->args()){
            TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();
            stoVar(cParam->name, new Variable(cParam->name, new TypedValue(&arg, paramTyNode->type), scope));
            if(!(cParam = (NamedValNode*)cParam->next.get())) break;
        }

        //actually compile the function, and hold onto the last value
        TypedValue *v = compileStmtList(fdn->child.get(), this);
        //End of the function, discard the function's scope.
        exitScope();

        builder.SetInsertPoint(&f->back());
        
        //llvm requires explicit returns, so generate a void return even if
        //the user did not in their void function.
        if(retNode->type == TT_Void && !dynamic_cast<ReturnInst*>(v->val)){
            builder.CreateRetVoid();
        }

        //apply function-level optimizations
        passManager->run(*f);
    }
    return f;
}


/*
 *  Registers a function for later compilation
 */
TypedValue* FuncDeclNode::compile(Compiler *c){
    name = c->funcPrefix + name;
    c->registerFunction(this);

    return nullptr;
}


TypedValue* ExtNode::compile(Compiler *c){
    c->funcPrefix = llvmTypeToStr(c->typeNodeToLlvmType(typeExpr.get())) + "_";
    compileStmtList(methods.get(), c);
    c->funcPrefix = "";
    return 0;
}


TypedValue* DataDeclNode::compile(Compiler *c){
    vector<Type*> tys;
    tys.reserve(fields);
    
    vector<string> fieldNames;
    fieldNames.reserve(fields);

    auto *nvn = (NamedValNode*)child.get();
    while(nvn){
        TypeNode *tyn = (TypeNode*)nvn->typeExpr.get();
        tys.push_back(c->typeNodeToLlvmType(tyn));
        fieldNames.push_back(nvn->name);
        nvn = (NamedValNode*)nvn->next.get();
    }

    auto *structTy = StructType::get(getGlobalContext(), tys);
    structTy->setName(name);

    auto *data = new DataType(fieldNames, structTy);

    c->stoType(data, name);
    return nullptr;
}


Function* Compiler::getFunction(string& name){
    Function *f = module->getFunction(name);
    if(!f){
        if(auto *fdNode = fnDecls[name]){
            //Function has been declared but not defined, so define it.
            BasicBlock *caller = builder.GetInsertBlock();
            f = compFn(fdNode);
            fnDecls.erase(name);
            builder.SetInsertPoint(caller);
        }
    }
    return f;
}

/*
 * imports a given ante file to the current module
 * inputted file must exist and be a valid ante source file.
 */
void Compiler::importFile(const char *fName){
    Compiler *c = new Compiler(fName, true);
    c->scanAllDecls();

    if(c->errFlag){
        cout << "Error when importing " << fName << endl;
        return;
    }

    //link functions from both files
    /*Module *m2 = c->module.get();
    c->module.release();
    //Linker::linkModules is currently bugged
    Linker *ln = new Linker(*module.get());
    if(ln->linkInModuleForCAPI(*m2)){
        cout << "Linking error\n";
        errFlag = true;
    } */

    //copy import's userTypes into importer
    for(const auto& it : c->userTypes){
        userTypes[it.first] = it.second;
    }

    for(const auto& it : c->fnDecls){
        fnDecls[it.first] = it.second;
    }

    delete c;
}

/*
 *  Creates an anonymous NamedValNode for use in function declarations.
 */
NamedValNode* mkAnonNVNode(TypeTag type){
    return new NamedValNode("", new TypeNode(type, "", nullptr));
}


TypeNode* mkAnonTypeNode(TypeTag type){
    return new TypeNode(type, "", nullptr);
}


unsigned int Compiler::getScope() const{
    return scope;
}


/*
 *  Declares functions to be included in every module without need of an import.
 *  These are registered but not compiled until they are called so that they
 *  do not pollute the module with unused definitions.
 */
void Compiler::compilePrelude(){
    if(fileName != "src/prelude.an")
        importFile("src/prelude.an");
}


/*
 *  Removes .an from a source file to get its module name
 */
string removeFileExt(string file){
    size_t len = file.length();
    if(len >= 4 && file[len-4] == '.') return file.substr(0, len-4);
    if(len >= 3 && file[len-3] == '.') return file.substr(0, len-3);
    if(len >= 2 && file[len-2] == '.') return file.substr(0, len-2);
    if(len >= 1 && file[len-1] == '.') return file.substr(0, len-1);
    return file;
}


/*
 *  Adds a function to the list of declared, but not defined functions.  A declared function's
 *  FuncDeclNode can be added to be compiled only when it is later called.  Useful to prevent pollution
 *  of a module with unneeded library functions.
 */
inline void Compiler::registerFunction(FuncDeclNode *fn){
    fnDecls[fn->name] = fn;
}

/*
 *  Sweeps through entire parse tree registering all function and data
 *  declarations.  Removes compiled functions.
 */
void Compiler::scanAllDecls(){
    Node *n = ast.get();
    while(n){
        if(dynamic_cast<FuncDeclNode*>(n) || dynamic_cast<ExtNode*>(n) || dynamic_cast<DataDeclNode*>(n)){
            n->compile(this); //register the function

            if(n->prev){
                n->prev->next.release();
                n->prev->next.reset(n->next.get());
            }else{
                ast.release();
                ast.reset(n->next.get());

                if(n->next)
                    n->next->prev = 0;
                else
                    ast.release();
            }
        }
        n = n->next.get();
    }
}


void Compiler::compile(){
    Function *main;
    scanAllDecls();

    //get or create the function type for the main method: void()
    FunctionType *ft = FunctionType::get(Type::getInt8Ty(getGlobalContext()), false);
    
    //Actually create the function in module m
    string fnName = isLib ? "init_" + removeFileExt(fileName) : "main";
    main = Function::Create(ft, Function::ExternalLinkage, fnName, module.get());

    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", main);
    builder.SetInsertPoint(bb);
    
    compilePrelude();

    //Compile the rest of the program
    compileStmtList(ast.get(), this);
    exitScope();

    //builder should already be at end of main function
    builder.CreateRet(ConstantInt::get(getGlobalContext(), APInt(8, 0, true)));
    passManager->run(*main);


    //flag this module as compiled.
    compiled = true;

    if(errFlag){
        puts("Compilation aborted.");
        exit(1);
    }
}


void Compiler::compileNative(){
    if(!compiled) compile();

    string modName = removeFileExt(fileName);
    //this file will become the obj file before linking
    string objFile = modName + ".o";

    cout << "Compiling " << modName << "...\n";
    if(!compileIRtoObj(objFile)){
        cout << "Linking...\n";
        linkObj(objFile, modName);
        remove(objFile.c_str());
    }
}

//returns 0 on success
int Compiler::compileObj(){
    if(!compiled) compile();

    string modName = removeFileExt(fileName);
    string objFile = modName + ".o";

    cout << "Compiling " << modName << " to .o file...\n";
    return compileIRtoObj(objFile);
}

/*
 *  Compiles a module into a .o file to be used for linking.
 *  Invokes llc.
 */
int Compiler::compileIRtoObj(string outFile){
    LLVMInitializeAllTargetInfos();
    LLVMInitializeAllTargets();
    LLVMInitializeAllTargetMCs();
    LLVMInitializeAllAsmPrinters();
    string err = "";

    string triple = "x86_64-unknown-linux-gnu";
    const Target* target = TargetRegistry::lookupTarget(triple, err);

    if(!err.empty()){
        cerr << err << endl;
        return 1;
    }

    string cpu = "haswell";
    string features = "";
    TargetOptions op;
    TargetMachine *tm = target->createTargetMachine(triple, cpu, features, op, Reloc::Model::Default, 
            CodeModel::Default, CodeGenOpt::Level::Aggressive);

    if(!tm){
        cerr << "Error when initializing TargetMachine.\n";
        return 1;
    }

    std::error_code errCode;
    raw_fd_ostream out{outFile, errCode, sys::fs::OpenFlags::F_RW};

    legacy::PassManager pm;
    int res = tm->addPassesToEmitFile(pm, out, llvm::TargetMachine::CGFT_ObjectFile);
    pm.run(*module);
    return res;
}


int Compiler::linkObj(string inFiles, string outFile){
    //invoke gcc to link the module.
    string cmd = "gcc " + inFiles + " -o " + outFile;
    return system(cmd.c_str());
}


/*
 *  Dumps current contents of module to stdout
 */
void Compiler::emitIR(){
    if(!compiled) compile();
    if(errFlag) puts("Partially compiled module: \n");
    module->dump();
}


inline void Compiler::enterNewScope(){
    scope++;
    auto *vtable = new map<string, Variable*>();
    varTable.push_back(unique_ptr<map<string, Variable*>>(vtable));
}


inline void Compiler::exitScope(){
    //iterate through all known variables, check for pointers at the end of
    //their lifetime, and insert calls to free for any that are found
    auto vtable = varTable.back().get();

    for(auto it = vtable->cbegin(); it != vtable->cend(); it++){
        if(it->second->isFreeable() && it->second->scope == scope){
            string freeFnName = "free";
            Function* freeFn = getFunction(freeFnName);

            auto *inst = dynamic_cast<AllocaInst*>(it->second->getVal());
            auto *val = inst? builder.CreateLoad(inst) : it->second->getVal();

            //cast the freed value to i32* as that is what free accepts
            Type *vPtr = freeFn->getFunctionType()->getFunctionParamType(0);
            val = builder.CreatePointerCast(val, vPtr);
            builder.CreateCall(freeFn, val);
        }
    }

    scope--;
    varTable.pop_back();
}


Variable* Compiler::lookup(string var) const{
    for(auto it = varTable.crbegin(); it != varTable.crend(); it++){
        try{
            auto *ret = (*it)->at(var);
            return ret;
        }catch(out_of_range r){}
    }
    return nullptr;
}


inline void Compiler::stoVar(string var, Variable *val){
    (*varTable.back())[var] = val;
}


DataType* Compiler::lookupType(string tyname) const{
    try{
        return userTypes.at(tyname);
    }catch(out_of_range r){
        return nullptr;
    }
}


inline void Compiler::stoType(DataType *ty, string &typeName){
    userTypes[typeName] = ty;
}


Compiler::Compiler(const char *_fileName, bool lib) : 
        builder(getGlobalContext()), 
        errFlag(false),
        compiled(false),
        isLib(lib),
        fileName(_fileName),
        funcPrefix(""){

    setLexer(new Lexer(_fileName));
    yy::parser p{};
    int flag = p.parse();
    if(flag != PE_OK){ //parsing error, cannot procede
        //print out remaining errors
        int tok;
        while((tok = yylexer->next()) != Tok_Newline && tok != 0);
        while(p.parse() != PE_OK && yylexer->peek() != 0);
        
        fputs("Syntax error, aborting.\n", stderr);
        exit(flag);
    }

    enterNewScope();
    ast.reset(parser::getRootNode());
    module.reset(new Module(removeFileExt(_fileName), getGlobalContext()));

    //add passes to passmanager.
    //TODO: change passes based on -O0 through -O3 flags
    passManager.reset(new legacy::FunctionPassManager(module.get()));
    //passManager->add(createBasicAliasAnalysisPass());
    passManager->add(createGVNPass());
    passManager->add(createCFGSimplificationPass());
    passManager->add(createTailCallEliminationPass());
    passManager->add(createPromoteMemoryToRegisterPass());
    passManager->add(createInstructionCombiningPass());
    passManager->add(createReassociatePass());
    passManager->doInitialization();
}

Compiler::~Compiler(){
    fnDecls.clear();
    if(yylexer){
        delete yylexer;
        yylexer = nullptr;
    }
}
