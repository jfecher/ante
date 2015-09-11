#ifndef INTERPRETER_H
#define INTERPRETER_H

#include "parser.h"
#include "types.h"
#include "stack.h"
#include "bignum.h"
#include "table.h"
#include "expression.h"

#define VERSION "v0.0.08"
#define VERDATE "2015-09-10"

Token*toks;
int tIndex;

#define ERR_NOT_INITIALIZED "%s has not been initialized.\n"
#define ERR_TYPE_MISMATCH "Attempted to set %s to an incompatible type.\n"
#define ERR_ALREADY_INITIALIZED "%s has already been initialized.\n"

#define runtimeError(x,y) {printf(x,y); return;}
#define getCoords(c,v) Coords c=lookupVar(v);if(c.x==-1){runtimeError(ERR_NOT_INITIALIZED,v);return;};

#define CPY_TO_STR(newStr, cpyStr) { int len=strlen(cpyStr); newStr=realloc(newStr,len+1); strcpy(newStr, cpyStr);}
#define CPY_TO_NEW_STR(newStr, cpyStr) char*newStr=NULL; CPY_TO_STR(newStr,cpyStr);
#define INC_POS(x) (tIndex += x)
#define IS_OPERATOR(t) (t==Tok_Plus||t==Tok_Minus||t==Tok_Multiply||t==Tok_Divide||t==Tok_Exponent||t==Tok_StrConcat)

void interpret(FILE*, char);
Coords lookupVar(char*);
Variable getValue(Token);

void op_initObject(void);
void op_assign(void);
void op_print(void);
void op_function(void);
void op_initNum(void);
void op_initInt(void);
void op_initStr(void);
void op_typeOf(void);

#endif
