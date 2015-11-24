#ifndef PARSER_H
#define PARSER_H

#include "lexer.h"
#include "tokens.h"
#include <vector>
#include <cstdarg>

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
        ~Node(){ delete next; }
        virtual bool *compile(void) = 0;
        virtual bool *exec(void) = 0;
};

class IntLitNode : public Node{
    public:
        string val;
        IntLitNode(string s) : val(s){}
        bool *compile(void);
        bool *exec(void);
};

class BinOpNode : public Node{
    public:
        string op;
        Node *lval, *rval;
        BinOpNode(string s, Node *lv, Node *rv) : op(s), lval(lv), rval(rv){}
        bool *compile(void);
        bool *exec(void);
};

class IfNode : public Node{
    public:
        Node *condition;
        Node *body;
        IfNode(Node *n1, Node *n2) : condition(n1), body(n2){}
        bool *compile(void);
        bool *exec(void);
};

class NamedValNode : public Node{
    public:
        string name;
        Token type;
        NamedValNode(string s, Token t) : name(s), type(t){}
        bool *compile(void);
        bool *exec(void);
};

class VarNode : public Node{
    public:
        string name;
        VarNode(string s) : name(s){}
        bool *compile(void);
        bool *exec(void);
};

class FuncCallNode : public Node{
    public:
        string name;
        Node *params;
        FuncCallNode(string s, Node *p) : name(s), params(p){}
        bool *compile(void);
        bool *exec(void);
};

class StrLitNode : public Node{
    public:
        string val;
        StrLitNode(string s) : val(s){}
        bool *compile(void);
        bool *exec(void);
};

class VarDeclNode : public Node{
    public:
        Token type;
        string name;
        Node *expr;
        VarDeclNode(string s, Token t, Node *exp) : name(s), type(t), expr(exp){}
        bool *compile(void);
        bool *exec(void);
};

class FuncDeclNode : public Node{
    public:
        string name;
        Token type;
        NamedValNode *params;
        Node *body;
        FuncDeclNode(string s, Token t, NamedValNode *p, Node *b) : name(s), type(t), params(p), body(b){}
        bool *compile(void);
        bool *exec(void);
};

class ClassDeclNode : public Node{
    public:
        string name;
        Node *body;
        ClassDeclNode(string s, Node *b) : name(s), body(b){}
        bool *compile(void);
        bool *exec(void);
};

class Parser{
    public:
        Parser(const char* file);
        ParseErr parse(void);

    private:
        Lexer lexer;
        Node *root;
        Node *branch;
        Token c, n;
        ParseErr errFlag;

        void parseErr(ParseErr e, string s, bool showTok);
        void incPos(void);
        bool accept(TokenType t);
        bool _expect(TokenType t);
        bool acceptOp(char op);
        bool expectOp(char op);

        bool isType(TokenType t);
        
        bool parseValue(void);
        bool parseVariable(void);
        bool parseOp(void);
        
        Node* buildParseTree(void);
        NamedValNode* parseTypeList(void);
        Node* parseStmt(void);
        Node* parseIfStmt(void);
        Node* parseBlock(void);
        Node* parseClass(void);
        Node* parseGenericVar(void);
        Node* parseExpr(void);
        Node* parseRExpr(void);
        Node* parseGenericDecl(void);
};

#endif
