#include "args.h"
#include <map>
#include <iostream>
using namespace ante;

map<string, Args> argsMap = {
    {"-O",         Args::OptLvl},
    {"-o",         Args::OutputName},
    {"-e",         Args::Eval},
    {"-p",         Args::Parse},
    {"-c",         Args::CompileToObj},
    {"-r",         Args::CompileAndRun},
    {"-help",      Args::Help},
    {"-lib",       Args::Lib},
    {"-emit-llvm", Args::EmitLLVM}
};

void CompilerArgs::addArg(Argument *a){
    args.push_back(a);
}
    
bool CompilerArgs::hasArg(Args a) const{
    for(auto &arg : args)
        if(arg->argTy == a)
            return true;
    
    return false;
}

ante::Argument* CompilerArgs::getArg(Args a) const{
    for(auto &arg : args)
        if(arg->argTy == a)
            return arg;
    
    return 0;
}

//returns true if there are no -<option> arguments.  Ignores filenames
bool CompilerArgs::empty() const{
    return args.empty();
}


enum ArgTy { None, Str, Int };

ArgTy requiresArg(Args a){
    if(a == OutputName)
        return ArgTy::Str;
     
    if(a == OptLvl)
        return ArgTy::Int;
    
    return ArgTy::None;
}

string argTyToStr(ArgTy ty){
    if(ty == ArgTy::Str) return "string";
    if(ty == ArgTy::Int) return "integer";
    return "none";
}


CompilerArgs* ante::parseArgs(int argc, const char** argv){
    CompilerArgs* ret = new CompilerArgs();

    for(int i = 1; i < argc; i++){
        if(argv[i][0] == '-'){
            try{
                Args a = argsMap.at(argv[i]);
                string s = "";

                //check to see if this argument requires an addition arg, eg -c <filename>
                ArgTy ty;
                if((ty = requiresArg(a)) != ArgTy::None){
                    if(i + 1 < argc and argv[i+1][0] != '-'){
                        s = argv[++i];
                    }else{
                        cerr << "Argument '" << argv[i] << "' requires a " << argTyToStr(ty) << " parameter.\n";
                        exit(1);
                    }
                }

                ret->addArg(new Argument(a, s));
            }catch(out_of_range r){
                cerr << "Ante: argument '" << argv[i] << "' was not recognized.\n";
                exit(1);
            }

        //if it is not an option denoted by '-' it is an input file
        //options requiring their own arguments are already taken care of
        }else{
            ret->inputFiles.push_back(argv[i]);
        }
    }
    return ret;
}

