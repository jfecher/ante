#ifndef COMPILER_H
#define COMPILER_H

#include <llvm/IR/IRBuilder.h>
#include <llvm/IR/LLVMContext.h>
#include <llvm/IR/Module.h>
#include <llvm/IR/Verifier.h>
#include <llvm/Bitcode/ReaderWriter.h>
#include <llvm/Support/FileSystem.h>
#include <llvm/Support/raw_ostream.h>
#include <memory>
#include <vector>
#include <stack>

using namespace llvm;
using namespace std;

/* Forward-declaration of Node defined in parser.h */
struct Node;

namespace ante{
    struct Compiler{
        unique_ptr<Node> ast;
        unique_ptr<Module> module;
        IRBuilder<> builder;
        stack<std::map<string, Value*>> varTable;
        bool errFlag;
        string fileName;
        
        Compiler(Node *_ast, char *_fileName) : 
                ast(_ast), 
                builder(getGlobalContext()), 
                errFlag(false),
                fileName(_fileName){
            module = unique_ptr<Module>(new Module(_fileName, getGlobalContext()));
            varTable.push(map<string, Value*>());
        }
        ~Compiler(){}

        void compile();
        void compilePrelude();
        void enterNewScope();
        void exitScope();
        
        template<typename T>
        Value* compErr(T msg);
        
        template<typename T, typename... Args>
        Value* compErr(T msg, Args... args);
        
        Value* lookup(string var);
        void stoVar(string var, Value *val);

        static AllocaInst* createBlockAlloca(Function *f, string var, Type *varType);
    };
}

#endif
