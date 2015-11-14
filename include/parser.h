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
        vector<Node> *children;
        Node *next;
        bool compile(void);
        char* operator<<(ostream o);
        ~Node(){
            delete next;
            delete children;
        }
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

        void parseErr(string s, bool showTok);
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
