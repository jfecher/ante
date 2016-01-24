#include "parser.h"
using namespace ante::parser;


inline void maybePrintArr(Node *n)
{
    if(n){
        cout << ", ";
        n->print();
    }
}

/*
 *  Prints a list of nodes, can print
 *  entire parse tree if passed the root.
 */
void parser::printBlock(Node *block)
{
    while(block){
        block->print();
        block = block->next.get();
        cout << endl;
    }
}

void IntLitNode::print()
{
    cout << val;
    maybePrintArr(next.get());
}

void FltLitNode::print()
{
    cout << val;
    maybePrintArr(next.get());
}

void BoolLitNode::print()
{
    if(val)
        cout << "true";
    else
        cout << "false";
    maybePrintArr(next.get());
}

void StrLitNode::print()
{
    cout << '"' << val << '"';
    maybePrintArr(next.get());
}

void TypeNode::print()
{
    if(type == Tok_Ident || type == Tok_UserType)
        cout << typeName;
    else
        ante::lexer::printTok(type);
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
    putchar(')');

    maybePrintArr(next.get());
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
    puts(" then");
    printBlock(child.get());
    cout << "endif";
}

void NamedValNode::print()
{
    typeExpr->print();
    putchar(' ');
    cout << name;

    maybePrintArr(next.get());
}

void VarNode::print()
{
    cout << name;
    maybePrintArr(next.get());
}

void FuncCallNode::print()
{
    cout << "fnCall " << name << '(';
    if(params) params->print();
    putchar(')');
}

void VarDeclNode::print()
{
    cout << "varDecl ";
    if(typeExpr) typeExpr->print();
    cout << ' ' << name << " = ";
    if(expr) expr->print();
    else cout << "(undef)";
}

void VarAssignNode::print()
{
    cout << "varAssign ";
    if(var) var->print();
    cout << " = ";
    if(expr) expr->print();
    else cout << "(undef)";
}

void FuncDeclNode::print()
{
    cout << "fnDecl ";
    type->print();
    cout << ' ' << name << ": ";
    if(params) params->print();
    puts("\nfnbody:");
    printBlock(child.get());
    puts("endfn");
}

void DataDeclNode::print()
{
    cout << "data " << name << "declared\n";
    printBlock(child.get());
}
