#include "moduletree.h"
#include <algorithm>

namespace ante {
    ModuleTree rootModuleTree{""};

    ModuleTree& ModuleTree::getRoot(){
        return rootModuleTree;
    }

    std::string const& ModuleTree::getName() const {
        return name;
    }

    Module* ModuleTree::getModule() const {
        return module.get();
    }

    void ModuleTree::setModule(Module *module){
        this->module.reset(module);
    }

    llvm::StringMap<ModuleTree>::iterator ModuleTree::findChild(std::string const& name) {
        return children.find(name);
    }

    template<class StringIt>
    llvm::StringMap<ModuleTree>::iterator ModuleTree::findPath(StringIt path) {
        ModuleTree *node = this;
        llvm::StringMap<ModuleTree>::iterator ret = children.end();

        for(std::string const& name : path){
            ret = node->findChild(name);
            if(ret == node->childrenEnd()){
                return children.end();
            }
        }
        return ret;
    }

    llvm::StringMap<ModuleTree>::iterator ModuleTree::childrenEnd() {
        return children.end();
    }

    ModuleTree& ModuleTree::addChild(std::string const& childName){
        children.try_emplace(childName, childName);
        return children.find(childName)->second;
    }

    template<class StringIt>
    ModuleTree& ModuleTree::addPath(StringIt path){
        ModuleTree *node = this;
        for(std::string const& name : path){
            auto child = node->findChild(name);
            if(child == node->childrenEnd()){
                node = &node->addChild(name);
            }else{
                node = &child->getValue();
            }
        }
        return *node;
    }
}
