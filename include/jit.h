#ifndef AN_JIT_H
#define AN_JIT_H

#include "llvm/ADT/STLExtras.h"
#include "llvm/ExecutionEngine/ExecutionEngine.h"
#include "llvm/ExecutionEngine/JITSymbol.h"
#include "llvm/ExecutionEngine/RTDyldMemoryManager.h"
#include "llvm/ExecutionEngine/SectionMemoryManager.h"
#include "llvm/ExecutionEngine/Orc/CompileOnDemandLayer.h"
#include "llvm/ExecutionEngine/Orc/CompileUtils.h"
#include "llvm/ExecutionEngine/Orc/IRCompileLayer.h"
#include "llvm/ExecutionEngine/Orc/LambdaResolver.h"
#include "llvm/ExecutionEngine/Orc/RTDyldObjectLinkingLayer.h"
#include "llvm/ExecutionEngine/Orc/IRTransformLayer.h"
#include "llvm/IR/DataLayout.h"
#include "llvm/IR/Mangler.h"
#include "llvm/Support/DynamicLibrary.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Target/TargetMachine.h"
#include "llvm/Transforms/Scalar.h"
#include "llvm/Transforms/Scalar/GVN.h"
#include <algorithm>
#include <memory>
#include <string>
#include <vector>

namespace ante {
    
    class JIT {
        private:
            std::unique_ptr<llvm::TargetMachine> tm;
            const llvm::DataLayout dl;
            llvm::orc::RTDyldObjectLinkingLayer objectLayer;
            llvm::orc::IRCompileLayer<decltype(objectLayer), llvm::orc::SimpleCompiler> compileLayer;

            using OptimizeFunction =
                std::function<std::shared_ptr<llvm::Module>(std::shared_ptr<llvm::Module>)>;

            llvm::orc::IRTransformLayer<decltype(compileLayer), OptimizeFunction> optimizeLayer;

            std::unique_ptr<llvm::orc::JITCompileCallbackManager> compileCallbackManager;
            llvm::orc::CompileOnDemandLayer<decltype(optimizeLayer)> codLayer;

            std::shared_ptr<llvm::Module> optimizeModule(std::shared_ptr<llvm::Module> m);

        public:
            using ModuleHandle = decltype(codLayer)::ModuleHandleT;

            JIT() : tm(llvm::EngineBuilder().selectTarget()), dl(tm->createDataLayout()),
                    objectLayer([](){ return std::make_shared<llvm::SectionMemoryManager>(); }),
                    compileLayer(objectLayer, llvm::orc::SimpleCompiler(*tm)),
                    optimizeLayer(compileLayer, [this](std::shared_ptr<llvm::Module> m){
                                return optimizeModule(std::move(m));
                    }),
                    compileCallbackManager(
                            llvm::orc::createLocalCompileCallbackManager(tm->getTargetTriple(),
                                (llvm::JITTargetAddress)&handleUnrecognizedFn)),
                    codLayer(optimizeLayer, [this](llvm::Function &f){
                                //Appease the "'this' parameter not used" warning
                                this->doNothing();
                                return std::set<llvm::Function*>({&f});
                            },
                            *compileCallbackManager,
                            llvm::orc::createLocalIndirectStubsManagerBuilder(tm->getTargetTriple())){
                        
                        //pass a nullptr to load the current process
                        llvm::sys::DynamicLibrary::LoadLibraryPermanently(nullptr);
                    }

            void doNothing() const {}

            static void handleUnrecognizedFn();

            llvm::TargetMachine& getTargetMachine() { return *tm; }

            JIT::ModuleHandle addModule(std::unique_ptr<llvm::Module> m);

            llvm::JITSymbol findSymbol(const std::string name);

            llvm::JITTargetAddress getSymbolAddress(const std::string name);

            void removeModule(JIT::ModuleHandle h);
    };
}

#endif
