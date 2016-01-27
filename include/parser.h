#ifndef PARSER_H
#define PARSER_H

#include <vector>
#include <memory> //For unique_ptr
#include "lexer.h"
#include "tokens.h"
#include "compiler.h"

enum ParseErr{
    PE_OK,
    PE_EXPECTED,
    PE_VAL_NOT_FOUND,
    PE_IDENT_NOT_FOUND,
    PE_INVALID_STMT,
};

using namespace llvm;
using namespace ante;

/* Base class for all nodes */
struct Node{
    unique_ptr<Node> next;
    Node *prev;
    virtual void print(void) = 0;
    virtual Value* compile(Compiler*, Module*) = 0;
    //virtual void exec(void) = 0;
    virtual ~Node(){}
};

/*
 * Class for all nodes that can contain child statement nodes,
 * if statements, function declarations, etc
 */
struct ParentNode : public Node{
    unique_ptr<Node> child;

    /*
        * The body should always be known when a
        * parent node is initialized, so it is required
        * in the constructor (unlike next and prev)
        */
    ParentNode(Node* c) : Node(), child(c){}
    ~ParentNode(){}
};

struct IntLitNode : public Node{
    string val;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    IntLitNode(char* s) : Node(), val(s){}
    ~IntLitNode(){}
};

struct FltLitNode : public Node{
    string val;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    FltLitNode(char* s) : Node(), val(s){}
    ~FltLitNode(){}
};

struct BoolLitNode : public Node{
    bool val;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    BoolLitNode(char b) : Node(), val(b){}
    ~BoolLitNode(){}
};

struct BinOpNode : public Node{
    int op;
    unique_ptr<Node> lval, rval;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    BinOpNode(int s, Node *lv, Node *rv) : Node(), op(s), lval(lv), rval(rv){}
    ~BinOpNode(){}
};

struct TypeNode : public Node{
    int type;
    string typeName; //used for usertypes
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    TypeNode(int ty, char* tName) : Node(), type(ty), typeName(tName){}
    ~TypeNode(){}
};

struct RetNode : public Node{
    unique_ptr<Node> expr;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    RetNode(Node* e) : Node(), expr(e){}
    ~RetNode(){}
};

struct NamedValNode : public Node{
    string name;
    unique_ptr<Node> typeExpr;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    NamedValNode(char* s, Node* t) : Node(), name(s), typeExpr(t){}
    ~NamedValNode(){}
};

struct VarNode : public Node{
    string name;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    VarNode(char* s) : Node(), name(s){}
    ~VarNode(){}
};

struct FuncCallNode : public Node{
    string name;
    unique_ptr<Node> params;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    FuncCallNode(char* s, Node* p) : Node(), name(s), params(p){}
    ~FuncCallNode(){}
};

struct StrLitNode : public Node{
    string val;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    StrLitNode(char* s) : Node(), val(s){}
    ~StrLitNode(){}
};

struct VarDeclNode : public Node{
    string name;
    unique_ptr<Node> typeExpr, expr;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    VarDeclNode(char* s, Node* t, Node* exp) : Node(), name(s), typeExpr(t), expr(exp){}
    ~VarDeclNode(){}
};

struct VarAssignNode : public Node{
    unique_ptr<VarNode> var;
    unique_ptr<Node> expr;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    VarAssignNode(Node* v, Node* exp) : Node(), var((VarNode*)v), expr(exp){}
    ~VarAssignNode(){}
};

struct IfNode : public ParentNode{
    unique_ptr<Node> condition;
    unique_ptr<IfNode> elseN;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    IfNode(Node* n1, Node* body, IfNode* els) : ParentNode(body), condition(n1), elseN(els){}
    ~IfNode(){}
};

struct FuncDeclNode : public ParentNode{
    string name;
    unique_ptr<Node> type;
    unique_ptr<NamedValNode> params;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    FuncDeclNode(char* s, Node* t, Node* p, Node* b) : ParentNode(b), name(s), type(t), params((NamedValNode*)p){}
    ~FuncDeclNode(){}
};

struct DataDeclNode : public ParentNode{
    string name;
    Value* compile(Compiler*, Module*);
    //void exec(void);
    void print(void);
    DataDeclNode(char* s, Node* b) : ParentNode(b), name(s){}
    ~DataDeclNode(){}
};


namespace ante{
    namespace parser{
        Node* getRootNode();
        void printBlock(Node *block);
        void parseErr(ParseErr e, string s, bool showTok);
    }
}

//extern "C" int yylex(...);
//extern "C" int yyparse();

#endif
