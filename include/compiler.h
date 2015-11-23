#ifndef COMPILER_H
#define COMPILER_H

#include <llvm/IR/IRBuilder.h>
#include <llvm/IR/LLVMContext.h>
#include <llvm/IR/Module.h>
#include <llvm/IR/Verifier.h>
#include <vector>
#include "parser.h"

using namespace llvm;

class Compiler{
    public:
        Node* ast;
        Module* module;
        IRBuilder<> builder;
        
        Compiler(Node* _ast) : ast(_ast), builder(getGlobalContext()){
            module = new Module("zy_mod", getGlobalContext());
        }
        ~Compiler(){ 
            free(ast); 
            delete module;
        }

        void compile(void);

};

#endif
