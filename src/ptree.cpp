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

/*
 *  Define strdup for non-posix environments
 */
/*#ifndef POSIX
char* strdup(char* src)
{
    size_t len = strlen(src);
    char* dest = new char[len];
    strncpy(dest, src, len);
    return dest;
}
#endif
*/
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
    cur->next.reset(nxt);
    nxt->prev = cur;
    return nxt;
}

/*
 *  Sets the else of an ifnode to a given ifnode representing
 *  either an else or an elif.
 */
Node* setElse(IfNode *c, IfNode *elif)
{
    c->elseN.reset(elif);
    return elif;
}

Node* mkIntLitNode(char* s)
{
    return new IntLitNode(strdup(s));
}

Node* mkFltLitNode(char* s)
{
    return new FltLitNode(strdup(s));
}

Node* mkStrLitNode(char* s)
{
    return new StrLitNode(strdup(s));
}

Node* mkBoolLitNode(char b)
{
    return new BoolLitNode(b);
}

Node* mkTypeNode(int type, char* typeName)
{
    return new TypeNode(type, strdup(typeName));
}

Node* mkBinOpNode(int op, Node* l, Node* r)
{
    return new BinOpNode(op, l, r);
}

Node* mkRetNode(Node* expr)
{
    return new RetNode(expr);
}

Node* mkNamedValNode(char* s, Node* tExpr)
{
    return new NamedValNode(strdup(s), tExpr);
}

Node* mkFuncCallNode(char* s, Node* p)
{
    return new FuncCallNode(strdup(s), p);
}

Node* mkVarNode(char* s)
{
    return new VarNode(strdup(s));
}

Node* mkVarDeclNode(char* s, Node* tExpr, Node* expr)
{
    return new VarDeclNode(strdup(s), tExpr, expr);
}

Node* mkVarAssignNode(Node* var, Node* expr)
{
    return new VarAssignNode(var, expr);
}

ParentNode* mkIfNode(Node* con, Node* body, Node* els = nullptr)
{
    return new IfNode(con, body, (IfNode*)els);
}

ParentNode* mkFuncDeclNode(char* s, Node* tExpr, Node* p, Node* b)
{
    return new FuncDeclNode(strdup(s), tExpr, p, b);
}

ParentNode* mkDataDeclNode(char* s, Node* b)
{
    return new DataDeclNode(strdup(s), b);
}
