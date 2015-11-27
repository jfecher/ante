#include "compiler.h"
using namespace llvm;

void IntLitNode::compile()
{

}

void FltLitNode::compile()
{

}

void BoolLitNode::compile()
{

}

void StrLitNode::compile()
{

}

void BinOpNode::compile()
{

}

void IfNode::compile()
{

}

void NamedValNode::compile()
{

}

void VarNode::compile()
{

}

void FuncCallNode::compile()
{

}

void VarDeclNode::compile()
{

}

void VarAssignNode::compile()
{
    
}

void FuncDeclNode::compile()
{

}

void ClassDeclNode::compile()
{

}



void IntLitNode::exec()
{

}

void FltLitNode::exec()
{

}

void BoolLitNode::exec()
{

}

void StrLitNode::exec()
{

}

void BinOpNode::exec()
{

}

void IfNode::exec()
{

}

void VarNode::exec()
{

}

void NamedValNode::exec()
{

}

void FuncCallNode::exec()
{

}

void VarDeclNode::exec()
{

}

void VarAssignNode::exec()
{
    
}

void FuncDeclNode::exec()
{

}

void ClassDeclNode::exec()
{

}


void IntLitNode::print()
{

}

void FltLitNode::print()
{

}

void BoolLitNode::print()
{

}

void StrLitNode::print()
{

}

void BinOpNode::print()
{

}

void IfNode::print()
{

}

void NamedValNode::print()
{

}

void VarNode::print()
{

}

void FuncCallNode::print()
{

}

void VarDeclNode::print()
{

}

void VarAssignNode::print()
{
    
}

void FuncDeclNode::print()
{
    cout << "function " << name << ": ";
    for(auto n : params){
        n->print();
        cout << ", ";
    }
    cout << "\n";
    for(auto n : body){
        n->print();
        cout << "\n";
    }
}

void ClassDeclNode::print()
{
    cout << "class " << name << "\n\t";
    for(auto n : body){
        cout << endl;
        n->print();
    }
    cout << endl;
}

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
