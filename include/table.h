#ifndef TABLE_H
#define TABLE_H

#include <stdlib.h>
#include "types.h"

#define ARR_SIZE(a) (sizeof(a) / sizeof(a[0]))

typedef struct{
    Variable *table;
    unsigned int size;
}VarTable;

void varTable_add();
void varTable_remove();
void varTable_free();

#endif
