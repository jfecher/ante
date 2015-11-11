#ifndef PARSER_H
#define PARSER_H

#include "lexer.h"
#include "tokens.h"
#include <vector>
#include <cstdarg>

enum ParseErr{
    PE_OK,
    PE_Expected,
};

class Node{
    public:
        vector<Node> children;
        virtual bool compile(void);
};

class Parser{
    public:
        Parser(const char* file);
        ParseErr parse(void);
    private:
        Lexer lexer;
        Node parseTree;
        Token c, n;

        void parseErr(string s, ...);
        void incPos(void);
        bool accept(TokenType t);
        bool expect(TokenType t);

        ParseErr parseTopLevelStatement();
        ParseErr parseStatement();
};

#endif
