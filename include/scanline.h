#ifndef SCANLINE_H
#define SCANLINE_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <termios.h>
#include <sys/ioctl.h>
#include <lexer.h>

void init_sl(void);
void freeHistory(void);
void scanBlock(char**);
void scanLine(char**);

#endif
