#include "module.h"
#include <algorithm>
#include <cctype>

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

    void ModulePath::removeTrailingFileType(){
        if(substr.length() >= 3 && substr.compare(substr.length() - 3, 3, ".an") == 0){
            substr = substr.substr(0, substr.length() - 3);
        }
    }

    ModulePath::ModulePath(std::string const& s)
        : s{s}, prev{0}, cur{0} {
            this->operator++();
        }

    ModulePath::iterator ModulePath::begin() const {
        return *this;
    }

    ModulePath::iterator ModulePath::end() const {
        ModulePath e{*this};
        e.cur = s.length();
        e.prev = s.length();
        return e;
    }

    constexpr bool isPathSeparator(char c){
        return c == '/' || c == '\\';
    }

    ModulePath::iterator& ModulePath::operator++()    /* prefix */          {
        prev = cur;
        while(cur < s.length() && !isPathSeparator(s[cur]))
            ++cur;
        substr = s.substr(prev, cur - prev);
        if(!substr.empty())
            substr[0] = std::toupper(substr[0]);
        while(isPathSeparator(s[cur]))
            ++cur;
        removeTrailingFileType();
        if(substr == ".")
            this->operator++();
        return *this;
    }

    ModulePath::reference ModulePath::operator* () const {
        return substr;
    }

    bool ModulePath::operator!=(const iterator& rhs) const {
        return s != rhs.s || cur != rhs.cur || prev != rhs.prev;
    }

    AnType* Module::lookupType(std::string const& name) const {
        TypeDecl *typeDecl = lookupTypeDecl(name);
        return typeDecl == nullptr ? nullptr : typeDecl->type;
    }

    TypeDecl* Module::lookupTypeDecl(std::string const& name) const {
        auto it = userTypes.find(name);
        if(it != userTypes.end())
            return (TypeDecl*)&it->second;

        for(auto &module : imports){
            it = module->userTypes.find(name);
            if(it != module->userTypes.end())
                return (TypeDecl*)&it->second;
        }
        return nullptr;
    }
}
