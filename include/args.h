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
        Check,
        CompileToObj,
        CompileAndRun,
        Help,
        Lib,
        EmitLLVM,
        NoColor
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
        bool hasArg(Args a) const;
        Argument* getArg(Args a) const;
        bool empty() const;
    };

    CompilerArgs* parseArgs(int argc, const char** argv);
}

#endif
