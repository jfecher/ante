#ifndef COMPILER_H
#define COMPILER_H

#include <llvm/IR/IRBuilder.h>
#include <llvm/IR/LLVMContext.h>
#include <llvm/IR/Module.h>
#include <llvm/IR/Verifier.h>
#include <memory>
#include <vector>

using namespace llvm;

/* Forward-declaration of Node defined in parser.h */
struct Node;

namespace ante{
    class Compiler{
        public:
            std::unique_ptr<Node> ast;
            std::unique_ptr<Module> module;
            IRBuilder<> builder;

            Compiler(Node* _ast) : ast(_ast), builder(getGlobalContext()){
                module = std::unique_ptr<Module>(new Module("ante_main_mod", getGlobalContext()));
            }
            ~Compiler(){}

            Module* getMod(){ return module.get(); }

            void compile(void);
    };
}

#endif
