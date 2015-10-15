/*
 *  Provides c functions always included by default in a .zy file.
 */
#include "integralLib.h"

//  void> system: string s
Variable zy_system(Variable params)
{
    struct Tuple *tup = params.value;
    if(tup->size != 1){
        printf("system: invalid number of arguments.  Expected 1, got %d.\n", tup->size);
        return VAR(NULL, Invalid);
    }else if(tup->tup[0].type != String){
        printf("system: type error in arguments.  Expected string, got %s.\n", typeDictionary[tup->tup[0].type]);
        return VAR(NULL, Invalid); 
    }

    system(tup->tup[0].value);
    return VAR(NULL, Invalid);
}

// void> typeof: v
Variable zy_typeof(Variable params){
    struct Tuple *tup = params.value;

    for(int i = 0; i < tup->size; i++){
        if(tup->tup[i].dynamic)
            fputs("dynamic ", stdout);
        
        fputs(typeDictionary[tup->tup[i].type], stdout);
        
        if(i + 1 < tup->size) 
            fputs(", ", stdout);
    }
    putchar('\n');
    return VAR(NULL, Invalid);
}
