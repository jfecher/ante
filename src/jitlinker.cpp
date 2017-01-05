/*
 *      jitlinker.cpp
 * Provides some helper functions for linking
 * an llvm::Module without destroying the src
 * and only linking needed functions.
 */
#include "jitlinker.h"


void copyDecls(const Compiler *src, Compiler *dest){
    dest->compUnit = src->compUnit;

    dest->mergedCompUnits = src->mergedCompUnits;

    dest->imports = src->imports;

    dest->allCompiledModules = src->allCompiledModules;
}

/*
 * Copies a function into a new module (named after the function)
 * and copies any functions that are needed by the copied function
 * into the new module as well.
 */
llvm::Module* wrapFnInModule(Compiler *c, Function *f){
    Compiler ccpy{c->ast.get(), f->getName(), c->fileName};
    copyDecls(c, &ccpy);
    
    //create an empty main function to avoid crashes with compFn when
    //trying to return to the caller function
    ccpy.createMainFn();
    //the ret comes separate
    ccpy.builder.CreateRet(ConstantInt::get(ccpy.ctxt, APInt(32, 1)));

    string name = f->getName().str();

    auto& flist = ccpy.getFunctionList(name);

    if(flist.size() == 1){
        ccpy.compFn((*flist.begin())->fdn, 0);
    }else if(flist.empty()){
        cerr << "No function '" << name << "'\n";
        c->errFlag = true;
        return 0;
    }else{
        cerr << "Too many candidates for function '" << name << "'\n";
        c->errFlag = true;
        return 0;
    }

    return ccpy.module.release();
}
