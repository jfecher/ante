#include "compiler.h"
using namespace llvm;


void Compiler::compile()
{
    vector<llvm::Type*> doubles{2, Type::getDoubleTy(getGlobalContext())};
    FunctionType *ft = FunctionType::get(Type::getDoubleTy(getGlobalContext()), doubles, false);
    Function *f = Function::Create(ft, Function::ExternalLinkage, "", module);

    BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", f);
    builder.SetInsertPoint(bb);

    Value *ret = ConstantFP::get(getGlobalContext(), APFloat(0.5f));
    builder.CreateRet(ret);

    verifyFunction(*f);
    module->dump();
}
