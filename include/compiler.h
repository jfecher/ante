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
        bool errFlag, compiled;
        string fileName;
        
        Compiler(char *fileName);
        ~Compiler(){}

        void compile();
        void compileNative();
        void compilePrelude();
        void emitIR();
        void enterNewScope();
        void exitScope();
        
        template<typename T>
        Value* compErr(T msg);
        
        template<typename T, typename... Args>
        Value* compErr(T msg, Args... args);
        
        Value* lookup(string var);
        void stoVar(string var, Value *val);

        static void compileIRtoObj(Module *m, string inFile, string outFile);
        static void linkObj(string inFiles, string outFile);
    };
}

#endif
