#include "util.h"

inline char streq(char *s1, char *s2){
    size_t len1 = strlen(s1);
    size_t len2 = strlen(s2);
    if(len1 != len2) return 0;
    for(int i = 0; i < len1; i++)
        if(s1[i] != s2[i])
            return 0;
    return 1;
}

/*
 *  Returns a fully initiallized char* copied
 *  from src string
 */
inline char* newstr(char *src){
    size_t len = strlen(src);
    char *ret = malloc(len+1);
    memcpy(ret, src, len+1);
    return ret;
}
