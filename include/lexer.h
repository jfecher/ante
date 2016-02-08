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

#define RETURN_PAIR(t) {incPos(2); return (t);}

namespace ante{
    /* Defined in src/compiler.cpp */
    /* General error function */
    void error(const char* msg, const char* fileName, unsigned int row, unsigned int col);
    
    class Lexer{
    public:
        const char* fileName; 
        
        Lexer(const char *file);
        ~Lexer();
        int next();
        int peek();
        unsigned int getRow();
        unsigned int getCol();
        
        static void printTok(int t);
        static string getTokStr(int t);
   
    private:
        /* the ifstream to take from */
        ifstream *in;
        
        /* Row and column number */
        unsigned int row, col;
        
        /* Row and column number of beginning of last token */
        unsigned int tokRow, tokCol;
        
        /* Current and next characters */
        char cur, nxt;



        /* Amount of spaces per indent */
        #define scStep 4

        /*
        *  Current scope (indent level) of file
        */
        int scope;

        /*
        *  Used to remember a new indentation level to issue multiple Indent
        *  or Unindent tokens when required.
        */
        int cscope;
        
        
        void incPos(void);
        void incPos(int end);
        
        void setlextxt(string *str);
        int handleComment(void);
        int genWsTok(void);
        int genNumLitTok(void);
        int genAlphaNumTok(void);
        int genStrLitTok(char delim);
    };
}


extern ante::Lexer *yylexer;
void setLexer(ante::Lexer *l);

#endif
