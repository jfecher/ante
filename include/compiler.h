#ifndef COMPILER_H
#define COMPILER_H

#include <climits> //required by llvm is using clang
#include <llvm/IR/LegacyPassManager.h>
#include <llvm/IR/IRBuilder.h>
#include <llvm/IR/Module.h>
#include <memory>
#include <stack>
#include <map>

using namespace llvm;
using namespace std;

/* Forward-declaration of Node defined in parser.h */
struct Node;
struct FuncDeclNode;

namespace ante{
    struct Compiler{
        unique_ptr<legacy::FunctionPassManager> passManager;
        unique_ptr<Module> module;
        unique_ptr<Node> ast;
        IRBuilder<> builder;

        //Stack of maps of variables mapped to their identifier.
        //Maps are seperated according to their scope.
        stack<std::map<string, Value*>> varTable;

        //Map of declared, but non-defined functions
        map<string, FuncDeclNode*> fnDecls;

        bool errFlag, compiled;
        string fileName;
        
        Compiler(char *fileName);
        ~Compiler();

        void compile();
        void compileNative();
        void compilePrelude();
        void emitIR();
        void enterNewScope();
        void exitScope();
        
        Value* compAdd(Type *t, Value *l, Value *r);
        Value* compSub(Type *t, Value *l, Value *r);
        Value* compMul(Type *t, Value *l, Value *r);
        Value* compDiv(Type *t, Value *l, Value *r);
        Value* compRem(Type *t, Value *l, Value *r);
        
        Value* compErr(string msg);

        Function* compFn(FuncDeclNode *fn);
        void registerFunction(FuncDeclNode *func);

        Value* lookup(string var);
        void stoVar(string var, Value *val);

        static int compileIRtoObj(Module *m, string inFile, string outFile);
        static int linkObj(string inFiles, string outFile);
    };
}

#endif
