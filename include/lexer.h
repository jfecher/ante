#ifndef LEXER_H
#define LEXER_H

#include "tokens.h"
#include "error.h"
#include <iostream>
#include <fstream>
#include <stack>
#include <map>

namespace ante { struct Node; }
#ifndef YYSTYPE
#  define YYSTYPE ante::Node*
#endif
#include "yyparser.h"

#define IS_COMMENT(c, n) ((c) == '/' && ((n) == '/' || (n) == '*'))
#define IS_NUMERICAL(c)  (c >= 48  && c <= 57)
#define IS_ALPHANUM(c)   (IS_NUMERICAL(c) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 95)
#define IS_WHITESPACE(c) (c == ' ' || c == '\t' || c == '\n' || c == 13) // || c == 130

#define RETURN_PAIR(t) {incPos(2); loc->end = getPos(); return (t);}

namespace ante{
    extern bool colored_output;

    class Lexer{
    public:
        std::string *fileName;

        Lexer(std::string* fileName);
        Lexer(std::string* fileName, std::string& pseudoFile, unsigned int rowOffset, unsigned int colOffset);
        ~Lexer();
        int next(yy::parser::location_type* yyloc);
        char peek() const;

        static void printTok(int t);
        static std::string getTokStr(int t);

    private:
        /* the ifstream to take from */
        std::ifstream *in;

        /* If this is set to true then the psuedoFile string should be parsed
         * as a string containing ante src code.  Used for Str interpolation */
        bool isPseudoFile;
        char* pseudoFile;

        /* Row and column number */
        unsigned int row, col;

        /* Offset given if lexer starts in the middle of a file */
        /* Used when lexing string interpolations */
        const unsigned int rowOffset, colOffset;

        /* Current and next characters */
        char cur, nxt;

        /*
        *  Current scope (indent level) of file
        */
        std::stack<unsigned int> *scopes;

        /*
         *  Current and previous tokens to match;
         *  All whitespace is insensitive while this is matching.
         *  Used with toks such as (), {}, and []
         */
        std::stack<char> matchingToks;

        /*
        *  Used to remember a new indentation level to issue multiple Indent
        *  or Unindent tokens when required.
        */
        unsigned int cscope;

        bool shouldReturnNewline;

        void lexErr(const char *msg, yy::parser::location_type* loc);

        void incPos(void);
        void incPos(int end);
        yy::position getPos(bool inclusiveEnd = true) const;

        void setlextxt(std::string *str);
        int handleComment(yy::parser::location_type* loc);
        int genWsTok(yy::parser::location_type* loc);
        int genNumLitTok(yy::parser::location_type* loc);
        int genAlphaNumTok(yy::parser::location_type* loc);
        int genStrLitTok(yy::parser::location_type* loc);
        int genCharLitTok(yy::parser::location_type* loc);
        int genOpTok(yy::parser::location_type* loc);
        int skipWsAndReturnNext(yy::parser::location_type* loc);
    };
}


extern ante::Lexer *yylexer;
void setLexer(ante::Lexer *l);

#endif
