#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include <cstring>
#include <iostream>
using namespace ante;

extern "C" int yyparse();

int main(int argc, char *argv[]){
    if(argc == 2){
        //default = compile
        Compiler zc = Compiler(0);
        zc.compile();
    }else if(argc == 3){
        //lex and print tokens
        if(strcmp(argv[1], "-l") == 0){
            lexer::init(argv[2]);
            int t = lexer::next();

            while(t){
                lexer::printTok(t);
                t = lexer::next();
            }

        //parse and print parse tree
        }else if(strcmp(argv[1], "-p") == 0){
            lexer::init(argv[2]);
            yyparse();
        }
    }else if(argc == 1){
        puts("Ante: no arguments give, exiting.");
    }
    return 0;
}
