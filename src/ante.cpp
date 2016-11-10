#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include "ptree.h"
#include "yyparser.h"
#include "args.h"
#include <cstring>
#include <iostream>
using namespace ante;

void parseFile(string &fileName){
    //parse and print parse tree
    setLexer(new Lexer(fileName.c_str()));
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
}

void printHelp(){
    puts("Compile for the Ante programming language\n");
    puts("Usage: ante [options] <inputs>");
}

int main(int argc, const char **argv){
    auto *args = parseArgs(argc, argv);
    if(args->hasArg(Args::Help)) printHelp();

    for(auto input : args->inputFiles){
        Compiler ante{input.c_str()};
        if(args->hasArg(Args::Parse))
            parseFile(input);

        ante.processArgs(args, input);
    }
    
    if(args->hasArg(Args::Eval))
        Compiler(0).eval();

    delete args;
    return 0;
}
