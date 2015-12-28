/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "parser.h"

Node *root = 0;
Node *stmt = 0;
Node *branch = 0;


#define attatchStmtNode(nodeDecl)    \
    if(root){                        \
        stmt->next = new nodeDecl;   \
        stmt = stmt->next;           \
    }else{                           \
        root = new nodeDecl;         \
        stmt = root;                 \
    }


Node* getRootNode()
{
    return root;
}

extern "C" Node* makeIntLitNode(char* s)
{
    return new IntLitNode(s);
}

extern "C" Node* makeFltLitNode(char* s)
{
    return new FltLitNode(s);
}

extern "C" Node* makeStrLitNode(char* s)
{
    return new StrLitNode(s);
}

extern "C" Node* makeBoolLitNode(char b)
{
    return new BoolLitNode(b);
}

extern "C" Node* makeBinOpNode(int op, Node* l, Node* r)
{
    return new BinOpNode(op, l, r);
}

extern "C" void attatchRetNode(Node* expr)
{
    attatchStmtNode(RetNode(expr));
}

extern "C" void attatchIfNode(Node* con, Node** body)
{
    attatchStmtNode(IfNode(con, body));
}

extern "C" Node* makeNamedValNode(char* s, Node* tExpr)
{
    return new NamedValNode(s, tExpr);
}

extern "C" Node* makeFuncCallNode(char* s, Node* p)
{
    return new FuncCallNode(s, p);
}

extern "C" Node* makeVarNode(char* s)
{
    return new VarNode(s);
}

extern "C" void attatchVarDeclNode(char* s, Node* tExpr, Node* expr)
{
    attatchStmtNode(VarDeclNode(s, tExpr, expr));
}

extern "C" void attatchVarAssignNode(char* s, Node* expr)
{
    attatchStmtNode(VarAssignNode(s, expr));
}

extern "C" void attatchFuncDeclNode(char* s, Node* tExpr, Node** p, Node** b)
{
    attatchStmtNode(FuncDeclNode(s, tExpr, p, b));
}

extern "C" void attatchDataDeclNode(char* s, Node** b)
{
    attatchStmtNode(DataDeclNode(s, b));
}

/*
 *  makeBlock transforms a Node* into a vector of Node*
 *  to be used as a block.
 */
vector<Node*> makeBlock(Node* nl)
{
    vector<Node*> body;
    while(nl){
        body.push_back(nl);
        nl = nl->next;
    }
    return body;
}
