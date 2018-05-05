#include "jit.h"
#include "compapi.h"
#include <iostream>

using std::string;
using std::unique_ptr;
using llvm::JITSymbol;
using llvm::JITSymbolFlags;
using llvm::JITTargetAddress;
using llvm::RTDyldMemoryManager;

namespace ante {
    JIT::ModuleHandle JIT::addModule(std::unique_ptr<llvm::Module> m){
        auto symResolver = llvm::orc::createLambdaResolver(
            //Look back into the JIT itself to find symbols part of the same dylib
            [&](const string &name){
                if(auto sym = codLayer.findSymbol(name, false))
                    return sym;
                return JITSymbol(nullptr);
            },
            //search for external symbols in the host process
            [](const string &name){
                if(auto symAddr = RTDyldMemoryManager::getSymbolAddressInProcess(name))
                    return JITSymbol(symAddr, JITSymbolFlags::Exported);

                if(capi::CtFunc *fn = capi::lookup(name)){
                    uint64_t addr = (uint64_t)fn->fn;
                    return JITSymbol(addr, JITSymbolFlags::Exported);
                }

                return JITSymbol(nullptr);
            }
        );

        return cantFail(codLayer.addModule(move(m), move(symResolver)));
    }

    std::shared_ptr<llvm::Module> JIT::optimizeModule(std::shared_ptr<llvm::Module> m){
        auto fpm = llvm::make_unique<llvm::legacy::FunctionPassManager>(m.get());

        fpm->add(llvm::createInstructionCombiningPass());
        fpm->add(llvm::createReassociatePass());
        fpm->add(llvm::createGVNPass());
        fpm->add(llvm::createCFGSimplificationPass());
        fpm->doInitialization();

        for(auto &f : *m){
            fpm->run(f);
        }
        return m;
    }

    JITSymbol JIT::findSymbol(const string name){
        string mangledName;
        llvm::raw_string_ostream mangledNameStream(mangledName);
        llvm::Mangler::getNameWithPrefix(mangledNameStream, name, dl);
        return codLayer.findSymbol(mangledNameStream.str(), true);
    }

    JITTargetAddress JIT::getSymbolAddress(const string name){
        return cantFail(findSymbol(name).getAddress());
    }

    void JIT::removeModule(ModuleHandle h){
        cantFail(codLayer.removeModule(h));
    }

    void JIT::handleUnrecognizedFn(){
        std::cerr << "JIT Error: Unrecognized function called, aborting!" << std::endl;
    }
}
