#include "table.h"

#include "stdio.h"

void free_var(Variable v)
{
    switch(v.type){
    case Num:
        mpf_clear(*(BigNum)v.value);
        break;
    case Int:
        mpz_clear(*(BigInt)v.value);
        break;
    default:
        NFREE(v.value);
        break;
    }
    NFREE(v.name);
}

inline void varTable_add(VarTable *t, Variable v)
{
    t->size++;
    t->table = realloc(t->table, t->size * sizeof(Variable));
    t->table[t->size-1] = v;
}

void varTable_remove(VarTable *t, unsigned int i)
{
    free_var(t->table[i]);

    for(i += 1; i < t->size; i++)
        t->table[i-1] = t->table[i]; 
    
    t->table = realloc(t->table, t->size--);
}

inline void varTable_free(VarTable t)
{
    for(int i=0; i < t.size; i++)
        free_var(t.table[i]);
    free(t.table);
}
