#ifndef COMPILER_H
#define COMPILER_H

#include "types.h"

#define COMP_NDEF_ERR "Variable '%s' is used but never declared.\n"

bool compile(Token);

#endif
