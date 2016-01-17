#include "compiler.h"
#include "parser.h"

using namespace llvm;

void IntLitNode::compile(Compiler *c, Module *m){}

void FltLitNode::compile(Compiler *c, Module *m){}

void BoolLitNode::compile(Compiler *c, Module *m){}

void TypeNode::compile(Compiler *c, Module *m){}

void StrLitNode::compile(Compiler *c, Module *m){}

void BinOpNode::compile(Compiler *c, Module *m){}

void RetNode::compile(Compiler *c, Module *m){}

void IfNode::compile(Compiler *c, Module *m){}

void NamedValNode::compile(Compiler *c, Module *m){}

void VarNode::compile(Compiler *c, Module *m){}

void FuncCallNode::compile(Compiler *c, Module *m){}

void VarDeclNode::compile(Compiler *c, Module *m){}

void VarAssignNode::compile(Compiler *c, Module *m){}

void FuncDeclNode::compile(Compiler *c, Module *m)
{
    vector<llvm::Type*> paramTypes{2, Type::getDoubleTy(getGlobalContext())};
    auto retType = Type::getDoubleTy(getGlobalContext());

    FunctionType *ft = FunctionType::get(retType, paramTypes, false);
    Function *f = Function::Create(ft, Function::ExternalLinkage, "FuncDeclNode", m);

    BasicBlock *bb = BasicBlock::Create(getGlobalContext(), "entry", f);
    c->builder.SetInsertPoint(bb);

    Value *ret = ConstantFP::get(getGlobalContext(), APFloat(0.5f));
    c->builder.CreateRet(ret);

    verifyFunction(*f);
}

void DataDeclNode::compile(Compiler *c, Module *m){}



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
    Node *n = ast.get();
    while(n){
        n->compile(this, module.get());
        n = n->next.get();
    }
    module->dump();
}
