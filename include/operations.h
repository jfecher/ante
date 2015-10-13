#ifndef OPERATIONS_H
#define OPERATIONS_H

#include <string.h>
#include "types.h"
#include "bignum.h"
#include "interpreter.h"

extern Variable copyVar(Variable);

Variable op_add(Variable, Variable);
Variable op_sub(Variable, Variable);
Variable op_mul(Variable, Variable);
Variable op_div(Variable, Variable);
Variable op_mod(Variable, Variable);
Variable op_pow(Variable, Variable);
Variable op_cnct(Variable, Variable);
Variable op_tup(Variable, Variable);

#endif
