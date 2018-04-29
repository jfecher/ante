#include "jit.h"
#include "compapi.h"
#include <iostream>

using namespace std;
using namespace llvm;
using namespace llvm::orc;

extern "C" void* Ante_store(ante::Compiler*, ante::TypedValue&, ante::TypedValue&);

namespace ante {
    JIT::ModuleHandle JIT::addModule(std::unique_ptr<llvm::Module> m){
        auto symResolver = createLambdaResolver(
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

                if(CtFunc *fn = compapi_lookup(name)){
                    cout << "Calling addr:          " << fn << endl;
                    cout << "Note: Ante_store addr: " << (void*)Ante_store << endl;
                    uint64_t addr = (uint64_t)fn;
                    return JITSymbol(addr, JITSymbolFlags::Exported);
                }

                return JITSymbol(nullptr);
            }
        );

        return cantFail(codLayer.addModule(move(m), move(symResolver)));
    }

    std::shared_ptr<llvm::Module> JIT::optimizeModule(std::shared_ptr<llvm::Module> m){
        auto fpm = llvm::make_unique<legacy::FunctionPassManager>(m.get());

        fpm->add(createInstructionCombiningPass());
        fpm->add(createReassociatePass());
        fpm->add(createGVNPass());
        fpm->add(createCFGSimplificationPass());
        fpm->doInitialization();

        for(auto &f : *m){
            fpm->run(f);
        }
        return m;
    }

    JITSymbol JIT::findSymbol(const string name){
        string mangledName;
        raw_string_ostream mangledNameStream(mangledName);
        Mangler::getNameWithPrefix(mangledNameStream, name, dl);
        return codLayer.findSymbol(mangledNameStream.str(), true);
    }

    JITTargetAddress JIT::getSymbolAddress(const string name){
        return cantFail(findSymbol(name).getAddress());
    }

    void JIT::removeModule(ModuleHandle h){
        cantFail(codLayer.removeModule(h));
    }

    void JIT::handleUnrecognizedFn(){
        cerr << "JIT Error: Unrecognized function called, aborting!" << endl;
    }
}
