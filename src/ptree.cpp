/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "parser.h"
#include <stack>

stack<Node*> roots;

Node* ante::parser::getRootNode()
{
    return roots.top();
}

#define RETURN_NODE(t, p) return new t p;

/*
 *  Saves the root of a new block and returns it.
 */
Node* setRoot(Node* node)
{
    roots.push(node);
    return node;
}

/*
 *  Pops and returns the root of the current block
 */
Node* getRoot()
{
    Node* ret = roots.top();
    roots.pop();
    return ret;
}

Node* setNext(Node* cur, Node* nxt)
{
    cur->next = nxt;
    nxt->prev = cur;
    printf("Setting %p's next to %p\n", (void*)cur, (void*)nxt);
    return nxt;
}

Node* mkIntLitNode(char* s)
{
    RETURN_NODE(IntLitNode, (s));
}

Node* mkFltLitNode(char* s)
{
    RETURN_NODE(FltLitNode, (s));
}

Node* mkStrLitNode(char* s)
{
    RETURN_NODE(StrLitNode, (s));
}

Node* mkBoolLitNode(char b)
{
    RETURN_NODE(BoolLitNode, (b));
}

Node* mkTypeNode(int type, char* typeName)
{
    RETURN_NODE(TypeNode, (type, typeName));
}

Node* mkBinOpNode(int op, Node* l, Node* r)
{
    RETURN_NODE(BinOpNode, (op, l, r));
}

Node* mkRetNode(Node* expr)
{
    RETURN_NODE(RetNode, (expr));
}

Node* mkNamedValNode(char* s, Node* tExpr)
{
    RETURN_NODE(NamedValNode, (s, tExpr));
}

Node* mkFuncCallNode(char* s, Node* p)
{
    RETURN_NODE(FuncCallNode, (s, p));
}

Node* mkVarNode(char* s)
{
    RETURN_NODE(VarNode, (s));
}

Node* mkVarDeclNode(char* s, Node* tExpr, Node* expr)
{
    RETURN_NODE(VarDeclNode, (s, tExpr, expr));
}

Node* mkVarAssignNode(char* s, Node* expr)
{
    RETURN_NODE(VarAssignNode, (s, expr));
}

ParentNode* mkIfNode(Node* con, Node* body)
{
    RETURN_NODE(IfNode, (con, body));
}

ParentNode* mkFuncDeclNode(char* s, Node* tExpr, Node* p, Node* b)
{
    RETURN_NODE(FuncDeclNode, (s, tExpr, p, b));
}

ParentNode* mkDataDeclNode(char* s, Node* b)
{
    RETURN_NODE(DataDeclNode, (s, b));
}
