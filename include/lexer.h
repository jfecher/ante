#ifndef LEXER_H
#define LEXER_H

#include "tokens.h"
#include <iostream>
#include <fstream>
#include <map>
using namespace std;

#define IS_COMMENT(c)    (c == '`' || c == '~')
#define IS_NUMERICAL(c)  (c >= 48  && c <= 57)
#define IS_ALPHANUM(c)   (IS_NUMERICAL(c) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 95)
#define IS_WHITESPACE(c) (c == ' ' || c == '\t' || c == '\n' || c == 13) // || c == 130

#define PAIR(a, b) (c==a && n==b)
#define RETURN_PAIR(t) {incPos(2); return (Token){t, NULL};}

class Lexer{
    public:
        Lexer(void);
        Lexer(const char* file);
        Lexer(ifstream** file);
        Token next(void);
    private:
        char c, n;
        ifstream *in;
        static const char scStep = 4;
        unsigned short scope;
        unsigned short cscope;

        void incPos(void);
        void incPos(int end);
        Token handleComment(void);
        Token genWsTok(void);
        Token genNumLitTok(void);
        Token genAlphaNumTok(void);
        Token genStrLitTok(void);
};


#endif
