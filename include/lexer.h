#ifndef LEXER_H_INCLUDED
#define LEXER_H_INCLUDED

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "types.h"

char printToks;
char isTty;
unsigned short row;
unsigned short col;

//A dictionary used for getting the human readable string of a particular token type.  Only used in debugging
extern char *tokenDictionary[];

//extern char *srcLine;
extern char *pos;

//Source file
FILE *src;

#define KEYWORD_COLOR  "\033[0;31m"
#define FUNCTION_COLOR "\033[0;32m"
#define STRINGL_COLOR  "\033[0;33m"
#define TYPE_COLOR     "\033[0;34m"
#define INTEGERL_COLOR "\033[0;35m"
#define RESET_COLOR    "\033[0;m"

//Returns 1 if character is an uppercase or lowercase letter, a number, or an underscore
#define IS_ALPHA_NUMERIC(c) ((c >= 48 && c <= 57) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 95)

//Returns 1 if character is a number
#define IS_NUMERIC(c) (c >= 48 && c <= 57)

//Returns 1 if character is whitespace
#define IS_WHITESPACE(c) (c==' ' || c=='\t' || c=='\n' || c==130 || c==13)

#define IS_WHITESPACE_TOKEN(t) (t.type==Tok_Newline||t.type==Tok_Indent||t.type==Tok_Unindent)

void   init_lexer(char*); //begins lexation of file
Token* lexer_next(char); //gets line of tokens.  if passed true, it prints them as well
void   freeToks(Token**);
void   freeSrcLine(void);
void   lexAndPrint(void);
void   lexer_printWhitespace(char);
void   lexer_printTokens(char);
void   ralloc(char**, size_t);

#endif // LEXER_H_INCLUDED
