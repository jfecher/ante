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
};

class Node{
    public:
        virtual ~Node(){}
        virtual bool *compile(void) = 0;
        virtual bool *exec(void) = 0;
};

class IntLitNode{
    public:
        string val;
        virtual bool *compile(void);
        virtual bool *exec(void);
};

class BinOpNode{
    public:
        string op;
        Node *lval, *rval;
        virtual bool *compile(void);
        virtual bool *exec(void);
};

class VarNode{
    public:
        string name;
        virtual bool *compile(void);
        virtual bool *exec(void);
};

class StrLitNode{
    public:
        string val;
        virtual bool *compile(void);
        virtual bool *exec(void);
};

class VarDeclNode : Node{
    public:
        string type;
        string name;
        Node *expr;
        virtual bool *compile(void);
        virtual bool *exec(void);
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

        ParseErr parseErr(ParseErr e, string s, bool showTok);
        void incPos(void);
        bool accept(TokenType t);
        bool _expect(TokenType t);
        bool acceptOp(char op);
        bool expectOp(char op);

        bool isType(TokenType t);
        
        bool parseValue(void);
        bool parseVariable(void);
        bool parseOp(void);
        
        ParseErr parseTopLevelStmt(void);
        ParseErr parseTypeList(void);
        ParseErr parseStmt(void);
        ParseErr parseIfStmt(void);
        ParseErr parseBlock(void);
        ParseErr parseClass(void);
        ParseErr parseGenericVar(void);
        ParseErr parseExpr(void);
        ParseErr parseRExpr(void);
        
        ParseErr parseGenericDecl(void);
};

#endif
