#include "module.h"
#include <algorithm>

namespace ante {
    Module rootModule{""};

    Module& Module::getRoot(){
        return rootModule;
    }

    llvm::StringMap<Module>::iterator Module::findChild(std::string const& name) {
        return children.find(name);
    }

    llvm::StringMap<Module>::iterator Module::childrenEnd() {
        return children.end();
    }

    Module& Module::addChild(std::string const& childName){
        children.try_emplace(childName, childName);
        return children.find(childName)->second;
    }
}
