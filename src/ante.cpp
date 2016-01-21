#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include <cstring>
#include <iostream>
using namespace ante;

void compile(char *fileName)
{
    lexer::init(fileName);
    int flag = yyparse();
    if(flag == PE_OK){
        Compiler *ac = new Compiler(parser::getRootNode());
        ac->compile();
    }else{ //parsing error, cannot compile
        puts("Compilation aborted.");
    }
}

int main(int argc, char *argv[]){
    if(argc == 2){
        //default = compile
        compile(argv[1]);
    }else if(argc >= 3){
        //lex and print tokens
        if(strcmp(argv[1], "-l") == 0){
            lexer::init(argv[2]);
            int t = lexer::next();
            while(t){
                lexer::printTok(t);
                putchar('\n');
                t = lexer::next();
            }
        //parse and print parse tree
        }else if(strcmp(argv[1], "-p") == 0){
            lexer::init(argv[2]);
            int flag = yyparse();
            cout << "Parser returned " << flag << endl;
            if(flag == PE_OK){
                parser::printBlock(parser::getRootNode());
            }
        //compile
        }else if(strcmp(argv[1], "-c") == 0){
            compile(argv[2]);
        }else{
            cout << "Ante: argument '" << argv[1] << "' was not recognized.\n";
        }
    }else if(argc == 1){
        puts("Ante: no arguments given, exiting.");
    }else{
        puts("Ante: Invalid argument count.");
    }
    return 0;
}
