#ifndef SCANLINE_H
#define SCANLINE_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <termios.h>
#include <lexer.h>

#define SL_HISTORY_LEN 15
char **sl_history;

void init_sl();
void freeHistory();
void setupTerm();
void scanLine();

#endif
