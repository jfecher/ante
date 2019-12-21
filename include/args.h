#ifndef ARGS_H
#define ARGS_H

#include <vector>
#include <string>
#include <memory>

namespace ante {
    /**
     * Enum of each command-line flag that can be passed to ante.
     * For detail on each option, see the output of $ ante -help
     */
    enum Args {
        Check,
        CompileAndRun,
        CompileToObj,
        EmitLLVM,
        Eval,
        Help,
        Lib,
        NoColor,
        OptLvl,
        OutputName,
        Parse,
        Time
    };

    struct Argument {
        Args argTy;
        std::string arg;

        Argument(Args a, std::string &s) : argTy(a), arg(s){}
    };

    struct CompilerArgs {
        std::vector<Argument> args;
        std::vector<std::string> inputFiles;

        void addArg(Args &&a, std::string &&s);
        bool hasArg(Args a) const;
        const Argument* getArg(Args a) const;
        bool empty() const;
    };

    CompilerArgs* parseArgs(int argc, const char** argv);
}

#endif
