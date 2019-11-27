#define NOMINMAX

#include <chrono>
#include <llvm/Support/TargetRegistry.h>
#include <llvm/Support/raw_os_ostream.h>
#include "compapi.h"
#include "target.h"
#include "module.h"
#include "typeinference.h"
#include "nameresolution.h"

using namespace std;
using namespace std::chrono;
using namespace ante;
using namespace ante::parser;

/**
 * @brief Prints the parse tree annotated with names and types
 *
 * @param root The RootNode of the parse tree to print out
 */
void showParseTree(RootNode *root, string const& fileName){
    // Must annotate parse tree with name/type information first
    try{
        NameResolutionVisitor v{fileName};
        v.visit(root);
        if(errorCount()) return;
        TypeInferenceVisitor::infer(root, v.compUnit);
    }catch(...){
        /* User should already be notified if an error occurred */
    }
    parser::printBlock(root, 0);
}

/**
 * @brief Outputs the help message explaining command line options.
 */
void printHelp(){
    puts("Compiler for the Ante programming language\n");
    puts("Usage: ante [options] <inputs>");
    puts("options:");
    puts("\t-c\t\tcompile to object file");
    puts("\t-o <filename>\tspecify output name");
    puts("\t-p\t\tprint parse tree");
    puts("\t-O <number>\tSet optimization level. Arg of 0 = none, 3 = all");
    puts("\t-r\t\tcompile and run");
    puts("\t-help\t\tprint this message");
    puts("\t-lib\t\tcompile as library (include all functions in binary and compile to object file)");
    puts("\t-emit-llvm\tprint llvm-IR as output");
    puts("\t-check\t\tCheck program for errors without compiling");
    puts("\t-no-color\tprint uncolored output");

    puts("\nNative target: " AN_TARGET_TRIPLE);

    llvm::raw_os_ostream os{std::cout};
    llvm::TargetRegistry::printRegisteredTargetsForVersion(os);
}

namespace ante {
    extern AnTypeContainer typeArena;
}

#ifndef NO_MAIN
int main(int argc, const char **argv){
    auto start = high_resolution_clock::now();

    LLVMInitializeNativeTarget();
    LLVMInitializeNativeAsmPrinter();

    capi::init();

    auto *args = parseArgs(argc, argv);
    if(args->hasArg(Args::Help)) printHelp();
    if(args->hasArg(Args::NoColor)) colored_output = false;

    for(auto input : args->inputFiles){
        Compiler ante{input.c_str()};
        if(args->hasArg(Args::Parse)){
            showParseTree(ante.getAST(), ante.getModuleName());
        }
        ante.processArgs(args);
    }
    if(args->hasArg(Args::Eval) || (args->args.empty() && args->inputFiles.empty()))
        Compiler(0).eval();
    if(yylexer)
        delete yylexer;
    //delete args;

    auto end = high_resolution_clock::now();
    cout << "Total: " << duration_cast<milliseconds>(end - start).count() << "ms\n";
    return 0;
}
#endif
