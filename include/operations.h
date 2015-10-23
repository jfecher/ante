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
Variable op_les(Variable, Variable);
Variable op_grt(Variable, Variable);
Variable op_eq(Variable, Variable);
Variable op_neq(Variable, Variable);
Variable op_geq(Variable, Variable);
Variable op_leq(Variable, Variable);

#endif
