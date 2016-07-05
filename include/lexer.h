#ifndef LEXER_H
#define LEXER_H

#include "tokens.h"
#include <iostream>
#include <fstream>
#include <stack>
#include <map>
using namespace std;

struct Node;
#ifndef YYSTYPE
#  define YYSTYPE Node*
#endif
#include "yyparser.h"

#define IS_COMMENT(c)    (c == '`' || c == '~')
#define IS_NUMERICAL(c)  (c >= 48  && c <= 57)
#define IS_ALPHANUM(c)   (IS_NUMERICAL(c) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 95)
#define IS_WHITESPACE(c) (c == ' ' || c == '\t' || c == '\n' || c == 13) // || c == 130

#define RETURN_PAIR(t) {incPos(2); loc->end = yy::position(fName, row, col); return (t);}

namespace ante{
    /* Defined in src/compiler.cpp */
    /* General error function */
    void error(const char* msg, yy::location& loc);

    class Lexer{
    public:
        const char* fileName; 
        
        Lexer(const char *file);
        ~Lexer();
        int next(yy::parser::location_type* yyloc);
        char peek() const;

        static void printTok(int t);
        static string getTokStr(int t);
   
    private:
        /* the ifstream to take from */
        ifstream *in;

        /* Row and column number */
        unsigned int row, col;
        
        /* Current and next characters */
        char cur, nxt;

        /*
        *  Current scope (indent level) of file
        */
        stack<unsigned int> *scopes;

        /*
         *  Current and previous tokens to match;
         *  All whitespace is insensitive while this is matching.
         *  Used with toks such as (), {}, and []
         */
        stack<char> matchingToks;

        /*
        *  Used to remember a new indentation level to issue multiple Indent
        *  or Unindent tokens when required.
        */
        unsigned int cscope;
        
        bool shouldReturnNewline;
        
        void lexErr(const char *msg, yy::parser::location_type* loc);
        
        void incPos(void);
        void incPos(int end);
        
        void setlextxt(string *str);
        int handleComment(yy::parser::location_type* loc);
        int genWsTok(yy::parser::location_type* loc);
        int genNumLitTok(yy::parser::location_type* loc);
        int genAlphaNumTok(yy::parser::location_type* loc);
        int genStrLitTok(char delim, yy::parser::location_type* loc);
        int genOpTok(yy::parser::location_type* loc);
        int skipWsAndReturnNext(yy::parser::location_type* loc);
    };
}


extern ante::Lexer *yylexer;
void setLexer(ante::Lexer *l);

#endif
