#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include "ptree.h"
#include "yyparser.h"
#include <cstring>
#include <iostream>
using namespace ante;

int main(int argc, char *argv[]){
    if(argc == 2){
        //eval
        if(strcmp(argv[1], "-e") == 0){
            Compiler ante{0};
            ante.eval();
        }else{
            //default = compile
            Compiler ante{argv[1]};
            ante.compileNative();
        }
    }else if(argc >= 3){
        //lex and print tokens
        if(strcmp(argv[1], "-l") == 0){
            Lexer lexer = Lexer(argv[2]);
            yy::location loc;
            loc.initialize();
            int t = lexer.next(&loc);
            while(t){
                lexer.printTok(t);
                putchar('\n');
                t = lexer.next(&loc);
            }
        //parse and print parse tree
        }else if(strcmp(argv[1], "-p") == 0){
            setLexer(new Lexer(argv[2]));
            yy::parser p{};
            int flag = p.parse();
            if(flag == PE_OK){
                Node* root = parser::getRootNode();
                parser::printBlock(root);
                delete root;
            }else{
                //print out remaining errors
                int tok;
                yy::location loc;
                loc.initialize();
                while((tok = yylexer->next(&loc)) != Tok_Newline && tok != 0);
                while(p.parse() != PE_OK && yylexer->peek() != 0);
            }
        //compile
        }else if(strcmp(argv[1], "-c") == 0){
            Compiler ante{argv[2]};
            ante.compileNative();
        }else if(strcmp(argv[1], "-r") == 0){ //compile and run
            Compiler ante{argv[2]};
            ante.compileNative();
            if(!ante.errFlag){
                system(("./" + removeFileExt(ante.fileName)).c_str());
            }
        }else if(strcmp(argv[1], "-emit-llvm") == 0){
            Compiler ante{argv[2]};
            ante.emitIR();
        }else if(strcmp(argv[1], "-o") == 0){
            if(strcmp(argv[2], "-lib") == 0){
                Compiler ante{argv[3], true};
                ante.compileObj();
            }else{
                Compiler ante{argv[2]};
                ante.compileObj();
            }
        }else{
            cout << "Ante: argument '" << argv[1] << "' was not recognized.\n";
        }
    }else if(argc == 1){
        puts("Ante: no arguments given, exiting.");
    }
    return 0;
}
