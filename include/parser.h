#ifndef PARSER_H_INCLUDED
#define PARSER_H_INCLUDED

#include "lexer.h"

#define NEW_BLOCK 3

int parse(); //Parses file, returns > 0 if an error occurs
void debugLog(const char *s);

char exitFlag;

#endif
