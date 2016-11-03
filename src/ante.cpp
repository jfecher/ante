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

int main(int argc, const char **argv){
    auto *args = parseArgs(argc, argv);

    for(auto input : args->inputFiles){
        Compiler ante{input.c_str(), args->hasArg(Args::Lib)};
        
        if(args->hasArg(Args::Parse)) parseFile(input);
        if(args->hasArg(Args::EmitLLVM)) ante.emitIR();
        
        if(args->hasArg(Args::OutputName)) ante.compileObj();
        else ante.compileNative();

        if(!ante.errFlag && args->hasArg(Args::CompileAndRun)){
            system(("./" + removeFileExt(ante.fileName)).c_str());
        }
    }
    
    if(args->hasArg(Args::Eval))
        Compiler(0).eval();

    delete args;
    return 0;
}
