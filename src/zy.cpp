#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include <cstring>
#include <iostream>

int main(int argc, char *argv[]){
    if(argc == 2){
        //default = compile
        Compiler zc = Compiler(0);
        zc.compile();
    }else if(argc == 3){
        if(strcmp(argv[1], "-l") == 0){
            Lexer *lexer = new Lexer(argv[2]);
            Token t = lexer->next();

            while(t.type != Tok_EndOfInput){
                if(t.type == Tok_Operator)
                    std::cout << tokDictionary[t.type] << " " << t.lexeme[0] << std::endl;
                else
                    std::cout << tokDictionary[t.type] << std::endl;
                t = lexer->next();
            }
        }else if(strcmp(argv[1], "-p") == 0){
            Parser p = Parser(argv[2]);
            cout << p.parse() << endl;
        }
    }
    return 0;
}
