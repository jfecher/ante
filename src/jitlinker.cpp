/*
 *      jitlinker.cpp
 * Provides some helper functions for linking
 * an llvm::Module without destroying the src
 * and only linking needed functions.
 */
#include "jitlinker.h"

void copyDecls(Compiler *src, Compiler *dest){
    for(const auto& it : src->userTypes){
        dest->userTypes[it.first] = it.second;
    }

    for(const auto& it : src->fnDecls){
        for(auto *fd : it.second)
            fd->scope = dest->scope;

        dest->fnDecls[it.first] = it.second;
    }
}

/*
 * Copies a function into a new module (named after the function)
 * and copies any functions that are needed by the copied function
 * into the new module as well.
 */
Module* wrapFnInModule(Compiler *c, Function *f){
    Compiler *ccpy = new Compiler(c->ast.get(), f->getName(), c->fileName);
    copyDecls(c, ccpy);

    string name = f->getName().str();
        
    auto flist = ccpy->getFunctionList(name);

    if(flist.size() == 1){
        ccpy->compFn((*flist.begin())->fdn, 0);
    }else if(flist.empty()){
        cerr << "No function '" << name << "'\n";
        c->errFlag = true;
        return 0;
    }else{
        cerr << "Too many candidates for function '" << name << "'\n";
        c->errFlag = true;
        return 0;
    }

    auto *mod = ccpy->module.release();
    //delete ccpy;
    return mod;
}

void linkFunction(Compiler *c, Function *f, Module *mod){
    Function *fcpy = Function::Create(f->getFunctionType(), Function::ExternalLinkage, f->getName(), mod);
    LLVMContext ctxt;

    for(auto &bb : *f){
        auto *bbcpy = BasicBlock::Create(ctxt, bb.getName(), fcpy);
        IRBuilder<> irb{bbcpy};

        for(auto &instr : bb){
            if(CallInst *ci = dyn_cast<CallInst>(&instr)){
                auto *calledFunc = ci->getCalledFunction();
                if(!mod->getFunction(calledFunc->getName())){
                    linkFunction(c, calledFunc, mod);
                }
            }
            irb.Insert(&instr);
        }
    }
}
