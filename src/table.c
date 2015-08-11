#include "table.h"

#include "stdio.h"

inline void varTable_add(VarTable *t, Variable v)
{
    t->size++;
    t->table = realloc(t->table, t->size * sizeof(Variable));
    t->table[t->size-1] = v;
}

void varTable_remove(VarTable *t, unsigned int i)
{
    Variable v = t->table[i];
    NFREE(v.value);
    NFREE(v.name);

    for(i += 1; i < t->size; i++){
        t->table[i-1] = t->table[i]; 
    }
    
    t->table = realloc(t->table, t->size--);
}

void varTable_free(VarTable t)
{
    int i;
    for(i=0; i < t.size; i++){
        NFREE(t.table[i].value);
        NFREE(t.table[i].name);
    }
    free(t.table);
}
