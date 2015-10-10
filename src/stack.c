#include "stack.h"

void 
stack_push( Stack*s, STACK_TYPE item )
{
    s->size++;
    STACK_TYPE *sPtr = realloc(s->items, sizeof(STACK_TYPE) * s->size );

    if(!sPtr)
    {
        puts("Insufficient memory while reallocating the stack");
        exit(10);
    }

    s->items = sPtr;
    s->items[s->size-1] = item;
}

STACK_TYPE
stack_pop( Stack*s )
{
    STACK_TYPE top = s->items[s->size-1];
    s->size--;
    STACK_TYPE *sPtr = realloc(s->items, sizeof(STACK_TYPE) * s->size);
    if(s->size > 0 && !sPtr)
    {
        puts("Insufficient memory while reallocating the stack");
        exit(11);
    }
    s->items = sPtr;
    return top;
}

void stack_free(Stack s)
{
    for(int i = 0; i < s.size; i++)
        varTable_free(s.items[i]);
    free(s.items);
}
