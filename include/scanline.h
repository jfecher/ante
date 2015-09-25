#ifndef SCANLINE_H
#define SCANLINE_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <termios.h>
#include <lexer.h>

void setupTerm();
void scanLine();

#endif
