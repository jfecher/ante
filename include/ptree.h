#ifndef PTREE_H
#define PTREE_H

typedef struct{} Node;

Node* getRootNode();

Node* makeIntLitNode(char* s);
Node* makeFltLitNode(char* s);
Node* makeStrLitNode(char* s);
Node* makeBoolLitNode(char b);
Node* makeBinOpNode(int op, Node* l, Node* r);
Node* makeNamedValNode(char* s, Node* tExpr);
Node* makeFuncCallNode(char* s, Node* p);
Node* makeVarNode(char* s);
void attatchRetNode(Node* expr);
void attatchIfNode(Node* con, Node** body);
void attatchVarDeclNode(char* s, Node* tExpr, Node* expr);
void attatchVarAssignNode(char* s, Node* expr);
void attatchFuncDeclNode(char* s, Node* tExpr, Node* p, Node** body);
void attatchDataDeclNode(char* s, Node* b);

#endif
