/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "parser.h"
#include <stack>

Node* root;

Node* ante::parser::getRootNode()
{
    return root;
}

extern "C" void setRoot(Node* node)
{
    printf("Node %p\n", node);
    /*while(1){
        Node* prev = node->prev;
        if(!prev) break;
        printf("Grappled root to %p\n", prev);
        node = prev;
    }*/
    root = node;
}

extern "C" Node* setNext(Node* cur, Node* nxt)
{
    cur->next = nxt;
    nxt->prev = cur;
    printf("Setting %p's next to %p\n", cur, nxt);
    return cur;
}

extern "C" Node* mkIntLitNode(char* s)
{
    return new IntLitNode(s);
}

extern "C" Node* mkFltLitNode(char* s)
{
    return new FltLitNode(s);
}

extern "C" Node* mkStrLitNode(char* s)
{
    return new StrLitNode(s);
}

extern "C" Node* mkBoolLitNode(char b)
{
    return new BoolLitNode(b);
}

extern "C" Node* mkTypeNode(int type, char* typeName)
{
    return new TypeNode(type, typeName);
}

extern "C" Node* mkBinOpNode(int op, Node* l, Node* r)
{
    return new BinOpNode(op, l, r);
}

extern "C" Node* mkRetNode(Node* expr)
{
    return new RetNode(expr);
}

extern "C" Node* mkNamedValNode(char* s, Node* tExpr)
{
    return new NamedValNode(s, tExpr);
}

extern "C" Node* mkFuncCallNode(char* s, Node* p)
{
    return new FuncCallNode(s, p);
}

extern "C" Node* mkVarNode(char* s)
{
    return new VarNode(s);
}

extern "C" Node* mkVarDeclNode(char* s, Node* tExpr, Node* expr)
{
    return new VarDeclNode(s, tExpr, expr);
}

extern "C" Node* mkVarAssignNode(char* s, Node* expr)
{
    return new VarAssignNode(s, expr);
}

extern "C" ParentNode* mkIfNode(Node* con, Node* body)
{
    return new IfNode(con, body);
}

extern "C" ParentNode* mkFuncDeclNode(char* s, Node* tExpr, Node* p, Node* b)
{
    return new FuncDeclNode(s, tExpr, p, b);
}

extern "C" ParentNode* mkDataDeclNode(char* s, Node* b)
{
    return new DataDeclNode(s, b);
}
