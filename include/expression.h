#ifndef EXPRESSION_H
#define EXPRESSION_H

#include "types.h"
#include "bignum.h"
#include "interpreter.h"
#include "operations.h"

typedef struct{
    TokenType op;
    uint8_t prec;
    uint8_t rAsso;
    Variable (*func)(Variable, Variable);
} Operator;

Variable expression(void);
Variable _expression(Variable, uint8_t);

#endif
