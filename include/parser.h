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
        ~Node(){ free(next); }
};

class IntLitNode : public Node{
    public:
        string val;
        void compile(void);
        void exec(void);
        void print(void);
        IntLitNode(string s) : val(s){}
};

class FltLitNode : public Node{
    public:
        string val;
        void compile(void);
        void exec(void);
        void print(void);
        FltLitNode(string s) : val(s){}
};

class BoolLitNode : public Node{
    public:
        bool val;
        void compile(void);
        void exec(void);
        void print(void);
        BoolLitNode(bool b) : val(b){}
};

class BinOpNode : public Node{
    public:
        Token op;
        Node *lval, *rval;
        ~BinOpNode(){ free(next); free(lval); free(rval); }
        void compile(void);
        void exec(void);
        void print(void);
        BinOpNode(Token s, Node *lv, Node *rv) : op(s), lval(lv), rval(rv){}
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
        vector<Node*> body;
        void compile(void);
        void exec(void);
        void print(void);
        IfNode(Node* n1, vector<Node*> n2) : condition(n1), body(n2){}
};

class NamedValNode : public Node{
    public:
        string name;
        Token type;
        void compile(void);
        void exec(void);
        void print(void);
        NamedValNode(string s, Token t) : name(s), type(t){}
};

class VarNode : public Node{
    public:
        string name;
        void compile(void);
        void exec(void);
        void print(void);
        VarNode(string s) : name(s){}
};

class FuncCallNode : public Node{
    public:
        string name;
        Node* params;
        void compile(void);
        void exec(void);
        void print(void);
        FuncCallNode(string s, Node* p) : name(s), params(p){}
};

class StrLitNode : public Node{
    public:
        string val;
        void compile(void);
        void exec(void);
        void print(void);
        StrLitNode(string s) : val(s){}
};

class VarDeclNode : public Node{
    public:
        string name;
        Token type;
        Node* expr;
        void compile(void);
        void exec(void);
        void print(void);
        VarDeclNode(string s, Token t, Node* exp) : name(s), type(t), expr(exp){}
};

class VarAssignNode : public Node{
    public:
        string name;
        Node* expr;
        void compile(void);
        void exec(void);
        void print(void);
        VarAssignNode(string s, Node* exp) : name(s), expr(exp){}
};

class FuncDeclNode : public Node{
    public:
        string name;
        Token type;
        vector<NamedValNode*> params;
        vector<Node*> body;
        void compile(void);
        void exec(void);
        void print(void);
        FuncDeclNode(string s, Token t, vector<NamedValNode*> p, vector<Node*> b) : name(s), type(t), params(p), body(b){}
};

class ClassDeclNode : public Node{
    public:
        string name;
        vector<Node*> body;
        void compile(void);
        void exec(void);
        void print(void);
        ClassDeclNode(string s, vector<Node*> b) : name(s), body(b){}
};

class Parser{
    public:
        Parser(const char* file);
        ParseErr parse(void);
        void printParseTree(void);

    private:
        Lexer lexer;
        vector<Node*> parseTree;
        Token c, n;
        ParseErr errFlag;

        void parseErr(ParseErr e, string s, bool showTok);
        void incPos(void);
        bool accept(TokenType t);
        bool expect(TokenType t);
        bool acceptOp(char op);
        bool expectOp(char op);

        bool isType(TokenType t);
        
        Node* parseValue(void);
        Node* parseVariable(void);
        Node* parseOp(void);
        
        void buildParseTree(void);
        
        vector<NamedValNode*> parseTypeList(void);
        Node* parseStmt(void);
        IfNode* parseIfStmt(void);
        RetNode* parseRetStmt(void);
        vector<Node*> parseBlock(void);
        ClassDeclNode* parseClass(void);
        Node* parseGenericVar(void);
        Node* parseExpr(void);
        Node* parseRExpr(void);
        Node* parseGenericDecl(void);
};

#endif
