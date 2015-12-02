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
#define RETURN_PAIR(t) {incPos(2); return (t);}

//Not a class because there will only ever be a single instance, which will need to be accessed
//by yacc's parser in c.
namespace ante{
    namespace lexer{
        void init(const char *file);
        int next(void);
        void printTok(int t);
    
        void incPos(void);
        void incPos(int end);
        int handleComment(void);
        int genWsTok(void);
        int genNumLitTok(void);
        int genAlphaNumTok(void);
        int genStrLitTok(char delim);
    }
}

//c api for yacc's parser
extern "C" int yylex(...);

#endif
