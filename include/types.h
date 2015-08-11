#ifndef TYPES_H
#define TYPES_H

#include "lexer.h"

typedef unsigned char uint8_t;
typedef enum{ Object, Num, String, Function, Invalid} Type;

//typedef union{ long long i; double d; char c; char *s; } Value;
#define NFREE(x) if(x!=NULL) free(x)

typedef void* Value;

typedef void (*funcPtr)();

extern char *typeDictionary[];

typedef struct{
    int x;
    int y;
} Coords;

typedef struct{
    TokenType op;
    uint8_t precedence;
    uint8_t associativity;
} Operator;

typedef struct{
    Value value;
    Type type;
    char dynamic;
    char *name;
} Variable;

typedef struct{
    unsigned char isOp;
    Variable v;
    TokenType t;
} ExprValue;

#endif
