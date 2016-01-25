#ifndef PTREE_H
#define PTREE_H

#include "parser.h"

//defined in lexer.cpp
extern char* lextxt;

Node* getRootNode(void);
Node* setRoot(Node* root);
Node* getRoot(void);
Node* setNext(Node* cur, Node* nxt);
Node* setElse(IfNode *c, IfNode *elif);

Node* mkIntLitNode(char* s);
Node* mkFltLitNode(char* s);
Node* mkStrLitNode(char* s);
Node* mkBoolLitNode(char b);
Node* mkTypeNode(int type, char* typeName);
Node* mkBinOpNode(int op, Node* l, Node* r);
Node* mkNamedValNode(char* s, Node* tExpr);
Node* mkFuncCallNode(char* s, Node* p);
Node* mkVarNode(char* s);
Node* mkRetNode(Node* expr);
Node* mkVarDeclNode(char* s, Node* tExpr, Node* expr);
Node* mkVarAssignNode(Node* var, Node* expr);

//These 3 actually return a ParentNode* but C doesn't need to know that
Node* mkIfNode(Node* con, Node* body, Node* els = nullptr);
Node* mkFuncDeclNode(char* s, Node* tExpr, Node* p, Node* body);
Node* mkDataDeclNode(char* s, Node* b);

#endif
