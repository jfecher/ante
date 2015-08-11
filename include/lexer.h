#ifndef LEXER_H_INCLUDED
#define LEXER_H_INCLUDED

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

//Enum containing every Token's Type
typedef enum TokenType{
    Tok_Greater, //Used to signal the initialization of variables as well as comparing values
    Tok_Identifier,
    Tok_Print,
    Tok_Function,
    Tok_Num,
    Tok_String,

    Tok_Invalid,
    Tok_Begin,
	Tok_Assign,
	Tok_Multiply,
	Tok_Divide,
	Tok_Plus,
	Tok_Minus,
	Tok_PlusEquals,
	Tok_MinusEquals,
    Tok_EqualsEquals,
	Tok_GreaterEquals,
	Tok_Equals,
	Tok_LesserEquals,
	Tok_Lesser,
    Tok_Modulus,
	Tok_BraceOpen,
	Tok_BraceClose,
	Tok_ParenOpen,
	Tok_ParenClose,
	Tok_BracketOpen,
	Tok_BracketClose,
	Tok_Underscore,
	Tok_Comma,
	Tok_Colon,
    Tok_ListInitializer, // The | in the example list of strings: string|>myStringList = "This", "is", "an", "example"
    Tok_Char,
    Tok_Boolean,
    Tok_BooleanOr,
    Tok_BooleanAnd,
    Tok_BooleanTrue,
    Tok_BooleanFalse,
	Tok_IntegerLiteral,
	Tok_DoubleLiteral,
	Tok_StringLiteral,
    Tok_CharLiteral,
	Tok_MultiplyEquals,
    Tok_DivideEquals,
	Tok_Return,
	Tok_If,
	Tok_Else,
	Tok_For,
	Tok_While,
	Tok_Continue,
	Tok_Break,
    Tok_Import,
    Tok_Newline,
    Tok_TypeDef,
    Tok_Indent,
    Tok_Unindent,
	Tok_EndOfInput,
    Tok_StrConcat,
    Tok_MalformedString,
    Tok_MalformedChar,
    Tok_Exponent
} TokenType;

char printToks;
char isTty;

//A dictionary used for getting the human readable string of a particular token type.  Only used in debugging
extern char *tokenDictionary[];

extern char *srcLine;
extern char *pos;
//The basic Token construct.
//TODO: possibly expand to include row and column number for use in syntax errors.
typedef struct Token{
    TokenType type;
    char *lexeme;
} Token;

//Source file
FILE *src;

#define KEYWORD_COLOR  "\033[0;31m"
#define STRINGL_COLOR  "\033[0;33m"
#define INTEGERL_COLOR "\033[0;36m"
#define FUNCTION_COLOR "\033[0;32m"
#define RESET_COLOR    "\033[0;m"
extern char*color;

//Returns 1 if character is an uppercase or lowercase letter, a number, or an underscore
#define IS_ALPHA_NUMERIC(c) ((c >= 48 && c <= 57) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 95)

//Returns 1 if character is a number
#define IS_NUMERIC(c) (c >= 48 && c <= 57)

//Returns 1 if character is whitespace
#define IS_WHITESPACE(c) (c==' ' || c=='\t' || c=='\n' || c==130 || c==13)

#define IS_ENDING_TOKEN(t) (t==Tok_EndOfInput||t==Tok_Unindent||t==Tok_Indent||t==Tok_Newline)

void   initialize_lexer(int tty); //begins lexation of file
Token* lexer_next(char b); //gets line of tokens.  if b is true, it prints them as well
void   freeToks(Token **t);
void   lexer_printWhitespace(char c);
void   lexer_printTokens(char c);
void   ralloc(char** ptr, size_t size);

#endif // LEXER_H_INCLUDED
