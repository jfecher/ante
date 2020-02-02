#include <algorithm>
#include <cctype>
#include "antype.h"
#include "module.h"
#include "antype.h"
#include "trait.h"
#include "unification.h"
#include "util.h"

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
        if(!typeDecl) return nullptr;

        auto pt = try_cast<AnProductType>(typeDecl->type);
        if(pt && pt->isAlias){
            return pt->getAliasedType();
        }else{
            return typeDecl->type;
        }
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

    /** Lookup the given Trait* and return it if found, null otherwise */
    TraitDecl* Module::lookupTraitDecl(std::string const& name) const {
        auto it = traitDecls.find(name);
        if(it != traitDecls.end()){
            return it->getValue();
        }
        for(Module *module : this->imports){
            auto it = module->traitDecls.find(name);
            if(it != module->traitDecls.end()){
                return it->getValue();
            }
        }
        return nullptr;
    }

    /** Lookup the given TraitInstance* and return it if found, null otherwise */
    TraitImpl* Module::lookupTraitImpl(std::string const& name, TypeArgs const& typeArgs) const {
        auto it = traitImpls.find(name);
        if(it != traitImpls.end()){
            for(auto *impl : it->getValue()){
                if(allApproxEq(impl->typeArgs, typeArgs)){
                    return impl;
                }
            }
        }
        for(Module *import : this->imports){
            auto it = import->traitImpls.find(name);
            if(it != import->traitImpls.end()){
                for(auto *impl : it->getValue()){
                    if(allApproxEq(impl->typeArgs, typeArgs)){
                        return impl;
                    }
                }
            }
        }
        return nullptr;
    }

    /** Lookup the TraitDecl and return a new, unimplemented instance of it */
    TraitImpl* Module::freshTraitImpl(std::string const& traitName) const {
        TraitDecl *decl = Module::lookupTraitDecl(traitName);
        if(!decl){
            yy::location loc;
            error("Could not find trait " + lazy_str(traitName, AN_TYPE_COLOR) + " in module " + this->name, loc);
        }
        auto typeArgs = ante::applyToAll(decl->typeArgs, [](AnType *a) -> AnType* {
            return nextTypeVar();
        });
        return new TraitImpl(decl, typeArgs);
    }

    /** Create a TraitImpl with the same type args as its TraitDecl */
    TraitImpl* Module::createTraitImplFromDecl(std::string const& traitName) const {
        TraitDecl *decl = lookupTraitDecl(traitName);
        if(!decl){
            yy::location loc;
            error("Could not find trait " + lazy_str(traitName, AN_TYPE_COLOR) + " in module " + this->name, loc);
        }
        return new TraitImpl(decl, decl->typeArgs);
    }

    bool TraitImpl::hasTrivialImpl() const {
        if(name == "Add" || name == "Sub" || name == "Mul" || name == "Div" || name == "Mod" || name == "Cmp" || name == "Neg"){
            return typeArgs[0]->isIntTy() || typeArgs[0]->typeTag == TT_C8;

        }else if(name == "Cast"){
            AnType *arg1 = typeArgs[0];
            AnType *arg2 = typeArgs[1];
            return (arg1->isIntTy() || arg1->typeTag == TT_Ptr || arg1->typeTag == TT_C8 || arg1->typeTag == TT_Bool) &&
                (arg2->isIntTy() || arg2->typeTag == TT_Ptr || arg2->typeTag == TT_C8 || arg2->typeTag == TT_Bool);

        }else if(name == "Eq" || name == "Is"){
            TypeTag tag = typeArgs[0]->typeTag;
            return typeArgs[0]->isIntTy()
                || tag == TT_C8 || tag == TT_Bool;

        }else if(name == "Extract"){
            return typeArgs[0]->typeTag == TT_Ptr
                && typeArgs[1]->typeTag == TT_Usz;

        }else if(name == "Inamesert"){
            auto ptrty = try_cast<AnPtrType>(typeArgs[0]);
            return ptrty
                && typeArgs[1]->typeTag == TT_Usz
                && *typeArgs[2] == *ptrty->extTy;

        }else if(name == "Deref"){
            return typeArgs[0]->typeTag == TT_Ptr;

        }else if(name == "Not"){
            return typeArgs[0]->typeTag == TT_Bool;
        }else{
            return false;
        }
    }
}