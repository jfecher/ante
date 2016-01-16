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
    cout << '"' << val << '"';
}

void TypeNode::print()
{
    if(type == Tok_Ident || type == Tok_UserType){
        cout << "Type: " << typeName;
    }else{
        cout << "Type: ";
        ante::lexer::printTok(type);
    }
}

void BinOpNode::print()
{
    putchar('(');
    if(lval) lval->print();
    putchar(' ');
    if(IS_LITERAL(op))
        cout << (char)op;
    else
        cout << TOK_TYPE_STR(op);
    putchar(' ');
    if(rval) rval->print();
    puts(")");
}

void RetNode::print()
{
    cout << "return ";
    if(expr) expr->print();
    putchar('\n');
}

void IfNode::print()
{
    cout << "if ";
    if(condition) condition->print();
    cout << "\nthen\n";
    if(child) child->print();
    cout << "EndIf\n";
}

void NamedValNode::print()
{
    cout << "{NamedValNode " << name << '}';
}

void VarNode::print()
{
    cout << name;
}

void FuncCallNode::print()
{
    cout << "fnCall " << name << " called with params (";
    if(params) params->print();
    cout << ")\n";
}

void VarDeclNode::print()
{
    cout << "varDecl " << name << " = ";
    if(expr) expr->print();
    else cout << "(undef)";
    putchar('\n');
}

void VarAssignNode::print()
{
    cout << "varAssign ";
    if(var) var->print(); 
    cout << " = ";
    if(expr) expr->print();
    else cout << "(undef)"; 
    putchar('\n');
}

void FuncDeclNode::print()
{
    cout << "function " << name << " declared of ";
    type->print();
    puts("With params: ");
    if(params) params->print();
    puts("FuncBody:");
    if(child.get()) child.get()->print();
    puts("EndFunc");
}

void DataDeclNode::print()
{
    cout << "Data " << name << "Declared\n";
    if(child.get()) child.get()->print();
    puts("");
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
