#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include <cstring>
#include <iostream>
using namespace ante;

int main(int argc, char *argv[]){
    if(argc == 2){
        //default = compile
        Compiler zc = Compiler(0);
        zc.compile();
    }else if(argc == 3){
        if(strcmp(argv[1], "-l") == 0){
            lexer::init(argv[2]);
            int t = lexer::next();

            while(t != Tok_EndOfInput){
                lexer::printTok(t);
                t = lexer::next();
            }
        }else if(strcmp(argv[1], "-p") == 0){
            //Parser p = Parser(argv[2]);
            //p.parse();
            //p.printParseTree();
            lexer::init(argv[2]);
            yyparse();
        }
    }
    return 0;
}
