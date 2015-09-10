#ifndef EXPRESSION_H
#define EXPRESSION_H

#include "types.h"
#include "interpreter.h"

#define VAR(v, t) ((Variable){v, t, 0, NULL})

typedef Variable (*opFunc)(Variable, Variable);

typedef struct{
    TokenType op;
    uint8_t prec;
    uint8_t rAsso;
    Variable (*func)(Variable, Variable);
} Operator;

Variable expression(void);
Variable _expression(Variable, uint8_t);

Variable op_add(Variable, Variable);
Variable op_mul(Variable, Variable);

#endif
