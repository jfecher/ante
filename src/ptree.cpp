/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "parser.h"
#include <stack>

stack<Node*> root;
stack<Node*> stmt;
Node* branch;

enum BlockState {
    Pop, Good, Push
} bState;

#define attatchStmtNode(nodeDecl)        \
    if(bState == Good){                  \
        stmt.top()->next = new nodeDecl; \
        stmt.top() = stmt.top()->next;   \
    }else if(bState == Push){            \
        root.push(new nodeDecl);         \
        stmt.push(root.top());           \
    }else if(bState == Pop){             \
        stmt.pop();                      \
        root.pop();                      \
        stmt.top()->next = new nodeDecl; \
        stmt.top() = stmt.top()->next;   \
    }


Node* getRootNode()
{
    while(root.size() > 1)
        root.pop();
    return root.top();
}

extern "C" void newBlock()
{
    bState = Push;
}

extern "C" void endBlock()
{
    bState = Pop;
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

extern "C" Node* makeTypeNode(int type, char* typeName)
{
    return new TypeNode(type, typeName);
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
