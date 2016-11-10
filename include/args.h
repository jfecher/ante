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
        CompileToObj,
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
        vector<ante::Argument*> args;
        vector<string> inputFiles;

        void addArg(ante::Argument *a);
        bool hasArg(ante::Args a) const;
        ante::Argument* getArg(Args a) const;
        bool empty() const;
    };

    CompilerArgs* parseArgs(int argc, const char** argv);
}

#endif
