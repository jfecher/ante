#ifndef STACK_H
#define STACK_H

#include <stdlib.h>
#include <stdio.h>
#include "types.h"
#include "table.h"

#define STACK_TYPE VarTable

typedef struct{
    STACK_TYPE *items;
    int size;
} Stack;

Stack stack;

#define stack_top(s) (s.items[s.size-1])
void stack_push( Stack*s, STACK_TYPE item );
STACK_TYPE stack_pop( Stack*s );

void stack_free();

#endif
