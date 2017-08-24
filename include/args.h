#ifndef ARGS_H
#define ARGS_H

#include <vector>
#include <string>
#include <memory>

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
        std::string arg;

        Argument(Args a, std::string &s) : argTy(a), arg(s){}
    };

    struct CompilerArgs {
        std::vector<std::unique_ptr<Argument>> args;
        std::vector<std::string> inputFiles;

        void addArg(Argument *a);
        bool hasArg(Args a) const;
        Argument* getArg(Args a) const;
        bool empty() const;
    };

    CompilerArgs* parseArgs(int argc, const char** argv);
}

#endif
