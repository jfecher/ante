#ifndef COMPILER_H
#define COMPILER_H

#include <llvm/IR/IRBuilder.h>
#include <llvm/IR/LLVMContext.h>
#include <llvm/IR/Module.h>
#include <llvm/IR/Verifier.h>
#include <memory>
#include <vector>
#include <stack>

using namespace llvm;

/* Forward-declaration of Node defined in parser.h */
struct Node;

namespace ante{
    struct Compiler{
        std::unique_ptr<Node> ast;
        std::unique_ptr<Module> module;
        IRBuilder<> builder;
        std::stack<std::map<std::string, Value*>> varTable;
        
        
        Compiler(Node* _ast) : ast(_ast), builder(getGlobalContext()){
            module = std::unique_ptr<Module>(new Module("ante_main_mod", getGlobalContext()));
        }
        ~Compiler(){}

        void compile(void);
    };
}

#endif
