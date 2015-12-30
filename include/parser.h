#ifndef PARSER_H
#define PARSER_H

#include "lexer.h"
#include "tokens.h"
#include "cstdlib"
#include <vector>

enum ParseErr{
    PE_OK,
    PE_EXPECTED,
    PE_VAL_NOT_FOUND,
    PE_IDENT_NOT_FOUND,
    PE_INVALID_STMT,
};

class Node{
    public:
        Node *next;
        virtual void print(void) = 0;
        virtual void compile(void) = 0;
        virtual void exec(void) = 0;
        Node() : next(0){}
        ~Node(){ free(next); }
};

class IntLitNode : public Node{
    public:
        char* val;
        void compile(void);
        void exec(void);
        void print(void);
        IntLitNode(char* s) : val(s){}
};

class FltLitNode : public Node{
    public:
        char* val;
        void compile(void);
        void exec(void);
        void print(void);
        FltLitNode(char* s) : val(s){}
};

class BoolLitNode : public Node{
    public:
        bool val;
        void compile(void);
        void exec(void);
        void print(void);
        BoolLitNode(char b) : val(b){}
};

class BinOpNode : public Node{
    public:
        int op;
        Node *lval, *rval;
        ~BinOpNode(){ free(next); free(lval); free(rval); }
        void compile(void);
        void exec(void);
        void print(void);
        BinOpNode(int s, Node *lv, Node *rv) : op(s), lval(lv), rval(rv){}
};

class TypeNode : public Node{
    public:
        int type;
        char* typeName; //used for usertypes
        void compile(void);
        void exec(void);
        void print(void);
        TypeNode(int ty, char* tName) : type(ty), typeName(tName){}
};

class RetNode : public Node{
    public:
        Node* expr;
        void compile(void);
        void exec(void);
        void print(void);
        RetNode(Node* e) : expr(e){}
};

class IfNode : public Node{
    public:
        Node* condition;
        Node** body;
        void compile(void);
        void exec(void);
        void print(void);
        IfNode(Node* n1, Node** n2) : condition(n1), body(n2){}
};

class NamedValNode : public Node{
    public:
        char* name;
        Node* typeExpr;
        void compile(void);
        void exec(void);
        void print(void);
        NamedValNode(char* s, Node* t) : name(s), typeExpr(t){}
};

class VarNode : public Node{
    public:
        char* name;
        void compile(void);
        void exec(void);
        void print(void);
        VarNode(char* s) : name(s){}
};

class FuncCallNode : public Node{
    public:
        char* name;
        Node* params;
        void compile(void);
        void exec(void);
        void print(void);
        FuncCallNode(char* s, Node* p) : name(s), params(p){}
};

class StrLitNode : public Node{
    public:
        char* val;
        void compile(void);
        void exec(void);
        void print(void);
        StrLitNode(char* s) : val(s){}
};

class VarDeclNode : public Node{
    public:
        char* name;
        Node* typeExpr;
        Node* expr;
        void compile(void);
        void exec(void);
        void print(void);
        VarDeclNode(char* s, Node* t, Node* exp) : name(s), typeExpr(t), expr(exp){}
};

class VarAssignNode : public Node{
    public:
        char* name;
        Node* expr;
        void compile(void);
        void exec(void);
        void print(void);
        VarAssignNode(char* s, Node* exp) : name(s), expr(exp){}
};

class FuncDeclNode : public Node{
    public:
        char* name;
        Node* type;
        Node** params;
        Node** body;
        void compile(void);
        void exec(void);
        void print(void);
        FuncDeclNode(char* s, Node* t, Node** p, Node** b) : name(s), type(t), params(p), body(b){}
};

class DataDeclNode : public Node{
    public:
        char* name;
        Node** body;
        void compile(void);
        void exec(void);
        void print(void);
        DataDeclNode(char* s, Node** b) : name(s), body(b){}
};


namespace ante{
    namespace parser{
        static vector<Node*> parseTree;
        void printParseTree(void);
        void parseErr(ParseErr e, string s, bool showTok);
    }
}

//extern "C" int yylex(...);
//extern "C" int yyparse();

#endif
