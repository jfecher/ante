#include "lexer.h"
#include <cstring>
#include <iostream>

int main(int argc, char *argv[]){
    if(argc == 2){
        if(strcmp(argv[1], "-l") == 0){
            istream *in = &cin;
            Lexer *lexer = new Lexer(&in);
            Token t = lexer->next();

            while(t.type != Tok_EndOfInput){
                std::cout << tokDictionary[t.type] << std::endl;
                t = lexer->next();
            }
        }
    }
    return 0;
}
