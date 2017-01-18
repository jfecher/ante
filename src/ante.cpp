#include "lexer.h"
#include "parser.h"
#include "compiler.h"
#include "ptree.h"
#include "yyparser.h"
#include "args.h"
#include "target.h"
#include <cstring>
#include <iostream>
#include <llvm/Support/TargetRegistry.h>
using namespace ante;

void parseFile(string &fileName){
    //parse and print parse tree
    setLexer(new Lexer(&fileName));
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
        while((tok = yylexer->next(&loc)) != Tok_Newline && tok != 0);
        while(p.parse() != PE_OK && yylexer->peek() != 0);
    }
}

void printHelp(){
    puts("Compiler for the Ante programming language\n");
    puts("Usage: ante [options] <inputs>");
    puts("options:");
    puts("\t-c\t\tcompile to object file");
    puts("\t-o <filename>\tspecify output name");
    puts("\t-p\t\tprint parse tree");
    puts("\t-0 <number>\tSet optimization level. Arg of 0 = none, 3 = all");
    puts("\t-r\t\tcompile and run");
    puts("\t-help\t\tprint this message");
    puts("\t-lib\t\tcompile as library (include all functions in binary and compile to object file)");
    puts("\t-emit-llvm\tprint llvm-IR as output");

    puts("\nNative target: " AN_TARGET_TRIPLE);

	TargetRegistry::printRegisteredTargetsForVersion();
}

int main(int argc, const char **argv){
	LLVMInitializeNativeTarget();
	LLVMInitializeNativeAsmPrinter();

    auto *args = parseArgs(argc, argv);
    if(args->hasArg(Args::Help)) printHelp();

    for(auto input : args->inputFiles){
        Compiler ante{input.c_str()};
        if(args->hasArg(Args::Parse))
            parseFile(input);

        ante.processArgs(args);
    }
    
    if(args->hasArg(Args::Eval))
        Compiler(0).eval();

    if(yylexer)
        delete yylexer;
    delete args;
    return 0;
}


win_console_color getBackgroundColor() {
	CONSOLE_SCREEN_BUFFER_INFO csbi;
	GetConsoleScreenBufferInfo(GetStdHandle(STD_OUTPUT_HANDLE), &csbi);
	int a = csbi.wAttributes;
	return (win_console_color)((a / 16) % 16);
}

void setcolor(win_console_color foreColor, win_console_color backColor) {
	int fc = foreColor % 16;
	int bc = backColor % 16;

	unsigned short wAttr = ((unsigned)backColor << 4) | (unsigned)foreColor;
	SetConsoleTextAttribute(GetStdHandle(STD_OUTPUT_HANDLE), wAttr);
}

std::ostream& operator<<(std::ostream& os, win_console_color color) {
	os.flush();
	setcolor(color, getBackgroundColor());
	return os;
}