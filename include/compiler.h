#ifndef COMPILER_H
#define COMPILER_H

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

namespace ante{
    struct Compiler{
        unique_ptr<legacy::FunctionPassManager> passManager;
        unique_ptr<Module> module;
        unique_ptr<Node> ast;
        IRBuilder<> builder;
        stack<std::map<string, Value*>> varTable;
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
        
        template<typename T>
        Value* compErr(T msg);
        
        template<typename T, typename... Args>
        Value* compErr(T msg, Args... args);
        
        Value* lookup(string var);
        void stoVar(string var, Value *val);

        static int compileIRtoObj(Module *m, string inFile, string outFile);
        static int linkObj(string inFiles, string outFile);
    };
}

#endif
