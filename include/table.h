#ifndef TABLE_H
#define TABLE_H

#include <stdlib.h>
#include "types.h"
#include "bignum.h"

typedef struct{
    Variable *table;
    unsigned int size;
}VarTable;

void varTable_add(VarTable*, Variable);
void varTable_remove(VarTable*, unsigned int);
void varTable_free(VarTable);

void free_var(Variable);
void free_value(Variable);

#endif
