#ifndef EXPRESSION_H
#define EXPRESSION_H

#include "types.h"
#include "bignum.h"
#include "interpreter.h"
#include "operations.h"

typedef char (*tCheckF)(Type, Type);

typedef struct{
    TokenType op;
    uint8_t prec;
    uint8_t rAsso;
    opFunc func;
    tCheckF typeImpl; //1 if op is implemented for given types
} Operator;

Variable expression(void);
Variable _expression(Variable, uint8_t);

char tc_any(Type, Type);
char tc_num(Type, Type);
char tc_str(Type, Type);

#endif
