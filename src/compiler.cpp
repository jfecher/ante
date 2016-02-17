#include "parser.h"
#include "compiler.h"
#include <llvm/IR/Verifier.h>          //for verifying basic structure of functions
#include <llvm/Bitcode/ReaderWriter.h> //for r/w when outputting bitcode
#include <llvm/Support/FileSystem.h>   //for r/w when outputting bitcode
#include <llvm/Support/raw_ostream.h>  //for ostream when outputting bitcode
#include "llvm/Transforms/Scalar.h"    //for most passes
#include "llvm/Analysis/Passes.h"      //for createBasicAliasAnalysisPass()

using namespace llvm;


/*
 *  Prints a given line (row) of a file, along with an arrow pointing to
 *  the specified column.
 */
void printErrLine(const char* fileName, unsigned int row, unsigned int col){
    ifstream f{fileName};
    unsigned int line = 1;

    //Premature newline error, show previous line as error instead
    if(col == 0) row--;

    //skip to line in question
    int c;
    if(line != row){
        while(true){
            c = f.get();
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
size_t getTupleSize(Node *tup){
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
TypedValue* compileStmtList(Node *nList, Compiler *c, Module *m){
    TypedValue *ret = nullptr;
    while(nList){
        ret = nList->compile(c, m);
        nList = nList->next.get();
    }
    return ret;
}

inline bool Compiler::isUnsignedTokTy(int tt){
    return tt==Tok_U8||tt==Tok_U16||tt==Tok_U32||tt==Tok_U64||tt==Tok_Usz;
}

TypedValue* IntLitNode::compile(Compiler *c, Module *m){
    return new TypedValue(ConstantInt::get(getGlobalContext(),
                            APInt(Compiler::getBitWidthOfTokTy(type), 
                            atol(val.c_str()), Compiler::isUnsignedTokTy(type))), type);
}

/*
 *  TODO: type field for float literals
 */
TypedValue* FltLitNode::compile(Compiler *c, Module *m){
    return new TypedValue(ConstantFP::get(getGlobalContext(), APFloat(APFloat::IEEEdouble, val.c_str())), Tok_F32);
}

TypedValue* BoolLitNode::compile(Compiler *c, Module *m){
    return new TypedValue(ConstantInt::get(getGlobalContext(), APInt(1, (bool)val, true)), Tok_Bool);
}


TypedValue* ModNode::compile(Compiler *c, Module *m){
    return nullptr;
}

//TODO: possibly implement as replacement for tokTypeToLlvmType
TypedValue* TypeNode::compile(Compiler *c, Module *m){
    return nullptr;
}

TypedValue* StrLitNode::compile(Compiler *c, Module *m){
    return new TypedValue(c->builder.CreateGlobalStringPtr(val), '*');
    //ConstantDataArray::getString(getGlobalContext(), val);
}

/*
 *  When a retnode is compiled within a block, care must be taken to not
 *  forcibly insert the branch instruction afterwards as it leads to dead code.
 */
TypedValue* RetNode::compile(Compiler *c, Module *m){
    return new TypedValue(c->builder.CreateRet(expr->compile(c, m)->val), Tok_Void);
}

void compileIfNodeHelper(IfNode *ifN, BasicBlock *mergebb, Function *f, Compiler *c, Module *m){
    BasicBlock *thenbb = BasicBlock::Create(getGlobalContext(), "then", f);

    if(ifN->elseN.get()){
        TypedValue *cond = ifN->condition->compile(c, m);

        BasicBlock *elsebb = BasicBlock::Create(getGlobalContext(), "else");
        c->builder.CreateCondBr(cond->val, thenbb, elsebb);

        //Compile the if statement's then body
        c->builder.SetInsertPoint(thenbb);

        //Compile the then block
        TypedValue *v = compileStmtList(ifN->child.get(), c, m);
        
        //If the user did not return from the function themselves, then
        //merge to the endif.
        if(!dynamic_cast<ReturnInst*>(v->val)){
            c->builder.CreateBr(mergebb);
        }

        f->getBasicBlockList().push_back(elsebb);
        c->builder.SetInsertPoint(elsebb);

        //if elseN is else, and not elif, insert merge instruction.
        if(ifN->elseN->condition.get()){
            compileIfNodeHelper(ifN->elseN.get(), mergebb, f, c, m);
        }else{
            //compile the else node's body directly
            TypedValue *elseBody = ifN->elseN->child->compile(c, m);
            if(!dynamic_cast<ReturnInst*>(elseBody->val)){
                c->builder.CreateBr(mergebb);
            }
        }
    }else{ //this must be an if or elif node with no proceeding elif or else nodes.
        TypedValue *cond = ifN->condition->compile(c, m);
        c->builder.CreateCondBr(cond->val, thenbb, mergebb);
        c->builder.SetInsertPoint(thenbb);
        TypedValue *v = compileStmtList(ifN->child.get(), c, m);
        if(!dynamic_cast<ReturnInst*>(v->val)){
            c->builder.CreateBr(mergebb);
        }
    }
}

TypedValue* IfNode::compile(Compiler *c, Module *m){
    //Create thenbb and forward declare the others but dont inser them
    //into function f just yet.
    BasicBlock *mergbb = BasicBlock::Create(getGlobalContext(), "endif");
    Function *f = c->builder.GetInsertBlock()->getParent();

    compileIfNodeHelper(this, mergbb, f, c, m);

    f->getBasicBlockList().push_back(mergbb);
    c->builder.SetInsertPoint(mergbb);
    return new TypedValue(f, Tok_Void);
}

//Since parameters are managed in Compiler::compfn, this need not do anything
TypedValue* NamedValNode::compile(Compiler *c, Module *m)
{ return nullptr; }

/*
 *  Loads a variable from the stack
 */
TypedValue* VarNode::compile(Compiler *c, Module *m){
    TypedValue *val = c->lookup(name);
    if(!val)
        return c->compErr("Variable " + name + " has not been declared.", this->row, this->col);

    return dynamic_cast<AllocaInst*>(val->val)? new TypedValue(c->builder.CreateLoad(val->val, name), val->type) : val;
}

TypedValue* FuncCallNode::compile(Compiler *c, Module *m){
    Function *f = m->getFunction(name);
    if(!f){
        if(auto *fdNode = c->fnDecls[name]){
            //Function has been declared but not defined, so define it.
            BasicBlock *caller = c->builder.GetInsertBlock();
            f = c->compFn(fdNode);
            c->fnDecls.erase(name);
            c->builder.SetInsertPoint(caller);
        }else{
            return c->compErr("Called function " + name + " has not been declared.", this->row, this->col);
        }
    }

    size_t paramSize = getTupleSize(params.get());
    if(f->arg_size() != paramSize && !f->isVarArg()){
        if(paramSize == 1)
            return c->compErr("Called function " + name + " was given 1 paramter but was declared to take " + to_string(f->arg_size()), this->row, this->col);
        else
            return c->compErr("Called function " + name + " was given " + to_string(paramSize) + " paramters but was declared to take " + to_string(f->arg_size()), this->row, this->col);
    }

    std::vector<Value*> args;
    Node *curParam = params.get();
    for(unsigned i = 0; i < paramSize; i++){
        args.push_back(curParam->compile(c, m)->val);
        curParam = curParam->next.get();
        if(!args.back())
            c->compErr("Argument " + to_string(i+1) + " of called function " + name + " evaluated to null.", this->row, this->col);
    }

    if(f->getReturnType() == Type::getVoidTy(getGlobalContext())){
        return new TypedValue(c->builder.CreateCall(f, args), Tok_Void);
    }else{
        return new TypedValue(c->builder.CreateCall(f, args, "tmp"), Compiler::llvmTypeToTokType(f->getReturnType()));
    }
}


TypedValue* LetBindingNode::compile(Compiler *c, Module *m){
    if(c->lookup(name)){ //check for redeclaration
        return c->compErr("Variable " + name + " was redeclared.", row, col);
    }
    
    TypedValue *val = expr->compile(c, m);
    if(!val) return nullptr;

    TypeNode *tyNode;
    if((tyNode = (TypeNode*)typeExpr.get())){
        if(val->type != tyNode->type){
            return c->compErr("Incompatible types in explicit binding.", row, col);
        }
    }

    c->stoVar(name, val);
    return val;
}


TypedValue* VarDeclNode::compile(Compiler *c, Module *m){
    if(c->lookup(name)){ //check for redeclaration
        return c->compErr("Variable " + name + " was redeclared.", row, col);
    }

    TypeNode *tyNode = (TypeNode*)typeExpr.get();
    Type *ty = Compiler::tokTypeToLlvmType(tyNode->type, tyNode->typeName);
    TypedValue *alloca = new TypedValue(c->builder.CreateAlloca(ty, 0, name.c_str()), tyNode->type);

    c->stoVar(name, alloca);
    if(expr.get()){
        TypedValue *val = expr->compile(c, m);
        if(!val) return nullptr;

        return new TypedValue(c->builder.CreateStore(val->val, alloca->val), tyNode->type);
    }else{
        return alloca;
    }
}

TypedValue* VarAssignNode::compile(Compiler *c, Module *m){
    TypedValue *v = c->lookup(var->name);
    if(!v)
        return c->compErr("Use of undeclared variable " + var->name + " in assignment.", var->row, var->col);
    if(!dynamic_cast<AllocaInst*>(v->val))
        return c->compErr("Cannot mutate immutable variable " + var->name, var->row, var->col);
    return new TypedValue(c->builder.CreateStore(expr->compile(c, m)->val, v->val), Tok_Void);
}


Function* Compiler::compFn(FuncDeclNode *fdn){
    //Get and translate the function's return type to an llvm::Type*
    TypeNode *retNode = (TypeNode*)fdn->type.get();
    Type *retType = tokTypeToLlvmType(retNode->type, retNode->typeName);

    //Count the number of parameters
    NamedValNode *param = fdn->params.get();
    size_t nParams = getTupleSize(param);

    //Get each and every parameter type and store them in paramTys
    NamedValNode *cParam = param;
    vector<Type*> paramTys;

    //Tell the vector to reserve space equal to nParam parameters so it does not have to reallocate.
    paramTys.reserve(nParams);
    for(size_t i = 0; i < nParams && cParam; i++){
        TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();
        paramTys.push_back(tokTypeToLlvmType(paramTyNode->type, paramTyNode->typeName));
        cParam = (NamedValNode*)cParam->next.get();
    }

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
        cParam = param;
        for(auto &arg : f->args()){
            TypeNode *paramTyNode = (TypeNode*)cParam->typeExpr.get();
            stoVar(param->name, new TypedValue(&arg, paramTyNode->type));//TODO: store type of parameters
            if(!(param = (NamedValNode*)param->next.get())) break;
        }

        TypedValue *v = compileStmtList(fdn->child.get(), this, module.get());

        builder.SetInsertPoint(&module->getFunction(fdn->name)->back());
        
        //llvm requires explicit returns, so generate a void return even if
        //the user did not in their void function.
        if(retNode->type == Tok_Void && !dynamic_cast<ReturnInst*>(v->val)){
            builder.CreateRetVoid();
        }

        //End of the function, discard the function's scope.
        exitScope();

        //Attribute attr = Attribute::get(getGlobalContext(), "nounwind");
        //f->addAttributes(0, AttributeSet::get(getGlobalContext(), AttributeSet::FunctionIndex, attr));

        //apply function-level optimizations
        passManager->run(*f);

        //Verify the function is syntactically correct, and return to compiling main.
        verifyFunction(*f);
        //c->builder.SetInsertPoint(&c->module->getFunction("main")->back());
    }
    return f;
}

/*
 *  Registers a function for later compilation
 */
TypedValue* FuncDeclNode::compile(Compiler *c, Module *m){
    c->registerFunction(this);
    return nullptr;
}


TypedValue* DataDeclNode::compile(Compiler *c, Module *m)
{ return nullptr; }


/*
 *  Creates an anonymous NamedValNode for use in function declarations.
 */
NamedValNode* mkAnonNVNode(int type){
    return new NamedValNode("", new TypeNode(type, "", nullptr));
}

TypeNode* mkAnonTypeNode(int type){
    return new TypeNode(type, "", nullptr);
}

/*
 *  Declares functions to be included in every module without need of an import.
 *  These are registered but not compiled until they are called so that they
 *  do not pollute the module with unused definitions.
 */
void Compiler::compilePrelude(){
    // void printf: c8* str, ... va
    registerFunction(new FuncDeclNode("printf", 0, mkAnonTypeNode(Tok_Void), mkAnonNVNode(Tok_StrLit), nullptr, true));

    // void puts: c8* str
    registerFunction(new FuncDeclNode("puts", 0, mkAnonTypeNode(Tok_Void), mkAnonNVNode(Tok_StrLit), nullptr));

    // void putchar: c8 c
    registerFunction(new FuncDeclNode("putchar", 0, mkAnonTypeNode(Tok_Void), mkAnonNVNode(Tok_I32), nullptr));

    // void exit: u8 status
    registerFunction(new FuncDeclNode("exit", 0, mkAnonTypeNode(Tok_Void), mkAnonNVNode(Tok_I32), nullptr));
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

void Compiler::compile(){
    compilePrelude();

    //get or create the function type for the main method: void()
    FunctionType *ft = FunctionType::get(Type::getInt8Ty(getGlobalContext()), false);
    
    //Actually create the function in module m
    Function *main = Function::Create(ft, Function::ExternalLinkage, "main", module.get());

    //Create the entry point for the function
    BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", main);
    builder.SetInsertPoint(bb);

    //Compile the rest of the program
    compileStmtList(ast.get(), this, module.get());

    //builder should already be at end of main function
    builder.CreateRet(ConstantInt::get(getGlobalContext(), APInt(8, 0, true)));

    passManager->run(*main);
    verifyFunction(*main);

    //flag this module as compiled.
    compiled = true;

    if(errFlag){
        puts("Compilation aborted.");
        return;
    }
}


void Compiler::compileNative(){
    if(!compiled) compile();
    if(errFlag) return;

    string modName = removeFileExt(fileName);
    //this file will become the obj file before linking
    string objFile = modName + ".o";

    cout << "Compiling " << modName << "...\n";
    if(!compileIRtoObj(module.get(), fileName, objFile)){
        cout << "Linking...\n";
        linkObj(objFile, modName);
        remove(objFile.c_str());
    }
}

/*
 *  Compiles a module into a .o file to be used for linking.
 *  Invokes llc.
 */
int Compiler::compileIRtoObj(Module *m, string inFile, string outFile)
{
    string llbcName = removeFileExt(inFile) + ".bc";

    string cmd = "llc -filetype obj -o " + outFile + " " + llbcName;

    //Write the temporary bitcode file
    std::error_code err;
    raw_fd_ostream out{llbcName, err, sys::fs::OpenFlags::F_RW};
    WriteBitcodeToFile(m, out);
    out.close();

    //invoke llc and compile an object file of the module
    int res = system(cmd.c_str());
    if(res) return res;

    //remove the temporary .bc file
    remove(llbcName.c_str());
    return 0;
}

int Compiler::linkObj(string inFiles, string outFile)
{
    //invoke gcc to link the module.
    string cmd = "gcc " + inFiles + " -o " + outFile;
    return system(cmd.c_str());
}

/*
 *  Dumps current contents of module to stdout
 */
void Compiler::emitIR()
{
    if(!compiled) compile();
    if(errFlag) puts("Partially compiled module: \n");
    module->dump();
}

inline void Compiler::enterNewScope()
{
    varTable.push(map<string, TypedValue*>());
}

inline void Compiler::exitScope()
{
    varTable.pop();
}

inline TypedValue* Compiler::lookup(string var)
{
    try{
        return varTable.top().at(var);
    }catch(out_of_range r){
        return nullptr;
    }
}

inline void Compiler::stoVar(string var, TypedValue *val)
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


Compiler::Compiler(char *_fileName) : 
        builder(getGlobalContext()), 
        errFlag(false),
        compiled(false),
        fileName(_fileName){

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

    ast.reset(parser::getRootNode());
    varTable.push(map<string, TypedValue*>());
    module.reset(new Module(removeFileExt(_fileName), getGlobalContext()));

    //add passes to passmanager.
    //TODO: change passes based on -O0 through -O3 flags
    passManager.reset(new legacy::FunctionPassManager(module.get()));
    passManager->add(createBasicAliasAnalysisPass());
    passManager->add(createGVNPass());
    passManager->add(createCFGSimplificationPass());
    passManager->add(createTailCallEliminationPass());
    passManager->add(createPromoteMemoryToRegisterPass());
    passManager->add(createInstructionCombiningPass());
    passManager->add(createReassociatePass());
    passManager->doInitialization();
}

Compiler::~Compiler(){}
