#ifndef TYPES_H
#define TYPES_H

#include "lexer.h"

typedef unsigned char uint8_t;

typedef enum{ Object, Num, Int, String, Function, Invalid} Type;

#define ARR_SIZE(a) (sizeof(a) / sizeof(*a))

#define NFREE(x) if(x!=NULL) free(x)

typedef void* Value;

typedef void (*funcPtr)();

extern char *typeDictionary[];

typedef struct{
    int x;
    int y;
} Coords;

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
