#include "lexer.h"
#include <cstring>
#include <iostream>

int main(int argc, char *argv[]){
    if(argc == 2){
        cout << -1;
        if(strcmp(argv[1], "-l") == 0){
            //Lexer *lexer = new Lexer(&in);
            //Token t = lexer->next();
            /*while(t.type != Tok_EndOfInput){
            cout << -6;
                std::cout << tokDictionary[t.type] << std::endl;
                t = lexer->next();
            }*/
        }
    }else if(argc == 3){
        if(strcmp(argv[1], "-l") == 0){
            Lexer *lexer = new Lexer(argv[2]);
            Token t = lexer->next();

            while(t.type != Tok_EndOfInput){
                std::cout << tokDictionary[t.type] << std::endl;
                t = lexer->next();
            }
        }
    }
    return 0;
}
