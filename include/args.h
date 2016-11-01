#ifndef ARGS_H
#define ARGS_H

#include <vector>
#include <string>
using namespace std;

namespace ante {
    enum Args {
        OptLvl,
        OutputName,
        Eval,
        Parse,
        CompileAs,
        CompileAndRun,
        Help,
        Lib,
        EmitLLVM
    };

    struct Argument {
        Args argTy;
        string arg;

        Argument(Args a, string &s) : argTy(a), arg(s){}
    };

    struct CompilerArgs {
        vector<Argument*> args;
        vector<string> inputFiles;

        void addArg(Argument *a);
        bool hasArg(Args a);
    };

    CompilerArgs* parseArgs(int argc, const char** argv);
}

#endif
