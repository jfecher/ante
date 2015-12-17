/*
 *      ptree.cpp
 *  Provide a C API to be used in parser.c generated from
 *  syntax.y which creates and links nodes to a parse tree.
 */
#include "parser.h"

Node *root;
Node *stmt;
Node *branch;

extern "C" void makeNode()
{
    
}
