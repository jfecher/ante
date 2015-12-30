#include "compiler.h"
using namespace llvm;

void IntLitNode::compile(){}

void FltLitNode::compile(){}

void BoolLitNode::compile(){}

void TypeNode::compile(){}

void StrLitNode::compile(){}

void BinOpNode::compile(){}

void RetNode::compile(){}

void IfNode::compile(){}

void NamedValNode::compile(){}

void VarNode::compile(){}

void FuncCallNode::compile(){}

void VarDeclNode::compile(){}

void VarAssignNode::compile(){}

void FuncDeclNode::compile(){}

void DataDeclNode::compile(){}



void IntLitNode::exec(){}

void FltLitNode::exec(){}

void BoolLitNode::exec(){}

void TypeNode::exec(){}

void StrLitNode::exec(){}

void BinOpNode::exec(){}

void RetNode::exec(){}

void IfNode::exec(){}

void VarNode::exec(){}

void NamedValNode::exec(){}

void FuncCallNode::exec(){}

void VarDeclNode::exec(){}

void VarAssignNode::exec(){}

void FuncDeclNode::exec(){}

void DataDeclNode::exec(){}



void IntLitNode::print()
{
    cout << val;
}

void FltLitNode::print()
{
    cout << val;
}

void BoolLitNode::print()
{
    if(val)
        cout << "true";
    else
        cout << "false";
}

void StrLitNode::print()
{
    cout << val;
}

void TypeNode::print()
{
    if(type == Tok_Ident || type == Tok_UserType){
        cout << "Type: " << typeName << endl;
    }else{
        cout << "Type: " << type;
    }
}

void BinOpNode::print()
{
    cout << '(';
    if(lval) lval->print();
    cout << " ";
    if(rval) rval->print();
    cout << ' ';
    if(IS_LITERAL(op))
        cout << op;
    else
        cout << TOK_TYPE_STR(op);
    cout << ')';
}

void RetNode::print()
{
    cout << "return ";
    if(expr) expr->print();
}

void IfNode::print()
{
    cout << "if ";
    if(condition) condition->print();
    cout << "\nthen\n";
    /*for(auto n : body){
        if(n) n->print();
        cout << endl;
    }*/
    cout << "end";
}

void NamedValNode::print()
{
    cout << "NVNode: " << name;
}

void VarNode::print()
{
    cout << name;
}

void FuncCallNode::print()
{
    cout << "fnCall " << name << '(';
    if(params) params->print();
    cout << ')';
}

void VarDeclNode::print()
{
    cout << "varDecl " << name << " = ";
    if(expr) expr->print();
    else cout << "(undef)";
}

void VarAssignNode::print()
{
    cout << "varAssign " << name << " = ";
    if(expr) expr->print();
    else cout << "(undef)"; 
}

void FuncDeclNode::print()
{
    cout << "function " << name << ": ";
    /*for(auto n : params){
        if(n) n->print();
        cout << ", ";
    }
    cout << "\n";
    for(auto n : body){
        if(n) n->print();
        cout << "\n";
    }*/
}

void DataDeclNode::print()
{
    cout << "class " << name << "\n\t";
    /*for(auto n : body){
        cout << endl;
        n->print();
    }*/
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
